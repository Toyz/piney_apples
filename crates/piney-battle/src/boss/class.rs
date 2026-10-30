//! What a boss class adds to `ccBoss`: [`BossClass`], the frame and the
//! affect its vtable runs (one dispatch, [`Class::with_rules`]), and
//! [`Patterns`], the calls `ccBoss`'s pattern code makes back into the
//! class, with what differs between the classes' copies of it
//! ([`BaseActs`]).

use piney_data::field::ee;

use super::{BossBase, BossData, Class, Cx, NEG_HALF_PI, Out, act};
use crate::geom;

type F = u32;

/// A class's `Main` and `Affect` on its `ccBoss` members (`b`) and its own
/// (`self`). A part's frame runs from its master's, so a part leaves
/// `main` as it is; a class with no `Affect` of its own keeps `ccBoss01`'s.
pub(crate) trait BossClass {
    fn main(&mut self, b: &mut BossBase, cx: &mut Cx) {
        b.skeith_main(cx);
    }

    fn affect(&mut self, b: &mut BossBase, cx: &mut Cx) {
        b.skeith_affect(cx);
    }

    /// `ccBossEffManager::Draw`'s pass where the class runs it before the
    /// blur and the boss's task (Kyvia's).
    fn effects_pass(&mut self, _b: &mut BossBase, _cx: &mut Cx) {}
}

/// Skeith (`ccBoss01`) and a bare `ccBoss`: the base's rules.
pub(crate) struct Skeith;

impl BossClass for Skeith {}

impl Class {
    /// The class's rules, one arm a class.
    pub(crate) fn with_rules<R>(&mut self, f: impl FnOnce(&mut dyn BossClass) -> R) -> R {
        match self {
            Class::Skeith | Class::Plain => f(&mut Skeith),
            Class::Innis(x) => f(&mut **x),
            Class::Magus(x) => f(&mut **x),
            Class::MagusLeaf(x) => f(&mut **x),
            Class::Kyvia(x) => f(&mut **x),
            Class::KyviaCore(x) => f(&mut **x),
            Class::Gomora(x) => f(&mut **x),
            Class::Fidchell(x) => f(&mut **x),
            Class::Gorre(x) => f(&mut **x),
            Class::GorreBrother(x) => f(&mut **x),
            Class::Macha(x) => f(&mut **x),
            Class::Tarvos(x) => f(&mut **x),
            Class::Corbenik(x) => f(&mut **x),
            Class::CorbenikSeed(x) => f(&mut **x),
            Class::CorbenikB(x) => f(&mut **x),
            Class::CorbenikRoot(x) => f(&mut **x),
            Class::CorbenikC(x) => f(&mut **x),
            Class::CorbenikEye(x) => f(&mut **x),
            Class::CorbenikKiller(x) => f(&mut **x),
        }
    }
}

/// Where the classes' ports of `ccBoss::ExecPatternIndex` part: the
/// neutral's forbid on a word the base does not know (1 lets it go, 3
/// holds it), and the w lanes of the dash's offset and of the still
/// `moveVector` a pattern starts from (0 or 1, as each class's code builds
/// them). The acts it sets are `ccBoss`'s ([`act`]) in every class.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BaseActs {
    pub other_forbid: i16,
    pub dash_w: F,
    pub still_w: F,
}

/// The calls `ccBoss`'s pattern code makes through a class's vtable: its
/// `ChangeAction`, `ExecPatternIndex` (the base's unless it has its own),
/// `ExecPattern` on a one-word table, `ChangeNextPattern` and
/// `CheckEpitaph`; and the table `patTbl` points at.
pub(crate) trait Patterns {
    const ACTS: BaseActs;

    /// `patTbl`'s words.
    fn words<'d>(&self, b: &BossBase, data: &'d BossData) -> &'d [i32];

    fn epitaph(&self, b: &BossBase) -> bool {
        b.epitaph != 0
    }

    fn change_action(&mut self, b: &mut BossBase, cx: &mut Cx, n: i16, forbid: i16, af: bool) {
        b.change_action(cx, n, forbid, af);
    }

    fn exec_pattern_index(&mut self, b: &mut BossBase, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
        base_exec_pattern_index(self, b, cx, tbl, i)
    }

    /// `ccBoss::ExecPattern(pat, p0, 0, 0)`.
    fn exec_pattern(&mut self, b: &mut BossBase, cx: &mut Cx, pat: i32, p0: i32) {
        self.exec_pattern_index(b, cx, &[pat, p0, 0, 0, -1], 0);
    }

    fn change_next_pattern(&mut self, b: &mut BossBase, cx: &mut Cx) {
        base_change_next_pattern(self, b, cx);
    }
}

/// `tbl[i]`, 0 past either end.
pub(crate) fn word(tbl: &[i32], i: i32) -> i32 {
    usize::try_from(i).ok().and_then(|k| tbl.get(k)).copied().unwrap_or(0)
}

/// `ccBoss::ChangeNextPattern` (INF gcmn 0x0045d210, QUA 0x00360da0): past
/// the table's end back to its start, still, the neutral (the epitaph's
/// held), then the next word run.
pub(crate) fn base_change_next_pattern<C: Patterns + ?Sized>(c: &mut C, b: &mut BossBase, cx: &mut Cx) {
    let tbl = c.words(b, cx.data);
    if word(tbl, b.pat_index) == -1 {
        b.pat_index = 0;
    }
    b.move_spd = 0;
    b.move_vector = [0, 0, 0, C::ACTS.still_w];
    if c.epitaph(b) {
        c.change_action(b, cx, act::EPITAPH_NEUTRAL, 3, true);
    } else {
        c.change_action(b, cx, act::NEUTRAL, 1, true);
    }
    b.pat_index = c.exec_pattern_index(b, cx, tbl, b.pat_index);
}

/// `ccBoss::ExecPatternIndex` (INF gcmn 0x0045cad0, QUA 0x00360630): 2 a
/// wait (-1 with the party up: 30), 3-7 the base's moves, 8 the neutral
/// held (forbid 3), 9 and 11 a skill on the target or on itself, 12 one on
/// every member; any other word the neutral with the class's forbid.
pub(crate) fn base_exec_pattern_index<C: Patterns + ?Sized>(
    c: &mut C,
    b: &mut BossBase,
    cx: &mut Cx,
    tbl: &[i32],
    i: i32,
) -> i32 {
    let a = C::ACTS;
    let orig = i;
    let mut i = i;
    let mut pat = word(tbl, i);
    i += 1;
    b.move_spd = 0;
    b.move_vector = [0, 0, 0, C::ACTS.still_w];
    if pat == -1 {
        pat = word(tbl, 0);
        i = 1;
    }
    b.pat_num = pat;
    let me = cx.me;
    let neutral = |c: &mut C, b: &mut BossBase, cx: &mut Cx, forbid: i16| {
        if c.epitaph(b) {
            c.change_action(b, cx, act::EPITAPH_NEUTRAL, 3, true);
        } else {
            c.change_action(b, cx, act::NEUTRAL, forbid, true);
        }
    };
    match pat {
        2 => {
            let mut t = word(tbl, i);
            i += 1;
            if t == -1 && !cx.annihilated() {
                t = 30;
            }
            b.stop_time = t;
            b.stop_counter = 0;
            neutral(c, b, cx, 3);
        }
        3 => c.change_action(b, cx, act::ESCAPE, 3, true),
        4 => c.change_action(b, cx, act::CHASE, 3, true),
        5 => c.change_action(b, cx, act::RETURN, 3, true),
        7 => c.change_action(b, cx, act::WANDER, 3, true),
        6 => {
            let d = ee::from_int(word(tbl, i));
            i += 1;
            let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), b.center_dirc), NEG_HALF_PI);
            let v = geom::apply_matrix(&m, [d, 0, 0, a.dash_w]);
            let p = geom::vadd(v, cx.scene.chars[me].pos_p);
            b.dash_pos = super::p2w(cx, p);
            b.dash_prev_pos = cx.scene.chars[me].pos;
            c.change_action(b, cx, act::DASH, 3, true);
        }
        8 => neutral(c, b, cx, 3),
        9 | 11 => {
            let ty = word(tbl, i);
            i += 1;
            let sid = word(tbl, i);
            i += 1;
            if pat == 9 {
                let t = b.select(cx, ty);
                if !cx.target_ok(t) {
                    c.exec_pattern(b, cx, 2, 30);
                    return orig;
                }
                b.calc_target_info(cx);
            }
            let on = if pat == 9 { cx.scene.chars[me].target_char } else { Some(me) };
            skill(cx, on, sid, 0);
            c.exec_pattern(b, cx, 2, 30);
        }
        12 => {
            let sid = word(tbl, i);
            i += 1;
            let members = cx.party.members;
            let last = members.into_iter().flatten().rfind(|&m| cx.target_ok(Some(m)));
            for m in members.into_iter().flatten() {
                if Some(m) != last && cx.target_ok(Some(m)) {
                    skill(cx, Some(m), sid, 0);
                }
            }
            if last.is_some_and(|l| cx.target_ok(Some(l))) {
                skill(cx, last, sid, 1);
            }
            c.exec_pattern(b, cx, 2, -2);
        }
        _ => neutral(c, b, cx, a.other_forbid),
    }
    i
}

/// `ItemSkillRequest(me, on, sid, last)` asked of the battle.
fn skill(cx: &mut Cx, on: Option<usize>, sid: i32, last: i32) {
    let me = cx.me;
    if let Some(tp) = on
        && let Some(k) = crate::item::item_skill_request(cx.t, cx.scene, me, tp, sid, last, false, cx.rand)
    {
        cx.out(Out::Skill(tp, k));
    }
}
