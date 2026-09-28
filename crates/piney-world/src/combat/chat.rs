//! The party's chat lines in the field's frame ([`piney_battle::party_chat`]).
//! The lines the decisions call for and `ChatMessageSender`'s balloon run in
//! the stage's runtime ([`super::stage::Stage`]); the lines a hit or an
//! affect raises come out of the rules as events ([`line_of`]), which the
//! frame runs after the task that raised them.

use piney_battle::enemy_ai::Enemy;
use piney_battle::event::{Event, Who};
use piney_battle::party_ai::{Chat, Ctx};
use piney_battle::tables::Tables;

/// What an event asks of a member's AI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    /// A chat line on the AI of the character.
    Say(usize, Chat),
    /// `ccAI::Greeting(from, n)` on the character's AI.
    Greeting(usize, Option<usize>),
}

fn idx(w: Who) -> Option<usize> {
    match w {
        Who::Char(i) => Some(i),
        _ => None,
    }
}

/// The chat line (or greeting) a rule's event stands for.
pub fn line_of(e: &Event) -> Option<Line> {
    Some(match *e {
        Event::ChatAttributeCritical(a) => Line::Say(idx(a)?, Chat::AttributeCritical),
        Event::ChatAttack { ai, on, dmg, sid } => Line::Say(idx(ai)?, Chat::Attack(idx(on), dmg, sid)),
        Event::ChatDamage { on, value, hp } => Line::Say(idx(on)?, Chat::Damage(value, hp)),
        Event::ChatResurrectPlz(on) => Line::Say(idx(on)?, Chat::ResurrectPlz),
        Event::AffectMessages { on, kind, by, n, hp } => Line::Say(idx(on)?, Chat::Affect(kind, idx(by), n, hp)),
        Event::Greeting { on, by, .. } => Line::Greeting(idx(on)?, idx(by)),
        _ => return None,
    })
}

/// The event's line on the member's AI, when it has one (the game calls
/// through `ccSpcChar.ai`, which a member always has).
pub fn run(ctx: &mut Ctx<'_>, e: &Event) {
    if let Event::NoteHit { on: Who::Char(i), value, .. } = *e {
        let max_hp = ctx.scene.chars[i].max_hp;
        if let Some(a) = ctx.crew.ais.get_mut(&i) {
            a.note_hit(value, max_hp);
        }
        return;
    }
    let Some(line) = line_of(e) else { return };
    match line {
        Line::Say(who, chat) if ctx.crew.ais.contains_key(&who) => {
            ctx.chat_line(who, chat);
        }
        Line::Greeting(who, from) if ctx.crew.ais.contains_key(&who) => ctx.greeting(who, from),
        _ => {}
    }
}

/// For each character, how many of its enemy skills change conditions
/// (`skillList[0..skillNum]`, `skiParam->type & 0x20000`), as
/// `ChatMessageAttackTarget` counts them; 0 for anything but an enemy.
pub fn condition_skills(t: &Tables, foes: &[Option<Enemy>], n: usize) -> Vec<i32> {
    (0..n)
        .map(|c| {
            let Some(Some(e)) = foes.get(c) else { return 0 };
            let k = usize::try_from(e.skill_num).unwrap_or(0).min(e.skill_list.len());
            e.skill_list[..k]
                .iter()
                .filter(|s| s.ski_param.and_then(|p| p.get(t, e.ene_id)).is_some_and(|p| p.ty & 0x20000 != 0))
                .count() as i32
        })
        .collect()
}

/// The runtime of the queued lines: they make no calls.
pub struct Quiet;

impl piney_battle::party_ai::Runtime for Quiet {
    fn call(
        &mut self,
        _call: piney_battle::party_ai::Call,
        _scene: &mut piney_battle::scene::Scene,
        _crew: &mut piney_battle::party_ai::Crew,
        _rng: &mut dyn piney_battle::rand::Rng,
    ) -> i32 {
        0
    }
    fn w2p(&mut self, pos: [u32; 4]) -> [u32; 4] {
        pos
    }
}
