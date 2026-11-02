//! A boss's systems and the one runner that steps and draws them. A def
//! ([`super::BossDef`]) names its systems ([`Kind`]) and its frame's
//! [`Step`]s in the game's order: `ccBoss::Main`'s blocks on the `ccBoss`
//! members every system shares ([`super::BossBase`], [`frame`]) and its
//! systems' own steps ([`Hook`]). A system holds its class's members and
//! writes what it shows into the boss's [`super::Output`]; its siblings
//! reach it only by kind ([`Others`]), inside the boss's own module.

pub mod frame;
pub mod patterns;
pub mod wave;

use super::{Boss, BossBase, Cx, Output};

/// One step of a boss's frame, in its def's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The effect manager's count on the slots ([`super::Effects::tick`]).
    TickEffects,
    /// `ccBoss::Main`'s blocks ([`frame`]).
    Prologue,
    TickStop,
    LockParty,
    Move,
    ForwardAndDraw,
    /// `ccBoss::Draw` alone while `drawSW` holds, after a class's own
    /// `PreDrawAnm`.
    Draw,
    /// The hold condition let go (`cond[HOLD] = 0`).
    ClearHold,
    /// `ccBoss::Affect`.
    BaseAffect,
    /// These while `exit` is clear: `ccBoss::Main`'s return.
    Live(&'static [Step]),
    /// A system's step.
    Run(Kind, Hook),
}

/// Which of a system's steps a [`Step::Run`] runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hook {
    /// `ccBossEffManager::Draw`'s pass before the blur and the task.
    Pass,
    Think,
    Action,
    /// What its `Main` does before `ccBoss::Main`'s (what it reads of the
    /// body as last drawn, its own counts).
    Early,
    /// Its `Move` in place of `ccBoss`'s.
    Move,
    /// Its lock of the party in place of `ccBoss`'s (what it does as the
    /// party is let go).
    Lock,
    /// Its `PreDrawAnm` before the body's.
    PreDraw,
    /// Its `Draw` in place of the body's (what it reads of the body drawn).
    Draw,
    /// What its `Main` does after `ccBoss::Main`'s.
    Late,
    /// Its `Affect`.
    Affect,
    /// The boss that takes its place once its frame ends.
    HandOver,
    /// Its parts out of their characters for the frame, and back.
    Lend,
    Return,
    /// Its parts' effect slots counted (the manager's one array).
    Tick,
}

/// What a system does: its steps on the shared members and its siblings,
/// what it shows (beside the boss's other systems, `all`), the boss it
/// hands over to.
pub(crate) trait BossSystem {
    fn step(&mut self, _hook: Hook, _b: &mut BossBase, _others: &mut Others, _cx: &mut Cx) {}

    fn draw(&self, _b: &BossBase, _all: &Systems, _cx: &mut Cx, _out: &mut Output) {}

    fn hand_over(&mut self, _b: &mut BossBase, _cx: &mut Cx) -> Option<Boss> {
        None
    }
}

/// A system's own type, found in a list by its kind.
pub trait SystemKind: Sized {
    const KIND: Kind;
    fn of(s: &System) -> Option<&Self>;
    fn of_mut(s: &mut System) -> Option<&mut Self>;
    fn wrap(x: Box<Self>) -> System;
    /// The object, or the system back when it is another kind.
    fn unwrap(s: System) -> Result<Box<Self>, System>;
}

/// The systems: the enum of their objects, their kinds, and the one
/// `match` that steps and draws each.
macro_rules! systems {
    ($($(#[$m:meta])* $v:ident($t:ty)),* $(,)?) => {
        /// One of a boss's systems.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum System {
            $($(#[$m])* $v(Box<$t>),)*
        }

        /// A system's kind, as a def names it.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Kind {
            $($v,)*
        }

        impl System {
            pub fn kind(&self) -> Kind {
                match self {
                    $(System::$v(_) => Kind::$v,)*
                }
            }

            /// A kind's system as its class's constructor starts it.
            fn new(k: Kind) -> System {
                match k {
                    $(Kind::$v => System::$v(Box::default()),)*
                }
            }

            fn step(&mut self, hook: Hook, b: &mut BossBase, others: &mut Others, cx: &mut Cx) {
                match self {
                    $(System::$v(s) => s.step(hook, b, others, cx),)*
                }
            }

            fn draw(&self, b: &BossBase, all: &Systems, cx: &mut Cx, out: &mut Output) {
                match self {
                    $(System::$v(s) => s.draw(b, all, cx, out),)*
                }
            }

            fn hand_over(&mut self, b: &mut BossBase, cx: &mut Cx) -> Option<Boss> {
                match self {
                    $(System::$v(s) => s.hand_over(b, cx),)*
                }
            }
        }

        $(impl SystemKind for $t {
            const KIND: Kind = Kind::$v;

            fn of(s: &System) -> Option<&Self> {
                match s {
                    System::$v(x) => Some(x),
                    #[allow(unreachable_patterns)]
                    _ => None,
                }
            }

            fn of_mut(s: &mut System) -> Option<&mut Self> {
                match s {
                    System::$v(x) => Some(x),
                    #[allow(unreachable_patterns)]
                    _ => None,
                }
            }

            fn wrap(x: Box<Self>) -> System {
                System::$v(x)
            }

            fn unwrap(s: System) -> Result<Box<Self>, System> {
                match s {
                    System::$v(x) => Ok(x),
                    #[allow(unreachable_patterns)]
                    s => Err(s),
                }
            }
        })*
    };
}

systems! {
    /// A wave's ring clip (`m_anmw`).
    Wave(wave::Wave),
    /// Skeith's own (`ccBoss01`).
    Skeith(super::skeith::Skeith),
    /// Fidchell's own (`ccBoss04`).
    Fidchell(super::fidchell::Fidchell),
    /// Gorre's own (`ccBoss05`), its brothers and its Kerse.
    Gorre(super::gorre::Gorre),
    Brothers(super::gorre::Brothers),
    Kerse(super::gorre::Kerse),
    /// A Gorre brother's own (`ccBoss05Brother`).
    Brother(super::gorre::brother::Brother),
    /// Magus's own (`ccBoss03`), its leaves, laser and needles.
    Magus(super::magus::Magus),
    Leaves(super::magus::Leaves),
    Laser(super::magus::Laser),
    Needles(super::magus::Needles),
    /// A Magus leaf's own (`ccBoss03Leaf`).
    Leaf(super::magus::leaf::Leaf),
    /// Macha's own (`ccBoss06`).
    Macha(super::macha::Macha),
    /// Tarvos's own (`ccBoss07`).
    Tarvos(super::tarvos::Tarvos),
    /// Kyvia's own (`ccBossKyvia01`-`04`), its core and the core's
    /// gomoras lent, its disc and the fourth fight's places.
    Kyvia(super::kyvia::Kyvia),
    SlaveCore(super::kyvia::SlaveCore),
    Gomoras(super::kyvia::Gomoras),
    Disc(super::kyvia::Disc),
    Places(super::kyvia::fourth::Places),
    /// Innis's own (`ccBoss02`), its images and its missiles' pass.
    Innis(super::innis::Innis),
    Images(super::innis::Images),
    Missiles(super::innis::Missiles),
    /// Corbenik's forms' own (`ccBoss08`, `ccBoss08_b`, `ccBoss08_c`).
    CorbenikFirst(super::corbenik::first::First),
    CorbenikSecond(super::corbenik::second::Second),
    CorbenikThird(super::corbenik::third::Third),
    /// Corbenik's parts' own: a seed (`ccBoss08Seed`), a root
    /// (`ccBoss08Root`), an eye (`ccBoss08Eye`), a killer eye
    /// (`ccBoss08KillerEye`).
    Seed(super::corbenik::seed::Seed),
    Root(super::corbenik::second::Root),
    Eye(super::corbenik::eye::Eye),
    Killer(super::corbenik::eye::Killer),
    /// Kyvia's core's own (`kyviaCore`) and a gomora's (`kyviaGomora`).
    KyviaCore(super::kyvia::core::Core),
    Gomora(super::kyvia::gomora::Gomora),
}

/// A boss's systems, in its def's order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Systems(Vec<System>);

impl Systems {
    /// Each of the kinds as its constructor starts it.
    pub fn of(kinds: &[Kind]) -> Systems {
        Systems(kinds.iter().map(|&k| System::new(k)).collect())
    }

    /// The system of `T`'s kind.
    pub fn get<T: SystemKind>(&self) -> Option<&T> {
        self.0.iter().find_map(T::of)
    }

    pub fn get_mut<T: SystemKind>(&mut self) -> Option<&mut T> {
        self.0.iter_mut().find_map(T::of_mut)
    }

    /// `T`'s object out of the list (a part's own, lent with it).
    pub(crate) fn take<T: SystemKind>(&mut self) -> Option<Box<T>> {
        let k = self.0.iter().position(|s| s.kind() == T::KIND)?;
        T::unwrap(self.0.remove(k)).ok()
    }

    /// `T`'s object back at its place in `def`'s list.
    pub(crate) fn put<T: SystemKind>(&mut self, def: &super::BossDef, x: Box<T>) {
        let k = def.systems.iter().position(|&k| k == T::KIND).unwrap_or(self.0.len());
        self.0.insert(k.min(self.0.len()), T::wrap(x));
    }

    /// The system of `T`'s kind and the others beside it.
    pub(crate) fn split<T: SystemKind>(&mut self) -> Option<(&mut T, Others<'_>)> {
        let k = self.0.iter().position(|s| s.kind() == T::KIND)?;
        let (s, others) = self.at(k);
        Some((T::of_mut(s)?, others))
    }

    fn at(&mut self, k: usize) -> (&mut System, Others<'_>) {
        let mut all: Vec<&mut System> = self.0.iter_mut().collect();
        let me = all.remove(k);
        (me, Others(all))
    }

    pub fn iter(&self) -> impl Iterator<Item = &System> {
        self.0.iter()
    }
}

/// The systems beside the one stepping.
pub struct Others<'a>(Vec<&'a mut System>);

impl<'a> Others<'a> {
    /// The sibling of `T`'s kind.
    pub fn get_mut<T: SystemKind>(&mut self) -> Option<&mut T> {
        self.0.iter_mut().find_map(|s| T::of_mut(s))
    }

    /// The sibling of `T`'s kind for the whole step, apart from the rest
    /// (so a step can hold several at once).
    pub fn take<T: SystemKind>(&mut self) -> Option<&'a mut T> {
        let k = self.0.iter().position(|s| s.kind() == T::KIND)?;
        T::of_mut(self.0.swap_remove(k))
    }
}

impl Boss {
    /// The steps in order: the base's blocks on the `ccBoss` members and
    /// the systems' own steps.
    fn run(&mut self, steps: &'static [Step], cx: &mut Cx) {
        for &s in steps {
            let b = &mut self.base;
            match s {
                Step::TickEffects => b.effects.tick(),
                Step::Prologue => b.prologue(cx),
                Step::TickStop => b.tick_stop(),
                Step::LockParty => {
                    b.lock_party(cx);
                }
                Step::Move => b.move_(cx),
                Step::ForwardAndDraw => b.forward_and_draw(cx),
                Step::Draw => {
                    if b.draw_sw != 0 {
                        b.draw(cx);
                    }
                }
                Step::ClearHold => cx.scene.chars[cx.me].cond[crate::param::cond::HOLD] = 0,
                Step::BaseAffect => b.base_affect(cx),
                Step::Live(inner) => {
                    if b.exit == 0 {
                        self.run(inner, cx);
                    }
                }
                Step::Run(kind, hook) => {
                    let k = self.systems.0.iter().position(|s| s.kind() == kind).expect("a system the def names");
                    let (s, mut others) = self.systems.at(k);
                    s.step(hook, &mut self.base, &mut others, cx);
                }
            }
        }
    }

    /// The boss task's frame ([`super::BossDef::order`]).
    pub fn main(&mut self, cx: &mut Cx) {
        self.run(self.def.order, cx);
    }

    /// What an `EntryAffect` on the boss does at once (`bossAffectFunc`,
    /// 0x0045f090): its class's `Affect` ([`super::BossDef::affect`]).
    pub fn affect(&mut self, cx: &mut Cx) {
        self.run(self.def.affect, cx);
    }

    /// `ccBossEffManager::Draw`'s pass over the effects whose `Draw` the
    /// boss keeps, before the blur's `Main` and the boss's task
    /// ([`super::BossDef::pass`], Kyvia's).
    pub fn effects_pass(&mut self, cx: &mut Cx) {
        self.run(self.def.pass, cx);
    }

    /// The boss task's own step after its `Main`: the boss that takes its
    /// place when a system hands over (`ccThBoss08` past the first form).
    pub fn hand_over(&mut self, cx: &mut Cx) {
        for &s in self.def.hand_over {
            let next = match s {
                Step::Run(kind, _) => {
                    let k = self.systems.0.iter().position(|s| s.kind() == kind).expect("a system the def names");
                    self.systems.0[k].hand_over(&mut self.base, cx)
                }
                _ => None,
            };
            if let Some(next) = next {
                *self = next;
                return;
            }
        }
    }

    /// What the frame shows ([`Output`]), read after it: `ccBoss::Draw`'s
    /// body, then each system's draw in order; then each part's own.
    pub fn show(&mut self, cx: &mut Cx) {
        let Boss { base, systems, output, .. } = self;
        output.clear();
        output.body = base.body_draw(cx);
        for s in &systems.0 {
            s.draw(base, systems, cx, output);
        }
        let me = cx.me;
        for k in 0..self.output.parts.len() {
            let p = self.output.parts[k];
            let Some(mut b) = cx.scene.chars.get_mut(p).and_then(|c| c.foe_state_mut()).and_then(|f| f.boss.take())
            else {
                continue;
            };
            cx.me = p;
            b.show(cx);
            cx.me = me;
            if let Some(f) = cx.scene.chars[p].foe_state_mut() {
                f.boss = Some(b);
            }
        }
    }
}
