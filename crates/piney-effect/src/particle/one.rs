//! `ccParticle` (main particle.cpp, 0xb0 bytes): one particle, set up by
//! `Setup` (0x001bf0f0: the object by style - a `ccEff`, `ccClump` or `ccAnm`
//! with its CLUT swapped in, fog off - and one `rand()` for a random pattern)
//! and run by `Main` (0x001bf8d0) once a frame: the force fields, the move,
//! the draw at `ccTransPosFW2LW`, the age and the fades (the colour 0-2048
//! times the transparency). The rules are in docs/engine/particles.md
//! ("ccParticle").

use piney_world::pose::Play;

use super::tables::Tables;
use super::{GenRef, Generator, force};
use crate::draw::DrawRec;
use crate::ee::{self, F, ONE, V4};
use crate::eff::Eff;
use crate::effect::{EFFECT_LAYER, check_camera_deg};
use crate::files::{Assets, ObjRef};
use crate::vu::{self, M4};
use crate::{Host, space};

/// What a particle draws (`pChar`).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Obj {
    #[default]
    None,
    /// Style 0: a `ccEff`.
    Eff(Box<Eff>),
    /// Style 1: a `ccClump` and the matrix its coordinate was last set to.
    Clump { obj: ObjRef, matrix: M4 },
    /// Style 2: a `ccAnm`, its playback and matrix.
    Anm { obj: ObjRef, play: Play, matrix: M4 },
}

/// A `ccParticle`.
#[derive(Clone, Debug, PartialEq)]
pub struct Particle {
    /// +0x00 bits: `style` (0-2, signed 3 bits), `distSW`, `dispSW`,
    /// `endFlag`, `killFlag`, `syncFlag`, `fadeFlag` (signed 4 bits: 0
    /// fading in, 1 shown, 2 fading out, 3 faded), `anmType` (signed 4),
    /// `strFlag`.
    pub style: i8,
    pub dist_sw: bool,
    pub disp_sw: bool,
    pub end_flag: bool,
    pub kill_flag: bool,
    pub sync_flag: bool,
    pub fade_flag: i8,
    pub anm_type: i8,
    pub str_flag: u8,
    /// +0x04 `gene`: the generator by serial number.
    pub gene: Option<GenRef>,
    /// +0x08 `pChar`.
    pub obj: Obj,
    /// The CLUT `Setup` swapped in: `particleCcsAdrs[texID]`, and (for a
    /// clump or anm, `ChangeClut`'s second argument) the one it replaces.
    pub clut: Option<(ObjRef, Option<ObjRef>)>,
    /// +0x0c `layer` (a priority; None the effect layer).
    pub layer: Option<i16>,
    /// +0x10 `sn`, +0x14 `anmPat`, +0x16 `texID` (-1: the slot is free).
    pub sn: u32,
    pub anm_pat: u16,
    pub tex_id: i16,
    /// +0x20 pos, +0x30 rot, +0x40 offset, +0x50 dirc, +0x60 scale, +0x70
    /// velocity.
    pub pos: V4,
    pub rot: V4,
    pub offset: V4,
    pub dirc: V4,
    pub scale: V4,
    pub velocity: V4,
    /// +0x80 transparency, +0x84 speed, +0x88 size, +0x8c temp.
    pub transparency: F,
    pub speed: F,
    pub size: F,
    pub temp: F,
    /// +0x90 color (2048 opaque), +0x92 fadeInD, +0x94 fadeOutD, +0x96
    /// lifeTime (-1 fading out), +0x98 age, +0x9a cnt.
    pub color: i16,
    pub fade_in_d: i16,
    pub fade_out_d: i16,
    pub life_time: i16,
    pub age: i16,
    pub cnt: i16,
    /// +0x9c `rotate[4]` (65536 a turn): [1] the force fields' spin,
    /// [3] a sprite's turn.
    pub rotate: [u16; 4],
    /// +0xa4 `ofstD`, +0xa6 `ofstR`: percentages along and out from the
    /// generator's line (rType 5 synced, force 16).
    pub ofst_d: i16,
    pub ofst_r: i16,
}

impl Default for Particle {
    /// A free slot as `InitParticleCtrl` leaves the zeroed array.
    fn default() -> Self {
        Particle {
            style: 0,
            dist_sw: false,
            disp_sw: false,
            end_flag: false,
            kill_flag: false,
            sync_flag: false,
            fade_flag: 0,
            anm_type: 0,
            str_flag: 0,
            gene: None,
            obj: Obj::None,
            clut: None,
            layer: None,
            sn: 0,
            anm_pat: 0,
            tex_id: -1,
            pos: [0; 4],
            rot: [0; 4],
            offset: [0; 4],
            dirc: [0; 4],
            scale: [0; 4],
            velocity: [0; 4],
            transparency: 0,
            speed: 0,
            size: 0,
            temp: 0,
            color: 0,
            fade_in_d: 0,
            fade_out_d: 0,
            life_time: 0,
            age: 0,
            cnt: 0,
            rotate: [0; 4],
            ofst_d: 0,
            ofst_r: 0,
        }
    }
}

/// 2048.0: `color`'s one.
const COLOR_ONE: F = 0x4500_0000;
/// The `distSW` fade: 7000, 5000, 2000.
const FADE_FAR: F = 0x45da_c000;
const FADE_ZERO: F = 0x459c_4000;
const FADE_LEN: F = 0x44fa_0000;
/// `ccCheckCameraDeg`'s cone.
const VIEW_CONE: i16 = 12288;

/// `ccParticle::Setup(gene, pTex, patNum, distCheck)` (main 0x001bf0f0).
#[allow(clippy::too_many_arguments)]
pub fn setup(
    p: &mut Particle,
    gene: Option<&Generator>,
    mut tex: i32,
    pat_num: i32,
    dist_check: bool,
    host: &mut dyn Host,
    assets: &Assets,
    serial: &mut u32,
) {
    let t: &Tables = &assets.particle;
    p.gene = gene.map(|g| g.sn);
    p.disp_sw = true;
    p.dist_sw = dist_check;
    p.str_flag = 0;
    match gene {
        Some(g) => {
            p.kill_flag = g.kill_flag != 0;
            p.sync_flag = g.p_sync;
        }
        None => {
            p.kill_flag = false;
            p.sync_flag = false;
        }
    }
    let (mut base, mut clut) = t.tex_base(tex);
    if pat_num != 0 {
        let r = host.rand() % pat_num;
        let row = t.part(base);
        if row.style != 0 || row.anm == 0 || row.anm == 1 {
            tex += r;
        }
        (base, clut) = t.tex_base(tex);
    }
    p.anm_pat = 0;
    p.tex_id = tex as i16;
    let row = t.part(base);
    // A 4-bit style into the signed 3-bit field.
    p.style = (((row.style & 7) << 5) as i8) >> 5;
    p.end_flag = false;
    p.fade_flag = 0;
    p.anm_type = ((row.anm << 4) as i8) >> 4;
    p.clut = None;
    let obj = t.adrs(base);
    let swap = if clut != 0 { t.adrs(tex) } else { None };
    match p.style {
        0 => {
            p.obj = match obj.and_then(|o| assets.eff_chunk(o).map(|(j, c)| (o, j, c))) {
                Some((o, j, c)) => {
                    let mut e = Eff::init(o.file, j, c, true, &assets.alpha_blend);
                    e.prim &= !0x20;
                    Obj::Eff(Box::new(e))
                }
                None => Obj::None,
            };
            if clut != 0 {
                p.clut = swap.map(|s| (s, None));
            }
        }
        1 => {
            p.obj = obj.map_or(Obj::None, |obj| Obj::Clump { obj, matrix: [[0; 4]; 4] });
            if clut != 0 {
                p.clut = swap.map(|s| (s, t.adrs(clut)));
            }
        }
        2 => {
            p.obj = match obj {
                Some(o) => match assets.files[o.file].anim(assets.name(o)) {
                    Some(anim) => {
                        Obj::Anm { obj: o, play: Play { anim, time: 0, posed: 0, frame_spd: 256 }, matrix: [[0; 4]; 4] }
                    }
                    None => Obj::None,
                },
                None => Obj::None,
            };
            if clut != 0 {
                p.clut = swap.map(|s| (s, t.adrs(clut)));
            }
        }
        _ => {}
    }
    if row.pat != 0 && (row.anm == 3 || (row.style == 0 && row.anm != 0)) {
        let n = match &p.obj {
            Obj::Eff(e) => i32::from(e.pat_num),
            _ => 0,
        };
        let r = host.rand();
        p.anm_pat = if n == 0 { 0 } else { (r % n) as u16 };
    }
    p.sn = *serial;
    *serial = serial.wrapping_add(1);
    p.dirc = [0; 4];
    p.rot = [0; 4];
    p.offset = [0; 4];
    p.velocity = [0; 4];
    p.scale = [ONE; 4];
    p.transparency = ONE;
    p.speed = 0;
    p.size = ONE;
    p.color = 0;
    (p.fade_in_d, p.fade_out_d) = match gene {
        Some(g) => g.param.map_or((512, 256), |p| (p.p_fade_in, p.p_fade_out)),
        None => (512, 256),
    };
    p.life_time = 1;
    p.age = 0;
    p.cnt = 0;
    p.rotate = [0; 4];
    p.ofst_d = 0;
    p.ofst_r = 0;
    p.layer = None;
    if p.style == 0
        && let Obj::Eff(e) = &mut p.obj
    {
        e.transparency = ee::div(ee::from_int(i32::from(p.color)), COLOR_ONE);
    }
}

/// What `Main` reads of the world.
pub struct View<'a> {
    pub host: &'a mut dyn Host,
    pub assets: &'a Assets,
    pub draws: &'a mut Vec<DrawRec>,
}

/// `cameraGetPos` / `ccCheckCameraDeg` / the fade, as each object's draw
/// does them: dispSW, and the alpha to draw with (None: not drawn).
fn shown(p: &mut Particle, at: V4, view: &View) -> Option<F> {
    let cam = view.host.camera();
    let player = view.host.player_pos();
    let bounds = view.host.bounds();
    p.disp_sw = check_camera_deg(at, player, bounds, cam.cam_pos, cam.cam_view, VIEW_CONE);
    let base = ee::mul(p.transparency, ee::div(ee::from_int(i32::from(p.color)), COLOR_ONE));
    if p.dist_sw {
        let d = ee::vsub(cam.eye, at);
        let d = ee::sqrtf(ee::dot(d, d));
        if !ee::lt(d, FADE_FAR) {
            p.disp_sw = false;
            return None;
        }
        let mut f = ee::div(ee::sub(FADE_ZERO, d), FADE_LEN);
        if ee::lt(f, 0) {
            f = 0;
        }
        if !ee::le(f, ONE) {
            f = ONE;
        }
        if p.disp_sw && !ee::eq(f, 0) {
            p.disp_sw = true;
            Some(ee::mul(f, base))
        } else {
            p.disp_sw = false;
            None
        }
    } else if p.disp_sw {
        Some(base)
    } else {
        None
    }
}

/// `ccParticle::Main()` (main 0x001bf8d0) on the effect layer or
/// `layer`; `gene` is the particle's generator, when it still has one.
pub fn main(p: &mut Particle, gene: Option<&Generator>, view: &mut View, layer: Option<i16>) {
    main_in(p, gene, view, layer, false);
}

/// `ccParticle::MainStr()` (main 0x001c0620), the stream demo's second
/// system's: as [`main`] without the field's frame (no `ccTransPosFW2LW`)
/// and without the camera's cone and distance, drawn while `dispSW` is set
/// at its transparency times its colour.
pub fn main_str(p: &mut Particle, gene: Option<&Generator>, view: &mut View, layer: Option<i16>) {
    main_in(p, gene, view, layer, true);
}

fn main_in(p: &mut Particle, gene: Option<&Generator>, view: &mut View, layer: Option<i16>, stream: bool) {
    if p.kill_flag {
        match gene.map(|g| g.kill_flag) {
            Some(2) => {
                if p.fade_flag == 3 {
                    p.age = p.life_time;
                } else {
                    p.fade_flag = 2;
                }
            }
            Some(3) => {
                p.age = p.life_time;
                p.fade_flag = 3;
            }
            _ => {}
        }
    }
    if p.age >= p.life_time && p.fade_flag == 3 {
        p.end_flag = true;
    }
    if p.end_flag {
        p.tex_id = -1;
        p.obj = Obj::None;
        return;
    }
    if let Some(g) = gene {
        for ff in g.ff.iter().flatten() {
            if force::calc(ff, p, Some(g)) {
                p.life_time = -1;
                p.fade_flag = 2;
            }
        }
    }
    p.pos = ee::vadd(p.pos, p.velocity);
    p.pos[3] = ONE;
    let layer = layer.unwrap_or(EFFECT_LAYER);
    let player = view.host.player_pos();
    let bounds = view.host.bounds();
    let mut at = ee::vadd(p.pos, p.offset);
    if p.sync_flag
        && let Some(g) = gene
    {
        at = ee::vadd(at, g.pos);
    }
    let at = if stream { at } else { space::fw2lw(at, player, bounds) };
    let shown = |p: &mut Particle, at: V4, view: &View| {
        if stream {
            let base = ee::mul(p.transparency, ee::div(ee::from_int(i32::from(p.color)), COLOR_ONE));
            p.disp_sw.then_some(base)
        } else {
            shown(p, at, view)
        }
    };
    let mut obj = std::mem::take(&mut p.obj);
    match &mut obj {
        Obj::Eff(e) => {
            e.rotate = ee::deg2rad(p.rotate[3] as i16);
            e.scale_x = p.scale[0];
            e.scale_y = p.scale[1];
            e.pos = at;
            if let Some(alpha) = shown(p, at, view) {
                e.transparency = alpha;
                let mut sent = e.clone();
                sent.clut = p.clut.map(|c| c.0);
                view.draws.push(DrawRec::Eff { eff: sent, pat: p.anm_pat, layer });
            }
        }
        Obj::Clump { obj, matrix } => {
            *matrix = vu::pos_rot_zyx_scale(at, p.rot, p.scale);
            if let Some(alpha) = shown(p, at, view) {
                // ccClump::ChangeClut's swap: the file's own CLUT drawn as
                // the particle's.
                let clut = p.clut.and_then(|(new, old)| old.map(|o| (o.object, new.object)));
                view.draws.push(DrawRec::Clump { obj: *obj, matrix: *matrix, alpha, layer, clut });
            }
        }
        Obj::Anm { obj, play, matrix } => {
            *matrix = vu::pos_rot_zyx_scale(at, p.rot, p.scale);
            if !p.end_flag {
                p.end_flag = play.forward(&view.assets.files[obj.file]);
            }
            if let Some(alpha) = shown(p, at, view) {
                view.draws.push(DrawRec::Anm { obj: *obj, play: play.clone(), matrix: *matrix, alpha, layer });
            }
        }
        Obj::None => {}
    }
    if p.life_time != -1 {
        p.age = p.age.wrapping_add(1);
        if p.age >= p.life_time {
            p.life_time = -1;
            p.fade_flag = 2;
        }
    }
    if let Obj::Eff(e) = &obj
        && !p.end_flag
        && (p.anm_type == 2 || p.anm_type == 3)
    {
        p.anm_pat = p.anm_pat.wrapping_add(1);
        if p.anm_pat >= e.pat_num {
            if p.anm_type == 3 {
                p.anm_pat = 0;
            } else {
                p.end_flag = true;
            }
        }
    }
    p.obj = obj;
    match p.fade_flag {
        0 => {
            p.color = p.color.wrapping_add(p.fade_in_d);
            if p.color >= 2048 {
                p.color = 2048;
                p.fade_flag = 1;
            }
        }
        2 => {
            p.color = p.color.wrapping_sub(p.fade_out_d);
            if p.color <= 0 {
                p.color = 1;
                p.fade_flag = 3;
            }
        }
        _ => {}
    }
    p.cnt = p.cnt.wrapping_add(1);
}
