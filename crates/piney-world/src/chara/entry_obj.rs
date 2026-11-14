//! `ccEntryObj` (gcmn entctrl.cpp, 0x1d0 bytes over `ccChar`): what the
//! entry control keeps of each of its objects beyond the character, and
//! `routine` (0x0042fa60), which `ccThEntryCtrl` runs on each before its
//! class's `main`. Every NPC class (`ccMerchan`, `ccRtownPC`, `ccDog`,
//! `ccPGuso`) and the town's gimmicks (the race's flags) are one.

use crate::chara::char::{Char, w2p};
use crate::ee::{self, F, ONE, V4};
use piney_battle::geom::{pi_limit, set_dirc};

/// `routine`: `dispSW` within 7000, frozen (and off the command lists)
/// beyond 10000.
pub const DISP_DIST: F = 0x45da_c000;
pub const FREEZE_DIST: F = 0x461c_4000;

/// `ccEntryObj`'s state beyond `ccChar` (+0xe0..+0x160) as `routine`
/// keeps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryObj {
    /// +0xe0 bits: `objFlag` (0: `initObject` switched it on), `freezeFlag`
    /// (2), `affectFlag` (3), `dispSW` (4), `cmndFlag` (6: kept off the
    /// command lists for good).
    pub obj_flag: bool,
    pub freeze: bool,
    pub affect: bool,
    pub disp_sw: bool,
    pub cmnd_flag: bool,
    /// On `ccEntryCmnd`'s object list (`cmndObjRoot`): what `ccCheckTarget`
    /// finds.
    pub listed: bool,
    /// +0xe4 `plDist` (Kite's distance on the ground), +0xe8 `plDirc` (the
    /// heading that faces him).
    pub pl_dist: F,
    pub pl_dirc: F,
    /// +0xf0 `fadeFlag` (1 in, 2 out) and +0xf2 `fadeCnt` (frames).
    pub fade_flag: i16,
    pub fade_cnt: i16,
    /// +0xf4 `grotDeg`, +0xf6 `grotSpd`: a turn to `grotDeg` under way.
    pub grot_deg: i16,
    pub grot_spd: i16,
    /// `entParam.id` (+0x124): the table row the entry made it from, which
    /// a class's later `SetBaseParam` leaves alone.
    pub ent_id: i32,
    /// `entParam.entRoot` (+0x140): -1 for merchants.
    pub ent_root: i32,
}

impl Default for EntryObj {
    /// `ccEntryObj::ccEntryObj` (gcmn 0x0042f7d0): every flag clear but
    /// `dispSW`.
    fn default() -> Self {
        EntryObj {
            obj_flag: false,
            freeze: false,
            affect: false,
            disp_sw: true,
            cmnd_flag: false,
            listed: false,
            pl_dist: 0,
            pl_dirc: 0,
            fade_flag: 0,
            fade_cnt: 0,
            grot_deg: 0,
            grot_spd: 0,
            ent_id: 0,
            ent_root: -1,
        }
    }
}

impl EntryObj {
    /// `ccEntryObj::routine` (gcmn 0x0042fa60) for a character `ch` with
    /// Kite at `player`: the event's turn, `plDist`/`plDirc`, `dispSW`, the
    /// command lists (`ccEntryCmnd` within 10000, `deleteCmnd` beyond), the
    /// fade.
    pub fn routine(&mut self, volume: piney_data::volume::Volume, ch: &mut Char, player: V4) {
        let pos = ch.pos;
        self.routine_parts(volume, pos, &mut ch.dirc, &mut ch.set_transparency, &mut ch.transparency, player);
    }

    /// [`EntryObj::routine`] on a `ccChar`'s parts: its place, heading and
    /// transparencies (a gimmick of no [`Char`]: the race's flags).
    pub fn routine_parts(
        &mut self,
        volume: piney_data::volume::Volume,
        pos: V4,
        dirc: &mut V4,
        set_transparency: &mut F,
        transparency: &mut F,
        player: V4,
    ) {
        if self.grot_spd != 0 {
            let r = ee::deg2rad(self.grot_deg);
            let z = set_dirc(dirc[2], r, i32::from(self.grot_spd));
            dirc[2] = z;
            // fabs((double) (z - r)) < (double) 0.003f
            if f64::from(ee::f(ee::sub(z, r))).abs() < f64::from_bits(0x3f68_9374_c000_0000) {
                self.grot_spd = 0;
            }
        }
        let mut p = w2p(pos, player);
        p[0] = ee::mul(p[0], 0xbf80_0000);
        p[1] = ee::mul(p[1], 0xbf80_0000);
        p[2] = 0;
        p[3] = ONE;
        self.pl_dist = ee::sqrtf_on(volume, ee::dot(p, p));
        self.pl_dirc = pi_limit(ee::add(0x3fc9_0fdb, ee::atan2f(p[1], p[0])));
        self.disp_sw = ee::le(self.pl_dist, DISP_DIST);
        if ee::le(self.pl_dist, FREEZE_DIST) {
            self.freeze = false;
            if !self.cmnd_flag {
                self.entry_cmnd();
            }
        } else {
            self.freeze = true;
            self.delete_cmnd(self.ent_root != -1 && self.ent_root != 0);
        }
        match self.fade_flag {
            1 => {
                let mut t = ee::add(*set_transparency, ee::div(ONE, ee::from_int(i32::from(self.fade_cnt))));
                if !ee::le(t, ONE) {
                    t = ONE;
                    self.fade_flag = 0;
                }
                *transparency = t;
                *set_transparency = t;
            }
            2 => {
                let mut t = ee::sub(*set_transparency, ee::div(ONE, ee::from_int(i32::from(self.fade_cnt))));
                if ee::lt(t, 0) {
                    t = 0;
                    self.fade_flag = 0;
                }
                *transparency = t;
                *set_transparency = t;
            }
            _ => {}
        }
    }

    /// `ccEntryCmnd(this)` (gcmn 0x00519630): onto the object command list.
    pub fn entry_cmnd(&mut self) {
        self.listed = true;
    }

    /// `ccDeleteCmnd(this)` (gcmn 0x00519700): off the command lists.
    pub fn delete_cmnd_list(&mut self) {
        self.listed = false;
    }

    /// `ccEntryObj::deleteCmnd(flg)` (0x0042fe40): off the command lists
    /// (`ccDeleteCmnd`) unless `cmndFlag` already keeps it off, which `flg`
    /// then sets.
    pub fn delete_cmnd(&mut self, flg: bool) {
        if !self.cmnd_flag {
            self.listed = false;
            self.cmnd_flag = flg;
        }
    }
}
