//! Kite's acts and the animation of each (`playerAnimTbl`, gcmn
//! 0x006f0560). His `ccPlayer::AnimCtrl` is the battle's
//! (`piney_battle::kite`), in every area.

/// `playerAnimTbl`: the animation of each act (26 entries of 21 bytes).
pub const PLAYER_ANIM_TBL: [&str; 26] = [
    "ANM_ctu1nut0",
    "ANM_ctu1nut1",
    "ANM_ctu1nut2",
    "ANM_ctu1nut3",
    "ANM_ctu1nut4",
    "ANM_ctu1run0",
    "ANM_ctu1wal0",
    "ANM_ctu1dmg0",
    "ANM_ctu1dmg1",
    "ANM_ctu1dwn0",
    "ANM_ctu1dwn0",
    "ANM_ctu1nut2",
    "ANM_ctu1nut2",
    "ANM_ctu1nut2",
    "ANM_ctu1nut2",
    "ANM_ctu1atc0",
    "ANM_ctu1atc1",
    "ANM_ctu1mag0",
    "ANM_ctu1mag1",
    "ANM_ctu1ski2",
    "ANM_ctu1ski3",
    "ANM_ctu1ski4",
    "ANM_ctu1nut2",
    "ANM_ctu1nut2",
    "ANM_ctu1hac4b",
    "ANM_ctu1atc0",
];

/// The acts the town reaches.
pub mod act {
    /// Field idle (`nut0`).
    pub const IDLE_FIELD: i16 = 0;
    pub const FIDGET_FIELD: i16 = 1;
    /// Town idle (`nut2`, looping).
    pub const IDLE: i16 = 2;
    /// The fidget after 451 frames standing (`nut3`, once), then `nut4`.
    pub const FIDGET: i16 = 3;
    pub const FIDGET_LOOP: i16 = 4;
    pub const RUN: i16 = 5;
    pub const WALK: i16 = 6;
    /// Leaving through a gate (`ccSpcChar::TransferOut`): `nut2` with the
    /// fade-out, then out of sight.
    pub const LEAVE: i16 = 12;
    /// Arriving at a town or gate: `nut2` with the fade-in.
    pub const ARRIVE: i16 = 13;
    /// Out of sight (`SetBootStatus` bit 1, the departure's end).
    pub const HIDDEN: i16 = 14;
}
