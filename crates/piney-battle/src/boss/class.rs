//! What a boss class not yet made of systems adds to `ccBoss`:
//! [`BossClass`], the frame and the affect its vtable runs and what it
//! shows (one dispatch, [`Class::with_rules`], which a def's
//! [`super::systems::Step::Class`] runs).

use super::output::Output;
use super::{BossBase, Class, Cx};

/// A class's `Main` and `Affect` on its `ccBoss` members (`b`) and its own
/// (`self`). A part's frame runs from its master's, so a part leaves
/// `main` as `ccBoss`'s; a class with no `Affect` of its own runs
/// `ccBoss::Affect`, as its vtable does (Kyvia's bodies, Corbenik's eyes
/// and roots, a bare slave).
pub(crate) trait BossClass {
    fn main(&mut self, b: &mut BossBase, cx: &mut Cx) {
        b.base_main(cx, &mut |_, _| {});
    }

    fn affect(&mut self, b: &mut BossBase, cx: &mut Cx) {
        b.base_affect(cx);
    }

    /// `ccBossEffManager::Draw`'s pass where the class runs it before the
    /// blur and the boss's task (Kyvia's).
    fn effects_pass(&mut self, _b: &mut BossBase, _cx: &mut Cx) {}

    /// The boss that takes its place once its frame ends (Corbenik's
    /// second form, made as the first exits).
    fn next(&mut self, _b: &mut BossBase, _cx: &mut Cx) -> Option<super::Boss> {
        None
    }

    /// Its frame's draw requests ([`Output`]) over `ccBoss::Draw`'s body,
    /// read after the frame.
    fn show(&self, _b: &BossBase, _cx: &mut Cx, _out: &mut Output) {}
}

/// A bare `ccBoss` (a part between its master's takes).
struct Bare;

impl BossClass for Bare {}

/// One arm a class: `$x` bound to its boxed members, `$plain` for a bare
/// `ccBoss`.
macro_rules! each_class {
    ($class:expr, $x:ident => $each:expr, $plain:expr) => {
        match $class {
            Class::Plain => $plain,
            Class::Innis($x) => $each,
            Class::Kyvia($x) => $each,
            Class::KyviaCore($x) => $each,
            Class::Gomora($x) => $each,
            Class::Corbenik($x) => $each,
            Class::CorbenikSeed($x) => $each,
            Class::CorbenikB($x) => $each,
            Class::CorbenikRoot($x) => $each,
            Class::CorbenikC($x) => $each,
            Class::CorbenikEye($x) => $each,
            Class::CorbenikKiller($x) => $each,
        }
    };
}

impl Class {
    /// The class's rules.
    pub(crate) fn with_rules<R>(&mut self, f: impl FnOnce(&mut dyn BossClass) -> R) -> R {
        each_class!(self, x => f(&mut **x), f(&mut Bare))
    }
}
