//! Corbenik in the field (`ccThBoss08`, QUA gcmn 0x003aca30): its task
//! plays the first form until it exits, then, in the same frame, resets
//! the menu, flashes, deletes it and makes the second form; or, with
//! `eventStatus[55]` 1, only the third. The rules are
//! [`piney_battle::boss::corbenik`]; this keeps their characters, bodies
//! and cameras (docs/engine/boss-corbenik.md "In the field").

use piney_battle::affect::AffectCtx;
use piney_battle::boss::{self, Boss, BossEnv, Class, Out};
use piney_battle::chara::Env;
use piney_battle::enemy_ai::World;
use piney_battle::event::Events;
use piney_battle::geom;
use piney_battle::world::CharHit;

use super::Combat;
use super::boss::{CAM_TRANSFER, Picture};
use crate::camera::Camera;
use crate::camera::bosscam::BossCam;
use crate::draw::CHAR_LAYER;
use crate::ee::{self, F, ONE, V4};
use piney_battle::geom::M4;

/// The clumps of x81 its forms' and parts' clips pose (the first is the
/// look's body, the first form's): the first form, the second, the
/// third's pieces, the seeds, the eyes, the killer eyes, the roots, the
/// dome and the barrier.
pub const BODIES: [&str; 14] = [
    "CMP_trall1",
    "CMP_trall2",
    "CMP_trall0",
    "CMP_trall3",
    "CMP_ex8body30",
    "CMP_ex8lbod06",
    "CMP_ex8aseed1",
    "CMP_trall5",
    "CMP_trall8",
    "CMP_trall2b",
    "CMP_trall2c",
    "CMP_trall2d",
    "CMP_trall4",
    "CMP_trallb",
];

/// An eye's flash (`ccBoss08Eye::PreDrawAnm` 0x003bed70):
/// `SetFogBlend(blend, 0xffffff)`.
pub const EYE_FLASH: u32 = 0x00ff_ffff;
/// `EntryFlash(30, 0x80e0ffff)` between the first form and the second.
const FLASH_T: i32 = 30;
const FLASH_COLOUR: u32 = 0x80e0_ffff;
/// The third form's camera (`ccBoss08_c::ccBoss08_c`, 0x003b95bc):
/// `InitBossCamera(980, 1500)`, the pitch `atan2f(500, 1000)`, its range
/// 1000, the eye raised by `1500 tanf(pitch)`, `SetRotXLimit(1.0)`.
pub fn third_camera(camera: &mut Camera, player: V4) -> BossCam {
    use boss::corbenik::third::{CAM_Y, CAM_Z};
    let mut cam = BossCam::new(camera, [0, CAM_Y, CAM_Z, ONE], player);
    let pitch = ee::atan2f(0x43fa_0000, 0x447a_0000);
    cam.xrot = pitch;
    cam.max_range = 0x447a_0000;
    cam.lift = ee::mul(CAM_Y, ee::tanf(pitch));
    cam.set_rot_x_limit(ONE);
    cam
}

impl Combat {
    /// `ccThBoss08` past the first form's exit (QUA gcmn 0x003acaf8): the
    /// menu's forbid, `forbidChatExcept` and `cursolOff` cleared, the
    /// flash, the first form deleted (its seeds with it) and the second
    /// made (`ccThBoss08b_Main`) on the same character with the manager's
    /// effects, its camera `InitBossCamera(200, 1000)`. What it asked, or
    /// None while the first form plays.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn corbenik_next_form(
        &mut self,
        b: &mut Boss,
        env: &Env,
        check: &dyn Fn(usize) -> i32,
        world: World,
        camera: &mut Camera,
        hits: &mut crate::hit::Hits,
    ) -> Option<Vec<Out>> {
        let run = self.boss.as_ref()?;
        if run.code != super::boss::CORBENIK || b.base.exit == 0 || !matches!(b.class, Class::Corbenik(_)) {
            return None;
        }
        let (me, center, clips) = (run.me, run.center, run.clips.clone());
        let mut out = vec![
            Out::MenuForbid { on: false, chat_except: false },
            Out::CursorOff(false),
            Out::Flash { t: FLASH_T, colour: FLASH_COLOUR },
        ];
        // The seeds go with the first form: their characters off the
        // lists, their bodies out of the field's.
        for k in b.class.shape().parts(&self.scene) {
            self.scene.ene_list.retain(|&c| c != k);
            self.cast.actors.remove(&k);
            let id = super::spc::body_id(k, self.kite);
            hits.chars.retain(|c| c.id != id);
            if let Some(f) = self.scene.chars[k].foe_state_mut() {
                f.boss = None;
            }
        }
        let kite = self.kite?;
        let kite_pos = self.scene.chars[kite].pos;
        let kite_dirc = self.kite_dirc();
        let d = self.data.clone();
        let data = d.bosses.as_ref()?;
        let clip = |name: &str| clips.get(name).copied();
        let party = self.party;
        let benv = BossEnv { t: &d.t, data, clips: &clip, env, game_over: false, cc: None };
        let actx =
            AffectCtx { party: &party, menu: true, skill_check: check, boss: Some(&benv), volume: self.data.volume };
        let mut none = |_| None;
        let mut land = |p: V4| p[2];
        let effects = std::mem::take(&mut b.base.effects);
        let mut cx = boss::Cx {
            t: &d.t,
            data,
            scene: &mut self.scene,
            party: &party,
            world,
            env,
            actx: &actx,
            rand: &mut self.rand,
            cc: &mut self.cc,
            clips: &clip,
            collide: &mut none,
            part_hits: &mut boss::NoHits,
            game_over: false,
            boss_cam: true,
            cam: super::boss::cam_view(camera, None, false),
            land: &mut land,
            disc: boss::DiscView::default(),
            gen_alive: &|_| true,
            me,
            out: Vec::new(),
            ev: Events::new(),
        };
        *b = boss::corbenik::second::new(&mut cx, kite_pos, kite_dirc, center, effects);
        out.extend(std::mem::take(&mut cx.out));
        drop(cx);
        let base = self.scene.chars[me].base();
        let (h, w, ty) = (base.height, base.width, base.ty as u32);
        let run = self.boss.as_mut()?;
        run.hit = CharHit { pos: self.scene.chars[me].pos, radius: w, height: h, kind: ty, ..CharHit::default() };
        run.parts = b.class.shape().parts(&self.scene);
        run.afterimages.clear();
        run.cam = Some(BossCam::new(camera, CAM_TRANSFER, kite_pos));
        run.cam_sw = true;
        for o in &out {
            if let Out::Blur(s) = o {
                run.blur.set(*s);
            }
        }
        Some(out)
    }
}

/// The second form's barrier object, and the node it rides.
const BARRIER: &str = "OBJ_ex8bria";
const BODY2: &str = "OBJ_ex8body2";
/// The clips its elements and its drain's root play (`ccBoss08_b`'s
/// constructor; `_InitElement` 0x003b74b0 copies the body's neutral clip
/// one step on; the drain's `ANM_ex8natc4`).
const ELEMENT_CLIP: &str = "ANM_ex8nut2";
const ELEMENT_TIME: u32 = 256;
const ROOT_L: &str = "ANM_ex8natc4";
/// The lasers' objects and head clip in xeffect, and their fog
/// (`SetFogBlend(50, 0xff0000)` over the second object).
const LES0: &str = "OBJ_ex31les0";
const LES1: &str = "OBJ_ex31les1";
const LES3: &str = "ANM_ex31les3";
const LASER_FOG: crate::chara::foe::FogBlend = crate::chara::foe::FogBlend { rate: 0x4248_0000, colour: 0x00ff_0000 };
const HALF_PI: F = 0x3fc9_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;

/// `OBJ_ex8body2`'s world matrix as the body was last drawn.
pub fn body2(
    body: &crate::chara::body::Body,
    play: &crate::chara::pose::Play,
    pos: V4,
    dirc: V4,
) -> Option<glam::Mat4> {
    let n = body.node(BODY2)?;
    body.worlds(play, crate::chara::body::root(pos, dirc)).get(&n).copied()
}

fn mat4(m: &M4) -> glam::Mat4 {
    glam::Mat4::from_cols_array_2d(&m.map(|r| r.map(f32::from_bits)))
}

/// `SetMatrix_PosRotXYZ(pos, rot)`.
fn pos_rot_xyz(pos: V4, rot: V4) -> M4 {
    let m = geom::rot_matrix_x(&geom::unit_matrix(), rot[0]);
    let m = geom::rot_matrix_y(&m, rot[1]);
    let m = geom::rot_matrix_z(&m, rot[2]);
    geom::trans_matrix(&m, pos)
}

/// A laser segment's matrix (`ccBossEffLaserRain::_Init`'s +0x240 moved
/// to the segment, `ccBossEffRefrectionLaser::Draw`'s per segment): its
/// length scale on z, turned -pi/2 about x, then about x by `pitch` and
/// z by `yaw`, at `pos`.
fn segment(pos: V4, pitch: F, yaw: F, len: F) -> M4 {
    let mut m = geom::unit_matrix();
    m[2][2] = len;
    let m = geom::rot_matrix_x(&m, NEG_HALF_PI);
    let m = geom::rot_matrix_x(&m, pitch);
    let m = geom::rot_matrix_z(&m, yaw);
    geom::trans_matrix(&m, pos)
}

/// A laser head's clip matrix: turned +pi/2 about x, then by `pitch` and
/// `yaw`, at `pos`.
fn head(pos: V4, pitch: F, yaw: F) -> M4 {
    let r = geom::rot_matrix_x(&geom::unit_matrix(), pitch);
    let r = geom::rot_matrix_z(&r, yaw);
    let x = geom::rot_matrix_x(&geom::unit_matrix(), HALF_PI);
    geom::trans_matrix(&geom::mul_matrix(&r, &x), pos)
}

/// `ccGetDist3D` and the pitch and heading from `a` to `b`
/// (`atan2f(a.z - b.z, ccGetDist(a, b))`, `ccGetDirc(a, b)`).
fn way(a: V4, b: V4) -> (F, F, F) {
    let d = ee::sqrt(ee::add(
        ee::mul(ee::sub(b[0], a[0]), ee::sub(b[0], a[0])),
        ee::mul(ee::sub(b[1], a[1]), ee::sub(b[1], a[1])),
    ));
    let dz = ee::sub(b[2], a[2]);
    let d3 = ee::sqrt(ee::add(ee::mul(d, d), ee::mul(dz, dz)));
    let pitch = ee::atan2f(ee::sub(a[2], b[2]), d);
    (d3, pitch, piney_battle::enemy_ai::get_dirc(a, b))
}

/// The two objects of a laser segment, the second in the red fog.
fn laser_pair(v: &mut Vec<Picture>, world: glam::Mat4, alpha: f32, first: &'static str, second: &'static str) {
    // OBJ_ex31les1's model is set to `alphaBlendTbl[1]` (and Z unwritten)
    // by both effects' constructors.
    let blend = |n: &str| (n == LES1).then_some(1);
    v.push(Picture::Obj { eff: true, name: first, world, alpha, fog: None, blend: blend(first) });
    v.push(Picture::Obj { eff: true, name: second, world, alpha, fog: LASER_FOG.fog(), blend: blend(second) });
}

/// What the frame's outs show of x81 and xeffect (`out` the boss's,
/// `barrier_at` where the barrier rides), on charLayer.
pub fn shown(b: &Boss, out: &[Out], barrier_at: Option<glam::Mat4>) -> Vec<Picture> {
    use boss::corbenik::Pic;
    let mut v = Vec::new();
    let body_clip = b.base.anm.clip.clone().unwrap_or_default();
    let alpha = ee::f(b.base.set_transparency);
    for o in out {
        let Out::Corbenik(p) = o else { continue };
        match p {
            // `PreDrawAnm` (0x003b3600): the object, or the hit's clip,
            // at the node, at `setTransparency`.
            Pic::Barrier => {
                if let Some(world) = barrier_at {
                    v.push(Picture::Obj { eff: false, name: BARRIER, world, alpha, fog: None, blend: None });
                }
            }
            Pic::BarrierHit { clip, time } => {
                if let (Some(root), Some(clip)) = (barrier_at, clip.clone()) {
                    v.push(Picture::Anm { eff: false, layer: CHAR_LAYER, clip, time: *time, root, alpha, blend: None });
                }
            }
            Pic::PressImage { time, pos, dirc, alpha } => v.push(Picture::Anm {
                eff: false,
                layer: CHAR_LAYER,
                clip: body_clip.clone(),
                time: *time,
                root: crate::chara::body::root(*pos, *dirc),
                alpha: ee::f(*alpha),
                blend: None,
            }),
            Pic::Element { pos, dirc, alpha } => v.push(Picture::Anm {
                eff: false,
                layer: CHAR_LAYER,
                clip: ELEMENT_CLIP.into(),
                time: ELEMENT_TIME,
                root: mat4(&pos_rot_xyz(*pos, *dirc)),
                alpha: ee::f(*alpha),
                blend: None,
            }),
            Pic::RootL { time, pos, alpha } => v.push(Picture::Anm {
                eff: false,
                layer: CHAR_LAYER,
                clip: ROOT_L.into(),
                time: *time,
                root: mat4(&geom::trans_matrix(&geom::unit_matrix(), *pos)),
                alpha: ee::f(*alpha),
                blend: None,
            }),
            // `ccBossEffLaserRain::Draw` (0x00379940): les1, then les0 in
            // the fog, a twentieth long; the head's clip.
            Pic::Laser { pos, pitch, yaw, alpha } => {
                let w = mat4(&segment(*pos, *pitch, *yaw, 0x3d4c_cccd));
                laser_pair(&mut v, w, ee::f(*alpha), LES1, LES0);
            }
            Pic::LaserHead { time, pos, pitch, yaw } => {
                let root = mat4(&head(*pos, *pitch, *yaw));
                v.push(Picture::Anm {
                    eff: true,
                    layer: CHAR_LAYER,
                    clip: LES3.into(),
                    time: *time,
                    root,
                    alpha: 1.0,
                    blend: Some(1),
                });
            }
            // `ccBossEffRefrectionLaser::Draw` (0x003773d0): les0, then les1
            // in the fog, `ccGetDist3D / 2000` long; the head's clip.
            Pic::RefrectSeg { from, to, alpha } => {
                let (d, pitch, yaw) = way(*from, *to);
                let w = mat4(&segment(*from, pitch, yaw, ee::div(d, 0x44fa_0000)));
                laser_pair(&mut v, w, ee::f(*alpha), LES0, LES1);
            }
            Pic::RefrectHead { time, from, to } => {
                let (_, pitch, yaw) = way(*from, *to);
                let root = mat4(&head(*from, pitch, yaw));
                v.push(Picture::Anm {
                    eff: true,
                    layer: CHAR_LAYER,
                    clip: LES3.into(),
                    time: *time,
                    root,
                    alpha: 1.0,
                    blend: Some(1),
                });
            }
            _ => {}
        }
    }
    v
}
