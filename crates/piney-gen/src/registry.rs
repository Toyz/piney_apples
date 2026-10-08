//! The Event NPC's Item List (menu 91, Mutation on), its tables found
//! through its code: the handler `ccThMenu`'s jump table gives for 91, and
//! its data addresses in the order the code first builds them (the
//! completion's record, then the groups' page counts and categories).
//! None on Infection.

use crate::race::{body, code, data, menu_handler, tables};
use crate::volume::{Ctx, Vol};

/// Item List's tables (MUT gcmn addresses in the docs).
#[derive(Clone, Copy, Debug, Default)]
pub struct RegistryAddrs {
    /// The record the Event NPC says when the list is complete (MUT
    /// 0x00669d90).
    pub done: u32,
    /// Each group's page count (0x006822e0) and its pages' categories
    /// (0x006822f0).
    pub pages: u32,
    pub cats: u32,
}

/// Item List's table addresses on the volume (Mutation on).
pub fn addrs(c: &Ctx) -> Result<RegistryAddrs, String> {
    if c.volume == Vol::Inf {
        return Err("Infection has no Item List".into());
    }
    let (lo, code) = code(c)?;
    let f = body(lo, &code, menu_handler(c, lo, &code, 91)?);
    let t: Vec<u32> = tables(c, lo, f).into_iter().filter(|&a| data(c, a)).collect();
    match t[..] {
        [done, pages, cats, ..] => Ok(RegistryAddrs { done, pages, cats }),
        _ => Err(format!("menu 91 builds {} tables ({t:x?}), want three", t.len())),
    }
}
