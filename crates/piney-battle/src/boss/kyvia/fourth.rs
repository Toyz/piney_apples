//! Kyvia's fourth fight (`ccBossKyvia04`; QUA gcmn 0x003d9b70-0x003e7100),
//! `bossFunc` code 15, Quarantine's event 312 on field 12. The core rises
//! at four places in turn as Kite reaches each; the gates between them
//! open as each core falls and close behind the party (field 12's
//! `body[n]` hits). At the last the body wakes into its EX turn with the
//! core at level 5: its first death the rebirth (act 24), its second the
//! body's. docs/engine/boss-kyvia-fourth.md.

use piney_data::field::ee;
use piney_data::libm;

use super::{
    Gen, GenOp, GenRow, Held, Kyvia, Place, Table, Tree, act, as_me, core_fade_check, core_ready, core_state,
    core_state_is, gen_op, particles,
};
use crate::boss::blur::Set;
use crate::boss::{Boss, Cx, Out, VF0};
use crate::enemy_ai::rand_f;
use crate::geom::{self, V4};

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
/// The fourth fight's own particle rows, `distSW` off but for the smoke,
/// the beams' bursts and the ball's gathering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Make {
    /// `InitWallGpStart`: each place's walls (`WallGp`, `WallGp2`) and its
    /// rising light (`CoreAppGp`, `CoreAppGp2`); the fourth place's rows
    /// are their own.
    Wall(u8),
    Wall2(u8),
    CoreApp(u8),
    CoreApp2(u8),
    /// `KyviaScene`'s light at its 155th frame (Kyvia row 27).
    Scene,
    /// The rebirth's two (rows 28 and 29).
    Rebirth(u8),
    /// `DeadKyviaRipus`'s at 30, 50 and 170 (rows 12-16).
    Ripus(u8),
    /// `BIGMegidFlame`'s three at its tenth frame (rows 20-22).
    BigMegid(u8),
    /// `Skill_Atk`'s (`MissileSmokeGenerator` row 29).
    Skill,
    /// `MagicBallAtk`'s two over Kite (Kyvia row 8).
    Ball(u8),
    /// `LightAtk`'s glow at its 40th frame (row 19).
    Glow,
    /// The beams' own as the first fights', `distSW` off here.
    Light,
    BeamsOut,
    BeamHead,
    BeamTail,
    BeamBurst,
    /// `CheckDmgSmokeEff`'s smoke (`KyviaDmgGenerator` row 0, ff 4 1).
    Smoke,
}

impl Make {
    pub fn rows(self) -> GenRow {
        let g = |table, row, ff: &'static [u8]| GenRow { table, row, ff, ff_table: table, dist_sw: false };
        let k = Table::Kyvia;
        match self {
            Make::Wall(3) => GenRow { ff_table: Table::MissileSmoke, ..g(k, 25, &[17]) },
            Make::Wall(_) => g(k, 11, &[18, 6]),
            Make::Wall2(3) => g(k, 26, &[20, 7, 9]),
            Make::Wall2(_) => g(k, 18, &[18, 6]),
            Make::CoreApp(3) => g(k, 23, &[16]),
            Make::CoreApp(_) => g(k, 10, &[16]),
            Make::CoreApp2(3) => g(k, 24, &[20, 7, 17]),
            Make::CoreApp2(_) => g(k, 9, &[15, 17]),
            Make::Scene => g(k, 27, &[7, 25]),
            Make::Rebirth(n) => g(k, 28 + n, &[6, 26]),
            Make::Ripus(0) => g(k, 12, &[21, 20]),
            Make::Ripus(1) => g(k, 13, &[2]),
            Make::Ripus(2) => g(k, 14, &[2]),
            Make::Ripus(3) => g(k, 15, &[22]),
            Make::Ripus(4) => g(k, 16, &[21, 20]),
            Make::Ripus(_) => g(k, 16, &[20]),
            Make::BigMegid(0) => g(k, 20, &[22]),
            Make::BigMegid(n) => g(k, 20 + n, &[22, 26, 27]),
            Make::Skill => g(Table::MissileSmoke, 29, &[10, 21]),
            Make::Ball(n) => GenRow { dist_sw: true, ..g(k, 8, if n == 0 { &[13] } else { &[14] }) },
            Make::Glow => g(k, 19, &[25]),
            Make::Light => g(k, 0, &[0]),
            Make::BeamsOut => g(k, 7, &[10, 11]),
            Make::BeamHead => g(k, 5, &[1]),
            Make::BeamTail => g(k, 5, &[8]),
            Make::BeamBurst => GenRow { dist_sw: true, ..g(k, 3, &[12, 4]) },
            Make::Smoke => GenRow { dist_sw: true, ..g(Table::KyviaDmg, 0, &[4, 1]) },
        }
    }
}

/// `ccBossKyvia04`'s own members (Infection's DWARF; Quarantine's
/// +0x29614 and +0x29618 the rebirth's flag and count).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fourth {
    /// `FightFlg`: the EX turn on.
    pub fight_flg: u8,
    /// +0x29351: Kite at the core's place, its rise running.
    pub rising: u8,
    pub core_lv_count: i32,
    /// The core's places, then the three words after them the rebirth's
    /// `CoreAppearPos[4]` reads: `CoreLvCount`'s bits and the padding
    /// (docs: never read on).
    pub core_appear_pos: [V4; 4],
    pub wall_pos: [V4; 4],
    pub wall_pos2: [V4; 4],
    pub dead_set_pos: V4,
    pub rebirth_flg: i32,
    pub rebirth_cnt: i32,
    pub wall_gp_flg: i32,
    /// `WallGp`, `WallGp2`, `CoreAppGp`, `CoreAppGp2` made, by place.
    pub made: [[bool; 4]; 4],
    /// `FadeNum`: the `scFadeDef` element `MagicBallAtk` flashed with (the
    /// first free one then).
    pub fade_num: i32,
}

impl Fourth {
    /// The constructor's (QUA gcmn 0x003d9c40).
    pub(super) fn new() -> Fourth {
        let v = |x, y, z| [x, y, z, ONE];
        Fourth {
            wall_gp_flg: 1,
            core_appear_pos: [
                v(0xc410_0000, 0xc61a_bc00, 0x4463_8000),
                v(0xc539_f000, 0xc5f1_a800, 0x4463_8000),
                v(0xc437_4000, 0xc582_1800, 0x4461_0000),
                v(0x4311_0000, 0xc508_0000, 0x4461_0000),
            ],
            wall_pos: [
                v(0xc4cc_2000, 0xc60f_7000, 0x447a_0000),
                v(0xc53f_5000, 0xc5c1_5000, 0x447a_0000),
                v(0xc434_0000, 0xc561_1000, 0x447a_0000),
                VF0,
            ],
            wall_pos2: [
                v(0xc496_a000, 0xc609_b000, 0x447a_0000),
                v(0xc522_1000, 0xc5c6_5000, 0x447a_0000),
                v(0x4180_0000, 0xc580_f000, 0x447a_0000),
                VF0,
            ],
            dead_set_pos: v(0x40e0_0000, 0xc4c1_2000, 0x447a_0000),
            ..Fourth::default()
        }
    }

    /// `CoreAppearPos[k]`: past the four, `CoreLvCount`'s bits and the
    /// object's padding (zeros here).
    fn appear(&self, k: i32) -> V4 {
        match usize::try_from(k).ok().and_then(|k| self.core_appear_pos.get(k)) {
            Some(&p) => p,
            None => [self.core_lv_count as u32, 0, 0, 0],
        }
    }
}

fn fourth_of(x: &mut Kyvia) -> &mut Fourth {
    x.fourth.get_or_insert_with(|| Box::new(Fourth::new()))
}

/// The EX turn.
pub(super) fn ex(x: &Kyvia) -> bool {
    x.fourth.as_ref().is_some_and(|f| f.fight_flg != 0)
}

fn add(a: V4, b: V4) -> V4 {
    geom::vadd(a, b)
}

// --- the walls ---------------------------------------------------------------------------

/// `InitWallGpStart` (QUA gcmn 0x003dbef0), the first frame: each place's
/// light and walls made (from `WallPos` to `WallPos2`), then all but the
/// first place's held still (`pauseFlag`).
fn init_wall_gp_start(x: &mut Kyvia, cx: &mut Cx) {
    let f = fourth_of(x);
    let one = [0, 0, 0, ONE];
    for k in 0..3u8 {
        let i = usize::from(k);
        let at = add(f.core_appear_pos[i], one);
        particles(cx, Gen::Fourth(Make::CoreApp(k)), Place::At(at), Some(Held::CoreApp(k)));
        particles(cx, Gen::Fourth(Make::CoreApp2(k)), Place::At(at), Some(Held::CoreApp2(k)));
        // The walls 50 down are computed and not used: the first stands
        // at its places as they are.
        particles(cx, Gen::Fourth(Make::Wall(k)), Place::Line(f.wall_pos[i], f.wall_pos2[i]), Some(Held::Wall(k)));
        let at = add(f.wall_pos[i], one);
        let to = add(f.wall_pos2[i], one);
        particles(cx, Gen::Fourth(Make::Wall2(k)), Place::Line(at, to), Some(Held::Wall2(k)));
        f.made[0][i] = true;
        f.made[1][i] = true;
        f.made[2][i] = true;
        f.made[3][i] = true;
    }
    let at = add(f.core_appear_pos[3], one);
    particles(cx, Gen::Fourth(Make::CoreApp(3)), Place::At(at), Some(Held::CoreApp(3)));
    particles(cx, Gen::Fourth(Make::CoreApp2(3)), Place::At(at), Some(Held::CoreApp2(3)));
    particles(cx, Gen::Fourth(Make::Wall(3)), Place::At(at), Some(Held::Wall(3)));
    let up = [0, 0, 0x42c8_0000, ONE];
    let at = add(f.core_appear_pos[3], up);
    let to = add(f.wall_pos2[3], up);
    particles(cx, Gen::Fourth(Make::Wall2(3)), Place::Line(at, to), Some(Held::Wall2(3)));
    for k in 0..4 {
        f.made[k][3] = true;
    }
    for k in 1..4u8 {
        for h in [Held::Wall(k), Held::Wall2(k), Held::CoreApp(k), Held::CoreApp2(k)] {
            gen_op(cx, h, GenOp::Pause(true));
        }
    }
    f.wall_gp_flg = 0;
}

/// `SwitchGateON(n)` (QUA gcmn 0x003decf0), as core `n` falls: field
/// 12's `body[n - 1]` let through (`HitDisable`), its walls held and the
/// next place's walls and light let run.
fn switch_gate_on(x: &mut Kyvia, cx: &mut Cx, n: i16) {
    let Ok(k) = u8::try_from(n - 1) else { return };
    if k > 2 {
        return;
    }
    cx.out(Out::FieldBodyHit { n: usize::from(k), on: false });
    let f = fourth_of(x);
    gen_op(cx, Held::Wall(k), GenOp::Pause(true));
    gen_op(cx, Held::Wall2(k), GenOp::Pause(true));
    let j = k + 1;
    gen_op(cx, Held::Wall(j), GenOp::Pause(false));
    if f.made[1][usize::from(j)] {
        gen_op(cx, Held::Wall2(j), GenOp::Pause(false));
    }
    gen_op(cx, Held::CoreApp(j), GenOp::Pause(false));
    gen_op(cx, Held::CoreApp2(j), GenOp::Pause(false));
}

/// `SwitchGateOFF(n)` (QUA gcmn 0x003defc0), as core `n` rises: the place
/// behind closed (`body[n - 2]`'s `HitEnable`), its walls let run, and
/// the walls two back (and the fourth place's) killed.
fn switch_gate_off(x: &mut Kyvia, cx: &mut Cx, n: i16) {
    let Ok(k) = u8::try_from(n - 2) else { return };
    if k > 2 {
        return;
    }
    cx.out(Out::FieldBodyHit { n: usize::from(k), on: true });
    gen_op(cx, Held::Wall(k), GenOp::Pause(false));
    gen_op(cx, Held::Wall2(k), GenOp::Pause(false));
    let f = fourth_of(x);
    match k {
        1 => {
            gen_op(cx, Held::Wall(0), GenOp::Kill);
            gen_op(cx, Held::Wall2(0), GenOp::Kill);
        }
        2 => {
            gen_op(cx, Held::Wall(1), GenOp::Kill);
            gen_op(cx, Held::Wall2(1), GenOp::Kill);
            gen_op(cx, Held::Wall(3), GenOp::Kill);
            if f.made[1][3] {
                gen_op(cx, Held::Wall2(3), GenOp::Kill);
            }
        }
        _ => {}
    }
}

// --- the frame ---------------------------------------------------------------------------

/// The fourth fight's `Think` (QUA gcmn 0x003dbcf0) before its turn.
pub(super) fn think_head(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    if fourth_of(x).wall_gp_flg != 0 {
        init_wall_gp_start(x, cx);
    }
    x.af_cou += 1;
    if x.af_timing < x.af_cou {
        x.af_cou = 0;
    }
    check_disc_move(b, x, t, cx);
    dead_check(b, x, t, cx);
}

/// `Think`'s turn: the EX turn's pattern or the places' (neither once the
/// party is gone or the game is over), idle in the attacks.
pub(super) fn think_turn(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    match b.act_num {
        act::NEUTRAL => {
            if cx.annihilated() || cx.game_over {
                return;
            }
            if ex(x) {
                kyvia_switch_action_pattern(b, x, t, cx);
            } else {
                switch_action_pattern(b, x, t, cx);
            }
        }
        1..=6 | 11..=14 | 20..=24 => {}
        _ => b.change_action(cx, act::NEUTRAL, 0, true),
    }
}

/// `CheckCorePoint` (QUA gcmn 0x003dec60): Kite within 250 of the next
/// place.
fn check_core_point(x: &Kyvia, cx: &Cx) -> bool {
    let f = x.fourth.as_deref().expect("the fourth fight");
    if f.core_lv_count > i32::from(x.disc_lv) {
        return false;
    }
    let d = super::super::dist(cx, x.disc_pos, f.appear(f.core_lv_count));
    !ee::le(0x437a_0000, d)
}

/// `CheckDiscMove` (QUA gcmn 0x003dd590): `DiscPos` is Kite's place. At a
/// place: the flash and the player held, its light killed; at 20 the party
/// gathered there; at 30 the core risen and the place behind closed, or at
/// the last place the body's scene, then the EX turn. Once a core falls
/// the next place opened.
fn check_disc_move(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let kite = cx.world.player.map_or(x.disc_pos, |k| cx.scene.chars[k].pos);
    x.disc_pos = kite;
    if x.st_flg != 1 {
        if t.core.as_ref().is_some_and(|c| c.x.dead_flg != 0) && x.disc_lv < x.disc_max_lv {
            x.wait_disc_count += 1;
            if x.wait_disc_count == 1 {
                let lv = x.disc_lv;
                switch_gate_on(x, cx, lv);
                x.st_flg = 1;
                x.disc_lv += 1;
                x.wait_disc_count = 0;
                x.atk_pat_mode = 1;
            }
        }
        return;
    }
    if check_core_point(x, cx) {
        fourth_of(x).rising = 1;
    }
    if fourth_of(x).rising == 0 {
        return;
    }
    x.wait_disc_count += 1;
    if x.wait_disc_count == 1 {
        b.change_action(cx, act::NEUTRAL, 0, true);
        cx.out(Out::FlashFade { t_in: 10, t_out: 5, hold: Some(10), colour: 0x80ff_ffff });
        b.lock_player(cx, false);
        let lv = i32::from(x.disc_lv);
        let f = fourth_of(x);
        if f.core_lv_count < lv {
            let k = f.core_lv_count as u8;
            gen_op(cx, Held::CoreApp(k), GenOp::Kill);
            gen_op(cx, Held::CoreApp2(k), GenOp::Kill);
            gen_op(cx, Held::CoreApp(k), GenOp::Forget);
            gen_op(cx, Held::CoreApp2(k), GenOp::Forget);
            f.made[2][usize::from(k)] = false;
            f.made[3][usize::from(k)] = false;
        }
    }
    if x.wait_disc_count == 20 {
        let k = (x.disc_lv - 1) as usize;
        let mut at = fourth_of(x).core_appear_pos[k];
        together_char_pos(cx, &mut at, 0);
        fourth_of(x).core_appear_pos[k] = at;
    }
    if fourth_of(x).core_lv_count + 1 < i32::from(x.disc_max_lv) {
        if x.wait_disc_count == 30 {
            x.st_flg = 0;
            fourth_of(x).rising = 0;
            x.wait_disc_count = 0;
            entry_slave(x, t, cx, 1);
            let core = t.core.as_ref().map_or(cx.me, |c| c.me);
            x.max_hp = i32::from(cx.scene.chars[core].max_hp);
            x.switch_hp = x.max_hp / 5;
            b.unlock_player();
            let lv = x.disc_lv;
            switch_gate_off(x, cx, lv);
        }
    } else if kyvia_scene(b, x, cx) {
        x.st_flg = 0;
        fourth_of(x).rising = 0;
        x.wait_disc_count = 0;
        ex_on(b, x, t, cx);
        b.unlock_player();
    }
}

/// `KyviaScene` (QUA gcmn 0x003dda00), by `WaitDiscCount`: its rumble each
/// 40th frame; at 10 the last place closed; at 30 the camera before the
/// body; from 40 the eye rising 5 (3 from 155) while it holds; at 155 the
/// body's light and roar (`ANM_ex04act0`); at 280 the camera back. True
/// at 280.
fn kyvia_scene(b: &mut Boss, x: &mut Kyvia, cx: &mut Cx) -> bool {
    let c = x.wait_disc_count;
    if c % 40 == 0 {
        cx.out(Out::Se { se: 232 });
        cx.out(Out::Se { se: 232 });
    }
    if c == 10 {
        let lv = x.disc_lv;
        switch_gate_off(x, cx, lv);
    }
    // `DMY_marker01`'s chunk is fetched and its place never read: the
    // body's point is (0, -700, 1100) of the field.
    let point = [0, 0xc42f_0000, 0x4489_8000, ONE];
    if c == 30 {
        let eye = add(point, [0, 0xc4fa_0000, 0x42c8_0000, ONE]);
        cx.out(Out::Blur(Set::Scale(0x3f82_8f5c)));
        cx.out(Out::Blur(Set::Abgr(x.ex_argb)));
        cx.out(Out::CamMode { mode: 5 });
        cx.out(Out::FreeCam { pos: eye, view: point });
        cx.out(Out::SeNote { se: 191, note: 60 });
    }
    if c >= 40 {
        let rise = if c < 155 { 0x40a0_0000 } else { 0x4040_0000 };
        let eye = add(cx.cam.eye, [0, rise, 0, ONE]);
        if cx.cam.reset == 5 {
            cx.out(Out::CamEye(eye));
        }
    }
    if c == 155 {
        particles(cx, Gen::Fourth(Make::Scene), Place::At([0, 0xc42f_0000, 0x4489_8000, ONE]), None);
        cx.out(Out::SeNote { se: 63, note: 48 });
        cx.out(Out::SeNote { se: 63, note: 48 });
        b.anm.set("ANM_ex04act0", cx.clips);
    }
    if c == 280 {
        cx.out(Out::Blur(Set::Abgr(x.default_argb)));
        cx.out(Out::CamMode { mode: 6 });
        return true;
    }
    false
}

/// `KyviaExOn` (QUA gcmn 0x003dcd70): the core at level 5 and in, its
/// HP a fifth to a smoke, the EX turn's clips; after the rebirth (never
/// reached) only the core again and the body at rest.
fn ex_on(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let reborn = fourth_of(x).rebirth_flg != 0;
    if !reborn {
        fourth_of(x).fight_flg = 1;
    }
    core_base_param_ex(x, t, cx);
    entry_slave(x, t, cx, 1);
    if !reborn && let Some(c) = t.core.as_mut() {
        c.x.k_atk_flg = 0;
        c.x.k_dmg_flg = 0;
    }
    let core = t.core.as_ref().map_or(cx.me, |c| c.me);
    x.max_hp = i32::from(cx.scene.chars[core].max_hp);
    x.switch_hp = x.max_hp / 5;
    if reborn {
        b.change_action(cx, act::NEUTRAL, 0, true);
    } else {
        b.anm_tbl = cx.data.kyvia.anims_fourth_ex.clone();
    }
}

/// The core's `SetCoreBaseParam(5, -1)`: `bossTbl` row 15, its `maxHP`
/// 9999 once the rebirth wrote it.
fn core_base_param_ex(x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let row15 = x.row15_hp;
    if let Some(c) = t.core.as_mut() {
        super::core::set_ex_param(c, cx, row15);
    }
}

/// `EntrySlave(0, on)` of the fourth fight (QUA gcmn 0x003e6fa0): the
/// exited core in, in the EX turn as such, at the next place.
pub(super) fn entry_slave(x: &mut Kyvia, t: &mut Tree, cx: &mut Cx, on: i32) -> i32 {
    let ex = ex(x);
    let Some(c) = t.core.as_mut() else { return -1 };
    if c.b.exit == 0 {
        return -1;
    }
    c.b.exit = 0;
    c.x.order_num = on;
    if ex {
        let gs = &mut t.gomoras;
        as_me(cx, c.me, |cx| super::core::kyvia_ex_mode_on(c, gs, cx));
    }
    let f = fourth_of(x);
    c.x.disc_pos = f.appear(f.core_lv_count);
    f.core_lv_count += 1;
    c.x.slave_id
}

/// `DeadCheck` (QUA gcmn 0x003dc7a0), in the EX turn: the core's first
/// death the rebirth, its second (its HP out, the party standing) the
/// body's.
fn dead_check(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    if !ex(x) || x.live_flg == 0 {
        return;
    }
    if fourth_of(x).rebirth_flg == 0 {
        // Its fourth death (the three places' and the EX turn's first).
        let dead = t.core.as_ref().is_some_and(|c| c.x.dead_count == 4 && c.x.dead_flg != 0);
        if dead && b.act_num != act::REBIRTH {
            b.change_action(cx, act::REBIRTH, 0, true);
        }
        return;
    }
    let core_hp = t.core.as_ref().map_or(0, |c| cx.scene.chars[c.me].hp);
    if core_hp == 0 && !cx.annihilated() && !cx.game_over {
        cx.out(Out::ClearSpcCondition);
        b.change_action(cx, act::DEAD, 2, true);
        x.live_flg = 0;
    }
}

/// `SwitchActionPattern` of the places (QUA gcmn 0x003dcf40): at the
/// core's turn (`kAtkFlg` 2, its state 0), the next of three attacks by
/// the place.
fn switch_action_pattern(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    super::switch_dead_check(b, x, t, cx);
    let Some(c) = t.core.as_mut() else { return };
    if c.x.k_atk_flg != 2 || c.x.core_state != 0 {
        return;
    }
    x.cinema = 1;
    use super::thunder::Kind;
    let (pick, next): (Option<(i32, i16, Option<Kind>)>, i32) = match x.atk_pat_mode {
        1 => (
            match x.disc_lv {
                1 | 2 => Some((44, act::HAND, None)),
                3 | 4 => Some((42, act::MAGIC_BALL, None)),
                _ => None,
            },
            x.atk_pat_mode + 1,
        ),
        2 => (
            match x.disc_lv {
                1 => Some((50, act::THUNDER, Some(Kind::Strike))),
                2 => Some((50, act::THUNDER, Some(Kind::Wide))),
                3 => Some((50, act::THUNDER, Some(Kind::Storm))),
                _ => None,
            },
            x.atk_pat_mode + 1,
        ),
        3 => (
            match x.disc_lv {
                1 => Some((50, act::THUNDER, Some(Kind::Strike))),
                2 => Some((42, act::MAGIC_BALL, None)),
                3 => Some((62, act::HELL, None)),
                _ => None,
            },
            1,
        ),
        4 => {
            b.change_action(cx, act::MAGIC_BALL, 0, true);
            x.atk_pat_mode = 2;
            c.x.k_atk_flg = 0;
            return;
        }
        _ => {
            c.x.k_atk_flg = 0;
            return;
        }
    };
    if let Some((cinema, n, kind)) = pick {
        cx.out(Out::Cinema(Some(cinema)));
        b.change_action(cx, n, 0, true);
        if let Some(k) = kind
            && let Some(th) = x.thunder.as_mut()
        {
            th.kind = k;
        }
    }
    x.atk_pat_mode = next;
    c.x.k_atk_flg = 0;
}

/// `KyviaSwitchActionPattern` (QUA gcmn 0x003dc8e0), the EX turn's: the
/// core's flags read (its turn, its hits), then at its turn the next of
/// five: the thunder or the ball after the rebirth, the skills, the
/// beams, hell's express, and the great meteor, the last waiting for the
/// menu with the player held.
fn kyvia_switch_action_pattern(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let flg = t.core.as_ref().map_or(0, |c| c.x.k_atk_flg);
    match flg {
        2 => {
            super::check_dmg_smoke_eff(x, t, cx);
        }
        1 => {
            b.change_action(cx, act::DAMAGE, 0, true);
            if let Some(c) = t.core.as_mut() {
                c.x.k_atk_flg = 3;
            }
        }
        3 => {
            if let Some(c) = t.core.as_mut() {
                c.x.k_atk_flg = 2;
            }
        }
        _ => {
            if t.core.as_ref().is_some_and(|c| c.x.k_dmg_flg == 4) {
                if b.act_num != act::DAMAGE2 && b.act_num != act::DAMAGE {
                    b.change_action(cx, act::DAMAGE2, 0, true);
                }
                if let Some(c) = t.core.as_mut() {
                    c.x.k_dmg_flg = 0;
                    let cm = c.me;
                    as_me(cx, cm, |cx| c.b.change_action(cx, super::core::act::NEUTRAL, 0, true));
                }
            }
        }
    }
    if flg != 2 {
        return;
    }
    if x.disc_lv == x.disc_max_lv && x.atk_pat_mode == 5 {
        b.lock_player(cx, false);
        if cx.env.menu_type != -1 {
            return;
        }
    }
    x.cinema = 1;
    let reborn = fourth_of(x).rebirth_flg != 0;
    let (cinema, n) = match x.atk_pat_mode {
        1 => {
            x.atk_pat_mode = 2;
            if reborn {
                (43, act::MAGIC_BALL)
            } else {
                if let Some(th) = x.thunder.as_mut() {
                    th.kind = super::thunder::Kind::Strike;
                }
                (50, act::THUNDER)
            }
        }
        2 => {
            x.atk_pat_mode = if reborn { 3 } else { 1 };
            (63, act::SKILL)
        }
        3 => {
            x.atk_pat_mode = 4;
            (52, act::LIGHT)
        }
        4 => {
            x.atk_pat_mode = 5;
            (62, act::HELL)
        }
        5 => {
            x.atk_pat_mode = 1;
            (55, act::BIG_MEGID)
        }
        _ => {
            if let Some(c) = t.core.as_mut() {
                c.x.k_atk_flg = 0;
            }
            return;
        }
    };
    cx.out(Out::Cinema(Some(cinema)));
    b.change_action(cx, n, 0, true);
    if let Some(c) = t.core.as_mut() {
        c.x.k_atk_flg = 0;
    }
}

/// `TogetherCharPos(pos, ex)` (QUA gcmn 0x003df1b0): each member on the
/// lists stood at `pos` 150 off (15 for `ex` 1 or 2) turned at random,
/// at its own height, which it writes into `pos`.
pub(super) fn together_char_pos(cx: &mut Cx, pos: &mut V4, ex: i32) {
    for m in cx.party.members {
        if !cx.valid(m) {
            continue;
        }
        let Some(m) = m else { continue };
        let r = rand_f(cx.cc, PI);
        let y = match ex {
            0 => 0xc316_0000,
            1 | 2 => 0x4170_0000,
            _ => 0,
        };
        pos[2] = cx.scene.chars[m].pos[2];
        let rm = geom::rot_matrix_z(&geom::unit_matrix(), r);
        let off = geom::apply_matrix(&rm, [0, y, 0, ONE]);
        cx.scene.chars[m].pos = add(off, *pos);
    }
}

/// `KyviaRndTarget` (QUA gcmn 0x003deb10): `SelectTarget(|ccRand() % 15|,
/// 1e6)` if alive, else the first member alive on the list, else what it
/// chose.
fn rnd_target(b: &mut Boss, cx: &mut Cx) -> Option<usize> {
    let ty = (cx.cc.rand() % 15).abs();
    let sel = b.select_target(cx, ty, super::super::FAR);
    if sel.is_some_and(|s| cx.scene.chars[s].hp != 0) {
        return sel;
    }
    cx.scene.pc_list.iter().copied().find(|&c| cx.scene.chars[c].hp > 0).or(sel)
}

/// `Move` (QUA gcmn 0x003db820): `ccBoss::Move` but the push kept whole
/// (no ground under the body).
pub(super) fn move_body(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    let mut pp = super::super::w2p(cx, cx.scene.chars[me].pos);
    if !geom::eq(0, b.move_spd) {
        pp[0] = ee::add(pp[0], ee::mul(b.move_spd, libm::sinf(b.move_dirc)));
        pp[1] = ee::sub(pp[1], ee::mul(b.move_spd, libm::cosf(b.move_dirc)));
    }
    pp = add(pp, b.move_vector);
    let mut pos = super::super::p2w(cx, pp);
    if b.body_hit_sw != 0
        && let Some(off) = (cx.collide)(pos)
    {
        pos = add(pos, off);
        pp = add(pp, off);
    }
    let ch = &mut cx.scene.chars[me];
    ch.pos_p = pp;
    ch.pos = pos;
}

// --- the attacks -------------------------------------------------------------------------

/// `Kyvia_HellE_Damage(target)` (QUA gcmn 0x003df4a0), hell's express's
/// callback: each member alive struck from Kyvia, `target` 80% of its HP
/// and the rest 40% (at least 400), and the act on to its fifth step.
pub(super) fn hell_damage(b: &mut Boss, cx: &mut Cx, target: Option<usize>) {
    let me = cx.me;
    for m in cx.party.members {
        let Some(m) = m.filter(|&m| cx.valid(Some(m)) && cx.scene.chars[m].hp > 0) else { continue };
        let hp = f64::from(cx.scene.chars[m].hp);
        let d = (hp * if Some(m) == target { 0.8 } else { 0.4 }) as i32;
        let d = if d < 401 { 400 } else { d };
        as_me(cx, me, |cx| cx.affect(m, 1, d as i16));
    }
    b.act_proccess = 5;
}

/// `HellE_Atk`, act 21 (QUA gcmn 0x003e5a20): once the core is away (the
/// EX turn: at once) hell's express at the place's centre, the party
/// gathered there; its callback moves the act on; 60 frames later the
/// core back (the EX turn: its fade at once) and the act's end.
pub(super) fn hell_e_atk(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let ex = ex(x);
    match b.act_proccess {
        0 => {
            if ex {
                b.act_proccess += 1;
                b.lock_player(cx, false);
            } else if t.core.as_ref().is_some_and(|c| super::core::get_act_num(c, &t.gomoras)) {
                core_state(t, cx, 8);
                b.act_proccess += 1;
                b.lock_player(cx, false);
            }
        }
        1 => {
            if !ex && !core_state_is(t, 6) {
                return;
            }
            let target = rnd_target(b, cx);
            let lv = i32::from(x.disc_lv);
            let k = (x.disc_lv - 1) as usize;
            let mut at = fourth_of(x).core_appear_pos[k];
            super::hell::create(b, cx, at, target, lv - 1);
            together_char_pos(cx, &mut at, if ex { 0 } else { 1 });
            if !ex {
                fourth_of(x).core_appear_pos[k] = at;
            }
            b.act_proccess += 1;
        }
        5 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 60 {
                if !ex {
                    core_state(t, cx, 7);
                }
                b.act_proccess += 1;
            }
        }
        6 => {
            if ex {
                super::core_end_fade_ex(t, cx);
            } else if !core_fade_check(t) {
                return;
            }
            b.change_action(cx, act::NEUTRAL, 1, true);
            b.unlock_player();
        }
        _ => {}
    }
}

/// `BIGMegidFlame`, act 22 (QUA gcmn 0x003e5470): at 10 the player held;
/// at 30 stream 111 (the party standing); then the flash, the camera
/// before the body and the party gathered before it; at its tenth frame
/// the great meteor's light and the core's boss skill 55 on Kite; at 90
/// the blur's colour fading back, then the act's end.
pub(super) fn big_megid(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    match b.act_proccess {
        10 => {
            b.lock_player(cx, false);
            b.act_proccess += 1;
        }
        30 => {
            if !cx.annihilated() && !cx.game_over && cx.env.menu_type == -1 {
                cx.out(Out::StreamMenu { stream: 111, mask: 0 });
                b.act_proccess += 1;
            }
        }
        31 => {
            if cx.env.menu_type != -1 {
                return;
            }
            b.act_proccess += 1;
            cx.out(Out::Flash { t: 50, colour: 0x80ff_ffff });
            cx.out(Out::CamMode { mode: 5 });
            let pos = [0, 0xc57a_0000, 0x44af_0000, ONE];
            let view = [0, 0xc42f_0000, 0x4496_0000, ONE];
            cx.out(Out::FreeCam { pos, view });
            cx.out(Out::Blur(Set::Abgr(0x7080_8080)));
            cx.out(Out::Blur(Set::Scale(0x3f82_8f5c)));
            b.lock_player(cx, false);
            if ex(x) {
                let mut at = [0, 0xc4d4_8000, 0x447a_0000, ONE];
                together_char_pos(cx, &mut at, 0);
            }
            cx.out(Out::SeNote { se: 35, note: 60 });
            cx.out(Out::SeNote { se: 56, note: 60 });
        }
        32 => {
            b.act_count += 1;
            if b.act_count == 90 {
                b.act_proccess += 1;
            }
            if b.act_count == 10 {
                let at = [0, 0xc42f_0000, 0x4496_0000, ONE];
                particles(cx, Gen::Fourth(Make::BigMegid(0)), Place::At(at), None);
                let at = [0, 0xc3c8_0000, 0x44e1_0000, ONE];
                particles(cx, Gen::Fourth(Make::BigMegid(1)), Place::At(at), None);
                particles(cx, Gen::Fourth(Make::BigMegid(2)), Place::At(at), None);
                let kite = cx.party.members[0];
                if let Some(c) = t.core.as_mut() {
                    let cm = c.me;
                    as_me(cx, cm, |cx| c.b.skill_damage(cx, kite, None, 55));
                }
            }
        }
        33 => {
            let abgr = cx.disc.blur_abgr;
            if abgr <= x.default_argb && b.anm_status != 0 {
                cx.out(Out::Blur(Set::Scale(ONE)));
                cx.out(Out::Blur(Set::Abgr(x.default_argb)));
                b.change_action(cx, act::NEUTRAL, 1, true);
                b.unlock_player();
                cx.out(Out::CamMode { mode: 6 });
                if ex(x) {
                    super::core_end_fade_ex(t, cx);
                }
                b.act_proccess += 1;
            } else if x.default_argb < abgr {
                cx.out(Out::Blur(Set::Abgr(abgr.wrapping_add(0xff00_0000))));
            }
        }
        _ => b.act_proccess += 1,
    }
}

/// `Skill_Atk`, act 23 (QUA gcmn 0x003e5dc0): the player held and the
/// camera on the core; at 3, 10, 20, 40 and 50 one of
/// `Skill_KYVIA_VARIOUS` from the core on a member at random; at the
/// body's clip's end the act's end and the core's fade.
pub(super) fn skill_atk(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            b.act_proccess += 1;
            // `DMY_marker01`'s chunk is fetched and not read.
            particles(cx, Gen::Fourth(Make::Skill), Place::At([0, 0xc461_0000, 0x44bb_8000, ONE]), None);
            b.lock_player(cx, false);
            cx.out(Out::CamMode { mode: 5 });
            let core = t.core.as_ref().map_or(cx.me, |c| c.me);
            let view = cx.scene.chars[core].pos;
            let r = rand_f(cx.cc, 0x3e19_999a);
            let eye = core_eye(cx, r, view);
            cx.out(Out::FreeCam { pos: eye, view });
        }
        1 => {
            b.act_count += 1;
            if matches!(b.act_count, 3 | 10 | 20 | 40 | 50) {
                let target = rnd_target(b, cx);
                cx.scene.chars[cx.me].target_char = target;
                let k = (cx.cc.rand() % 7).unsigned_abs() as usize;
                let sid = cx.data.kyvia.kyvia_various.get(k).copied().unwrap_or(1);
                if let (Some(c), Some(tg)) = (t.core.as_ref(), target) {
                    let user = c.me;
                    if let Some(skill) =
                        crate::item::item_skill_request(cx.t, cx.scene, user, tg, i32::from(sid), 0, false, cx.rand)
                    {
                        cx.out(Out::SkillFrom { user, target: tg, skill });
                    }
                }
            }
            if b.anm_status != 0 {
                b.act_proccess += 1;
            }
        }
        2 => {
            cx.out(Out::CamMode { mode: 6 });
            b.unlock_player();
            b.change_action(cx, act::NEUTRAL, 0, true);
            super::core_end_fade_ex(t, cx);
        }
        _ => {}
    }
    let _ = x;
}

/// The eye `Skill_Atk` and the EX ball put 4500 off the core turned by
/// `r`, 800 up. The vector's x and w are a local the code never writes:
/// the stack's word there, the last `Think` callee's (docs), 0.
pub(super) fn core_eye(cx: &Cx, r: F, core: V4) -> V4 {
    let _ = cx;
    let m = geom::rot_matrix_z(&geom::unit_matrix(), r);
    add(geom::apply_matrix(&m, [0, 0xc58c_a000, 0x4448_0000, 0]), core)
}

/// `MagicBallAtk` of the fourth fight, act 20 (QUA gcmn 0x003e3c60):
/// the gathering over Kite and the flash, the party gathered at the place
/// at 11, the second gathering at 35; once the flash is out the ball
/// (the EX turn's eye on the core); the ball's callback moves it on; the
/// colour back, then the act's end.
pub(super) fn magic_ball_atk(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let ex = ex(x);
    match b.act_proccess {
        0 => {
            if b.act_count == 0 {
                particles(cx, Gen::Fourth(Make::Ball(0)), Place::At(x.disc_pos), None);
            }
            if b.act_count == 20 {
                b.act_proccess += 1;
                b.act_count = 0;
                cx.out(Out::FlashFade { t_in: 10, t_out: 5, hold: Some(10), colour: 0x80ff_ffff });
                let free = cx.disc.fades.iter().position(|e| e.status == 0);
                fourth_of(x).fade_num = free.map_or(-1, |k| k as i32);
                b.lock_player(cx, false);
            }
            b.act_count += 1;
        }
        1 => {
            if b.act_count == 11 {
                let k = (x.disc_lv - 1) as usize;
                let mut at = fourth_of(x).core_appear_pos[k];
                together_char_pos(cx, &mut at, 0);
                fourth_of(x).core_appear_pos[k] = at;
            }
            if b.act_count == 35 {
                b.act_proccess += 1;
                b.act_count = 0;
                particles(cx, Gen::Fourth(Make::Ball(1)), Place::At(x.disc_pos), None);
            }
            b.act_count += 1;
        }
        2 => {
            // `CheckFade(FadeNum)`: the element still counting.
            let n = fourth_of(x).fade_num;
            let busy = usize::try_from(n).ok().and_then(|k| cx.disc.fades.get(k)).is_some_and(|e| e.cnt < e.tcnt);
            if !busy {
                b.act_proccess = 5;
            }
        }
        5 => {
            if ex {
                b.act_proccess += 1;
            } else if core_ready(t) {
                core_state(t, cx, 8);
                b.act_proccess += 1;
            }
        }
        6 => {
            if ex {
                b.act_proccess += 1;
                cx.out(Out::CamMode { mode: 5 });
                cx.out(Out::Blur(Set::Abgr(0x6080_8080)));
                let core = t.core.as_ref().map_or(cx.me, |c| c.me);
                let r = rand_f(cx.cc, 0x3e19_999a);
                let view = cx.scene.chars[core].pos;
                let eye = core_eye(cx, r, view);
                cx.out(Out::SeNote { se: 191, note: 60 });
                cx.out(Out::FreeCam { pos: eye, view });
            } else if core_state_is(t, 6) {
                b.act_proccess += 1;
                let me = cx.me;
                let d = super::super::dist(cx, cx.scene.chars[me].pos, x.disc_pos);
                cx.out(Out::CamModeRange { mode: 3, range: ee::add(0x44fa_0000, d) });
                cx.out(Out::Blur(Set::Abgr(0x6080_8080)));
                cx.out(Out::SeNote { se: 191, note: 60 });
            }
        }
        7 => {
            if ex || cx.cam.reset == 0 {
                b.act_proccess += 1;
                super::magic_ball::create(b, x, cx);
                if ex {
                    x.quake_vector[..3].copy_from_slice(&[0x4120_0000; 3]);
                }
            }
        }
        8 => {
            b.act_count += 1;
            if ex && b.act_count >= 36 {
                let q = x.quake_vector;
                cx.quake_vec([q[0], q[1], q[2]]);
            }
        }
        9 => {
            if ex || cx.cam.reset == 0 {
                b.act_proccess += 1;
                cx.out(Out::Blur(Set::Abgr(x.default_argb)));
            }
        }
        10 => {
            if ex {
                cx.out(Out::CamMode { mode: 6 });
                super::core_end_fade_ex(t, cx);
            } else if !core_fade_check(t) {
                return;
            }
            b.change_action(cx, act::NEUTRAL, 1, true);
            b.unlock_player();
        }
        _ => {}
    }
}

/// `KyviaMagicalBallDamage` of the fourth fight (QUA gcmn 0x003df3d0): 0
/// and 1 the core's boss skills 74 and 75 round `pos`; 2 the attack on,
/// the core's state 7 and the camera back.
pub(super) fn ball_damage(b: &mut Boss, t: &mut Tree, cx: &mut Cx, pos: V4, flg: i32) {
    match flg {
        0 | 1 => {
            let Some(c) = t.core.as_mut() else { return };
            let cm = c.me;
            let skill = if flg == 0 { 74 } else { 75 };
            as_me(cx, cm, |cx| c.b.skill_damage(cx, None, Some(pos), skill));
        }
        2 => {
            b.act_proccess += 1;
            core_state(t, cx, 7);
            cx.out(Out::CamMode { mode: 4 });
        }
        _ => {}
    }
}

// --- the rebirth and the death -----------------------------------------------------------

/// The rebirth, act 24 (QUA gcmn 0x003dde40): the player held and the
/// body off the lists; at 2 the flash, the blur, the body's light and the
/// camera before it; its cries each eighth frame; at 30 (the clip done)
/// stream 85; at 35 `bossTbl` row 15's `maxHP` 9999 and the EX turn
/// again with the core reborn, the body's HP new; at 36 the body back.
pub(super) fn rebirth(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            b.lock_player(cx, false);
            as_me(cx, me, |cx| b.erase_cmnd_target(cx));
            b.act_proccess += 1;
            fourth_of(x).rebirth_cnt = 0;
        }
        2 => {
            b.act_proccess += 1;
            cx.out(Out::FlashFade { t_in: 5, t_out: 5, hold: Some(5), colour: 0x80ff_ffff });
            cx.out(Out::Blur(Set::Scale(0x3f82_8f5c)));
            cx.out(Out::Blur(Set::Abgr(0x3080_8080)));
            let at = add(cx.scene.chars[me].pos, [0, 0xc42f_0000, 0x44ce_4000, 0]);
            b.anm.speed = 128;
            particles(cx, Gen::Fourth(Make::Rebirth(0)), Place::At(at), None);
            particles(cx, Gen::Fourth(Make::Rebirth(1)), Place::At(at), None);
            cx.out(Out::CamMode { mode: 5 });
            let pos = [0, 0xc57a_0000, 0x44af_0000, ONE];
            let view = [0, 0xc42f_0000, 0x4496_0000, ONE];
            cx.out(Out::FreeCam { pos, view });
        }
        30 => {
            if (b.anm.frame() as u32) < 68 && fourth_of(x).rebirth_cnt % 8 == 0 {
                let n = 65 + cx.cc.rand() % 10;
                cx.out(Out::SeNote { se: 191, note: n as u8 });
            }
            if b.anm_status != 0 {
                cx.out(Out::CamMode { mode: 6 });
                b.act_proccess += 1;
                cx.out(Out::StreamMenu { stream: 85, mask: 0 });
                cx.out(Out::Blur(Set::Scale(ONE)));
                cx.out(Out::Blur(Set::Abgr(x.default_argb)));
                b.anm.speed = 256;
            }
        }
        35 => {
            x.row15_hp = Some(9999);
            ex_on(b, x, t, cx);
            super::set_base_param(cx, me, 15);
            apply_row15(x, cx, me);
            b.act_proccess += 1;
        }
        36 => {
            as_me(cx, me, |cx| b.entry_cmnd_target(cx));
            b.unlock_player();
            b.change_action(cx, act::NEUTRAL, 0, true);
            fourth_of(x).rebirth_flg = 1;
        }
        _ => {
            b.act_proccess += 1;
            if fourth_of(x).rebirth_cnt % 8 == 0 {
                let n = 65 + cx.cc.rand() % 10;
                let pos = cx.scene.chars[me].pos;
                cx.out(Out::Se3dNote { se: 191, pos, note: n as u8 });
            }
        }
    }
    fourth_of(x).rebirth_cnt += 1;
}

/// `bossTbl` row 15 as the rebirth left it: `maxHP` 9999.
pub(super) fn apply_row15(x: &Kyvia, cx: &mut Cx, who: usize) {
    if let Some(hp) = x.row15_hp {
        let ch = &mut cx.scene.chars[who];
        ch.max_hp = hp;
        ch.hp = hp;
        if let Some(f) = ch.foe_state_mut() {
            f.row.max_hp = hp;
        }
    }
}

/// The body's death, act 14 of the fourth fight (QUA gcmn 0x003de350),
/// after the rebirth only: the cinema, the boss's gauge (`dummyTarget`)
/// filling with the core's HP, the flash and the quake; the smoke rising,
/// the cries, the camera and the light (`DeadKyviaRipus`); the party
/// gathered at 11; then the exit.
pub(super) fn dead(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    if fourth_of(x).rebirth_flg == 0 {
        return;
    }
    let me = cx.me;
    match b.act_proccess {
        0 => {
            b.lock_player(cx, false);
            cx.out(Out::Cinema(Some(71)));
            cx.out(Out::DummyTarget(Some(me)));
            cx.out(Out::CursorOff(true));
            let core_hp = t.core.as_ref().map_or(0, |c| cx.scene.chars[c.me].hp);
            cx.scene.chars[me].hp = core_hp;
            x.dead_gp = false;
            b.act_proccess += 1;
            b.act_count = 0;
            let pos = cx.scene.chars[me].pos;
            cx.out(Out::Se3dNote { se: 192, pos, note: 48 });
            cx.out(Out::FlashFade { t_in: 5, t_out: 5, hold: Some(5), colour: 0x80ff_ffff });
            cx.out(Out::Blur(Set::Abgr(0x7080_8080)));
            cx.out(Out::Blur(Set::Scale(0x3f82_8f5c)));
            cx.out(Out::MusicFade { t: 30 });
            x.quake_vector[..3].copy_from_slice(&[0x41a0_0000, 0x4120_0000, 0x41a0_0000]);
        }
        1 => {
            b.act_count += 1;
            let c = b.act_count;
            if c == 60 {
                for k in (0..4u8).filter(|&k| x.dmg_gp[usize::from(k)]) {
                    gen_op(cx, Held::Smoke(k), GenOp::Kill);
                }
            }
            if c == 11 {
                let mut at = fourth_of(x).dead_set_pos;
                together_char_pos(cx, &mut at, 1);
                fourth_of(x).dead_set_pos = at;
            }
            if c < 11 {
                x.ded_dmg_vec[0] = ee::add(x.ded_dmg_vec[0], 0x40a0_0000);
            }
            if c >= 135 {
                x.ded_dmg_vec[0] = ee::add(x.ded_dmg_vec[0], ONE);
                x.ded_dmg_vec[1] = ee::sub(x.ded_dmg_vec[1], 0x3f00_0000);
            }
            if ee::lt(ONE, ee::mul(0x42c8_0000, x.cam_pitch)) {
                let v = ee::sub(x.cam_pitch, ee::div(x.cam_pitch, 0x41a0_0000));
                super::set_pitch(x, cx, v);
            }
            super::dead_voice(b, x.fight, cx);
            super::dead_cam(b, x, cx);
            dead_ripus(b, cx);
            x.ded_dmg_vec[2] = ee::sub(x.ded_dmg_vec[2], ONE);
            x.ded_dmg_vec[1] = ee::add(x.ded_dmg_vec[1], 0x4080_0000);
            let q = x.quake_vector;
            cx.quake_vec([q[0], q[1], q[2]]);
            let max = cx.scene.chars[me].max_hp;
            if let Some(core) = t.core.as_ref() {
                let ch = &mut cx.scene.chars[core.me];
                if ch.hp < max {
                    ch.hp = ch.hp.wrapping_add(20);
                } else {
                    ch.hp = max;
                    b.act_proccess += 1;
                    b.act_count = 0;
                }
                let hp = cx.scene.chars[core.me].hp;
                cx.scene.chars[me].hp = hp;
            }
        }
        2 => {
            if x.dead_gp {
                gen_op(cx, Held::Smoke(4), GenOp::Kill);
                gen_op(cx, Held::Smoke(4), GenOp::Forget);
            }
            x.dead_gp = false;
            b.act_proccess += 1;
        }
        3 => {
            b.exit = 1;
            b.unlock_player();
            cx.out(Out::DummyTarget(None));
            cx.out(Out::CursorOff(false));
            cx.out(Out::Cinema(None));
        }
        _ => {}
    }
}

/// `DeadKyviaRipus` (QUA gcmn 0x003db430): the death's lights 700 before
/// the body and 1500 up, at 30, 50 and 170.
fn dead_ripus(b: &Boss, cx: &mut Cx) {
    let at = add(cx.scene.chars[cx.me].pos, [0, 0xc42f_0000, 0x44bb_8000, ONE]);
    let rows: &[u8] = match b.act_count {
        30 => &[0],
        50 => &[1, 2],
        170 => &[3, 4, 5],
        _ => &[],
    };
    for &r in rows {
        particles(cx, Gen::Fourth(Make::Ripus(r)), Place::At(at), None);
    }
}
