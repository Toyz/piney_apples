//! What the rules ask of the rest of the game, in the order the game's code
//! makes the calls: `ccChar::EntryAffect` (the affect a character takes:
//! damage, healing, a condition), the particles and effects, the protect
//! gauge's display. The rules never draw; the runtime turns these into
//! effects and applies each [`Event::Affect`] through
//! [`crate::affect`].

/// Who an event is about: the acting character (`this`, the attacker, the
/// caster), the single target, or a character by its index in the slice
/// an area rule was given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Who {
    Me,
    Target,
    Char(usize),
    /// A null character pointer (a skill whose caster is gone).
    Nobody,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// `on->EntryAffect(by, kind, p0, p1, p2)` (gcmn 0x0056b020): `kind` 1
    /// damage (p0 the amount, p1 the skill), 3 poison, 4 SP loss, 7
    /// healing, 9 HP gain, 10 SP gain, 16 a cure, 17 a buff, 18 a
    /// debuff, 20 revival ...; see [`crate::affect`].
    Affect {
        on: Who,
        by: Who,
        kind: i16,
        p: [i16; 3],
    },
    /// `ccParticleAttributeGuard(target, attacker)`: an Exdefense immunity.
    AttributeGuard {
        on: Who,
        by: Who,
    },
    /// `ccParticleCritical(target)`.
    Critical(Who),
    /// `ccParticleDying(target)`.
    Dying(Who),
    /// `ccParticleNoDamage(target)`.
    NoDamage(Who),
    /// `effDrainCtrl(from, to, kind, a, b)`: 0 HP, 1 SP drawn to `from`.
    DrainCtrl {
        from: Who,
        to: Who,
        kind: i32,
        a: i32,
        b: i32,
    },
    /// `effProtect(ch, broken, kind)`: the gauge broken (0) or restored (1).
    Protect {
        on: Who,
        broken: i32,
        kind: i32,
    },
    /// `ccMenuCtrl::SetProtect(state, ch)`.
    SetProtect {
        state: i32,
        on: Who,
    },
    /// `ccEntryRecoveryReq(ch, amount)`: HP absorbed from an element.
    RecoveryReq {
        on: Who,
        amount: i32,
    },
    /// `ccChar::DispConditionEffect()`.
    DispCondition(Who),
    /// `ccParticleAttributeCritical(target)`.
    AttributeCriticalParticle(Who),
    /// `ccAI::ChatMessageAttributeCritical()` of the attacker's AI.
    ChatAttributeCritical(Who),
    /// `ccAI::ChatMessageAttack(target, dmg, sid)` of the attacker's AI.
    ChatAttack {
        ai: Who,
        on: Who,
        dmg: i32,
        sid: i32,
    },
    /// `effHealSkill(ch, sid)`.
    HealSkill {
        on: Who,
        sid: i32,
    },
    /// `effCure(ch)`, `effSanity(ch)`, `effResurrect(ch)`.
    Cure(Who),
    Sanity(Who),
    Resurrect(Who),
    /// `ccEntryFlyFontNew(kind, value, pos, ch, 1.0, 1.0)`: the number
    /// over a character: 2 damage (-1 a miss), 3 SP lost, 5 SP gained, 20
    /// HP gained.
    FlyFont {
        on: Who,
        kind: i32,
        value: i32,
    },
    /// `ccHitMarkDisp(ch, attacker)`: the hit spark, turned from the
    /// attacker.
    HitMark {
        on: Who,
        by: Who,
    },
    /// `ccPlayer::DamageActuate(dmg)`: Kite's pad rumble. `Influence`
    /// calls it before it sets the hurt or down act, so `act` is Kite's
    /// `actNum` as the affect found it (the rumble is off in act 14).
    DamageActuate {
        value: i32,
        act: i16,
    },
    /// `ccMenuCtrl::SetPanelBure(slot, n)`: the party panel shakes.
    PanelBure {
        slot: i32,
        n: i32,
    },
    /// `ccSkillRequest(ch, 0, 0)`: the character's running normal attack
    /// is cancelled.
    CancelAttack(Who),
    /// `ccCharHit::HitEnable()` / `HitDisable()` on the body.
    HitEnable(Who),
    HitDisable(Who),
    /// `ccChar::ClearConditionEffect()`: the condition's effect removed
    /// (`conditionNum` is set to -1 with it).
    ClearConditionEffect(Who),
    /// `ccAI::ChatMessageDamage(dmg)`, `ChatMessageResurrectPlz()`.
    /// `hp` is the character's HP as the affect found it: `Influence`
    /// stores the new HP after the line.
    ChatDamage {
        on: Who,
        value: i32,
        hp: i16,
    },
    ChatResurrectPlz(Who),
    /// From Mutation on, the hit (`value`) a party member or Kite took,
    /// for its AI's record ([`crate::party_ai::Ai::note_hit`], gcmn
    /// 0x005c1a60).
    NoteHit {
        on: Who,
        value: i32,
        /// The character has an AI (the game writes through a null pointer
        /// otherwise).
        ai: bool,
    },
    /// `ccAI::AffectMessages(kind, from, n)` and `Greeting(from, n)`.
    AffectMessages {
        on: Who,
        kind: i32,
        by: Who,
        n: i32,
        hp: i16,
    },
    Greeting {
        on: Who,
        by: Who,
        n: i32,
    },
    /// `ccAISysMsgSendP(0x1000c, -1, id, 0xffff, 0, 30, -1, ch)`: "down"
    /// sent to the other members' AI; `id` is the character's
    /// `CheckSysMsgID()`.
    SysMsgDown {
        on: Who,
        id: i16,
    },
    /// `ccAISysMsgDeleteDelay(0x1000d, id, id)`: revival withdraws it.
    SysMsgUp {
        on: Who,
        id: i16,
    },
    /// `effAfterDrain(ch, 0)`.
    AfterDrain(Who),
    /// `ccEnemy::selectTarget()`: an enemy hit picks its target again.
    EnemyRetarget(Who),
    /// `effResistantShield(ch, magic, -1)`: an Exdefense immunity shown on
    /// a hit enemy.
    ResistantShield {
        on: Who,
        magic: i32,
    },
    /// The AI's `talkFlag` cleared (affect kind 0 on a party member).
    TalkOff(Who),
}

impl Event {
    pub fn affect(on: Who, by: Who, kind: i16, p0: i16, p1: i16, p2: i16) -> Event {
        Event::Affect { on, by, kind, p: [p0, p1, p2] }
    }

    /// Every character the event names, rewritten by `f`.
    pub fn map_who(&mut self, f: impl Fn(Who) -> Who) {
        let g = |w: &mut Who| *w = f(*w);
        match self {
            Event::Affect { on, by, .. }
            | Event::AttributeGuard { on, by }
            | Event::HitMark { on, by }
            | Event::AffectMessages { on, by, .. }
            | Event::Greeting { on, by, .. } => {
                g(on);
                g(by);
            }
            Event::DrainCtrl { from, to, .. } => {
                g(from);
                g(to);
            }
            Event::ChatAttack { ai, on, .. } => {
                g(ai);
                g(on);
            }
            Event::Critical(w)
            | Event::Dying(w)
            | Event::NoDamage(w)
            | Event::DispCondition(w)
            | Event::AttributeCriticalParticle(w)
            | Event::ChatAttributeCritical(w)
            | Event::Cure(w)
            | Event::Sanity(w)
            | Event::Resurrect(w)
            | Event::CancelAttack(w)
            | Event::HitEnable(w)
            | Event::HitDisable(w)
            | Event::ClearConditionEffect(w)
            | Event::ChatResurrectPlz(w)
            | Event::AfterDrain(w)
            | Event::EnemyRetarget(w)
            | Event::TalkOff(w) => g(w),
            Event::Protect { on, .. }
            | Event::SetProtect { on, .. }
            | Event::RecoveryReq { on, .. }
            | Event::HealSkill { on, .. }
            | Event::FlyFont { on, .. }
            | Event::ChatDamage { on, .. }
            | Event::NoteHit { on, .. }
            | Event::SysMsgDown { on, .. }
            | Event::SysMsgUp { on, .. }
            | Event::ResistantShield { on, .. } => g(on),
            Event::DamageActuate { .. } | Event::PanelBure { .. } => {}
        }
    }
}

/// The events of one call, in order.
pub type Events = Vec<Event>;
