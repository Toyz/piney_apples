//! The game's one `ccRand` (INF main 0x001d9a10) is the battle's
//! [`Genrand`]: every area holds it as `Combat::cc`, seeded by `ccInitRand`
//! (0x001d9900) on every scene change, and the walking PCs, the Grunties,
//! the party's AI, the enemies and the map's effects draw from it. The save
//! carries it in its own form ([`CcRand`]) for the menus beside the tasks.

use piney_battle::rand::Genrand;
use piney_desktop::staffroll::CcRand;

/// The battle's `ccRand` into the save's form `c`, in place (a host lends
/// it to the menus through `SaveState::cc`).
pub fn put(g: &Genrand, c: &mut CcRand) {
    *c.mt = g.mt;
    c.mti = g.mti as usize;
}

/// The battle's `ccRand` back from the save's form `c`.
pub fn take(g: &mut Genrand, c: &CcRand) {
    g.mt = *c.mt;
    g.mti = c.mti as i32;
}
