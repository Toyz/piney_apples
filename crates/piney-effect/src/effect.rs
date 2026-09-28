//! `ccEffect` (main effect.cpp, 0xc0 bytes) and `ccEffectCtrl` (0x2c0): the
//! 500 effect slots `ccThEffect` runs (`ccEffectCtrl::Main` 0x001c38a0, each
//! on its own layer or the effect layer, 20), `InitEffect` (0x001c3af0) and
//! `Main` (0x001c3e10: the first switch on id, the draw, the second chain,
//! then the count and age). The ids' own code lives with each kind of effect
//! ([`crate::arrival`] and the rest); an id not ported only draws and ages.
//! The rules are in docs/engine/effects.md ("ccEffect").

use piney_world::pose::Play;

use crate::draw::DrawRec;
use crate::ee::{self, F, ONE, V4};
use crate::eff::Eff;
use crate::files::{Kind, ObjRef};
use crate::{
    CharRef, Cx, VecRef, ability, arrival, convergence, debris, drain, fall, gimmick, heal, hit, levelup, shockwave,
    skillstart, space, summons, thunder, tornado, upheaval, vu,
};

/// `effWork`: the slots.
pub const SLOTS: usize = 500;
/// `effStrWork` (main 0x003fc930): the stream demo's `ccEffectCtrl(1)`'s.
pub const SLOTS_STR: usize = 50;
/// `WORLD_MAN::SetActiveLayer(3)`: +0x4c8, priority 20.
pub const EFFECT_LAYER: i16 = 20;
/// `oneVector` (main 0x0033f230): (1, 1, 1, 0).
pub const ONE_VECTOR: V4 = [ONE, ONE, ONE, 0];

/// Distances of the `distSW` fade: `Main`'s f20 (7000), f21 (2000), and
/// f22 = f20 - f21.
const FADE_FAR: F = 0x45da_c000;
const FADE_LEN: F = 0x44fa_0000;
/// `ccCheckCameraDeg`'s cone: 67.5 degrees either side.
const VIEW_CONE: i16 = 12288;

/// `effPtr`: what an effect draws.
#[derive(Clone, Debug, PartialEq)]
pub enum Obj {
    None,
    /// A `ccClump` of a `CMP_` object.
    Clump(ObjRef),
    /// A `ccAnm` of an `ANM_` object, its playback (`ccAnm` +0xac set by
    /// `SetAnm`: it steps) and the matrix its coordinate was last set to.
    Anm {
        obj: ObjRef,
        play: Play,
        matrix: vu::M4,
    },
    /// A `ccEff`.
    Eff(Box<Eff>),
}

/// A `ccEffect`.
#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    /// +0x00 pos, +0x10 offset, +0x20 rot, +0x30 speed, +0x40 scale, +0x50
    /// posT.
    pub pos: V4,
    pub offset: V4,
    pub rot: V4,
    pub speed: V4,
    pub scale: V4,
    pub pos_t: V4,
    /// +0x60 bits: distSW (0), dispSW (1), pauseSW (2), endFlag (3),
    /// zyxFlag (4), level (5-8, signed).
    pub dist_sw: bool,
    pub disp_sw: bool,
    pub pause_sw: bool,
    pub end_flag: bool,
    pub zyx_flag: bool,
    pub level: i8,
    /// +0x64, +0x68.
    pub param: i32,
    pub flags: i32,
    /// +0x6c id, +0x6e status (0 free), +0x70 lifeTime (-1: forever),
    /// +0x72 age, +0x74 cnt, +0x76 texAnmPat.
    pub id: i16,
    pub status: i16,
    pub life_time: i16,
    pub age: i16,
    pub cnt: i16,
    pub tex_anm_pat: u16,
    /// +0x78.
    pub rot_speed: [u16; 4],
    /// +0x80, +0x84.
    pub velocity: F,
    pub transparency: F,
    /// +0x88 targetPtr.
    pub target: Option<CharRef>,
    /// +0x8c effPtr.
    pub obj: Obj,
    /// +0x90 posPtr, +0x94 rotPtr, +0x98 linkPtr (a slot), +0x9c layer.
    pub pos_ptr: Option<VecRef>,
    pub rot_ptr: Option<VecRef>,
    pub link: Option<usize>,
    pub layer: Option<i16>,
    /// +0xa0 `FreeTemp temp[4]`: each id's own use, as bits.
    pub temp: [u32; 4],
    /// +0xb0 `effectSerialNum` when made.
    pub sn: u32,
    /// A clump's CLUT swapped by `ccClump::ChangeClut` (its duplicate's):
    /// (the CLUT object drawn instead of, the one drawn), in the object's
    /// file; for a `ccEff`, (0, the CLUT its `ccTex` draws with: +0x3c).
    pub clut_swap: Option<(u32, u32)>,
}

impl Default for Effect {
    fn default() -> Self {
        Effect {
            pos: [0; 4],
            offset: [0; 4],
            rot: [0; 4],
            speed: [0; 4],
            scale: [0; 4],
            pos_t: [0; 4],
            dist_sw: false,
            disp_sw: false,
            pause_sw: false,
            end_flag: false,
            zyx_flag: false,
            level: 0,
            param: 0,
            flags: 0,
            id: 0,
            status: 0,
            life_time: 0,
            age: 0,
            cnt: 0,
            tex_anm_pat: 0,
            rot_speed: [0; 4],
            velocity: 0,
            transparency: 0,
            target: None,
            obj: Obj::None,
            pos_ptr: None,
            rot_ptr: None,
            link: None,
            layer: None,
            temp: [0; 4],
            sn: 0,
            clut_swap: None,
        }
    }
}

impl Effect {
    /// `FadeIn(fadeInTime, tt)` (main 0x001cc0c0): transparency cnt / time
    /// (or `tt` / time when `tt` is not negative) until it reaches the
    /// time, then 1.
    pub fn fade_in(&mut self, time: i32, tt: i32) {
        let t = if tt < 0 { i32::from(self.cnt) } else { tt };
        self.transparency = if time < t { ONE } else { ee::div(ee::from_int(t), ee::from_int(time)) };
    }

    /// `FadeOut(fadeOutTime, tt)` (0x001cc120): 1 until `tt - time`, then
    /// `(tt - cnt) / time`, not below 0.
    pub fn fade_out(&mut self, time: i32, tt: i32) {
        let cnt = i32::from(self.cnt);
        if cnt < tt - time {
            self.transparency = ONE;
        } else {
            let t = ee::div(ee::from_int(tt - cnt), ee::from_int(time));
            self.transparency = if ee::lt(t, 0) { 0 } else { t };
        }
    }

    /// `FadeInOut(fadeInTime, fadeOutTime, life)` (0x001cc190).
    pub fn fade_in_out(&mut self, fin: i32, fout: i32, life: i32) {
        let cnt = i32::from(self.cnt);
        if fin >= cnt {
            self.transparency = ee::div(ee::from_int(cnt), ee::from_int(fin));
        } else if cnt < life - fout {
            self.transparency = ONE;
        } else {
            let t = ee::div(ee::from_int(life - cnt), ee::from_int(fout));
            self.transparency = if ee::lt(t, 0) { 0 } else { t };
        }
    }
}

/// What a case of `Main`'s first switch does next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// On to the draw and the second chain (`b .L001c724c`).
    Draw,
    /// Straight to the end's counting (`b .L001cc000`).
    End,
}

/// `ccEffectCtrl`: the slots, and `effectSerialNum`.
#[derive(Clone, Debug)]
pub struct EffectCtrl {
    pub effects: Vec<Effect>,
    pub serial: u32,
    /// `ccEffectCtrl(0)` in a town resolves `effectTbl2`'s objects only.
    pub town: bool,
    /// Vectors other effect systems publish for generators to follow
    /// ([`VecRef::Anchor`]): the boss effects' photons ([`crate::boss`]).
    pub anchors: std::collections::HashMap<u32, V4>,
    /// `ccEffectCtrl(1)`, the stream demo's (`effcStr`): `effectStrTbl`'s
    /// objects, `MainStr` ([`crate::strfx`]).
    pub stream: bool,
    /// `active__7ccLayer` through `MainStr`'s pass: set from each effect
    /// that has a layer and never put back per slot, so an effect with none
    /// draws on the last one set (the streams' effect layer at the start).
    active: i16,
}

impl Default for EffectCtrl {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectCtrl {
    /// `ccEffectCtrl(0)` in a town: every slot free.
    pub fn new() -> Self {
        EffectCtrl {
            effects: vec![Effect::default(); SLOTS],
            serial: 0,
            town: true,
            anchors: Default::default(),
            stream: false,
            active: EFFECT_LAYER,
        }
    }

    /// `ccEffectCtrl(1)` (`ccThEffectStr`'s, `effcStr`): 50 slots, every
    /// one free.
    pub fn stream() -> Self {
        EffectCtrl { effects: vec![Effect::default(); SLOTS_STR], town: false, stream: true, ..EffectCtrl::new() }
    }

    /// The first free slot, as every `eff*` function finds it (`status`
    /// 0, scanning from 0).
    pub fn free_slot(&self) -> Option<usize> {
        self.effects.iter().position(|e| e.status == 0)
    }

    /// `ccNewEffect(id)` (main 0x001c3a60) and the `eff*` functions'
    /// search: the first free slot, `InitEffect(id, 0)`.
    pub fn new_effect(&mut self, cx: &Cx, id: i16) -> Option<usize> {
        let i = self.free_slot()?;
        self.init_effect(cx, i, id);
        Some(i)
    }

    /// `ccEffect::InitEffect(id, 0)` (main 0x001c3af0) on slot `i`: offset,
    /// rot and speed zero, scale (1, 1, 1, 1); distSW and dispSW set, the
    /// rest of the flags clear but zyxFlag; status 1, lifeTime -1; opaque;
    /// the next serial number; no target, pointers or layer; then the
    /// object `effectTbl[id].type` says, from `adrs[id]`. `pos` and `posT`
    /// are left as they were.
    pub fn init_effect(&mut self, cx: &Cx, i: usize, id: i16) {
        let sn = self.serial;
        self.serial = self.serial.wrapping_add(1);
        let town = self.town;
        let e = &mut self.effects[i];
        e.offset = [0; 4];
        e.rot = [0; 4];
        e.speed = [0; 4];
        e.scale = ONE_VECTOR;
        e.disp_sw = true;
        e.dist_sw = true;
        e.zyx_flag = true;
        e.pause_sw = false;
        e.param = 0;
        e.level = 0;
        e.id = id;
        e.status = 1;
        e.flags = 0;
        e.life_time = -1;
        e.age = 0;
        e.cnt = 0;
        e.tex_anm_pat = 0;
        e.end_flag = false;
        e.rot_speed = [0; 4];
        e.velocity = 0;
        e.transparency = ONE;
        e.sn = sn;
        e.target = None;
        e.obj = Obj::None;
        e.pos_ptr = None;
        e.rot_ptr = None;
        e.link = None;
        e.layer = None;
        e.temp = [0; 4];
        e.clut_swap = None;
        if id < 0 {
            return;
        }
        let (kind, adrs) = if self.stream {
            (cx.assets.kind_str(id), cx.assets.adrs_str(id))
        } else {
            (cx.assets.kind(id), cx.assets.adrs(id, town))
        };
        let Some(o) = adrs else { return };
        e.obj = match kind {
            Kind::Clump => Obj::Clump(o),
            Kind::Anm => {
                let file = &cx.assets.files[o.file];
                match file.anim(cx.assets.name(o)) {
                    Some(anim) => {
                        Obj::Anm { obj: o, play: Play { anim, time: 0, posed: 0, frame_spd: 256 }, matrix: vu::UNIT }
                    }
                    None => Obj::None,
                }
            }
            k if k.is_eff() => match cx.assets.eff_chunk(o) {
                // new ccEff; Init(chunk, 1); then PRIM's FGE (bit 5) off:
                // no fog after all.
                Some((j, c)) => {
                    let mut eff = Eff::init(o.file, j, c, true, &cx.assets.alpha_blend);
                    eff.prim &= !0x20;
                    Obj::Eff(Box::new(eff))
                }
                None => Obj::None,
            },
            _ => Obj::None,
        };
    }

    /// `ccEffectCtrl::Main` (main 0x001c38a0): every live slot's `Main`, in
    /// slot order (a slot started during the pass runs in it when it is
    /// later in the list).
    pub fn main(&mut self, cx: &mut Cx) {
        // ccEffectCtrl::MainStr (main 0x001c39c0) switches the active layer
        // to each live effect's own before its MainStr.
        self.active = EFFECT_LAYER;
        for i in 0..self.effects.len() {
            if self.effects[i].status != 0 {
                if let Some(l) = self.effects[i].layer {
                    self.active = l;
                }
                self.main_one(cx, i);
            }
        }
    }

    /// The vector a pointer names, now.
    fn resolve(&self, cx: &Cx, r: VecRef) -> V4 {
        match r {
            VecRef::CharPos(c) => cx.host.char_pos(c),
            VecRef::CharDirc(c) => cx.host.char_dirc(c),
            VecRef::EffectPos(k) => self.effects[k].pos,
            VecRef::EffectRot(k) => self.effects[k].rot,
            VecRef::EffectPosT(k) => self.effects[k].pos_t,
            VecRef::CharAt(c, off) => cx.host.char_vec(c, off),
            VecRef::Anchor(k) => self.anchors.get(&k).copied().unwrap_or([0, 0, 0, ONE]),
        }
    }

    /// `ccEffect::Main` (main 0x001c3e10) of slot `i`.
    pub fn main_one(&mut self, cx: &mut Cx, i: usize) {
        if self.effects[i].end_flag {
            let e = &mut self.effects[i];
            e.status = 0;
            e.obj = Obj::None;
            return;
        }
        if let Some(p) = self.effects[i].pos_ptr {
            self.effects[i].pos = self.resolve(cx, p);
        }
        if let Some(r) = self.effects[i].rot_ptr {
            self.effects[i].rot = self.resolve(cx, r);
        }
        if let Some(l) = self.effects[i].link {
            let (p, r) = (self.effects[l].pos, self.effects[l].rot);
            self.effects[i].pos = p;
            self.effects[i].rot = r;
        }
        if self.stream {
            // MainStr: its own first switch, the draw without the field's
            // frame or the camera, its own second chain.
            if crate::strfx::pre(self, cx, i) == Next::Draw {
                self.draw_one_str(cx, i);
                crate::strfx::post(self, cx, i);
            }
        } else if pre(self, cx, i) == Next::Draw {
            self.draw_one(cx, i);
            post(self, cx, i);
        }
        let e = &mut self.effects[i];
        e.cnt = e.cnt.wrapping_add(1);
        if e.life_time != -1 {
            let old = e.age;
            e.age = old.wrapping_add(1);
            if e.life_time < old {
                e.end_flag = true;
            }
        }
    }

    /// `MainStr`'s draw (0x001d8c0c-0x001d8fb4): the anm stepped and drawn,
    /// the eff drawn and its pattern stepped twice, the clump drawn, each at
    /// the effect's own position and transparency (no `ccTransPosFW2LW`, no
    /// camera cone or distance), while `dispSW` is set.
    fn draw_one_str(&mut self, cx: &mut Cx, i: usize) {
        let kind = cx.assets.kind_str(self.effects[i].id);
        let active = self.active;
        let e = &mut self.effects[i];
        let layer = e.layer.unwrap_or(active);
        let matrix = |e: &Effect| {
            if e.zyx_flag {
                vu::pos_rot_zyx_scale(e.pos, e.rot, e.scale)
            } else {
                vu::pos_rot_xyz_scale(e.pos, e.rot, e.scale)
            }
        };
        if !e.pause_sw && matches!(e.obj, Obj::Anm { .. }) {
            let m = vu::pos_rot_zyx_scale(e.pos, e.rot, e.scale);
            let mut ended = false;
            if let Obj::Anm { obj, play, matrix } = &mut e.obj {
                *matrix = m;
                ended = play.forward(&cx.assets.files[obj.file]);
            }
            if ended {
                e.end_flag = true;
            }
        }
        if e.pause_sw {
            return;
        }
        let alpha = e.transparency;
        let (disp, pos, pat, clut) = (e.disp_sw, e.pos, e.tex_anm_pat, e.clut_swap);
        let clump_matrix = matrix(e);
        match &mut e.obj {
            Obj::Anm { obj, play, matrix } if disp => {
                cx.draws.push(DrawRec::Anm { obj: *obj, play: play.clone(), matrix: *matrix, alpha, layer });
            }
            Obj::Eff(eff) => {
                if disp {
                    eff.pos = pos;
                    eff.transparency = alpha;
                    cx.draws.push(DrawRec::Eff { eff: eff.clone(), pat, layer });
                }
                // Drawn or not, the pattern steps, then (not paused) again.
                step_pattern(e, kind);
                step_pattern(e, kind);
            }
            Obj::Clump(obj) if disp => {
                cx.draws.push(DrawRec::Clump { obj: *obj, matrix: clump_matrix, alpha, layer, clut });
            }
            _ => {}
        }
    }

    /// `Main`'s draw (0x001c724c-0x001c77f8), recording what it sends.
    fn draw_one(&mut self, cx: &mut Cx, i: usize) {
        let player = cx.host.player_pos();
        let bounds = cx.host.bounds();
        let cam = cx.host.camera();
        let kind = cx.assets.kind(self.effects[i].id);
        let e = &mut self.effects[i];
        let layer = e.layer.unwrap_or(EFFECT_LAYER);
        let matrix = |pos: V4, e: &Effect| {
            if e.zyx_flag {
                vu::pos_rot_zyx_scale(pos, e.rot, e.scale)
            } else {
                vu::pos_rot_xyz_scale(pos, e.rot, e.scale)
            }
        };
        if !e.pause_sw && matches!(e.obj, Obj::Anm { .. }) {
            e.pos = space::fw2lw(e.pos, player, bounds);
            let m = matrix(e.pos, e);
            let mut ended = false;
            if let Obj::Anm { obj, play, matrix } = &mut e.obj {
                *matrix = m;
                ended = play.forward(&cx.assets.files[obj.file]);
            }
            if ended {
                // id 170 also takes its target off the attribute guard's
                // list (effCheckAttributeGuardEntry, effDelAttributeGuardEntry).
                if e.id == hit::ATTRIBUTE_GUARD
                    && let Some(t) = e.target
                {
                    cx.hits.del_guard(t);
                }
                e.end_flag = true;
            }
        }
        if e.pause_sw {
            return;
        }
        let eye = cam.eye;
        let lw = space::fw2lw(e.pos, player, bounds);
        e.disp_sw = check_camera_deg(e.pos, player, bounds, cam.cam_pos, cam.cam_view, VIEW_CONE);
        let fade = if e.dist_sw {
            let d = ee::vsub(eye, lw);
            let d = ee::sqrtf(ee::dot(d, d));
            if ee::lt(d, FADE_FAR) {
                let mut f = ee::div(ee::sub(ee::sub(FADE_FAR, FADE_LEN), d), FADE_LEN);
                if ee::lt(f, 0) {
                    f = 0;
                }
                if !ee::le(f, ONE) {
                    f = ONE;
                }
                if e.disp_sw && ee::eq(f, 0) {
                    e.disp_sw = false;
                }
                f
            } else {
                e.disp_sw = false;
                // s3[2] is left as the scratch held it; nothing reads it
                // while dispSW is clear.
                0
            }
        } else {
            ONE
        };
        let alpha = ee::mul(e.transparency, fade);
        let (disp, pos, pat, clut) = (e.disp_sw, e.pos, e.tex_anm_pat, e.clut_swap);
        let clump_matrix = matrix(space::fw2lw(pos, player, bounds), e);
        match &mut e.obj {
            Obj::Anm { obj, play, matrix } if disp => {
                cx.draws.push(DrawRec::Anm { obj: *obj, play: play.clone(), matrix: *matrix, alpha, layer });
            }
            Obj::Eff(eff) => {
                if disp {
                    eff.pos = space::fw2lw(pos, player, bounds);
                    eff.transparency = alpha;
                    cx.draws.push(DrawRec::Eff { eff: eff.clone(), pat, layer });
                }
                step_pattern(e, kind);
                // Not paused: the pattern steps again (0x001c76f4).
                step_pattern(e, kind);
            }
            Obj::Clump(obj) if disp => {
                cx.draws.push(DrawRec::Clump { obj: *obj, matrix: clump_matrix, alpha, layer, clut });
            }
            _ => {}
        }
    }
}

/// `ccEffect::Main`'s pattern step after a `ccEff` draw, by
/// `effectTbl[id].type`: 3 on to the end (then `endFlag`), 4 round to 0,
/// 5 holding the last.
fn step_pattern(e: &mut Effect, kind: Kind) {
    let Obj::Eff(eff) = &e.obj else { return };
    let n = i32::from(eff.pat_num);
    match kind {
        Kind::EffOnce => {
            e.tex_anm_pat = e.tex_anm_pat.wrapping_add(1);
            if i32::from(e.tex_anm_pat) >= n {
                e.end_flag = true;
            }
        }
        Kind::EffLoop => {
            e.tex_anm_pat = e.tex_anm_pat.wrapping_add(1);
            if i32::from(e.tex_anm_pat) >= n {
                e.tex_anm_pat = 0;
            }
        }
        Kind::EffHold => {
            e.tex_anm_pat = e.tex_anm_pat.wrapping_add(1);
            if i32::from(e.tex_anm_pat) >= n {
                e.tex_anm_pat = (n as i16).wrapping_sub(1) as u16;
            }
        }
        _ => {}
    }
}

/// `ccCheckCameraDeg(pos, deg)` (main 0x001da710): is `pos` within `deg`
/// either side of the line from the active camera's eye to the point it
/// looks at, on the ground (all three through `W2PPos`)? The bounds are
/// open.
pub fn check_camera_deg(pos: V4, player: V4, bounds: [F; 4], eye: V4, view: V4, deg: i16) -> bool {
    let p = space::w2p(pos, player, bounds);
    let c = space::w2p(eye, player, bounds);
    let v = space::w2p(view, player, bounds);
    let a = ee::rad2deg(ee::atan2f(ee::sub(p[1], c[1]), ee::sub(p[0], c[0])));
    let b = ee::rad2deg(ee::atan2f(ee::sub(v[1], c[1]), ee::sub(v[0], c[0])));
    let d = (i32::from(deg) + (i32::from(a) - i32::from(b))) as i16;
    d > 0 && i32::from(d) < 2 * i32::from(deg)
}

/// `Main`'s first switch (0x001c4018, on id + 15): the effect's motion.
fn pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    match ctrl.effects[i].id {
        levelup::LEVEL_UP => levelup::pre(ctrl, cx, i),
        3 => arrival::ring_pre(ctrl, cx, i),
        132 | 133 => drain::orb_pre(ctrl, cx, i),
        hit::HIT_RING => hit::ring_pre(ctrl, cx, i),
        hit::ATTRIBUTE_CRITICAL => hit::critical_pre(ctrl, cx, i),
        150..=154 => hit::word_pre(ctrl, cx, i),
        tornado::RING_A | tornado::RING_B | tornado::RING_C => tornado::ring_pre(ctrl, cx, i),
        tornado::OBJ_FIRST..=tornado::OBJ_LAST => tornado::obj_pre(ctrl, cx, i),
        fall::FIRE_FLAME => fall::flame_pre(ctrl, cx, i),
        fall::FIRE_METEOR | fall::SOIL_METEOR | fall::DARK_METEOR => fall::meteor_pre(ctrl, cx, i),
        fall::FLARE_RING => fall::flare_ring_pre(ctrl, cx, i),
        convergence::PIECE_FIRST..=convergence::PIECE_LAST => convergence::piece_pre(ctrl, cx, i),
        upheaval::PILLAR_FIRST..=upheaval::PILLAR_LAST => upheaval::pillar_pre(ctrl, cx, i),
        summons::RING => summons::ring_pre(ctrl, cx, i),
        summons::CREATURE_FIRST..=summons::CREATURE_LAST | 164 | 165 => summons::creature_pre(ctrl, cx, i),
        summons::WAVE => summons::wave_pre(ctrl, cx, i),
        summons::WAVE_OUTER => summons::wave_outer_pre(ctrl, cx, i),
        summons::LOCKON_MARK => summons::mark_pre(ctrl, cx, i),
        summons::STACK_NUM => summons::stack_num_pre(ctrl, cx, i),
        summons::HUGE_FLARE_RING => summons::huge_flare_ring_pre(ctrl, cx, i),
        summons::STACK_RANGE_RING => summons::range_ring_pre(ctrl, cx, i),
        summons::DRILL => summons::drill_pre(ctrl, cx, i),
        shockwave::WAVE => shockwave::wave_pre(ctrl, cx, i),
        shockwave::WAVE2 => shockwave::wave2_pre(ctrl, cx, i),
        shockwave::WAVE3 => shockwave::wave3_pre(ctrl, cx, i),
        skillstart::EXEC_RING => skillstart::exec_ring_pre(ctrl, cx, i),
        skillstart::FORCE_RING => skillstart::force_ring_pre(ctrl, cx, i),
        skillstart::FORCE_RING2 => skillstart::force_ring2_pre(ctrl, cx, i),
        skillstart::SUMMONS_CIRCLE | skillstart::CIRCLE => skillstart::circle_pre(ctrl, cx, i),
        heal::HEAL_RING => heal::ring_pre(ctrl, cx, i),
        id if debris::has_motion(id) => debris::debris_pre(ctrl, cx, i),
        _ => Next::Draw,
    }
}

/// `Main`'s second chain (0x001c77f8): children, timing, sounds.
fn post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    match ctrl.effects[i].id {
        drain::DRAIN_CTRL => drain::ctrl_post(ctrl, cx, i),
        levelup::LEVEL_UP => levelup::post(ctrl, cx, i),
        3 => arrival::ring_post(ctrl, cx, i),
        4 => arrival::transfer_post(ctrl, cx, i),
        arrival::WARP_TRANSFER => arrival::warp_post(ctrl, cx, i),
        hit::HIT_PHOTON => hit::photon_post(ctrl, cx, i),
        19..=21 => hit::protect_post(ctrl, cx, i),
        tornado::OBJECT_POS => tornado::object_pos_post(ctrl, cx, i),
        thunder::THUNDER_POS => thunder::thunder_pos_post(ctrl, cx, i),
        fall::FIRE_METEOR | fall::SOIL_METEOR | fall::DARK_METEOR => fall::meteor_post(ctrl, cx, i),
        fall::THUNDER_METEOR => fall::thunder_meteor_post(ctrl, cx, i),
        convergence::CHARGE => convergence::charge_post(ctrl, cx, i),
        convergence::PIECE_FIRST..=convergence::PIECE_LAST => convergence::piece_post(ctrl, cx, i),
        upheaval::FRAGMENT_FIRST..=upheaval::FRAGMENT_BOUNCING => upheaval::fragment_post(ctrl, cx, i),
        50..=upheaval::FLASH_LAST => upheaval::flash_post(ctrl, cx, i),
        summons::RING => summons::ring_post(ctrl, cx, i),
        summons::FRAGMENT_FIRST..=summons::FRAGMENT_LAST => summons::fragment_post(ctrl, cx, i),
        summons::LOCKON => summons::lockon_post(ctrl, cx, i),
        summons::DRILL => summons::drill_post(ctrl, cx, i),
        155..=163 => debris::radiate2_post(ctrl, cx, i),
        skillstart::START | skillstart::START_B => skillstart::start_post(ctrl, cx, i),
        heal::HEAL => heal::heal_post(ctrl, cx, i),
        heal::RESURRECT | heal::CURE | heal::SANITY => heal::cure_post(ctrl, cx, i),
        heal::REMOVE_TRAP => heal::remove_trap_post(ctrl, cx, i),
        crate::pg::EVOLVE => crate::pg::evolve_post(ctrl, cx, i),
        gimmick::OPEN_TRAP_BOX => gimmick::open_trap_box_post(ctrl, cx, i),
        gimmick::VIRUS_CRYSTAL => gimmick::virus_crystal_post(ctrl, cx, i),
        ability::UP_START => ability::up_start_post(ctrl, cx, i),
        ability::UP => ability::up_post(ctrl, cx, i),
        ability::DOWN => ability::down_post(ctrl, cx, i),
        id if debris::has_bounce(id) => debris::debris_post(ctrl, cx, i),
        _ => {}
    }
}
