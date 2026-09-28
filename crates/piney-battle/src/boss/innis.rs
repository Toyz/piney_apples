//! Innis (`ccBoss02`, boss02.cpp) and its three images (`ccBoss02Slave`,
//! each with a `BreakMirror`), as Mutation's gcmn.prg has them (0x004941b0-
//! 0x0049da1c, BreakMirror 0x00533d90-0x00534a7c; names and layouts from
//! Infection's DWARF). `ccBossEntryStart(1)` starts `ccThBoss02`, which news
//! the boss ([`new`]) and runs [`main`] each frame; event 107 fights it. Its
//! missiles' flights ([`Missile`]) are part of its rules: their `Draw` calls
//! its `MagicDamage`. docs/engine/boss-innis.md.

use piney_data::field::ee;
use piney_data::libm;

use super::{Anm, Boss, Class, Cx, EffKind, Out, VF0};
use crate::chara::{self, AffectFunc};
use crate::enemy_ai::{get_dirc, get_dist, rand_f};
use crate::geom::{self, M4, V4};
use crate::item;
use crate::param::cond;

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const NEG_PI: F = 0xc049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const HALF_PI: F = 0x3fc9_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
/// 1.0471976: the side steps' turn off the camera's heading (pi / 3).
const THIRD_PI: F = 0x3f86_0a92;
/// `SelectTarget`'s radius: 1e6.
const FAR: F = 0x4974_23f0;
/// `MAXRangeX`, `MAXRangeY`: how far from the centre Innis may stray.
const RANGE: F = 0x4509_8000;

/// Innis's acts (`actNum`).
pub mod act {
    pub const NEUTRAL: i16 = 0;
    pub const DAMAGE0: i16 = 1;
    pub const DAMAGE1: i16 = 2;
    /// The spin up and the fall (`AllAttack`).
    pub const SPIN: i16 = 3;
    /// Waiting for the magic to land (`MagicDamage` ends it).
    pub const MAGIC: i16 = 4;
    /// The images sent out (`EnemyAttack`).
    pub const IMAGES: i16 = 5;
    /// A member drained through menu 74.
    pub const DRAIN: i16 = 6;
    pub const SHAKE: i16 = 7;
    /// Drained by Kite: to the Epitaph's patterns.
    pub const EPITAPH: i16 = 11;
    pub const EPITAPH_NEUTRAL: i16 = 12;
    pub const EPITAPH_SPIN: i16 = 13;
    pub const DEAD: i16 = 14;
}

/// A missile's element: `IceAttack`, `ThunderAttack`, `BlazeAttack`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Element {
    Ice,
    Lightning,
    Blaze,
}

/// The particle generators Innis starts, by table and row (0x38 bytes a
/// row): `InisFieldGenerator` (MUT gcmn 0x0061d350), `TornadoGenerator`
/// (0x0061d4f0), `BurstGenerator` (0x0061d590).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gen {
    InisField(u8),
    Tornado,
    Burst(u8),
}

/// Innis's tables (`tables::combat`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InnisData {
    /// `Pattern`: the action words, three runs each ended by a 29.
    pub pattern: Vec<i32>,
    /// `EPITAPH_Pattern`: once drained.
    pub epitaph: Vec<i32>,
    /// `boss02AnmTbl`: the act's clip.
    pub anm_tbl: Vec<Option<String>>,
    /// `Skill_VARIOUS_INIS`, `Skill_DOWNER_INIS`.
    pub various: Vec<i16>,
    pub downer: Vec<i16>,
    /// `Mon1`-`Mon3`: the images' clips by act.
    pub monsters: Vec<Vec<Option<String>>>,
    /// The images' burst rings' model (`EnemyBurst`'s @2702).
    pub ring_models: Vec<i32>,
}

impl InnisData {
    /// The volume's.
    pub fn of(volume: piney_data::volume::Volume) -> InnisData {
        let t = piney_data::tables::combat::of(volume);
        let s = |v: &[Option<&str>]| v.iter().map(|a| a.map(str::to_string)).collect::<Vec<_>>();
        InnisData {
            pattern: t.innis_pattern().to_vec(),
            epitaph: t.innis_epitaph().to_vec(),
            anm_tbl: s(t.innis_anims()),
            various: t.innis_various_skills().to_vec(),
            downer: t.innis_downer_skills().to_vec(),
            monsters: t.innis_monster_anims().iter().map(|m| s(m)).collect(),
            ring_models: t.innis_ring_models().to_vec(),
        }
    }
}

/// `ccBoss02`'s own members (+0x29350 on).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Innis {
    /// `anmw`: `xeffect`'s `ANM_xx11wave`, played in the fall.
    pub anm_w: Anm,
    pub init_vector: V4,
    pub action_flg: i32,
    pub escape: u8,
    pub eny_flg: u8,
    pub eny_int: i32,
    pub eny_float: F,
    pub entry_flg: i32,
    pub epitaph_flg: i32,
    pub dd_flg: i32,
    /// `Hipos`: the height Innis floats at.
    pub hipos: F,
    pub now_mode: i32,
    pub pat_end_mode: [i32; 3],
    pub pat_end_flg: u8,
    pub lock_target_flg: i32,
    pub flg: i32,
    pub move_flg: i32,
    pub end_flg: i32,
    pub temphi: i32,
    pub rotate: F,
    pub dircsub: F,
    pub sub_hi: F,
    /// `ActionStartFlg`, a float (0 or 1).
    pub action_start_flg: F,
    pub pos_b: F,
    pub pos_a: F,
    pub p_flg: u8,
    pub p_dist: F,
    pub cou: i32,
    pub af_cou: i32,
    pub af_timing: i32,
    pub no: i32,
    pub sub_no: i32,
    pub mirror_pros: i32,
    pub monster_id: i32,
    pub alpha: F,
    pub back_step_dist: i32,
    pub ex_spin_back_flg: u8,
    pub escape_act_cou: i32,
    pub dmg_count: i32,
    pub dmg_wait: i32,
    /// `SkAtk`: `ccGetBossSkillParam(4)` once the fall lands.
    pub sk_atk: Option<usize>,
    pub default_argb: u32,
    pub ex_argb: u32,
    pub cam_dist: F,
    pub zoom_vec: V4,
    pub zoom_view: V4,
    pub cam_pos: V4,
    pub cam_view: V4,
    pub cam_mat: M4,
    pub skill_id: i32,
    pub quake_vector: V4,
    /// `CamRot`: `cameraGetRot(camID)` as `SetCameraEyes` last read it.
    pub cam_rot: V4,
    pub rot_vec: V4,
    pub sub_vec: V4,
    pub rot: F,
    pub sub_rot: F,
    pub vec_center: V4,
    pub vec_center_p: V4,
    /// `targetPOS`: `&InitVector` (None) or the target's `pos`.
    pub target_pos_of: Option<usize>,
    /// `DeadEff`: the dead effect's slot.
    pub dead_eff: Option<i32>,
    /// `bossBlur`'s colour (+0x1c).
    pub blur: u32,
    /// `ccMenu.forbid` as act 5 set it this frame (its direct write,
    /// which `Main`'s check after reads).
    pub forbid_now: Option<i16>,
    /// `slaveCtrl[0]`: the three images.
    pub slaves: Vec<Slave>,
}

/// `ccBoss02Slave`'s own members (+0x29350 on) and its `ccBoss` base.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Slave {
    pub b: Boss,
    /// The image's scene index (a character on no command list).
    pub me: usize,
    /// `slaveID` (`SetMaster`), `orderNum` (`Order`).
    pub slave_id: i32,
    pub order_num: i32,
    pub samon_id: i32,
    pub dist: F,
    pub dirc: F,
    pub transparency: F,
    pub eny_flg: u8,
    /// `Pos`: where its ring would sit, 150 above it.
    pub pos: V4,
    pub rot: V4,
    pub scale: V4,
    pub spin: V4,
    pub target_pos: V4,
    pub start_count: i32,
    pub my_no: i32,
    pub flg: i32,
    pub end_flg: u8,
    pub mirror: Mirror,
    pub quake_vector: V4,
    pub quake_time: i32,
}

/// `BreakMirror` (0x16f0 bytes): the sixty shards an image breaks into.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mirror {
    /// `f`: 0 not begun, 1 falling, 2 done.
    pub f: i8,
    pub set_point_flg: u8,
    /// Which set of shard models (`SetStream`'s ID: `CMP_ex21ef0n`,
    /// `ef1n`, `ef2n`); None once `EndData` deleted them.
    pub models: Option<i32>,
    pub v0: Vec<F>,
    pub theta: Vec<F>,
    pub zrot: Vec<F>,
    pub vx: Vec<F>,
    pub vz: Vec<F>,
    pub x: Vec<F>,
    pub z: Vec<F>,
    pub spin: Vec<V4>,
    pub m_rot: Vec<V4>,
    pub nut_pos: V4,
    pub nut_rot: V4,
    /// The shards drawn this frame (`DrawPartsClump`): model k % 5 at the
    /// place and turn.
    pub drawn: Vec<(usize, V4, V4)>,
}

/// A missile's flight: `ccBossEff{Ice,Lightning,Blaze}Missile`'s members
/// its `Draw` steps.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Missile {
    pub t: F,
    pub eny_flg: u8,
    pub eny_int: i32,
    pub transparency: F,
    pub scale: V4,
    pub pos: V4,
    pub rot: V4,
    /// `m_cbFunc` is `MagicDamage` and `bossCam` is set: the first missile.
    pub hits: bool,
}

// --- small rules ------------------------------------------------------------------------

/// An angle into -pi..pi as the game does it, once each way.
fn wrap(mut v: F) -> F {
    if !ee::le(v, PI) {
        v = ee::sub(v, TWO_PI);
    }
    if ee::lt(v, NEG_PI) {
        v = ee::add(v, TWO_PI);
    }
    v
}

fn dp(x: F) -> f64 {
    f64::from(f32::from_bits(x))
}

/// `dptofp`: a double rounded to the nearest float.
fn fp(d: f64) -> F {
    (d as f32).to_bits()
}

/// A table word; outside the table (index -1) the padding before it, 0.
fn word(tbl: &[i32], i: i32) -> i32 {
    usize::try_from(i).ok().and_then(|k| tbl.get(k)).copied().unwrap_or(0)
}

/// C's `ccRand() % n`.
fn rand_mod(cx: &mut Cx, n: i32) -> i32 {
    cx.cc.rand() % n
}

fn vec3(x: F, y: F, z: F) -> V4 {
    [x, y, z, ONE]
}

fn innis_of(b: &mut Boss) -> Box<Innis> {
    match std::mem::replace(&mut b.class, Class::Plain) {
        Class::Innis(i) => i,
        other => {
            b.class = other;
            Box::default()
        }
    }
}

// --- the constructor ----------------------------------------------------------------------

/// `ccBoss02::ccBoss02` (0x004942c0) and `ccBoss02Slave::ccBoss02Slave`
/// (0x0049c310) for its three images, whose characters it adds to the
/// scene (on no list). The boss is `scene.chars[cx.me]` (`bossTbl` row 1);
/// Innis stands 1500 behind Kite and 20 up. `center` is the arena's
/// `DMY_center01` (world).
pub fn new(cx: &mut Cx, kite_pos: V4, kite_dirc: V4, center: V4) -> Boss {
    let data = &cx.data.innis;
    let mut pat_end = [-1; 3];
    let mut n = 0;
    for (k, &w) in data.pattern.iter().enumerate().take(150) {
        if w == 29 && n < 3 {
            pat_end[n] = k as i32;
            n += 1;
        }
    }
    let me = cx.me;
    let mut b = Boss {
        anm_tbl: data.anm_tbl.clone(),
        wait_pat_num: -2,
        pat_num: 13,
        stage_eff_id: -1,
        use_stage_eff: true,
        transparency: ONE,
        set_transparency: ONE,
        anm: Anm::new(),
        anm_wave: Anm::new(),
        animate: super::Fade { transparency: ONE, ..super::Fade::default() },
        animate_dead: super::Fade { transparency: ONE, ..super::Fade::default() },
        ..Boss::default()
    };
    let mut x = Innis {
        anm_w: Anm::new(),
        end_flg: 1,
        p_flg: 1,
        af_timing: 3,
        alpha: ONE,
        pat_end_flg: 1,
        dmg_wait: 500,
        // maxHP as ccChar::ccChar left it, before SetBaseParam: 0.
        dmg_count: 0,
        lock_target_flg: 2,
        init_vector: VF0,
        pat_end_mode: pat_end,
        default_argb: 0x3080_8080,
        ex_argb: 0x6080_8080,
        quake_vector: [0x40a0_0000, 0x40a0_0000, 0x40a0_0000, ONE],
        cam_mat: geom::unit_matrix(),
        ..Innis::default()
    };
    // SetBaseParam(ccGetBossParam(1)) is the character's; then cheatHP,
    // actAnmTbl, OffExit, OnDraw, OnBodyHit, OnCheatHP.
    b.cheat_hp = 1;
    b.exit = 0;
    b.draw_sw = 1;
    b.body_hit_sw = 1;
    let mut pos = kite_pos;
    pos[1] = ee::add(pos[1], 0x44bb_8000);
    pos[2] = ee::add(pos[2], 0x41a0_0000);
    cx.scene.chars[me].pos = pos;
    b.dirc = kite_dirc;
    x.hipos = pos[2];
    b.anm.set("ANM_ex21nut0", cx.clips);
    b.change_action(cx, act::NEUTRAL, 0, true);
    // The blur (the manager's +4): on, never ending, DefaultARGB.
    x.blur = x.default_argb;
    cx.out(Out::Blur { colour: x.blur });
    x.anm_w.set(super::ANM_WAVE, cx.clips);
    for k in 0..3 {
        x.slaves.push(Slave::new(cx, k));
    }
    x.vec_center = [center[0], center[1], center[2], ONE];
    x.vec_center_p = super::w2p(cx, x.vec_center);
    crate::fellow::entry_cmnd(cx.scene, me);
    b.class = Class::Innis(Box::new(x));
    b
}

impl Slave {
    /// `ccBoss02Slave::ccBoss02Slave` (0x0049c310) and `SetMaster(boss,
    /// k)`: a bare `ccBoss` of `bossTbl` row 10, not drawn, exited.
    fn new(cx: &mut Cx, k: i32) -> Slave {
        let row = cx.t.bosses.get(10).cloned().unwrap_or_default();
        let mut ch = crate::chara::Char::foe(row);
        ch.condition_num = -1;
        ch.affect.func = AffectFunc::None;
        let me = cx.scene.add(ch, 3);
        let mut anm = Anm::new();
        anm.set("ANM_ex21mon1", cx.clips);
        let b = Boss {
            class: Class::Plain,
            wait_pat_num: -2,
            pat_num: 13,
            stage_eff_id: -1,
            exit: 1,
            transparency: ONE,
            set_transparency: ONE,
            anm,
            anm_wave: Anm::new(),
            ..Boss::default()
        };
        Slave {
            b,
            me,
            slave_id: k,
            my_no: k,
            eny_flg: 1,
            transparency: ONE,
            quake_vector: [0x41a0_0000, 0x4180_0000, 0x4160_0000, 0],
            mirror: Mirror::new(),
            ..Slave::default()
        }
    }
}

// --- the frame ------------------------------------------------------------------------------

/// `ccThBossEffect`'s pass and `ccBoss02::Main` (0x00495d50).
pub(super) fn main(b: &mut Boss, cx: &mut Cx) {
    let mut x = innis_of(b);
    manager_pass(b, &mut x, cx);
    if b.exit == 0 {
        frame(b, &mut x, cx);
    }
    b.class = Class::Innis(x);
}

fn frame(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    let me = cx.me;
    chara::calc_real(cx.t, &mut cx.scene.chars[me], 0, cx.env, &mut cx.ev);
    b.calc_target_info(cx);
    b.center_pos_p = super::w2p(cx, b.center_pos);
    let pp = cx.scene.chars[me].pos_p;
    b.center_dist = get_dist(pp, b.center_pos_p);
    b.center_dirc = get_dirc(pp, b.center_pos_p);
    if b.stop != 0 {
        b.stop_count += 1;
        if b.stop_count >= 61 {
            b.stop_count = 0;
            b.stop = 0;
        }
        b.move_spd = 0;
    } else {
        b.stop_count = 0;
    }
    think(b, x, cx);
    action(b, x, cx);
    mov(b, x, cx);
    if b.lock_player != 0 {
        for m in cx.party.members.into_iter().flatten() {
            if cx.valid(Some(m)) {
                cx.affect(m, 5, 32767);
            }
        }
        let forbid = x.forbid_now.take().unwrap_or(cx.env.menu_forbid);
        if b.reserve_forbid_menu == 1 && forbid == 0 && cx.env.menu_type == -1 {
            cx.out(Out::MenuForbid { on: true, chat_except: b.reserve_forbid_chat_except != 0 });
            b.reserve_forbid_menu = 0;
            b.reserve_forbid_chat_except = 0;
        }
    } else if b.reserve_forbid_menu == -1 && cx.env.menu_type == -1 {
        cx.out(Out::MenuForbid { on: false, chat_except: false });
        b.reserve_forbid_menu = 0;
        b.reserve_forbid_chat_except = 0;
    }
    x.forbid_now = None;
    if b.draw_sw != 0 {
        b.anm_status = i8::from(b.anm.forward());
        b.transparency = x.alpha;
        b.set_transparency = x.alpha;
    }
    // Slave(): each image's ccBoss::Main.
    for k in 0..x.slaves.len() {
        slave_main(b, x, k, cx);
    }
    cx.scene.chars[me].cond[cond::HOLD] = 0;
}

/// `ccBoss02::Move` (0x00494cd0): as `ccBoss::Move`, the body pushed out
/// whole, then held at `Hipos`.
fn mov(b: &mut Boss, x: &Innis, cx: &mut Cx) {
    let me = cx.me;
    let mut pp = super::w2p(cx, cx.scene.chars[me].pos);
    if !geom::eq(0, b.move_spd) {
        pp[0] = ee::add(pp[0], ee::mul(b.move_spd, libm::sinf(b.move_dirc)));
        pp[1] = ee::sub(pp[1], ee::mul(b.move_spd, libm::cosf(b.move_dirc)));
    }
    pp = geom::vadd(pp, b.move_vector);
    let mut pos = super::p2w(cx, pp);
    if b.body_hit_sw != 0
        && let Some(off) = (cx.collide)(pos)
    {
        pos = geom::vadd(pos, off);
        pp = geom::vadd(pp, off);
        pos[2] = x.hipos;
    }
    let ch = &mut cx.scene.chars[me];
    ch.pos_p = pp;
    ch.pos = pos;
}

// --- the effect manager's pass and the missiles --------------------------------------------------

/// `ccBossEffManager::Draw` (MUT gcmn 0x004780f0) over Innis's effects:
/// a missile's `Draw` may call `MagicDamage` and make a WaveShock, which
/// takes the first free slot (drawn this pass if past this one).
fn manager_pass(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    for k in 0..b.effects.slots.len() {
        let Some(e) = b.effects.slots[k].as_mut() else { continue };
        if !e.enabled {
            b.effects.slots[k] = None;
            continue;
        }
        let EffKind::Missile { element, no, ctrl } = e.kind else {
            e.draw();
            continue;
        };
        let Some(mut m) = e.missile.take() else { continue };
        let alive = m.draw(element, no, &ctrl, b, x, cx);
        if let Some(e) = b.effects.slots[k].as_mut() {
            e.missile = Some(m);
            if !alive {
                e.enabled = false;
            }
        }
    }
}

impl Missile {
    fn new(ctrl: &[V4; 4], hits: bool) -> Missile {
        Missile { transparency: ONE, pos: ctrl[0], rot: VF0, scale: VF0, hits, ..Missile::default() }
    }

    /// `Spline` (Ice 0x00479420): the cubic Bezier through `ctrl` at `t`,
    /// its terms as the game's `mula`/`madd` chain.
    fn spline(&mut self, ctrl: &[V4; 4]) {
        let t = self.t;
        let u = ee::sub(ONE, t);
        let f2 = ee::mul(u, ee::mul(u, u));
        let f4 = ee::mul(0x4040_0000, u);
        let f3 = ee::mul(t, ee::mul(f4, u));
        let f4 = ee::mul(t, ee::mul(f4, t));
        let f5 = ee::mul(t, ee::mul(t, t));
        for (i, p) in self.pos.iter_mut().take(3).enumerate() {
            let acc = ee::mul(ctrl[0][i], f2);
            let f1 = ee::add(acc, ee::mul(ctrl[1][i], f3));
            let acc = ee::add(ee::mul(ctrl[2][i], f4), f1);
            *p = ee::add(acc, ee::mul(ctrl[3][i], f5));
        }
        self.pos[3] = ONE;
    }

    /// The missile's `Draw` as far as the rules go (Ice 0x00478d50,
    /// Lightning 0x00479930, Blaze 0x0047a220): its sounds, its landing,
    /// the camera's shake and the call of `MagicDamage`. False once it
    /// clears `m_bEnabled`.
    #[allow(clippy::too_many_arguments)]
    fn draw(&mut self, el: Element, no: i32, ctrl: &[V4; 4], b: &mut Boss, x: &mut Innis, cx: &mut Cx) -> bool {
        let se = |cx: &mut Cx, se: i32, pos: V4, note: u8| cx.out(Out::Se3dNote { se, pos, note });
        if self.t == 0 {
            se(cx, 57, self.pos, 60);
            if el == Element::Ice && no == 1 {
                se(cx, 61, self.pos, 65);
            }
        }
        let fade = match el {
            Element::Ice => 0x3ca3_d70a,
            Element::Lightning => 0,
            Element::Blaze => 0x3c54_fdf4,
        };
        let grow = match el {
            Element::Blaze => 0x3f4c_cccd,
            _ => ONE,
        };
        if self.eny_flg == 0 {
            self.spline(ctrl);
            let d = get_dirc(super::w2p(cx, self.pos), super::w2p(cx, ctrl[3]));
            match el {
                Element::Ice => self.rot[2] = d,
                _ => self.rot[0] = d,
            }
        } else if el != Element::Lightning {
            for k in 0..3 {
                self.scale[k] = ee::add(self.scale[k], grow);
                if !ee::le(self.scale[k], 0x42c8_0000) {
                    self.scale[k] = 0x42c8_0000;
                }
            }
            self.transparency = ee::sub(self.transparency, fade);
            if el == Element::Ice && no == 1 {
                self.transparency = ee::sub(self.transparency, fade);
            }
        }
        if dp(self.t) > 0.984 && self.eny_flg == 0 {
            self.rot = VF0;
            self.eny_flg = 1;
            if no == 1 {
                let p = self.pos;
                match el {
                    Element::Ice => {
                        for (s, n) in [(35, 60), (66, 60), (66, 50), (35, 52)] {
                            se(cx, s, p, n);
                        }
                    }
                    Element::Lightning => {
                        let id = b.effects.create(EffKind::WaveShock);
                        cx.out(Out::Effect { id, kind: EffKind::WaveShock, pos: p, dirc: VF0 });
                        se(cx, 35, p, 53);
                        se(cx, 68, p, 60);
                    }
                    Element::Blaze => se(cx, 35, p, 60),
                }
            }
        }
        if self.eny_flg != 0 {
            if el != Element::Ice {
                self.eny_int += 1;
            }
            if self.hits {
                cx.quake_vec([0x4120_0000; 3]);
            }
        }
        let mut alive = true;
        let due = match el {
            Element::Ice => ee::lt(self.transparency, 0),
            _ => self.eny_int >= 21,
        };
        if due {
            let mode = match el {
                Element::Ice => 5,
                Element::Lightning => 6,
                Element::Blaze => 7,
            };
            if !self.hits || magic_damage(b, x, mode, cx) {
                alive = false;
            }
        }
        self.t = match el {
            Element::Ice => {
                let t = ee::add(self.t, 0x3ca3_d70a);
                if dp(t) > 0.35 { ee::add(t, 0x3ba3_d70a) } else { t }
            }
            Element::Lightning => ee::add(self.t, 0x3c75_c28f),
            Element::Blaze => ee::add(self.t, 0x3c54_fdf4),
        };
        alive
    }
}

/// `ccBoss02::MagicDamage(pos, mode)` (0x0049a7d0), the first missile's
/// callback: the skill `mode` on the target at its first call, the camera
/// shaken for 20 calls, and at the 60th the magic over (true).
fn magic_damage(b: &mut Boss, x: &mut Innis, mode: i32, cx: &mut Cx) -> bool {
    if x.eny_int == 0 {
        let t = cx.scene.chars[cx.me].target_char;
        b.skill_damage(cx, t, None, mode as usize);
    }
    x.eny_int += 1;
    if x.eny_int < 21 {
        let q = x.quake_vector;
        cx.quake_vec([q[0], q[1], q[2]]);
    }
    if x.eny_int == 60 {
        cx.out(Out::Cinema(None));
        b.change_action(cx, act::NEUTRAL, 1, true);
        b.unlock_player();
        cx.out(Out::CamMode { mode: 6 });
        return true;
    }
    false
}

// --- Think ------------------------------------------------------------------------------------

/// `ccBoss02::Think` (0x004964b0).
fn think(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    x.af_cou += 1;
    if x.af_timing < x.af_cou {
        x.af_cou = 0;
    }
    dmg_check_count(b, x, cx);
    match b.act_num {
        act::NEUTRAL | act::EPITAPH_NEUTRAL => {
            if x.ex_spin_back_flg == 0 {
                if x.end_flg == 0 && x.action_start_flg == ONE {
                    action_end(b, x, cx);
                }
                if x.end_flg == 1 && geom::eq(x.action_start_flg, 0) {
                    action_decision(b, x, cx);
                }
            }
            let me = cx.me;
            if cx.valid(cx.scene.chars[me].target_char) {
                if geom::eq(x.action_start_flg, 0) {
                    return;
                }
                if cx.scene.chars[me].cond[cond::HOLD] == 0 || x.ex_spin_back_flg != 0 || x.escape != 0 {
                    let f = x.move_flg;
                    switch_action_pattern(b, x, f, cx);
                } else {
                    b.move_spd = 0;
                }
            } else {
                set_camera_eyes(b, x, cx);
                take_target(b, x, cx);
            }
        }
        act::DAMAGE0 | act::DAMAGE1 => particular_count(x),
        3..=8 | act::EPITAPH_SPIN | act::DEAD => {}
        act::EPITAPH => {
            x.epitaph_flg = 1;
            x.now_mode = 3;
        }
        _ => b.change_action(cx, act::NEUTRAL, 0, true),
    }
}

/// `targetChar = RndTarget()`; with one, `moveDirc` its heading and
/// `PosA` its distance.
fn take_target(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    let t = rnd_target(b, cx);
    cx.scene.chars[cx.me].target_char = t;
    if t.is_some() {
        b.move_dirc = b.target_dirc;
        x.pos_a = b.target_dist;
    }
}

/// `ccBoss02::RndTarget` (0x00499950): `SelectTarget(|ccRand() % 15|,
/// 1e6)`, else the first member alive on the party's list.
fn rnd_target(b: &mut Boss, cx: &mut Cx) -> Option<usize> {
    let ty = rand_mod(cx, 15).abs();
    let t = b.select_target(cx, ty, FAR);
    if t.is_some() {
        return t;
    }
    cx.scene.pc_list.iter().copied().find(|&c| cx.scene.chars[c].hp > 0)
}

/// `ccBoss02::ParticularCount` (0x00499a90): the waits count on through
/// a hit.
fn particular_count(x: &mut Innis) {
    match x.move_flg {
        15 | 14 | 5 => x.cou += 1,
        4 => x.cou += 2,
        _ => {}
    }
}

/// `ccBoss02::ActionEnd` (0x00496f50).
fn action_end(_b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    if cx.scene.chars[cx.me].target_char.is_some() {
        x.no += 1;
    }
    x.end_flg = 1;
    x.action_start_flg = 0;
    if x.sub_no == 0 {
        x.move_flg = 0;
    }
}

/// `ccBoss02::ActionDecision` (0x00496a50): the pattern run by the
/// protect gauge, the next word, the target.
fn action_decision(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    x.ex_spin_back_flg = 0;
    let me = cx.me;
    if x.epitaph_flg == 0 {
        let (pp, count) = cx.scene.chars[me].foe_state().map_or((0, 0), |f| (f.pp, f.pp_count));
        let max = f64::from(cx.t.bosses.get(1).map_or(0, |r| r.max_pp));
        if count == 0 && 0.2 * max >= f64::from(pp) {
            if x.now_mode != 0 {
                x.no = -1;
                x.now_mode = 0;
            }
        } else if count == 0 && 0.7 * max >= f64::from(pp) {
            if x.now_mode != 1 {
                x.no = x.pat_end_mode[0] + 1;
                x.now_mode = 1;
            }
        } else if x.now_mode != 2 {
            x.no = x.pat_end_mode[1] + 1;
            x.now_mode = 2;
        }
    }
    if x.move_flg == 0 {
        match x.epitaph_flg {
            1 => x.move_flg = word(&cx.data.innis.epitaph, x.no),
            0 => x.move_flg = word(&cx.data.innis.pattern, x.no),
            _ => {}
        }
    }
    if x.move_flg == 7 {
        x.cou = 20;
    }
    match x.lock_target_flg {
        2 => take_target(b, x, cx),
        0 => {
            take_target(b, x, cx);
            x.lock_target_flg = 1;
        }
        1 | 3 => {
            if !cx.valid(cx.scene.chars[me].target_char) {
                take_target(b, x, cx);
                x.lock_target_flg = 3;
            } else if x.lock_target_flg == 3 {
                x.lock_target_flg = 1;
            }
        }
        _ => {}
    }
    x.cou = 0;
    x.sub_no = 0;
    x.action_start_flg = ONE;
    let t = cx.scene.chars[me].target_char;
    if t.is_some() {
        x.target_pos_of = t;
    } else {
        x.end_flg = 0;
        x.action_start_flg = ONE;
    }
    if x.escape != 0 {
        x.escape = 0;
    }
}

/// `ccBoss02::ResetData` (0x00496900): an action over.
fn reset_data(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    if b.lock_player == 0 && b.erase_target != 0 {
        b.body_hit_sw = 1;
        b.entry_cmnd_target(cx);
        cx.out(Out::CursorOff(false));
        x.entry_flg = 0;
    }
    x.action_flg = 0;
    x.action_start_flg = ONE;
    x.cou = 0;
    b.move_spd = 0;
    x.end_flg = 0;
    x.p_flg = 1;
    x.eny_flg = 1;
    x.eny_int = 0;
    x.eny_float = 0;
    x.back_step_dist = 0;
    b.spd_up(0, 0x40a0_0000);
    x.dmg_count = i32::from(cx.scene.chars[cx.me].hp);
    x.ex_spin_back_flg = 0;
    x.escape_act_cou = 0;
}

/// `ccBoss02::SetCameraEyes` (0x00499b40): face the camera (its heading
/// turned round), 256 at a time.
fn set_camera_eyes(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    x.cam_rot = cx.cam.rot;
    let to = wrap(ee::add(x.cam_rot[2], PI));
    b.dirc[2] = geom::set_dirc(b.dirc[2], to, 256);
}

/// `ccBoss02::CheckRange(pos)` (0x004997d0): outside the arena
/// (`vecCenter` +- 2200, each axis, in doubles).
fn check_range(x: &Innis, pos: V4) -> bool {
    let ax = dp(pos[0]).abs() as f32;
    let ay = dp(pos[1]).abs() as f32;
    let lx = (dp(RANGE) + dp(x.vec_center[0]).abs()) as f32;
    let ly = (dp(RANGE) + dp(x.vec_center[1]).abs()) as f32;
    !(ee::le(ax.to_bits(), lx.to_bits()) && ee::le(ay.to_bits(), ly.to_bits()))
}

/// `ccBoss02::CharALLHold` (0x00499c10).
fn char_all_hold(cx: &mut Cx) {
    for m in cx.party.members.into_iter().flatten() {
        if cx.valid(Some(m)) {
            cx.affect(m, 5, -1);
        }
    }
}

/// `ccBoss02::DmgCheckCount` (0x00499d10): enough damage since the last
/// check (a fifth of `DmgWait` before the drain, all of it after) spins
/// Innis away.
fn dmg_check_count(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    let hp = i32::from(cx.scene.chars[cx.me].hp);
    if b.act_num == act::EPITAPH && x.epitaph_flg == 0 {
        x.dmg_count = hp;
        return;
    }
    match x.epitaph_flg {
        0 => {
            let d = x.dmg_count - hp;
            if d * 10 < x.dmg_wait * 2 {
                if d < 0 {
                    x.dmg_count = hp;
                }
                return;
            }
            x.dmg_count = hp;
            if matches!(x.move_flg, 4..=6) {
                b.change_action(cx, act::NEUTRAL, 0, true);
                spin_away(b, x, cx);
            } else {
                x.ex_spin_back_flg = 1;
                if b.act_num < 3 {
                    b.change_action(cx, act::NEUTRAL, 0, true);
                }
            }
        }
        1 => {
            if x.dmg_count - hp < x.dmg_wait {
                return;
            }
            x.dmg_count = hp;
            if matches!(x.move_flg, 4 | 6) {
                b.change_action(cx, act::EPITAPH_NEUTRAL, 0, true);
                x.action_start_flg = ONE;
                spin_away(b, x, cx);
            } else {
                x.ex_spin_back_flg = 1;
                b.change_action(cx, act::EPITAPH_NEUTRAL, 0, true);
            }
        }
        _ => {}
    }
}

/// The waits cut short: `MoveFlg` 15 (the escape, turned back), off the
/// party's targets.
fn spin_away(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    x.move_flg = 15;
    x.eny_int = 0;
    x.ex_spin_back_flg = 1;
    b.erase_cmnd_target(cx);
}

// --- SwitchActionPattern ------------------------------------------------------------------------

/// `EntryFlg` 1 unless it is -1 (the action begins off the targets).
fn entry_flg(x: &mut Innis) {
    if x.entry_flg != -1 {
        x.entry_flg = 1;
    }
}

/// `EntryAfterImage(&t)` when `AFcou` is `at`.
fn after_image(x: &Innis, at: i32, cx: &mut Cx) {
    if x.af_cou == at {
        cx.out(Out::AfterImage);
    }
}

/// `ccBoss02::SwitchActionPattern(pattern)` (0x00496fc0): the word's
/// action, a frame of it.
fn switch_action_pattern(b: &mut Boss, x: &mut Innis, pattern: i32, cx: &mut Cx) {
    set_camera_eyes(b, x, cx);
    let field = vec3(0, 0, 0x4348_0000);
    if x.dd_flg == 0 {
        cx.out(Out::Particles { which: Gen::InisField(2), pos: field });
        cx.out(Out::Particles { which: Gen::InisField(3), pos: field });
        x.dd_flg = 1;
    }
    if x.dd_flg == 1 {
        cx.out(Out::Particles { which: Gen::InisField(0), pos: field });
        cx.out(Out::Particles { which: Gen::InisField(1), pos: field });
        x.dd_flg = -1;
    }
    let at = x.af_timing;
    match pattern {
        1 => {
            entry_flg(x);
            after_image(x, at, cx);
            pattern01(b, x, cx);
        }
        2 => {
            entry_flg(x);
            after_image(x, at, cx);
            pattern06(b, x, cx);
        }
        3 => {
            entry_flg(x);
            after_image(x, at, cx);
            pattern02(b, x, cx);
        }
        7 => {
            entry_flg(x);
            after_image(x, at, cx);
            pattern07(b, x, cx);
        }
        8 => {
            entry_flg(x);
            after_image(x, 1, cx);
            pattern05(b, x, 2200, cx);
        }
        9 => {
            entry_flg(x);
            pattern05(b, x, 500, cx);
        }
        10 => {
            entry_flg(x);
            after_image(x, 1, cx);
            pattern11(b, x, 2200, cx);
        }
        11 => {
            entry_flg(x);
            if x.back_step_dist == 0 {
                x.no += 1;
                x.back_step_dist = word(&cx.data.innis.pattern, x.no);
                if x.back_step_dist <= 0 {
                    x.back_step_dist = 1;
                }
            }
            let d = x.back_step_dist;
            pattern05(b, x, d, cx);
        }
        13 => {
            entry_flg(x);
            after_image(x, at, cx);
            pattern08(b, x, cx);
        }
        4 => pattern03(b, x, 200, cx),
        5 => pattern03(b, x, 100, cx),
        6 => pattern03(b, x, 20, cx),
        22 => checkstate(b, x, cx),
        16 => {
            if x.epitaph_flg == 0 {
                b.change_action(cx, act::SPIN, 1, true);
            }
            if x.epitaph_flg == 1 {
                b.change_action(cx, act::EPITAPH_SPIN, 3, true);
            }
            cx.se3d(223);
            x.ex_spin_back_flg = 0;
            x.end_flg = 0;
            x.action_start_flg = ONE;
        }
        17 => {
            x.mirror_pros = 0;
            entry_flg(x);
            enemy_attack(b, x, cx);
        }
        19 => {
            entry_flg(x);
            magic_square(b, x, cx);
        }
        20 => skill_attack(b, x, cx),
        21 => {
            b.lock_player(cx, false);
            b.change_action(cx, act::DRAIN, 0, true);
            reset_data(b, x, cx);
        }
        25 => {
            entry_flg(x);
            x.escape = 1;
            x.lock_target_flg = 0;
            reset_data(b, x, cx);
        }
        26 => {
            x.lock_target_flg = 2;
            reset_data(b, x, cx);
            entry_flg(x);
        }
        27 => {
            x.escape = 1;
            entry_flg(x);
            x.blur = x.ex_argb;
            cx.out(Out::Blur { colour: x.blur });
            b.lock_player(cx, false);
            reset_data(b, x, cx);
        }
        28 => {
            entry_flg(x);
            x.blur = x.default_argb;
            cx.out(Out::Blur { colour: x.blur });
            reset_data(b, x, cx);
            b.unlock_player();
        }
        29 => {
            x.no = match x.now_mode {
                1 => x.pat_end_mode[0] + 1,
                2 => x.pat_end_mode[1] + 1,
                0 | 3 => -1,
                _ => x.no,
            };
            x.end_flg = 0;
            x.action_start_flg = ONE;
            x.pat_end_flg = 1;
            x.ex_spin_back_flg = 0;
        }
        14 => pattern10(b, x, cx),
        15 => {
            x.ex_spin_back_flg = 1;
            pattern10(b, x, cx);
        }
        _ => {
            entry_flg(x);
            x.end_flg = 0;
            x.action_start_flg = ONE;
            x.ex_spin_back_flg = 0;
            x.pat_end_flg = 1;
        }
    }
    if x.entry_flg == 1 {
        b.erase_cmnd_target(cx);
        x.entry_flg = -1;
    }
}

/// `ccBoss02::Pattern01` (0x00497b80): run at the target, easing to an
/// eighth of the gap, until within 500.
fn pattern01(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    b.move_dirc = b.target_dirc;
    x.pos_b = ee::div(ee::sub(x.pos_a, x.pos_b), 0x4100_0000);
    b.spd_up(x.pos_b, 0x40a0_0000);
    if ee::le(b.target_dist, 0x43fa_0000) {
        reset_data(b, x, cx);
    }
}

/// The step past the arena's edge: over, and the escape next.
fn out_of_range(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    reset_data(b, x, cx);
    x.sub_no = x.no;
    x.move_flg = 14;
    x.eny_int = 0;
}

/// `ccBoss02::Pattern02` (0x00497c40): off to one side of the camera's
/// heading (pi / 3 either way by `ccRand`), 900 farther at most.
fn pattern02(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    if x.p_flg != 0 {
        x.p_dist = b.target_dist;
        x.p_flg = 0;
        let side = cx.cc.rand() % 2 != 0;
        let r = cx.cam.rot[2];
        b.move_dirc = if side { ee::sub(r, THIRD_PI) } else { ee::add(THIRD_PI, r) };
    }
    b.spd_up(0x4316_0000, 0x40a0_0000);
    if check_range(x, cx.pos()) {
        out_of_range(b, x, cx);
    } else if ee::lt(ee::add(0x4461_0000, x.p_dist), b.target_dist) {
        reset_data(b, x, cx);
    }
}

/// `ccBoss02::Pattern03(time)` (0x00497df0): a wait, back on the targets.
fn pattern03(b: &mut Boss, x: &mut Innis, time: i32, cx: &mut Cx) {
    x.cou += 1;
    if b.erase_target != 0 {
        b.body_hit_sw = 1;
        b.entry_cmnd_target(cx);
        cx.out(Out::CursorOff(false));
        x.entry_flg = 0;
    }
    if time < x.cou {
        reset_data(b, x, cx);
    }
}

/// `ccBoss02::Pattern05(dist)` (0x00497eb0): straight along the camera's
/// heading (back toward it once off the edge), `dist` farther.
fn pattern05(b: &mut Boss, x: &mut Innis, dist: i32, cx: &mut Cx) {
    if x.p_flg != 0 {
        x.p_dist = b.target_dist;
        x.p_flg = 0;
        let r = cx.cam.rot[2];
        b.move_dirc = wrap(if x.action_flg == 0 { r } else { ee::add(PI, r) });
    }
    b.spd_up(0x4348_0000, 0x41f0_0000);
    if check_range(x, cx.pos()) {
        out_of_range(b, x, cx);
    } else if ee::lt(ee::add(x.p_dist, ee::from_int(dist)), b.target_dist) {
        reset_data(b, x, cx);
    }
}

/// `ccBoss02::Pattern06` (0x004980b0): a dash at the target.
fn pattern06(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    b.move_dirc = b.target_dirc;
    let d = b.target_dist;
    if ee::le(d, 0x43fa_0000) {
        reset_data(b, x, cx);
    } else if x.eny_int < 20 {
        b.spd_up(0x4316_0000, 0x4248_0000);
        x.eny_int += 1;
    } else if ee::lt(d, 0x4461_0000) {
        b.spd_up(0x42c8_0000, 0x4248_0000);
        x.eny_int = 30;
    }
}

/// `ccBoss02::Pattern07` (0x004981d0): once round the target, a
/// thirty-sixth of a turn a frame, either way.
fn pattern07(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    let step = ee::div(PI, 0x4190_0000);
    let me = cx.me;
    if x.eny_flg != 0 {
        b.move_spd = 0;
        b.move_dirc = 0;
        x.rot = 0;
        x.sub_rot = 0;
        x.eny_int = (cx.cc.rand() % 2).abs();
        x.eny_flg = 0;
        x.rot_vec = cx.scene.chars[me].pos;
        x.sub_vec = VF0;
        if let Some(t) = cx.scene.chars[me].target_char {
            x.sub_vec = cx.scene.chars[t].pos;
        }
    }
    x.rot = if x.eny_int == 1 { ee::add(x.rot, step) } else { ee::sub(x.rot, step) };
    x.sub_rot = ee::add(x.sub_rot, step);
    x.rot = wrap(x.rot);
    if ee::lt(ee::sub(TWO_PI, step), x.sub_rot) {
        reset_data(b, x, cx);
    }
    let d = geom::vsub(x.rot_vec, x.sub_vec);
    let m = geom::rot_matrix_z(&geom::unit_matrix(), x.rot);
    let p = geom::vadd(geom::apply_matrix(&m, d), x.sub_vec);
    cx.scene.chars[me].pos = p;
}

/// `ccBoss02::Pattern08` (0x00498500): a zigzag off the camera's heading,
/// a new leg every 10 frames, 31 frames.
fn pattern08(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    if x.cou == 0 && check_range(x, cx.pos()) {
        x.action_flg = 1;
    }
    b.spd_up(0x4316_0000, 0x4316_0000);
    if x.cou >= 31 {
        reset_data(b, x, cx);
        return;
    }
    if x.cou % 10 == 0 {
        let pos = cx.pos();
        cx.out(Out::Se3dNote { se: 57, pos, note: 55 });
        let r = cx.cam.rot[2];
        if x.eny_flg != 0 {
            b.move_dirc = ee::add(THIRD_PI, r);
            x.eny_flg = 0;
        } else {
            b.move_dirc = ee::sub(r, THIRD_PI);
            x.eny_flg = 1;
        }
        if x.action_flg == 1 {
            b.move_dirc = ee::add(b.move_dirc, PI);
        }
        b.move_dirc = wrap(b.move_dirc);
    }
    x.cou += 1;
    if x.action_flg == 0 && check_range(x, cx.pos()) {
        reset_data(b, x, cx);
        if ee::to_int(rand_f(cx.cc, 0x3fc0_0000)) < 0 {
            x.sub_no = x.no;
            x.move_flg = 14;
            x.eny_int = 0;
        }
    }
}

/// `ccBoss02::Pattern10` (0x004987d0): the escape: turn, fade out,
/// reappear 1100 from the world's origin at a random heading, fade in.
fn pattern10(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    let step = ee::div(PI, 0x4100_0000);
    let fade = 0x3ca3_d70a;
    let me = cx.me;
    match x.escape_act_cou {
        0 => {
            b.move_spd = 0;
            b.move_dirc = 0;
            x.rot = b.dirc[2];
            x.sub_rot = 0;
            x.eny_flg = 0;
            x.alpha = ONE;
            x.escape_act_cou = 1;
            x.sub_vec = VF0;
            x.rot_vec = cx.scene.chars[me].pos;
            if x.ex_spin_back_flg == 0 {
                b.body_hit_sw = 1;
                b.entry_cmnd_target(cx);
                cx.out(Out::CursorOff(false));
                x.cou = 0;
            } else {
                x.cou = 40;
                b.erase_cmnd_target(cx);
                cx.out(Out::CursorOff(true));
            }
        }
        1 => {
            x.cou += 1;
            if x.cou >= 40 {
                x.escape_act_cou = 2;
                cx.out(Out::Particles { which: Gen::Tornado, pos: x.rot_vec });
                b.erase_cmnd_target(cx);
            }
            set_camera_eyes(b, x, cx);
        }
        2 => {
            x.rot = turn(x.rot, step, 52, cx);
            b.dirc[2] = x.rot;
            x.alpha = ee::sub(x.alpha, fade);
            if !ee::le(0, x.alpha) {
                x.alpha = 0;
                x.escape_act_cou = 3;
                let m = geom::rot_matrix_z(&geom::unit_matrix(), rand_f(cx.cc, PI));
                let v = geom::apply_matrix(&m, [0x4489_8000, 0, 0, ONE]);
                let mut p = geom::vadd(v, x.sub_vec);
                p[2] = x.hipos;
                cx.scene.chars[me].pos = p;
            }
        }
        3 => {
            x.alpha = ee::add(x.alpha, fade);
            x.rot = turn(x.rot, step, 48, cx);
            b.dirc[2] = x.rot;
            if dp(x.alpha) > 1.0 {
                x.alpha = ONE;
                x.escape_act_cou = 4;
                x.cou = 30;
                b.body_hit_sw = 1;
                b.entry_cmnd_target(cx);
                cx.out(Out::CursorOff(false));
            }
        }
        4 => {
            x.cou += 1;
            set_camera_eyes(b, x, cx);
            if x.cou >= 40 {
                x.escape_act_cou = 5;
            }
        }
        5 => {
            if x.ex_spin_back_flg == 0 {
                x.no -= 1;
            }
            reset_data(b, x, cx);
            x.move_flg = 0;
        }
        _ => x.escape_act_cou = 0,
    }
    if !ee::lt(x.alpha, ONE) {
        x.alpha = ONE;
    }
    if ee::le(x.alpha, 0) {
        x.alpha = 0;
    }
}

/// The escape's spin: `step` on, a whoosh (SE 169) each time it passes pi.
fn turn(r: F, step: F, note: u8, cx: &mut Cx) -> F {
    let mut r = ee::add(r, step);
    if !ee::le(r, PI) {
        r = ee::sub(r, TWO_PI);
        let pos = cx.pos();
        cx.out(Out::Se3dNote { se: 169, pos, note });
    }
    if ee::lt(r, NEG_PI) {
        r = ee::add(r, TWO_PI);
    }
    r
}

/// `ccBoss02::Pattern11(dist)` (0x00498eb0): away along the camera's
/// heading holding the party, until `dist` off (and the camera still).
fn pattern11(b: &mut Boss, x: &mut Innis, dist: i32, cx: &mut Cx) {
    let d = ee::from_int(dist);
    if x.cou == 0 {
        if check_range(x, cx.pos()) {
            x.action_flg = 1;
            x.eny_int = 35;
        }
        x.cou += 1;
        if ee::lt(d, b.target_dist) {
            reset_data(b, x, cx);
            return;
        }
    }
    if x.p_flg != 0 {
        x.p_dist = b.target_dist;
        x.p_flg = 0;
        let r = cx.cam.rot[2];
        b.move_dirc = wrap(if x.action_flg == 0 { r } else { ee::add(PI, r) });
    }
    char_all_hold(cx);
    if ee::lt(d, b.target_dist) {
        if !cx.cam.moving {
            reset_data(b, x, cx);
        } else {
            b.move_spd = 0;
        }
    } else {
        b.spd_up(0x437a_0000, 0x40a0_0000);
    }
}

/// `ccBoss02::Checkstate` (0x00499770): not near the target, back two
/// words.
fn checkstate(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    if !ee::le(b.target_dist, 0x43fa_0000) {
        x.no -= 2;
    }
    reset_data(b, x, cx);
}

/// `ccBoss02::GetRndSkill_INIS(atb)` (0x004996c0): 0 one of
/// `Skill_VARIOUS_INIS`, 1 of `Skill_DOWNER_INIS`, else skill 1.
fn rnd_skill(atb: i32, cx: &mut Cx) -> i32 {
    let (tbl, n) = match atb {
        0 => (&cx.data.innis.various, 7),
        1 => (&cx.data.innis.downer, 12),
        _ => return 1,
    };
    let tbl = tbl.clone();
    let k = rand_mod(cx, n).unsigned_abs() as usize;
    i32::from(tbl.get(k).copied().unwrap_or(0))
}

/// `ccBoss02::SkillAttack` (0x00499110): the camera closes on the target,
/// a skill named by the next word at count 60, over at 90.
fn skill_attack(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    let me = cx.me;
    let target = cx.scene.chars[me].target_char;
    if !cx.valid(target) {
        b.unlock_player();
        reset_data(b, x, cx);
        b.entry_cmnd_target(cx);
        cx.out(Out::CursorOff(false));
        return;
    }
    let c = x.cou;
    x.cou += 1;
    if c == 90 {
        cx.out(Out::Cinema(None));
        x.blur = x.default_argb;
        cx.out(Out::Blur { colour: x.blur });
        cx.out(Out::CamMode { mode: 6 });
        cx.out(Out::CamPitch { add: false, v: 0 });
        b.unlock_player();
        reset_data(b, x, cx);
        b.entry_cmnd_target(cx);
        cx.out(Out::CursorOff(false));
        return;
    }
    match x.cou {
        60 => {
            if let Some(tp) = target
                && let Some(s) = item::item_skill_request(cx.t, cx.scene, me, tp, x.skill_id, 0, false, cx.rand)
            {
                cx.out(Out::Skill(tp, s));
            }
        }
        30 => {
            x.zoom_vec = VF0;
            x.zoom_vec[2] = ee::add(x.zoom_vec[2], 0x437a_0000);
            x.zoom_vec = geom::vadd(x.zoom_vec, b.target_pos);
            cx.out(Out::Particles { which: Gen::InisField(4), pos: x.zoom_vec });
            let tp = b.target_pos;
            cx.out(Out::Se3d { se: 225, pos: tp });
            cx.out(Out::Se3d { se: 225, pos: tp });
            cx.out(Out::Se3d { se: 257, pos: tp });
        }
        1 => {
            b.lock_player(cx, false);
            cx.out(Out::Cinema(Some(70)));
            x.zoom_vec = VF0;
            x.zoom_view = VF0;
            x.cam_pos = cx.cam.pos;
            x.cam_view = cx.cam.view;
            let pos = cx.pos();
            x.cam_mat = geom::rot_matrix_z(&geom::unit_matrix(), get_dirc(b.target_pos, pos));
            cx.out(Out::CamMode { mode: 5 });
            x.zoom_vec[1] = ee::add(x.zoom_vec[1], 0x4448_0000);
            x.zoom_vec[2] = ee::add(x.zoom_vec[2], 0x4248_0000);
            // The view's x and y are the stack's leftover, SetCameraEyes'
            // saved return address, which the FPU reads as zero.
            let view = [pos[0], pos[1], ee::add(0x437a_0000, pos[2]), pos[3]];
            let eye = geom::vadd(geom::apply_matrix(&x.cam_mat, x.zoom_vec), b.target_pos);
            cx.out(Out::FreeCam { pos: eye, view });
            b.erase_cmnd_target(cx);
            cx.out(Out::CursorOff(true));
            x.no += 1;
            let atb = match x.epitaph_flg {
                1 => word(&cx.data.innis.epitaph, x.no),
                0 => word(&cx.data.innis.pattern, x.no),
                _ => return,
            };
            x.skill_id = rnd_skill(atb, cx);
        }
        _ => cx.out(Out::CamPitch { add: true, v: 0x3c23_d70a }),
    }
}

// --- the attacks ----------------------------------------------------------------------------------

/// `ccBoss02::EnemyAttack` (0x0049a900): with the gauge at half, a
/// `ccRand` drawn and dropped; the cinema, act 5.
fn enemy_attack(b: &mut Boss, _x: &mut Innis, cx: &mut Cx) {
    let me = cx.me;
    let max = i32::from(cx.t.bosses.get(1).map_or(0, |r| r.max_pp)) / 2;
    let pp = i32::from(cx.scene.chars[me].foe_state().map_or(0, |f| f.pp));
    if pp >= max {
        let _ = cx.cc.rand();
    }
    cx.out(Out::Cinema(Some(8)));
    b.change_action(cx, act::IMAGES, 3, true);
}

/// `ccBoss02::MagicSquare` (0x0049a000): the camera's moves, the square
/// (ice, lightning or fire by `ccRand() % 3`), and at count 10 after its
/// last move the missiles; act 4 until they land.
fn magic_square(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    let pos = cx.pos();
    let c = x.cou;
    match c {
        0 => {
            b.lock_player(cx, false);
            x.zoom_vec = VF0;
            x.zoom_view = VF0;
            x.cam_pos = cx.cam.pos;
            x.cam_view = cx.cam.view;
            x.cam_mat = geom::rot_matrix_z(&geom::unit_matrix(), b.dirc[2]);
            cx.out(Out::CamMode { mode: 5 });
            x.cou = 1;
            x.eny_flg = 0;
            x.blur = x.ex_argb;
            cx.out(Out::Blur { colour: x.blur });
            x.cam_dist = get_dist(x.cam_pos, pos);
            return;
        }
        1 if x.eny_flg == 0 => {
            x.cam_mat = geom::rot_matrix_z(&geom::unit_matrix(), b.dirc[2]);
            x.zoom_vec[1] = 0xc4bb_8000;
            x.zoom_vec[2] = 0x4348_0000;
            x.zoom_view[1] = ee::add(ONE, x.zoom_vec[1]);
            let eye = geom::vadd(geom::apply_matrix(&x.cam_mat, x.zoom_vec), pos);
            let view = geom::vadd(geom::apply_matrix(&x.cam_mat, x.zoom_view), x.cam_view);
            cx.out(Out::FreeCam { pos: eye, view });
            x.cou += 1;
        }
        2 if x.eny_flg == 0 => {
            x.blur = x.default_argb;
            cx.out(Out::Blur { colour: x.blur });
            b.erase_cmnd_target(cx);
            cx.out(Out::CursorOff(true));
            x.eny_int = rand_mod(cx, 3).abs();
            cx.out(Out::Cinema(Some(5 + x.eny_int)));
            let id = b.effects.create(EffKind::MagicSquare { n: x.eny_int });
            cx.out(Out::Effect { id, kind: EffKind::MagicSquare { n: x.eny_int }, pos, dirc: b.dirc });
            let sid = b.begin_stage_effect(cx, 0x6400_0000, [15, 15, 32277]);
            x.eny_float = ee::from_int(sid);
            x.zoom_vec = VF0;
            x.zoom_view = VF0;
            x.cam_pos = cx.cam.pos;
            x.cam_view = cx.cam.view;
            x.cam_mat = geom::rot_matrix_z(&geom::unit_matrix(), get_dirc(b.target_pos, pos));
            x.zoom_vec[1] = ee::add(x.zoom_vec[1], 0x442f_0000);
            x.zoom_vec[2] = ee::add(x.zoom_vec[2], 0x4248_0000);
            let eye = geom::vadd(geom::apply_matrix(&x.cam_mat, x.zoom_vec), x.cam_pos);
            cx.out(Out::FreeCam { pos: eye, view: pos });
            x.cou += 1;
        }
        50 if x.eny_flg == 0 => {
            x.zoom_vec = VF0;
            x.zoom_vec[1] = 0x448f_c000;
            x.zoom_vec[2] = 0x4316_0000;
            let eye = geom::vadd(geom::apply_matrix(&x.cam_mat, x.zoom_vec), b.target_pos);
            cx.out(Out::FreeCam { pos: eye, view: pos });
            x.zoom_vec[1] = ee::add(x.zoom_vec[1], 0x42b4_0000);
            x.zoom_view[1] = ee::sub(x.zoom_vec[1], ONE);
            x.eny_flg = 1;
            x.cou = 1;
        }
        1 | 2 | 50 => {}
        _ if x.eny_flg == 0 => {
            x.cou = c + 1;
            let mut eye = cx.cam.pos;
            eye[2] = ee::add(eye[2], 0x4120_0000);
            cx.out(Out::FreeCam { pos: eye, view: pos });
        }
        _ => {}
    }
    if x.eny_flg == 0 {
        return;
    }
    x.cou += 1;
    if x.cou != 10 {
        return;
    }
    match x.eny_int {
        0 => ice_attack(b, cx),
        1 => thunder_attack(b, cx),
        2 => blaze_attack(b, cx),
        _ => {}
    }
    let id = ee::to_int(x.eny_float);
    b.end_stage_effect(cx, id, 0x6400_0000, 35);
    reset_data(b, x, cx);
    b.change_action(cx, act::MAGIC, 3, true);
}

/// `ccBoss02::AllAttack` (0x0049aa10): the fall to the floor, the wave, a
/// ring, skill 4 on those near at count 40, back up to `Hipos`.
fn all_attack(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    let me = cx.me;
    match x.flg {
        1 => {
            let z = cx.scene.chars[me].pos[2];
            if ee::le(z, 0xc080_0000) {
                b.move_vector[2] = 0;
                cx.scene.chars[me].pos[2] = 0xc080_0000;
                x.flg += 1;
                x.anm_w.set(super::ANM_WAVE, cx.clips);
                x.quake_vector = [0x4170_0000, 0x4170_0000, 0x4170_0000, x.quake_vector[3]];
                x.sk_atk = Some(4);
            } else if x.temphi != 0 {
                b.move_vector[2] = 0xc2c8_0000;
            }
        }
        2 => {
            let range = x.sk_atk.and_then(|k| cx.t.boss_skills.get(k)).map_or(0, |s| s.target_range);
            let r = ee::from_int(ee::to_int(range));
            let pos = cx.pos();
            for m in cx.party.members.into_iter().flatten() {
                if cx.valid(Some(m)) && !ee::lt(r, get_dist(pos, cx.scene.chars[m].pos)) {
                    cx.affect(m, 5, -1);
                }
            }
            b.act_count += 1;
            if b.act_count < 51 {
                let q = x.quake_vector;
                cx.quake_vec([q[0], q[1], q[2]]);
            }
            if b.act_count == 3 {
                cx.out(Out::Flash { t: 30, colour: 0x80ff_ffff });
                let id = b.effects.create(EffKind::WaveShock);
                cx.out(Out::Effect { id, kind: EffKind::WaveShock, pos, dirc: b.dirc });
                x.eny_flg = 0;
            }
            if b.act_count == 30 {
                let k =
                    EffKind::SamonRing { model: 192, scale: [ONE; 4], spoint: [0x3f00_0000; 4], tpoint: 0x3ca3_d70a };
                let id = b.effects.create(k);
                cx.out(Out::Effect { id, kind: k, pos, dirc: VF0 });
                cx.out(Out::Particles { which: Gen::Burst(2), pos });
            }
            if b.act_count == 40 {
                b.skill_damage(cx, None, Some(pos), 4);
            }
            let z = cx.scene.chars[me].pos[2];
            if !ee::lt(x.hipos, z) {
                cx.scene.chars[me].pos[2] = ee::add(z, 0x4000_0000);
            }
            let done = x.anm_w.forward();
            cx.out(Out::DrawWave);
            if done {
                cx.scene.chars[me].pos[2] = x.hipos;
                b.act_proccess = 3;
                x.quake_vector = [0x40a0_0000, 0x40a0_0000, 0x40a0_0000, x.quake_vector[3]];
                b.body_hit_sw = 1;
                b.entry_cmnd_target(cx);
                cx.out(Out::CursorOff(false));
                if x.epitaph_flg == 0 {
                    b.change_action(cx, act::NEUTRAL, 0, true);
                }
                if x.epitaph_flg == 1 {
                    b.change_action(cx, act::EPITAPH_NEUTRAL, 3, true);
                }
            }
        }
        _ => {}
    }
}

/// `|(float)n|` as the attacks build their numbers: `abs(fptosi(n))`.
fn abs_f(n: i32) -> F {
    ee::from_int(ee::to_int(ee::from_int(n)).abs())
}

/// The three missiles' control points (`sp+128`: the boss, then two
/// spline points, then the target 10 up) and their making.
fn missile(b: &mut Boss, el: Element, ctrl: [V4; 4], first: bool, cx: &mut Cx) {
    let no = i32::from(first);
    let kind = EffKind::Missile { element: el, no, ctrl };
    let id = b.effects.create(kind);
    if let Some(e) = usize::try_from(id).ok().and_then(|k| b.effects.slots.get_mut(k)).and_then(|s| s.as_mut()) {
        e.missile = Some(Box::new(Missile::new(&ctrl, first)));
    }
    cx.out(Out::Effect { id, kind, pos: ctrl[0], dirc: VF0 });
}

/// `ccBoss02::IceAttack` (0x0049afa0).
fn ice_attack(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    // The dirc turned by ccRandF(0.2) is worked out and not used.
    let _ = rand_f(cx.cc, 0x3e4c_cccd);
    let pos = cx.pos();
    // The two spline points keep their y (and w) from the last missile.
    let (mut v2, mut v3) = (VF0, VF0);
    for s in 0..3 {
        v2[0] = abs_f(rand_mod(cx, 5) * 100 + 1200);
        v3[0] = abs_f(rand_mod(cx, 6) * 100);
        v2[2] = ee::add(0x43fa_0000, abs_f(rand_mod(cx, 5) * 100));
        v3[2] = ee::add(0x4448_0000, abs_f(rand_mod(cx, 5) * 100));
        let t = cx.scene.chars[me].target_char.map_or(VF0, |t| cx.scene.chars[t].pos);
        let mut v4 = t;
        v4[2] = 0x4120_0000;
        match s {
            0 => v3[0] = ee::from_int(rand_mod(cx, 5).abs() * 100 + 900),
            1 => {
                v2[0] = libm::neg(v2[0]);
                v3[0] = ee::from_int(-rand_mod(cx, 5).abs() * 100 - 900);
            }
            _ => {
                let k = dp(rand_f(cx.cc, ONE)) + 0.5;
                v2[0] = fp(dp(v2[0]) * k);
                v3[0] = ee::mul(v3[0], rand_f(cx.cc, 0x3e99_999a));
                v2[2] = ee::add(0x4248_0000, ee::from_int(rand_mod(cx, 5).abs() * 100 + 1200));
                v3[2] = ee::add(0x42c8_0000, ee::from_int(rand_mod(cx, 5).abs() * 100));
            }
        }
        let m = geom::rot_matrix_z(&geom::unit_matrix(), b.dirc[2]);
        v2 = geom::apply_matrix(&m, v2);
        let m = geom::rot_matrix_z(&m, NEG_PI);
        v3 = geom::apply_matrix(&m, v3);
        v2 = geom::vadd(v2, pos);
        v3 = geom::vadd(v3, v4);
        missile(b, Element::Ice, [pos, v2, v3, v4], s == 0, cx);
    }
}

/// `ccBoss02::ThunderAttack` (0x0049b560).
fn thunder_attack(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    let mut turn = wrap(ee::add(b.dirc[2], rand_f(cx.cc, 0x3e4c_cccd)));
    let pos = cx.pos();
    // The two spline points keep their y (and w) from the last missile.
    let (mut v2, mut v3) = (VF0, VF0);
    for s in 0..3 {
        v2[0] = abs_f(rand_mod(cx, 5) * 100 + 1200);
        v3[0] = abs_f(rand_mod(cx, 6) * 100);
        v2[2] = ee::add(0x43fa_0000, abs_f(rand_mod(cx, 5) * 100));
        v3[2] = ee::add(0x4448_0000, abs_f(rand_mod(cx, 5) * 100));
        let t = cx.scene.chars[me].target_char.map_or(VF0, |t| cx.scene.chars[t].pos);
        let mut v4 = t;
        v4[2] = 0x4120_0000;
        match s {
            0 => v3[0] = ee::from_int(rand_mod(cx, 5) * 100 + 200),
            1 => {
                v2[0] = libm::neg(v2[0]);
                v3[0] = ee::from_int(-rand_mod(cx, 5) * 100 + 200);
            }
            _ => {
                let k = dp(rand_f(cx.cc, ONE)) + 0.5;
                v2[0] = fp(dp(v2[0]) * k);
                v3[0] = ee::mul(v3[0], rand_f(cx.cc, 0x3e99_999a));
                turn = b.dirc[2];
                v2[2] = ee::add(0x4248_0000, abs_f(rand_mod(cx, 5) * 100 + 800));
                v3[2] = ee::add(0x42c8_0000, ee::from_int(rand_mod(cx, 5).abs() * 100));
            }
        }
        let m = geom::rot_matrix_z(&geom::unit_matrix(), turn);
        v2 = geom::apply_matrix(&m, v2);
        v3 = geom::apply_matrix(&m, v3);
        v2 = geom::vadd(v2, pos);
        v3 = geom::vadd(v3, v4);
        missile(b, Element::Lightning, [pos, v2, v3, v4], s == 0, cx);
    }
}

/// `ccBoss02::BlazeAttack` (0x0049bb30).
fn blaze_attack(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    let mut turn = wrap(ee::add(b.dirc[2], rand_f(cx.cc, 0x3e4c_cccd)));
    let pos = cx.pos();
    // The two spline points keep their y (and w) from the last missile.
    let (mut v2, mut v3) = (VF0, VF0);
    for s in 0..3 {
        v2[0] = abs_f(rand_mod(cx, 5) * 100 + 1200);
        v3[0] = abs_f(rand_mod(cx, 6) * 100);
        v2[2] = ee::add(0x43fa_0000, abs_f(rand_mod(cx, 5) * 100));
        v3[2] = ee::add(0x4448_0000, abs_f(rand_mod(cx, 5) * 100));
        let t = cx.scene.chars[me].target_char.map_or(VF0, |t| cx.scene.chars[t].pos);
        let mut v4 = t;
        v4[2] = 0x4120_0000;
        match s {
            0 => v3[0] = ee::from_int(rand_mod(cx, 5).abs() * 100 + 400),
            1 => {
                v2[0] = libm::neg(v2[0]);
                v3[0] = ee::from_int(-rand_mod(cx, 5).abs() * 100 - 400);
            }
            _ => {
                let k = dp(rand_f(cx.cc, ONE)) + 0.5;
                v2[0] = fp(dp(v2[0]) * k);
                v3[0] = ee::mul(v3[0], rand_f(cx.cc, 0x3e99_999a));
                turn = b.dirc[2];
                v2[2] = ee::add(0x4248_0000, abs_f(rand_mod(cx, 5) * 100 + 800));
                v3[2] = ee::add(0x42c8_0000, ee::from_int(rand_mod(cx, 5).abs() * 100));
            }
        }
        let m = geom::rot_matrix_z(&geom::unit_matrix(), turn);
        v2 = geom::apply_matrix(&m, v2);
        v3 = geom::apply_matrix(&m, v3);
        v2 = geom::vadd(v2, pos);
        v3 = geom::vadd(v3, v4);
        missile(b, Element::Blaze, [pos, v2, v3, v4], s == 0, cx);
    }
}

// --- Action ------------------------------------------------------------------------------------

/// `ccBoss02::Action` (0x00494e10).
fn action(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    let me = cx.me;
    match b.act_num {
        act::NEUTRAL | 8 | act::EPITAPH_NEUTRAL => {}
        act::DAMAGE0 | act::DAMAGE1 => {
            if b.act_proccess == 0 {
                let base = if b.act_num == act::DAMAGE0 { 63 } else { 67 };
                let note = (base + cx.cc.rand() % 4) as u8;
                let pos = cx.pos();
                cx.out(Out::Se3dNote { se: 195, pos, note });
                b.act_proccess += 1;
            }
            if b.anm_status != 0 {
                back_to_neutral(b, x, 0, cx);
            }
            b.spd_down(0, 0x40a0_0000);
        }
        act::SHAKE => match b.act_proccess {
            0 => {
                b.anm.speed = 3840;
                b.act_proccess += 1;
                x.blur = x.ex_argb;
                cx.out(Out::Blur { colour: x.blur });
            }
            1 => {
                let c = b.act_count;
                b.act_count += 1;
                if c < 121 {
                    if b.anm_status != 0 {
                        if let Some(Some(clip)) = b.anm_tbl.get(act::SHAKE as usize).cloned() {
                            b.anm.set(&clip, cx.clips);
                        }
                        let note = (67 + cx.cc.rand() % 4) as u8;
                        let pos = cx.pos();
                        cx.out(Out::Se3dNote { se: 195, pos, note });
                    }
                } else {
                    x.blur = x.default_argb;
                    cx.out(Out::Blur { colour: x.blur });
                    b.anm.speed = 256;
                    b.act_proccess += 1;
                    b.change_action(cx, act::NEUTRAL, 0, true);
                }
            }
            _ => {}
        },
        act::SPIN | act::EPITAPH_SPIN => {
            b.spd_down(0, 0x40a0_0000);
            match b.act_proccess {
                0 => {
                    x.rotate = 0x3dcc_cccd;
                    x.dircsub = b.dirc[2];
                    x.sub_hi = cx.scene.chars[me].pos[2];
                    x.flg = -1;
                    x.temphi = 0;
                    b.move_dirc = 0;
                    b.dirc[2] = geom::set_dirc(b.dirc[2], b.target_dirc, 256);
                    b.erase_cmnd_target(cx);
                    b.act_proccess += 1;
                }
                1 if x.temphi < 25 => {
                    b.move_vector[2] = 0x4248_0000;
                    x.temphi += 1;
                    x.rotate = ee::add(x.rotate, 0x3f19_999a);
                    b.dirc[2] = ee::add(x.dircsub, x.rotate);
                    if dp(x.rotate) > f64::from_bits(0x4009_1eb8_51eb_851f) {
                        x.flg = 1;
                        x.temphi += 1;
                        x.rotate = 0xc048_f5c3;
                    }
                }
                1 if b.act_num == act::EPITAPH_SPIN => all_attack(b, x, cx),
                1 => {
                    cx.se3d(224);
                    b.act_proccess = 2;
                }
                2 if b.act_num == act::SPIN => all_attack(b, x, cx),
                _ => {}
            }
        }
        act::MAGIC => {
            set_camera_eyes(b, x, cx);
            if b.act_proccess == 0 {
                b.spd_down(0, 0x40a0_0000);
                b.lock_player(cx, false);
            }
        }
        act::IMAGES => images(b, x, cx),
        act::DRAIN => {
            set_camera_eyes(b, x, cx);
            let t = cx.scene.chars[me].target_char;
            match b.act_proccess {
                0 => {
                    if cx.valid(t) {
                        let id = t.map_or(-1, |t| i32::from(cx.scene.chars[t].id()));
                        let slot = cx.party.slot_of(id);
                        cx.out(Out::StreamMenu { stream: 42, mask: 1 << slot });
                        b.act_proccess = 1;
                    } else {
                        b.act_proccess = 5;
                    }
                }
                1 if cx.env.menu_type == -1 => {
                    x.ex_spin_back_flg = 0;
                    if let Some(tp) = t {
                        cx.affect(tp, 13, 0);
                    }
                    b.change_action(cx, act::NEUTRAL, 1, true);
                    b.unlock_player();
                }
                5 => {
                    x.ex_spin_back_flg = 0;
                    b.unlock_player();
                    b.change_action(cx, act::NEUTRAL, 1, true);
                }
                _ => {}
            }
        }
        act::EPITAPH => {
            b.cheat_hp = 0;
            b.anm.set("ANM_ex2xnut0", cx.clips);
            b.change_action(cx, act::EPITAPH_NEUTRAL, 3, true);
            reset_data(b, x, cx);
            x.no = -1;
        }
        act::DEAD => match b.act_proccess {
            0 => {
                b.body_hit_sw = 0;
                crate::fellow::delete_cmnd(cx.scene, me);
                cx.out(Out::DeleteCmnd);
                b.move_spd = 0;
                b.move_vector = VF0;
                x.alpha = ONE;
                x.dead_eff = Some(b.begin_dead_effect(cx, 0x455a_c000, 0x42c8_0000));
                b.act_proccess += 1;
            }
            1 => {
                if !b.effects.enabled(x.dead_eff) {
                    b.act_proccess += 1;
                }
            }
            2 => {
                let c = b.act_count;
                b.act_count += 1;
                if c == 30 {
                    b.exit = 1;
                }
            }
            _ => {}
        },
        _ => b.change_action(cx, act::NEUTRAL, 0, true),
    }
}

/// A damage act over: back to 0, or 12 once drained.
fn back_to_neutral(b: &mut Boss, x: &Innis, forbid: i16, cx: &mut Cx) {
    if x.epitaph_flg == 0 {
        b.change_action(cx, act::NEUTRAL, forbid, true);
    }
    if x.epitaph_flg == 1 {
        b.change_action(cx, act::EPITAPH_NEUTRAL, forbid, true);
    }
}

/// Act 5 (0x00495448): the stream of the images, the camera behind the
/// target, the three images sent at it; over once all three have broken.
fn images(b: &mut Boss, x: &mut Innis, cx: &mut Cx) {
    set_camera_eyes(b, x, cx);
    b.spd_down(0, 0x40a0_0000);
    let pos = cx.pos();
    match b.act_proccess {
        0 => {
            b.lock_player(cx, false);
            cx.out(Out::MenuForbid { on: true, chat_except: false });
            x.forbid_now = Some(1);
            let stream = 39 + x.monster_id.clamp(0, 2);
            if (0..3).contains(&x.monster_id) {
                cx.out(Out::StreamMenu { stream, mask: 0 });
            }
            b.act_proccess = 1;
            return;
        }
        1 => {
            if cx.env.menu_type != -1 {
                return;
            }
            b.act_proccess = 5;
            x.zoom_vec = VF0;
            x.cam_mat = geom::unit_matrix();
            cx.out(Out::CamMode { mode: 5 });
            x.cam_mat = geom::rot_matrix_z(&x.cam_mat, get_dirc(b.target_pos, pos));
            x.zoom_vec[1] = ee::add(x.zoom_vec[1], 0x44bb_8000);
            x.zoom_vec[2] = ee::add(x.zoom_vec[2], 0x43af_0000);
            let eye = geom::vadd(geom::apply_matrix(&x.cam_mat, x.zoom_vec), b.target_pos);
            cx.out(Out::FreeCam { pos: eye, view: pos });
            return;
        }
        5 => {
            let tp = match x.target_pos_of {
                Some(t) => cx.scene.chars[t].pos,
                None => x.init_vector,
            };
            for k in 0..x.slaves.len() {
                x.slaves[k].end_flg = 0;
                let id = x.monster_id;
                x.slaves[k].set_monster(id, cx);
                x.slaves[k].target_pos = tp;
                entry_slave(x);
            }
            b.act_proccess += 1;
        }
        6 => {}
        _ => return,
    }
    let mut done = false;
    for (k, s) in x.slaves.iter().enumerate() {
        if s.end_flg == 0 {
            break;
        }
        if k == 2 && s.end_flg == 1 {
            done = true;
        }
    }
    if !done {
        return;
    }
    b.unlock_player();
    cx.out(Out::CamMode { mode: 6 });
    b.act_proccess = 0;
    x.cou = 0;
    x.end_flg = 0;
    x.action_start_flg = ONE;
    x.ex_spin_back_flg = 0;
    x.monster_id += 1;
    if x.monster_id == 3 {
        x.monster_id = 0;
    }
    b.change_action(cx, act::NEUTRAL, 1, true);
    cx.out(Out::Cinema(None));
}

/// `ccBoss02::EntrySlave(0, 1)` (0x0049c1e0): the first exited image
/// back in, ordered out.
fn entry_slave(x: &mut Innis) {
    if let Some(s) = x.slaves.iter_mut().find(|s| s.b.exit != 0) {
        s.b.exit = 0;
        s.order_num = 1;
    }
}

// --- Affect ------------------------------------------------------------------------------------

/// `ccBoss02::Affect` (0x004960f0): as `ccBoss::Affect`, with the shield
/// for a spell's hit.
pub(super) fn affect(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    let (t, p0, p1, person) = {
        let a = &cx.scene.chars[me].affect;
        (a.ty, a.param[0], a.param[1], a.person)
    };
    if t != 21 && (b.act_forbid == 2 || b.lock_player != 0) {
        return;
    }
    let hp = cx.scene.chars[me].hp;
    let mhp = cx.scene.chars[me].max_hp;
    match t {
        6 => b.stop = 0,
        5 => b.stop = 1,
        21 => {
            let v = (i32::from(mhp) / 10) as i16;
            let ch = &mut cx.scene.chars[me];
            ch.hp = v;
            ch.max_hp = v;
            if let Some(f) = ch.foe_state_mut() {
                f.pp = -1;
                f.pp_count = 0;
            }
        }
        13 => {
            b.change_action(cx, act::EPITAPH, 2, true);
            cx.scene.chars[me].affect.ty = 0;
        }
        7 | 9 => {
            cx.out(Out::FlyFont { kind: 20, n: i32::from(p0) });
            cx.scene.chars[me].hp = (hp + p0).min(mhp);
        }
        1 | 3 => {
            if cx.annihilated() || cx.game_over {
                return;
            }
            cx.out(Out::FlyFont { kind: 2, n: i32::from(p0) });
            if p0 >= 0 {
                cx.out(Out::HitMark { by: person });
                let mut h = if b.cheat_hp != 0 { (hp - p0 / 10).max(mhp / 2) } else { hp - p0 };
                if h <= 0 {
                    h = 0;
                    cx.out(Out::ClearSpcCondition);
                    crate::affect::clear_conditions(&mut cx.scene.chars[me]);
                    b.change_action(cx, act::DEAD, 2, true);
                } else if t == 1 {
                    let n = if cx.env.count & 1 != 0 { act::DAMAGE0 } else { act::DAMAGE1 };
                    b.change_action(cx, n, 0, true);
                }
                cx.scene.chars[me].hp = h;
            }
            if crate::skill::check_type_of(cx.t, i32::from(p1)) == 1 {
                cx.out(Out::Shield);
            }
        }
        _ => {}
    }
}

// --- the images ---------------------------------------------------------------------------------

/// `ccBoss::Main` (0x0045c270) for image `k`, its `Think` the slave's.
fn slave_main(master: &mut Boss, x: &mut Innis, k: usize, cx: &mut Cx) {
    let master_me = cx.me;
    let mut s = std::mem::take(&mut x.slaves[k]);
    let mut sb = std::mem::take(&mut s.b);
    cx.me = s.me;
    sb.base_main(cx, &mut |sb, cx| s.think(sb, master, master_me, cx));
    cx.me = master_me;
    s.b = sb;
    x.slaves[k] = s;
}

impl Slave {
    /// `ccBoss02Slave::SetMonster(ID)` (0x0049c520): the image's clip
    /// (`Mon1`-`Mon3`), its mirror's shard models.
    fn set_monster(&mut self, id: i32, cx: &mut Cx) {
        if let Some(tbl) = usize::try_from(id).ok().and_then(|k| cx.data.innis.monsters.get(k)).cloned() {
            if let Some(Some(clip)) = tbl.first() {
                self.b.anm.set(clip, cx.clips);
            }
            self.b.anm_tbl = tbl;
            self.mirror.models = Some(id);
        }
        self.samon_id = id;
    }

    /// `ccBoss02Slave::Think` (0x0049cce0).
    fn think(&mut self, sb: &mut Boss, master: &mut Boss, master_me: usize, cx: &mut Cx) {
        let me = self.me;
        match sb.act_num {
            0 => {
                if self.order_num != 0 {
                    self.transparency = ONE;
                    self.eny_flg = 1;
                    let mp = cx.scene.chars[master_me].pos;
                    cx.scene.chars[me].pos = mp;
                    sb.dirc = master.dirc;
                    sb.dirc[2] = wrap(ee::sub(sb.dirc[2], PI));
                    cx.scene.chars[me].pos_p = super::w2p(cx, mp);
                    sb.draw_sw = 1;
                    sb.change_action(cx, act::SHAKE, 0, true);
                    self.rot = VF0;
                    self.scale = VF0;
                    sb.move_vector = VF0;
                }
                cx.scene.chars[me].target_char = cx.scene.chars[master_me].target_char;
                self.set_start_position(sb, cx);
            }
            7 => {
                if self.start_count > 0 {
                    self.start_count -= 1;
                }
                let pos = cx.scene.chars[me].pos;
                sb.target_dirc = get_dirc(pos, self.target_pos);
                sb.target_dist = get_dist(pos, self.target_pos);
                self.dist = sb.target_dist;
                self.dirc = sb.target_dirc;
                if self.start_count == 1 {
                    cx.out(Out::Se3dNote { se: 61, pos, note: 53 });
                }
                if self.start_count <= 0 && self.flg == 0 {
                    self.set_lotate(sb, cx);
                    self.set_tan(sb, cx);
                }
                if self.flg == 0 && ee::lt(sb.target_dist, 0x428c_0000) {
                    self.flg = 1;
                    sb.draw_sw = 0;
                    self.enemy_burst(master, cx);
                    self.quake_time = 40;
                    self.quake_vector = [0x41f0_0000, 0x4208_0000, 0x4220_0000, self.quake_vector[3]];
                    let pos = cx.scene.chars[me].pos;
                    cx.me = master_me;
                    master.skill_damage(cx, None, Some(pos), 8);
                    cx.me = me;
                }
                if self.flg != 0 {
                    self.mirror.main(me, cx);
                    if self.quake_time == 20 {
                        self.quake_vector = [0x4120_0000, 0x4100_0000, 0x40e0_0000, self.quake_vector[3]];
                    }
                    let q = self.quake_time;
                    self.quake_time -= 1;
                    if q >= 0 {
                        let v = self.quake_vector;
                        cx.quake_vec([v[0], v[1], v[2]]);
                    }
                    if self.mirror.check_anm_end() {
                        sb.change_action(cx, 0, 0, true);
                        self.flg = 0;
                        self.end_flg = 1;
                        sb.act_num = 0;
                        sb.exit = 1;
                        sb.anm_tbl = Vec::new();
                    }
                }
            }
            _ => {}
        }
    }

    /// `ccBoss02Slave::SetStartPosition` (0x0049c810): where image
    /// `slaveID` starts from, round the camera's back, and its spins.
    fn set_start_position(&mut self, sb: &mut Boss, cx: &mut Cx) {
        let me = self.me;
        let f20 = wrap(ee::add(PI, cx.cam.rot[2]));
        sb.dirc[2] = f20;
        let mut v = VF0;
        let unit = geom::unit_matrix();
        let (m, count, spin) = match self.slave_id {
            0 => {
                v[2] = ee::add(v[2], 0x43fa_0000);
                (geom::rot_matrix_z(&unit, f20), 0, [0x3d23_d70a, 0x3dcc_cccd, 0x3c23_d70a])
            }
            1 => {
                sb.move_dirc = sb.dirc[2];
                v[0] = ee::add(v[0], 0x43fa_0000);
                v[2] = ee::add(v[2], 0x43fa_0000);
                let m = geom::rot_matrix_z(&unit, NEG_PI);
                (geom::rot_matrix_z(&m, f20), 30, [0x3c23_d70a, 0x3e4c_cccd, 0x3e99_999a])
            }
            2 => {
                v[0] = ee::sub(v[0], 0x43fa_0000);
                v[2] = 0x43fa_0000;
                let m = geom::rot_matrix_z(&unit, NEG_PI);
                (geom::rot_matrix_z(&m, f20), 60, [0x3cf5_c28f, 0x3db8_51ec, 0x3e99_999a])
            }
            _ => return,
        };
        let v = geom::apply_matrix(&m, v);
        let pos = geom::vadd(cx.scene.chars[me].pos, v);
        cx.scene.chars[me].pos = pos;
        self.start_count = count;
        for (k, s) in spin.into_iter().enumerate() {
            self.spin[k] = rand_f(cx.cc, s);
        }
        self.scale = [0x40a0_0000, 0x40a0_0000, 0x40a0_0000, self.scale[3]];
        self.pos = pos;
        self.pos[2] = ee::add(self.pos[2], 0x4316_0000);
    }

    /// `ccBoss02Slave::SetLotate` (0x0049d820): the spin on its turn.
    fn set_lotate(&mut self, sb: &mut Boss, cx: &mut Cx) {
        sb.dirc[0] = ee::add(sb.dirc[0], self.spin[0]);
        sb.dirc[2] = ee::add(sb.dirc[2], self.spin[2]);
        for k in 0..3 {
            if !ee::le(sb.dirc[k], PI) {
                sb.dirc[k] = ee::sub(sb.dirc[k], TWO_PI);
            }
        }
        for k in 0..3 {
            if ee::lt(sb.dirc[k], NEG_PI) {
                sb.dirc[k] = ee::add(sb.dirc[k], TWO_PI);
            }
        }
        self.pos = cx.scene.chars[self.me].pos;
        self.pos[2] = ee::add(self.pos[2], 0x4316_0000);
    }

    /// `ccBoss02Slave::SetTan` (0x0049d6b0): the step toward the target,
    /// climbing by `atan2(posP.z, Dist)`: 150, or 100 within 100.
    fn set_tan(&mut self, sb: &mut Boss, cx: &mut Cx) {
        let pz = cx.scene.chars[self.me].pos_p[2];
        let up = wrap(libm::atan2f(pz, self.dist));
        let m = geom::rot_matrix_y(&geom::unit_matrix(), up);
        let m = geom::rot_matrix_z(&m, self.dirc);
        let m = geom::rot_matrix_z(&m, NEG_HALF_PI);
        let mut v = VF0;
        v[0] = if ee::le(self.dist, 0x42c8_0000) { 0x42c8_0000 } else { 0x4316_0000 };
        sb.move_vector = geom::apply_matrix(&m, v);
    }

    /// `ccBoss02Slave::EnemyBurst` (0x0049d1b0): the burst's particles, two
    /// rings (the image's model, turned by the camera), sounds.
    fn enemy_burst(&mut self, master: &mut Boss, cx: &mut Cx) {
        let tp = self.target_pos;
        cx.out(Out::Particles { which: Gen::Burst(3), pos: tp });
        let mut up = tp;
        up[2] = ee::add(up[2], 0x4248_0000);
        cx.out(Out::Particles { which: Gen::Burst(4), pos: up });
        let r = cx.cam.rot;
        let mut rot = VF0;
        rot[1] = fp(dp(rand_f(cx.cc, 0x3e4c_cccd)) + 0.785_398_185_253_143_3);
        rot[2] = wrap(fp(dp(r[2]) + 0.628_318_548_202_514_6));
        let model =
            usize::try_from(self.samon_id).ok().and_then(|k| cx.data.innis.ring_models.get(k)).copied().unwrap_or(191);
        let kind = EffKind::SamonRing { model, scale: [0x3fc0_0000; 4], spoint: [0x3f00_0000; 4], tpoint: 0x3d23_d70a };
        let pos = self.pos;
        for n in 0..2 {
            if n == 1 {
                rot[0] = libm::neg(rot[0]);
                rot[2] = wrap(ee::add(rot[2], HALF_PI));
            }
            let id = master.effects.create(kind);
            cx.out(Out::Effect { id, kind, pos, dirc: rot });
        }
        let p = cx.scene.chars[self.me].pos;
        cx.out(Out::Se3dNote { se: 56, pos: p, note: 72 });
        cx.out(Out::Se3dNote { se: 66, pos: p, note: 45 });
    }
}

// --- the mirror ---------------------------------------------------------------------------------

impl Mirror {
    /// `BreakMirror::BreakMirror` (0x00533fd0).
    fn new() -> Mirror {
        Mirror {
            v0: vec![0; 60],
            theta: vec![0; 60],
            zrot: vec![0; 60],
            vx: vec![0; 60],
            vz: vec![0; 60],
            x: vec![0; 60],
            z: vec![0; 60],
            spin: vec![VF0; 60],
            m_rot: vec![VF0; 60],
            ..Mirror::default()
        }
    }

    /// `BreakMirror::MainMirror` (0x00533d90).
    fn main(&mut self, slave: usize, cx: &mut Cx) {
        if self.f == 0 {
            self.set_data(cx);
        }
        if self.f == 1 {
            self.mbreak(slave, cx);
        }
        if self.f == 2 {
            self.set_data(cx);
            self.f = 2;
            self.models = None;
        }
    }

    /// `BreakMirror::SetData` (0x005340e0): each shard's speed (80-130),
    /// spin, 55-degree launch and heading.
    fn set_data(&mut self, cx: &mut Cx) {
        self.set_point_flg = 1;
        self.m_rot = vec![VF0; 60];
        for k in 0..60 {
            let r = rand_mod(cx, 6) * 10;
            self.v0[k] = ee::add(abs_f(r), 0x42a0_0000);
        }
        for k in 0..60 {
            self.spin[k][0] = rand_f(cx.cc, 0x3ecc_cccd);
            self.spin[k][1] = rand_f(cx.cc, 0x3f19_999a);
            self.spin[k][2] = rand_f(cx.cc, 0x3f00_0000);
        }
        for k in 0..60 {
            let _ = cx.rand.rand();
            self.theta[k] = ee::div(ee::mul(PI, 0x425c_0000), 0x4334_0000);
        }
        for k in 0..60 {
            self.zrot[k] = rand_f(cx.cc, 0x4048_f5c3);
        }
        for k in 0..60 {
            let t = dp(self.theta[k]);
            self.vx[k] = fp(dp(self.v0[k]) * t.cos());
            self.vz[k] = fp(dp(self.v0[k]) * t.sin());
        }
        self.x = vec![0; 60];
        self.z = vec![0; 60];
        self.f = 1;
    }

    /// `BreakMirror::Mbreak` (0x00534430): a step of each shard's flight
    /// (g 9.8, dt 0.6, bouncing off the ground at 0.8 with 0.7 of its
    /// run); done (`f` 2) once no shard is still drawn.
    fn mbreak(&mut self, slave: usize, cx: &mut Cx) {
        const DT: F = 0x3f19_999a;
        const AZ: F = 0xc11c_cccd;
        if self.set_point_flg != 0 {
            self.nut_pos = cx.scene.chars[slave].pos;
            self.set_point_flg = 0;
        }
        self.drawn.clear();
        let mut m = geom::unit_matrix();
        let mut any = false;
        for k in 0..60 {
            self.vx[k] = ee::add(self.vx[k], ee::mul(0, DT));
            self.vz[k] = ee::add(self.vz[k], ee::mul(AZ, DT));
            self.x[k] = ee::add(self.x[k], ee::mul(self.vx[k], DT));
            self.z[k] = ee::add(self.z[k], ee::mul(self.vz[k], DT));
            let r = &mut self.m_rot[k];
            r[0] = ee::add(r[0], self.spin[k][0]);
            r[2] = ee::add(r[2], self.spin[k][2]);
            for a in r.iter_mut().take(3) {
                if !ee::le(*a, PI) {
                    *a = ee::sub(*a, TWO_PI);
                }
            }
            for a in r.iter_mut().take(3) {
                if ee::lt(*a, NEG_PI) {
                    *a = ee::add(*a, TWO_PI);
                }
            }
            m = geom::rot_matrix_z(&m, NEG_HALF_PI);
            m = geom::rot_matrix_z(&m, self.zrot[k]);
            let mut p = geom::vadd(geom::apply_matrix(&m, [self.x[k], 0, self.z[k], ONE]), self.nut_pos);
            if ee::lt(p[2], 0x40a0_0000) && ee::lt(self.vz[k], 0) {
                let g = (cx.land)(p);
                if ee::lt(p[2], g) {
                    self.vx[k] = fp(0.7 * dp(self.vx[k]));
                    self.vz[k] = ee::mul(libm::neg(0x3f4c_cccd), self.vz[k]);
                    p[2] = g;
                }
            }
            if !(ee::lt(self.vx[k], 0x41a0_0000) || ee::lt(p[2], 0xc396_0000)) {
                any = true;
                self.drawn.push((k % 5, p, self.m_rot[k]));
            }
        }
        if !any {
            self.f = 2;
        }
    }

    /// `BreakMirror::CheckAnmEnd` (0x00534a50): done, once.
    fn check_anm_end(&mut self) -> bool {
        if self.f == 2 {
            self.f = 0;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::affect::{self, AffectCtx};
    use crate::boss::{BossData, BossEnv, CamView};
    use crate::chara::{Char, Env};
    use crate::enemy_ai::{Genrand, IDENTITY, World};
    use crate::event::Events;
    use crate::exp::Party;
    use crate::param::{Base, SpcParam};
    use crate::rand::Rand;
    use crate::scene::Scene;
    use crate::tables::Tables;
    use piney_data::volume::Volume;

    /// Innis over Kite and a member, 500 apart, on Mutation's tables.
    struct Fight {
        t: Tables,
        data: BossData,
        scene: Scene,
        party: Party,
        me: usize,
        rand: Rand,
        cc: Genrand,
        count: u32,
    }

    fn pc(ty: i32, id: i16, x: f32) -> Char {
        let p = SpcParam {
            base: Base { ty, id, level: 50, ..Base::default() },
            max_hp: 900,
            max_sp: 90,
            ..Default::default()
        };
        let mut c = Char::pc(p);
        c.pos = [x.to_bits(), 0, 0, ONE];
        c.pos_p = c.pos;
        c.affect.func = AffectFunc::None;
        c
    }

    fn fight() -> Option<Fight> {
        if !piney_data::store::work_tables(Volume::Mut).join("combat.bin").is_file() {
            return None;
        }
        let t = Tables::of(Volume::Mut);
        let data = BossData::of(Volume::Mut);
        let mut scene = Scene::default();
        let kite = scene.add(pc(7, 0, 0.0), 0);
        let member = scene.add(pc(6, 1, 500.0), 0);
        let mut ch = Char::foe(t.bosses[1].clone());
        ch.condition_num = -1;
        ch.affect.func = AffectFunc::Boss;
        let me = scene.add(ch, 3);
        let party = Party { members: [Some(kite), Some(member), None], ids: [0, 1, -1], num: 2 };
        let mut f = Fight { t, data, scene, party, me, rand: Rand(7), cc: Genrand::seeded(4357), count: 0 };
        let b = f.with_cx(|cx| new(cx, [0, 0, 0, ONE], VF0, VF0));
        f.scene.chars[me].foe_state_mut().unwrap().boss = Some(Box::new(b));
        Some(f)
    }

    impl Fight {
        fn with_cx<R>(&mut self, run: impl FnOnce(&mut Cx) -> R) -> R {
            let clip = |_: &str| Some((60, false));
            let env = Env { count: self.count, menu_type: -1, ..Env::default() };
            let check = |_: usize| 0;
            let benv = BossEnv { t: &self.t, data: &self.data, clips: &clip, env: &env, game_over: false };
            let actx = AffectCtx {
                party: &self.party,
                menu: true,
                skill_check: &check,
                boss: Some(&benv),
                volume: Volume::Mut,
            };
            let mut none = |_| None;
            let mut land = |_| 0;
            let mut cx = Cx {
                t: &self.t,
                data: &self.data,
                scene: &mut self.scene,
                party: &self.party,
                world: World { player: self.party.members[0], frame: &IDENTITY, ..World::default() },
                env: &env,
                actx: &actx,
                rand: &mut self.rand,
                cc: &mut self.cc,
                clips: &clip,
                collide: &mut none,
                game_over: false,
                boss_cam: true,
                cam: CamView::default(),
                land: &mut land,
                disc: crate::boss::DiscView::default(),
                me: self.me,
                out: Vec::new(),
                ev: Events::new(),
            };
            run(&mut cx)
        }

        /// A frame of the manager's pass and `Main`: what it asked.
        fn frame(&mut self) -> Vec<Out> {
            self.count += 1;
            let me = self.me;
            let mut b = self.scene.chars[me].foe_state_mut().unwrap().boss.take().unwrap();
            let mut out = b.take_pending();
            out.extend(self.with_cx(|cx| {
                b.main(cx);
                std::mem::take(&mut cx.out)
            }));
            self.scene.chars[me].foe_state_mut().unwrap().boss = Some(b);
            out
        }

        /// `EntryAffect(ty, p0)` on the boss from Kite.
        fn affect(&mut self, ty: i16, p0: i16) {
            let (me, kite) = (self.me, self.party.members[0]);
            let clip = |_: &str| Some((60, false));
            let env = Env { count: self.count, menu_type: -1, ..Env::default() };
            let check = |_: usize| 0;
            let benv = BossEnv { t: &self.t, data: &self.data, clips: &clip, env: &env, game_over: false };
            let actx = AffectCtx {
                party: &self.party,
                menu: true,
                skill_check: &check,
                boss: Some(&benv),
                volume: Volume::Mut,
            };
            let mut ev = Events::new();
            affect::entry_affect(&self.t, &mut self.scene, &actx, me, kite, ty, [p0, 0, 0], &mut self.rand, &mut ev);
        }

        fn boss(&self) -> &Boss {
            self.scene.chars[self.me].foe_state().unwrap().boss.as_ref().unwrap()
        }

        fn innis(&mut self) -> &mut Innis {
            let b = self.scene.chars[self.me].foe_state_mut().unwrap().boss.as_mut().unwrap();
            let Class::Innis(x) = &mut b.class else { panic!("not Innis") };
            x
        }
    }

    #[test]
    fn stands_behind_kite_and_waits_first() {
        let Some(mut f) = fight() else { return };
        let me = f.me;
        assert_eq!(f.scene.chars[me].pos[1], 0x44bb_8000, "1500 behind Kite");
        assert_eq!(f.scene.chars[me].pos[2], 0x41a0_0000, "20 up");
        assert!(f.scene.ene_list.contains(&me));
        let slaves: Vec<usize> = f.innis().slaves.iter().map(|s| s.me).collect();
        assert_eq!(slaves.len(), 3);
        assert!(slaves.iter().all(|&s| !f.scene.listed(s)));
        assert!(f.innis().slaves.iter().all(|s| s.b.exit == 1));
        let out = f.frame();
        // Pattern[0] is 6, Pattern03(20); the four InisField generators.
        assert_eq!(f.innis().move_flg, 6);
        assert_eq!(out.iter().filter(|o| matches!(o, Out::Particles { .. })).count(), 4);
    }

    #[test]
    fn drained_to_the_epitaph_then_dies_and_exits() {
        let Some(mut f) = fight() else { return };
        f.frame();
        f.affect(13, 0);
        f.frame();
        assert_eq!(f.boss().act_num, act::EPITAPH_NEUTRAL);
        assert_eq!(f.innis().epitaph_flg, 1);
        assert_eq!(f.boss().cheat_hp, 0);
        f.affect(21, 0);
        assert_eq!(f.scene.chars[f.me].hp, 3000);
        let mut dead = false;
        for _ in 0..400 {
            if f.boss().act_num != act::DEAD {
                f.affect(1, 1000);
            }
            f.frame();
            dead |= f.boss().act_num == act::DEAD;
            if f.boss().exit != 0 {
                break;
            }
        }
        assert!(dead, "hit to death");
        assert_eq!(f.boss().exit, 1, "the dead effect over, 30 frames on, the task's exit");
        assert!(!f.scene.ene_list.contains(&f.me));
    }

    #[test]
    fn the_magic_lands_and_lets_the_party_go() {
        let Some(mut f) = fight() else { return };
        f.innis().move_flg = 19;
        let mut outs = Vec::new();
        let mut waited = false;
        for _ in 0..400 {
            outs.extend(f.frame());
            waited |= f.boss().act_num == act::MAGIC;
            if waited && f.boss().act_num == act::NEUTRAL {
                break;
            }
        }
        assert!(waited, "act 4 while the missiles fly");
        assert_eq!(f.boss().act_num, act::NEUTRAL);
        assert_eq!(f.boss().lock_player, 0);
        assert_eq!(f.innis().eny_int, 60, "MagicDamage called 60 times");
        assert!(outs.contains(&Out::CamMode { mode: 6 }));
        let missiles = outs.iter().filter(|o| matches!(o, Out::Effect { kind: EffKind::Missile { .. }, .. })).count();
        assert_eq!(missiles, 3);
    }

    #[test]
    fn the_images_break_and_come_back() {
        let Some(mut f) = fight() else { return };
        f.innis().move_flg = 17;
        let mut sent = false;
        for _ in 0..1500 {
            f.frame();
            sent |= f.innis().slaves.iter().any(|s| s.b.exit == 0);
            if sent && f.boss().act_num == act::NEUTRAL {
                break;
            }
        }
        assert!(sent, "the three images went out");
        assert_eq!(f.boss().act_num, act::NEUTRAL);
        assert_eq!(f.innis().monster_id, 1);
        assert!(f.innis().slaves.iter().all(|s| s.b.exit == 1 && s.end_flg == 1));
    }
}
