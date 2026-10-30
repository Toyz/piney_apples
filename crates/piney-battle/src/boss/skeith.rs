//! Skeith (`ccBoss01`, boss01.cpp, `bossTbl` row 0), the first boss: his
//! tables, his constructor ([`new`]), his `Main`, `Affect`, `Think`,
//! `Action` and pattern code over `ccBoss`'s (docs/engine/boss.md). No
//! later boss derives from `ccBoss01` (the DWARF of every boss class names
//! `ccBoss` as its base), so nothing here is shared.

use super::*;

/// Skeith's tables (`tables::combat`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkeithData {
    /// `boss01NormalActTbl`, `boss01SuperActTbl`, `boss01EpitaphActTbl`
    /// (INF gcmn 0x005eb430, 0x005eb490, 0x005eb530, listed by @1035 at
    /// 0x005eb598): the words to their `-1` and one past it.
    pub act_tbls: [Vec<i32>; 3],
    /// @1378 (INF 0x005eb608): pattern 10's skills, one by `ccRand() % 3`.
    pub rand_skills: [i32; 3],
    /// `Boss01AnmTbl` (INF 0x005eb5b0): the act's animation (22 names,
    /// None for none).
    pub anm_tbl: Vec<Option<String>>,
}

impl SkeithData {
    /// `patTbl`'s words.
    pub(crate) fn words(&self, t: Tbl) -> &[i32] {
        match t {
            Tbl::None => &[],
            Tbl::Normal => &self.act_tbls[0],
            Tbl::Super => &self.act_tbls[1],
            Tbl::Epitaph => &self.act_tbls[2],
        }
    }

    /// The volume's.
    pub fn of(volume: piney_data::volume::Volume) -> SkeithData {
        let t = piney_data::tables::combat::of(volume);
        let acts = t.skeith_acts;
        let r = t.skeith_rand_skills;
        SkeithData {
            act_tbls: [acts[0].to_vec(), acts[1].to_vec(), acts[2].to_vec()],
            rand_skills: [r[0], r[1], r[2]],
            anm_tbl: t.skeith_anims.iter().map(|a| a.map(str::to_string)).collect(),
        }
    }
}

/// `ccBoss01`'s own members (Skeith's): its spell's force and ice
/// effects, its death's effect and fades, the stage effect it holds, the
/// wave's ring clip, `m_patMode` (+0x29644) and `m_actDirc` (+0x29650).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Skeith {
    pub eff_force: Option<i32>,
    pub eff_ice_break: Option<i32>,
    pub eff_dead: Option<i32>,
    pub animate: Fade,
    pub animate_dead: Fade,
    pub stage_eff_id: i32,
    pub anm_wave: Anm,
    pub pat_mode: i32,
    pub act_dirc: V4,
}

impl Shape for Skeith {
    fn wave(&self) -> Option<&Anm> {
        Some(&self.anm_wave)
    }

    /// `ccBoss01::DrawCross` (0x0047c390): `m_lattice` (+0x2935c), a row a
    /// frame in the cross, while it stands.
    fn sword_trail(&self, b: &BossBase) -> Option<bool> {
        Some(b.act_num == act::CROSS && b.exit == 0)
    }
}

impl BossClass for Skeith {
    fn main(&mut self, b: &mut BossBase, cx: &mut Cx) {
        main(b, self, cx);
    }

    fn affect(&mut self, b: &mut BossBase, cx: &mut Cx) {
        affect(b, self, cx);
    }
}

/// The constructors (`ccBoss::ccBoss` 0x0045bcf0, `ccBoss01::ccBoss01`
/// 0x0047b410) as far as the rules go: Skeith stands 500 behind Kite
/// (`plw.pos` less 500 on y, Kite's heading), its break-proof HP on,
/// drawn and hit, in act 0 (its animation set), on the command lists,
/// the Normal patterns set (the first run's index discarded: the first
/// `ChangeNextPattern` runs pattern 0 again). `centerPos` is the event
/// area's `DMY_center01` (`center`, world).
pub fn new(cx: &mut Cx, kite_pos: V4, kite_dirc: V4, center: V4) -> Boss {
    let mut b = BossBase {
        anm_tbl: cx.data.skeith.anm_tbl.clone(),
        wait_pat_num: -2,
        pat_num: 13,
        // InitStageEffect: the stage fader and its layer made.
        use_stage_eff: true,
        transparency: ONE,
        set_transparency: ONE,
        anm: Anm::new(),
        ..BossBase::default()
    };
    let mut skeith = Skeith {
        stage_eff_id: -1,
        anm_wave: Anm::new(),
        // the ccAnimateObjects start opaque
        animate: Fade { transparency: ONE, ..Fade::default() },
        animate_dead: Fade { transparency: ONE, ..Fade::default() },
        ..Skeith::default()
    };
    // ccBoss01's On/Off: exit off, drawn, body hit on, cheat HP on.
    b.exit = 0;
    b.draw_sw = 1;
    b.body_hit_sw = 1;
    b.cheat_hp = 1;
    let me = cx.me;
    let ch = &mut cx.scene.chars[me];
    let mut pos = kite_pos;
    pos[1] = ee::sub(pos[1], 0x43fa_0000);
    ch.pos = pos;
    b.dirc = kite_dirc;
    b.change_action(cx, act::NEUTRAL, 0, true);
    crate::fellow::entry_cmnd(cx.scene, me);
    b.stop_time = 0;
    b.stop_counter = 0;
    b.set_pattern_tbl(Tbl::Normal);
    let tbl = cx.data.skeith.words(Tbl::Normal);
    let _ = skeith.exec_pattern_index(&mut b, cx, tbl, 0);
    skeith.pat_mode = 0;
    b.center_pos = [center[0], center[1], center[2], ONE];
    b.center_pos_p = w2p(cx, b.center_pos);
    // The blur on under `ccBoss03BlurParamCB` (INF gcmn 0x0047bc40).
    cx.out(Out::Blur(blur::Set::Enable));
    cx.out(Out::Blur(blur::Set::Func(blur::BlurFunc::Boss03)));
    Boss { base: b, class: Class::Skeith(Box::new(skeith)) }
}

/// Skeith's patterns: `ccBoss01::ExecPatternIndex` over the base's, the
/// neutral let go, both w lanes 0.
impl Patterns for Skeith {
    const ACTS: BaseActs = BaseActs { dash_w: 0, still_w: 0 };

    fn words<'d>(&self, b: &BossBase, data: &'d BossData) -> &'d [i32] {
        data.skeith.words(b.pat_tbl)
    }

    /// `ccBoss01::ExecPatternIndex(tbl, i)` (0x0047c500), falling back on
    /// `ccBoss::ExecPatternIndex` (0x0045cad0): the next index.
    fn exec_pattern_index(&mut self, b: &mut BossBase, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
        let mut orig = i;
        let mut i = i;
        let mut pat = word(tbl, i);
        i += 1;
        if pat == -1 {
            i = 1;
            orig = 1;
            pat = word(tbl, 0);
        }
        b.pat_num = pat;
        match pat {
            2 => {
                let t = word(tbl, i);
                i += 1;
                b.stop_time = t;
                b.stop_counter = 0;
                if t == -1 && !cx.annihilated() {
                    b.stop_time = 30;
                    b.stop_counter = 0;
                }
                neutral_act(b, cx, 3, 1);
            }
            1 => {
                if b.epitaph != 0 {
                    b.change_action(cx, act::EPITAPH_WAVE, 3, true);
                } else {
                    b.change_action(cx, act::WAVE, 3, true);
                }
            }
            14 => b.change_action(cx, act::CROSS, 3, true),
            15 => b.change_action(cx, act::DRAIN, 3, true),
            16 => b.change_action(cx, act::MAGIC, 3, true),
            9..=11 => {
                let me = cx.me;
                let mut target = cx.scene.chars[me].target_char;
                if pat != 11 {
                    let ty = word(tbl, i);
                    i += 1;
                    target = b.select_target(cx, ty, FAR);
                    cx.scene.chars[me].target_char = target;
                }
                if !cx.valid(target) || target.is_some_and(|t| cx.dead(t)) {
                    target = b.select_target(cx, 3, FAR);
                    cx.scene.chars[me].target_char = target;
                    if !cx.valid(target) || target.is_some_and(|t| cx.dead(t)) {
                        self.exec_pattern(b, cx, 2, 30);
                        return orig;
                    }
                }
                b.calc_target_info(cx);
                let (on, sid) = if pat == 10 {
                    let k = (cx.cc.rand() % 3).unsigned_abs() as usize;
                    (target, cx.data.skeith.rand_skills[k])
                } else {
                    let sid = word(tbl, i);
                    i += 1;
                    (if pat == 9 { target } else { Some(me) }, sid)
                };
                if let Some(tp) = on
                    && let Some(s) = item::item_skill_request(cx.t, cx.scene, me, tp, sid, 0, false, cx.rand)
                {
                    cx.out(Out::Skill(tp, s));
                }
                self.exec_pattern(b, cx, 2, -2);
            }
            _ => return base_exec_pattern_index(self, b, cx, tbl, orig),
        }
        i
    }
}

/// `ccBoss01::Main` (gcmn 0x0047bd50), with the manager's pass before:
/// `ccBoss::Main`'s blocks around `Think` and `Action`, and its own
/// `PreDrawAnm` (0x0047c340): the wave's speed 512 in its acts.
fn main(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    b.effects.tick();
    if b.exit == 0 {
        b.prologue(cx);
        b.tick_stop();
        think(b, s, cx);
        action(b, s, cx);
        b.lock_party(cx);
        b.move_(cx);
        if b.draw_sw != 0 {
            if b.act_num == act::WAVE || b.act_num == act::EPITAPH_WAVE {
                s.anm_wave.speed = 512;
            }
            b.forward_and_draw(cx);
        }
    }
    let me = cx.me;
    cx.scene.chars[me].cond[cond::HOLD] = 0;
    // DrawCross (0x0047c390): in the cross it finds the sword's dummies
    // once (proccess 0 to 1) and draws its trail (the runtime's).
    if b.act_num == act::CROSS && b.act_proccess == 0 {
        b.act_proccess = 1;
    }
    if (b.lock_player != 0 || b.erase_target != 0) && b.epitaph == 0 {
        cx.pp_count_tick();
    }
}

/// `ccBoss01::Think` (0x0047c1e0).
fn think(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    match b.act_num {
        act::NEUTRAL => on_neutral(b, s, cx),
        act::DAMAGE0 | act::DAMAGE1 => on_damage(b, s, cx),
        act::CROSS => on_cross(b, s, cx),
        act::WAVE => on_wave(b, s, cx),
        act::DRAIN => on_drain(b, s, cx),
        act::MAGIC => on_magic(b, s, cx),
        act::DASH => on_dash(b, s, cx),
        act::WANDER => on_wander(b, s, cx),
        act::EPITAPH_NEUTRAL => on_epitaph_neutral(b, s, cx),
        act::EPITAPH_WAVE => on_epitaph_wave(b, s, cx),
        act::ESCAPE => on_escape(b, s, cx),
        act::RETURN => on_return(b, s, cx),
        act::CHASE => on_chase(b, s, cx),
        act::RAND_DRIVE => {}
        act::EPITAPH | act::DEAD => {}
        _ => b.change_action(cx, act::NEUTRAL, 0, true),
    }
}

/// `OnThinkEpitaphWave` (0x0047cdc0).
fn on_epitaph_wave(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            s.anm_wave.set(ANM_WAVE, cx.clips);
            b.act_proccess += 1;
            cx.se3d(223);
            return;
        }
        1 => {
            let f = b.anm.frame();
            if f == 20 {
                cx.se3d(224);
            } else if f >= 29 {
                if f == 29 {
                    cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
                    b.effect(cx, EffKind::WaveShock);
                }
                let done = s.anm_wave.forward();
                s.anm_wave.speed = 512;
                cx.out(Out::DrawWave);
                cx.quake_cam(0x420c_0000);
                if b.anm_status != 0 && done {
                    b.change_action(cx, act::EPITAPH_NEUTRAL, 3, true);
                }
            }
        }
        _ => return,
    }
    let c = b.act_count;
    b.act_count += 1;
    if c == 30 {
        let pos = cx.pos();
        b.skill_damage(cx, None, Some(pos), SKILL_WAVE);
    }
}

/// `OnThinkChase` (0x0047d000).
fn on_chase(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            b.spd_up(0x4316_0000, 0x4316_0000);
            b.erase_cmnd_target(cx);
            cx.cursor(true);
            s.animate.set(b.transparency, HALF, 0x4170_0000);
            cx.se3d(229);
            let t = b.select_target(cx, 0, FAR);
            cx.scene.chars[cx.me].target_char = t;
            b.calc_target_info(cx);
            b.act_proccess += 1;
        }
        1 => {
            if c & 3 == 3 {
                cx.out(Out::AfterImage);
                if c == 3 {
                    cx.se3d(57);
                }
            }
            let me = cx.me;
            let t = cx.scene.chars[me].target_char;
            if cx.valid(t) {
                b.move_dirc = b.target_dirc;
                if ee::lt(b.target_dist, 0x43c8_0000) {
                    s.change_next_pattern(b, cx);
                    chase_end(b, cx);
                    return;
                }
            } else {
                let t = b.select_target(cx, 0, FAR);
                cx.scene.chars[me].target_char = t;
                if !cx.valid(t) {
                    s.exec_pattern(b, cx, 2, -1);
                    chase_end(b, cx);
                    return;
                }
                b.calc_target_info(cx);
            }
            if c >= 90 {
                b.act_proccess += 1;
            }
            b.dirc[2] = b.move_dirc;
            s.animate.animate();
            b.set_transparency = s.animate.transparency;
        }
        2 => chase_end(b, cx),
        _ => {}
    }
}

/// `OnThinkDash` (0x0047ddf0).
fn on_dash(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    b.spd_up(0x4248_0000, 0x4248_0000);
    let me = cx.me;
    if b.act_proccess == 0 {
        s.animate.set(b.transparency, HALF, 0x4170_0000);
        b.erase_cmnd_target(cx);
        cx.cursor(true);
        b.act_proccess += 1;
        cx.se3d(229);
    }
    match b.act_proccess {
        1 => {
            if b.act_count & 3 == 3 {
                cx.out(Out::AfterImage);
                if b.act_count == 3 {
                    cx.se3d(57);
                }
            }
            if b.act_count >= 121 {
                b.act_proccess += 1;
                return;
            }
            let p = w2p(cx, b.dash_pos);
            let pp = cx.scene.chars[me].pos_p;
            b.move_dirc = get_dirc(pp, p);
            if ee::lt(dist(cx, pp, p), 0x4348_0000) {
                b.act_proccess += 1;
            }
            b.dirc[2] = b.move_dirc;
            s.animate.animate();
            b.set_transparency = s.animate.transparency;
            b.act_count += 1;
        }
        2 => {
            b.entry_cmnd_target(cx);
            cx.cursor(false);
            s.change_next_pattern(b, cx);
            b.set_transparency = ONE;
        }
        _ => {}
    }
}

/// `OnWaveAtk` (0x0047e1a0).
fn on_wave(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            let c = b.act_count;
            b.act_count += 1;
            if c >= 73 {
                s.anm_wave.set(ANM_WAVE, cx.clips);
                b.act_count = 0;
                b.act_proccess += 1;
                cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
                b.effect(cx, EffKind::WaveShock);
            }
            match b.anm.frame() {
                10 => cx.se3d(223),
                65 => cx.se3d(224),
                _ => {}
            }
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 25 {
                let pos = cx.pos();
                b.skill_damage(cx, None, Some(pos), SKILL_WAVE);
            }
            cx.quake_cam(0x420c_0000);
            let done = s.anm_wave.forward();
            cx.out(Out::DrawWave);
            if b.anm_status != 0 && done {
                s.change_next_pattern(b, cx);
            }
        }
        _ => {}
    }
    b.spd_down(0, 0x40a0_0000);
}

/// `OnMagicAtk` (0x0047e6a0): its steps wait on its effects.
fn on_magic(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            let t = cx.scene.chars[me].target_char;
            if !cx.valid(t) {
                let t = b.select_target(cx, 0, FAR);
                cx.scene.chars[me].target_char = t;
                if !cx.valid(t) {
                    s.change_next_pattern(b, cx);
                    return;
                }
                b.calc_target_info(cx);
            }
            b.lock_player(cx, true);
            if b.use_stage_eff && s.stage_eff_id >= 0 {
                cx.out(Out::StageEnd { rgba: 0, t: -1 });
                s.stage_eff_id = -1;
            }
            cx.out(Out::SwitchLayer);
            if b.use_stage_eff {
                s.stage_eff_id = 0;
                cx.out(Out::StageBegin { rgba: 0x6400_0000, t: [15, 15, 32767] });
            }
            b.erase_cmnd_target(cx);
            cx.cursor(true);
            b.anm.set(ANM_MAGIC, cx.clips);
            b.effect(cx, EffKind::MagicSquare { n: 0 });
            cx.se3d(225);
            cx.out(Out::Cinema(Some(2)));
            b.act_proccess += 1;
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 30 {
                for m in cx.party.members.into_iter().flatten() {
                    if cx.valid(Some(m)) && !cx.dead(m) {
                        let mut p = cx.scene.chars[m].pos;
                        p[2] = 0x447a_0000;
                        let rot = [0x3fc9_0fdb, 0, 0, ONE];
                        let k = EffKind::ForceGenerator {
                            num: 8,
                            life: 100,
                            speed: 0x4120_0000,
                            r0: 0x4348_0000,
                            r1: 0x4348_0000,
                            clt: 8,
                        };
                        s.eff_force = Some(b.effect_at(cx, k, p, rot));
                        cx.se3d(226);
                    }
                }
                b.act_proccess += 1;
            }
        }
        2 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 130 {
                cx.se3d(227);
            }
            if b.effects.enabled(s.eff_force) {
                return;
            }
            let rot = [0, 0, b.target_dirc, 0];
            s.eff_force = Some(b.effect_at(cx, EffKind::AutoSamonRing { n: 35 }, b.target_pos, rot));
            b.act_count = 0;
            b.act_proccess += 1;
        }
        3 => {
            if b.effects.enabled(s.eff_force) {
                return;
            }
            let mut none = true;
            for m in cx.party.members.into_iter().flatten() {
                if cx.valid(Some(m)) && !cx.dead(m) {
                    let p = cx.scene.chars[m].pos;
                    s.eff_ice_break = Some(b.effect_at(cx, EffKind::IceBreak, p, VF0));
                    none = false;
                }
            }
            if none {
                let p = b.target_pos;
                s.eff_ice_break = Some(b.effect_at(cx, EffKind::IceBreak, p, VF0));
            }
            b.draw_sw = 0;
            if b.use_stage_eff && s.stage_eff_id >= 0 {
                cx.out(Out::StageEnd { rgba: 0x6400_0000, t: 15 });
                s.stage_eff_id = -1;
            }
            cx.out(Out::Se { se: 228 });
            b.act_proccess += 1;
        }
        4 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 15 {
                cx.out(Out::SwitchLayer);
            }
            if b.effects.enabled(s.eff_ice_break) {
                cx.out(Out::Reverse);
                return;
            }
            b.draw_sw = 1;
            b.entry_cmnd_target(cx);
            cx.cursor(false);
            b.unlock_player();
            if b.use_stage_eff && s.stage_eff_id >= 0 {
                cx.out(Out::StageEnd { rgba: 0x6400_0000, t: 15 });
                s.stage_eff_id = -1;
            }
            s.eff_ice_break = None;
            let tp = b.target_pos;
            b.skill_damage(cx, None, Some(tp), SKILL_MAGIC);
            let pos = cx.pos();
            cx.out(Out::Se3dNote { se: 66, pos, note: 52 });
            cx.se3d(65);
            cx.se3d(56);
            cx.out(Out::Cinema(None));
            s.change_next_pattern(b, cx);
        }
        _ => {}
    }
}

/// `ccBoss01::Action` (0x0047bf80).
fn action(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    match b.act_num {
        act::EPITAPH => {
            b.cheat_hp = 0;
            b.set_pattern_tbl(Tbl::Epitaph);
            let tbl = cx.data.skeith.words(Tbl::Epitaph);
            // The index is discarded, as in the constructor.
            let _ = s.exec_pattern_index(b, cx, tbl, 0);
            s.pat_mode = 2;
            b.change_action(cx, act::EPITAPH_NEUTRAL, 3, true);
            b.move_spd = 0;
        }
        act::DEAD => match b.act_proccess {
            0 => {
                b.move_spd = 0;
                b.move_vector = [0; 4];
                crate::fellow::delete_cmnd(cx.scene, cx.me);
                cx.out(Out::DeleteCmnd);
                s.eff_dead = Some(b.begin_dead_effect(cx, 0x44fa_0000, 0x44fa_0000));
                s.animate_dead.set(ONE, 0, 0x41f0_0000);
                b.act_proccess += 1;
            }
            1 => {
                if s.eff_dead.is_some() && !b.effects.enabled(s.eff_dead) {
                    s.eff_dead = None;
                }
                if b.anm_status != 0 && s.eff_dead.is_none() {
                    b.act_proccess += 1;
                }
            }
            2 => {
                let c = b.act_count;
                b.act_count += 1;
                if c >= 30 {
                    b.exit = 1;
                }
            }
            _ => {}
        },
        _ => {}
    }
}

/// `ccBoss01::Affect` (Skeith's, and a plain boss's).
fn affect(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    let me = cx.me;
    let t = cx.scene.chars[me].affect.ty;
    if t != 21 && (b.act_forbid == 2 || b.lock_player != 0) {
        return;
    }
    b.base_affect(cx);
    let t = cx.scene.chars[me].affect.ty;
    if t == 21 {
        let ch = &mut cx.scene.chars[me];
        ch.hp = 4500;
        ch.max_hp = 4500;
    } else if (t == 1 || t == 3) && s.pat_mode == 0 {
        let pp = cx.scene.param(me).map_or(0, |p| p.pp);
        let id = usize::try_from(cx.scene.chars[me].id()).unwrap_or(0);
        let max = cx.t.bosses.get(id).map_or(0, |r| r.max_pp);
        if pp >= max >> 1 {
            b.set_pattern_tbl(Tbl::Super);
            let tbl = cx.data.skeith.words(Tbl::Super);
            let _ = s.exec_pattern_index(b, cx, tbl, 0);
            s.pat_mode = 1;
        }
    }
}

/// `OnThinkNeutral` (0x0047ca50).
fn on_neutral(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    b.move_spd = 0;
    b.move_vector = [0; 4];
    b.set_dirc(b.target_dirc);
    if !b.wait_over(cx) {
        return;
    }
    let t = b.select_target(cx, 0, FAR);
    cx.scene.chars[cx.me].target_char = t;
    if cx.valid(t) {
        b.calc_target_info(cx);
        s.change_next_pattern(b, cx);
    }
}

/// `OnThinkDamage` (0x0047cba0).
fn on_damage(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    on_neutral(b, s, cx);
    if b.anm.frame() == 0 {
        cx.se3d(195);
        b.change_action(cx, act::NEUTRAL, 1, false);
    }
}

/// `OnThinkEpitaphNeutral` (0x0047cc20).
fn on_epitaph_neutral(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    let c = b.act_count;
    b.act_count += 1;
    b.move_spd = 0;
    b.move_vector = [0; 4];
    b.set_dirc(b.move_dirc);
    if !b.wait_over(cx) {
        return;
    }
    let t = b.select_target(cx, 0, FAR);
    cx.scene.chars[cx.me].target_char = t;
    if !cx.valid(t) {
        return;
    }
    b.calc_target_info(cx);
    if b.pat_num == 8 && c < 300 && !ee::le(b.target_dist, 0x4461_0000) {
        return;
    }
    s.change_next_pattern(b, cx);
}

fn chase_end(b: &mut BossBase, cx: &mut Cx) {
    b.set_transparency = ONE;
    b.entry_cmnd_target(cx);
    cx.cursor(false);
}

/// `OnThinkEscape` (0x0047d2f0).
fn on_escape(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    let c = b.act_count;
    b.act_count += 1;
    if c >= 91 {
        b.entry_cmnd_target(cx);
        cx.cursor(false);
        s.change_next_pattern(b, cx);
        return;
    }
    if b.stop != 0 || cx.scene.chars[cx.me].cond[cond::HOLD] != 0 {
        b.move_spd = 0;
        return;
    }
    if c == 0 {
        b.move_dirc = b.center_dirc;
        s.act_dirc[2] = b.center_dirc;
    }
    b.move_dirc =
        if b.act_proccess != 0 { ee::add(0x3f06_0a92, s.act_dirc[2]) } else { ee::sub(s.act_dirc[2], 0x3f06_0a92) };
    if c & 31 == 0 {
        b.act_proccess ^= 1;
    }
    b.move_dirc = wrap(b.move_dirc);
    b.move_spd = 0x420c_0000;
    if !valid_area(b, cx, cx.scene.chars[cx.me].pos_p) {
        s.change_next_pattern(b, cx);
        b.entry_cmnd_target(cx);
        cx.cursor(false);
    }
}

/// `OnThinkReturn` (0x0047dac0).
fn on_return(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            b.erase_cmnd_target(cx);
            cx.cursor(true);
            b.spd_up(0x42c8_0000, 0x42c8_0000);
            b.move_dirc = b.center_dirc;
            b.act_proccess += 1;
        }
        1 if ee::lt(b.center_dist, 0x43c8_0000) => {
            b.move_spd = 0;
            b.move_vector = [0; 4];
            b.entry_cmnd_target(cx);
            cx.cursor(false);
            s.change_next_pattern(b, cx);
        }
        _ => {}
    }
}

/// `OnThinkWander` (0x0047dbd0).
fn on_wander(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    let me = cx.me;
    if b.stop != 0 || cx.scene.chars[me].cond[cond::HOLD] != 0 {
        b.move_spd = 0;
        return;
    }
    b.move_spd = 0x4234_0000;
    let c = b.act_count;
    b.act_count += 1;
    if c >= 61 || !valid_area(b, cx, cx.scene.chars[me].pos_p) {
        s.change_next_pattern(b, cx);
        return;
    }
    if b.act_proccess != 0 {
        return;
    }
    b.move_dirc = ee::add(PI, b.target_dirc);
    let t = cx.scene.chars[me].target_char;
    if !cx.valid(t) || t.is_some_and(|t| cx.dead(t)) {
        let t = b.select_target(cx, 0, FAR);
        cx.scene.chars[me].target_char = t;
        if !cx.valid(t) || t.is_some_and(|t| cx.dead(t)) {
            b.move_dirc = b.center_dirc;
        } else {
            b.calc_target_info(cx);
            b.move_dirc = ee::add(PI, b.center_dirc);
        }
    }
    if ee::lt(b.move_dirc, PI) {
        b.move_dirc = ee::add(b.move_dirc, TWO_PI);
    } else if !ee::le(b.move_dirc, PI) {
        b.move_dirc = ee::sub(b.move_dirc, TWO_PI);
    }
    b.act_proccess += 1;
}

/// `OnCrossAtk` (0x0047e030): the target held while the cross swings,
/// struck at frame 40.
fn on_cross(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    let f = b.anm.frame();
    if f == 15 || f == 35 {
        cx.se3d(169);
    }
    let me = cx.me;
    let t = cx.scene.chars[me].target_char;
    if let Some(tp) = t
        && cx.valid(t)
        && !cx.dead(tp)
    {
        cx.affect(tp, 5, 32767);
        if b.anm.frame() == 40 {
            b.skill_damage(cx, t, None, SKILL_CROSS);
            cx.affect(tp, 6, 0);
            cx.se3d(171);
        }
    }
    if b.anm_status != 0 {
        s.change_next_pattern(b, cx);
    } else {
        b.spd_down(0, 0x40a0_0000);
        b.set_dirc(b.target_dirc);
    }
}

/// `OnDataDrainAtk` (0x0047e410): a member drained through menu 74.
fn on_drain(b: &mut BossBase, s: &mut Skeith, cx: &mut Cx) {
    let c = b.act_count;
    b.act_count += 1;
    let me = cx.me;
    if b.act_proccess == 0 {
        let mut t = b.select_target(cx, 0, FAR);
        cx.scene.chars[me].target_char = t;
        if !cx.valid(t) {
            t = b.select_target(cx, 0, FAR);
            cx.scene.chars[me].target_char = t;
            if t.is_none() {
                s.change_next_pattern(b, cx);
                return;
            }
        }
        b.calc_target_info(cx);
        b.lock_player(cx, false);
        cx.out(Out::Cinema(Some(-1)));
        b.act_count = 0;
        b.act_proccess += 1;
    }
    match b.act_proccess {
        1 if c == 45 => {
            let id = cx.scene.chars[me].target_char.map_or(-1, |t| i32::from(cx.scene.chars[t].id()));
            let slot = cx.party.slot_of(id);
            cx.out(Out::StreamMenu { stream: 20, mask: 1 << slot });
            b.act_count = 0;
            b.act_proccess += 1;
        }
        2 if cx.env.menu_type == -1 => {
            let c2 = b.act_count;
            b.act_count += 1;
            if c2 >= 2 {
                let t = cx.scene.chars[me].target_char;
                if let Some(tp) = t
                    && cx.valid(t)
                {
                    cx.affect(tp, 13, 0);
                }
                b.unlock_player();
                cx.out(Out::Cinema(None));
                s.change_next_pattern(b, cx);
            }
        }
        _ => {}
    }
    b.spd_down(0, 0x40a0_0000);
}

fn neutral_act(b: &mut BossBase, cx: &mut Cx, epitaph_forbid: i16, forbid: i16) {
    if b.epitaph != 0 {
        b.change_action(cx, act::EPITAPH_NEUTRAL, epitaph_forbid, true);
    } else {
        b.change_action(cx, act::NEUTRAL, forbid, true);
    }
}

/// `IsValidArea(p)` (0x0047ca00): within 1500 of the centre.
fn valid_area(b: &BossBase, cx: &Cx, p: V4) -> bool {
    ee::lt(dist(cx, p, b.center_pos_p), 0x44bb_8000)
}

/// Skeith's odd wrap: above pi a turn off, below pi a turn on.
fn wrap(v: F) -> F {
    if !ee::le(v, PI) {
        ee::sub(v, TWO_PI)
    } else if ee::lt(v, PI) {
        ee::add(v, TWO_PI)
    } else {
        v
    }
}
