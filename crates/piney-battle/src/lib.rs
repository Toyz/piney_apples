//! .hack//Infection's combat, from `GCMN.PRG`, as logic: the rules
//! (`common.cpp`, `skill.cpp`, `useitem.cpp`, the AI's decisions in
//! `personal.cpp` and `enemy.cpp`) and the motion layer that carries them
//! on the field (the entry control and its magic circles, the enemies'
//! movement and animation, Kite's and the party members' frames and acts,
//! following and path finding). Nothing here draws: what the game's code
//! asks of the collision, the camera, the player's frame and the animation
//! players goes through the [`world::World`] trait (and a few sub-traits),
//! and what only shows or sounds comes back as outputs in the game's order.
//!
//! Every function is a transcription of the game's, checked against the
//! game's own code run in the EE interpreter (`tools/eemu.py`, or the Rust
//! machine `tools/eemu_rs.so`) by the `tools/test_battle_*_rs.py`
//! harnesses (the `battle_probe` example answers them). Addresses are
//! Infection's (`gcmn.prg` unless marked main); `docs/engine/battle.md` has
//! the rules and the motion in prose.
//!
//! The rules:
//!
//! - [`tables`]: skills, items, equipment, characters and their growth,
//!   enemies (stats, skills, drops, elements), bosses, the default item
//!   lists, the AI settings and the box and trap item lists, read from the
//!   disc at run time.
//! - [`param`], [`chara`]: the structures, and a character's stats
//!   (`CalcReal`), timers (`ConditionTimeCount`), equipment effects and
//!   levels.
//! - [`damage`]: `CalcBattleDamage`, the protect gauge, `ccSkillDamage`.
//! - [`skill`]: skill types and costs, healing, holding, conditions,
//!   cures, the start of a skill (`_ccSkillRequest`).
//! - [`exp`]: experience, infection, drops, the items boxes, traps and
//!   idols hold.
//! - [`flow`]: a skill's life (`ccSkill`: request, frames, notes,
//!   systems) and the normal attack's hit.
//! - [`affect`]: what an affect (damage, healing, revival, a drain) does
//!   to each kind of character.
//! - [`drain`]: Data Drain (the drops, the side effects, the bracelet).
//! - [`item`]: using items, and the save's item lists.
//! - [`enemy_ai`]: the enemies' decisions (`enemy.cpp`), on their own
//!   generator ([`rand::Genrand`], `ccRand`).
//! - [`party_ai`]: the party members' decisions (`personal.cpp`).
//!
//! The motion layer:
//!
//! - [`world`], [`geom`]: the world's queries, and the EE arithmetic
//!   (libvu0's vector and matrix routines, the angle helpers).
//! - [`frame`]: the party's bookkeeping task (`ccThSpc`).
//! - [`kite`]: `ccPlayer` - Kite's frame, stick, acts, notes, his AI's
//!   attack, the player's frame conversions.
//! - [`fellow`], [`follow`]: `ccFellow` - a party member's frame, acts and
//!   collision; the AI's following.
//! - [`navi`], [`ai_move`]: `ccNavi` (dungeon ways, the town's landmarks)
//!   and the AI's own movers (`MoveP2P`, `FollowBeacon`, `ManualControl`,
//!   `ActInTown`).
//! - [`party_motion`]: the party AI's movement calls performed by the
//!   above, as one [`party_ai::Runtime`].
//! - [`entry`], [`races`]: the entry control (`ccThEntryCtrl`), the magic
//!   circles, where the field's and dungeon's entries come from, the race
//!   constructors.
//! - [`enemy_motion`]: the enemies' movement and animation, and
//!   `ccEnemy::main` as a whole frame.
//! - [`ride`]: `ccPucciguso`, the Grunty the flute calls, Kite riding it.
//! - [`prim`], [`weapon`]: `ccPrimRadiate` (the boxes' rays, the flashes) and
//!   the enemies' weapon trails and flashes (`ccEnemyWeaponCtrl`).
//! - [`breath`]: the dogs', wyrms' and dragons' fire breath
//!   (`ccEnemyBreath`, `ccEnemyBrPart`).
//! - [`evparty`]: the event instructions that act on the party and the
//!   enemies (the remote walks, the puts, `pc_command`, `enemy_put`,
//!   `remove`), the `ccThEvHold` task `hold` starts, and `player_skill`.
//!
//! # A frame
//!
//! In the game's task order: [`frame::SpcThread::frame`] (`ccThSpc`, 48);
//! [`party_ai::Crew::tick`] (`ccThAISystem`, 48); [`kite::main`]
//! (`ccThPlayer`, 49); [`fellow::Frame::main`] for each party member
//! (`ccThFellowNN`, 50), its runtime a [`party_motion::Movement`];
//! [`entry::EntryCtrl::frame`] (`ccThEntryCtrl`, 64), whose enemies run
//! [`enemy_motion::Motion::enemy_main`]; [`flow::Skills::frame`]
//! (`ccThSkill`, 82). Per action: [`flow::Skills::request`] when anyone
//! starts a skill (the runtimes' `SkillRequest`), [`item::menu_use_item`]
//! from the item menu, the [`drain`] steps from the Data Drain menu. The
//! rules return [`Event`]s in the game's call order and [`affect::apply`]
//! runs the affects among them; the motion layer applies the affects it
//! makes where the game makes them.

pub mod affect;
pub mod ai_move;
pub mod blocks;
pub mod boss;
pub mod breath;
pub mod chara;
pub mod damage;
pub mod drain;
pub mod enemy_ai;
pub mod enemy_motion;
pub mod entry;
pub mod event;
pub mod evparty;
pub mod exp;
pub mod fellow;
pub mod flow;
pub mod follow;
pub mod frame;
pub mod geom;
pub mod gimetc;
pub mod gimmick;
pub mod item;
pub mod kite;
pub mod navi;
pub mod param;
pub mod party_ai;
pub mod party_chat;
pub mod party_motion;
pub mod prim;
pub mod races;
pub mod rand;
pub mod ride;
pub mod scene;
pub mod skill;
pub mod tables;
pub mod weapon;
pub mod world;

pub use chara::{Body, Char, Env, Foe};
pub use event::{Event, Events, Who};
pub use rand::{Genrand, Rand, Rng};
pub use scene::Scene;
pub use tables::Tables;
