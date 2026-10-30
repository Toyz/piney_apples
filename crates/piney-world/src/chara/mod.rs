//! The characters on the field: `ccChar` and its body, pose and motion, Kite
//! (`ccPlayer`), the party (`ccSPC`, `ccAI`, the routes), the enemies on the
//! field and the town NPCs' table.

pub mod ai;
pub mod arms;
pub mod body;
pub mod char;
pub mod foe;
pub mod kite;
pub mod motion;
#[doc(alias = "ccnavi")]
pub mod navi;
pub mod npc;
pub mod party;
#[doc(alias = "ccPlayer")]
pub mod player;
pub mod pose;
