//! .hack//Infection's combat, from `GCMN.PRG`, as logic: the rules (`common.cpp`,
//! `skill.cpp`, `useitem.cpp`, the AI in `personal.cpp` and `enemy.cpp`) and the
//! motion layer that carries them on the field (entries and magic circles, the
//! enemies, Kite, the party members, following, path finding). Nothing here
//! draws: the world's queries go through [`world::World`], and what shows or
//! sounds comes back as outputs in the game's order. Each function is checked
//! against the game's code by `tools/test_battle_*_rs.py`. Addresses are gcmn's
//! unless marked main; docs/engine/battle.md has the rules and the frame order.

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
pub mod restock;
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
