//! The Root Towns' merchants: `ccMerchan` (gcmn merchan.cpp, 0x210 bytes, a
//! `ccGimmick` and so a `ccEntryObj`), the shopkeepers standing in their
//! booths. Mac Anu has five, `npcTbl` rows 0-4, all `CTR1.CCS`:
//!
//! ```text
//! row  name         flags   dummy          position              heading
//!  0   Weapon Shop  0x0100  DMY_merchant1  ( 2400, -2450,   0)   -180 deg
//!  1   Elf's Haven  0x0200  DMY_merchant4  ( 2400,  2450,   0)      0
//!  2   Item Shop    0x0400  DMY_merchant3  (-2400,  2450,   0)      0
//!  3   Magic Shop   0x0800  DMY_merchant2  (-2400, -2450,   0)   -180 deg
//!  4   Recorder     0x1000  DMY_merchant5  ( -850,  3600, 300)     90 deg
//! ```
//!
//! How the game places them (`ccEntryEventMng`, main 0x001b62e0, in a town:
//! `ccSetMerchant(0)` before `ccSetChaosGate` and the walking PCs):
//!
//! ```text
//! ccSetMerchant(0) 0x005057f0    area 0: setMerchant of the town's rows (town 0:
//!                                0-4; 1: 5-10; 2: 11-16; 3: 17-22; 4: 23-28)
//! setMerchant(id) 0x00505590     ccEntryParamClear (all -1, type/id 0, pos and
//!                                dirc (0,0,0,1)); type 2 (an NPC), id, the game's
//!                                area/town/floor/block, entRoot -1; the dummy of
//!                                the jump table 0x006ac890 (ids 0-4: merchant 1,
//!                                4, 3, 2, 5; the breeders 10/16/22/28: 6) from the
//!                                town's stream: pos = its +0x10 (w 1), dirc = its
//!                                +0x20 (Decode_DummyPosRot's pi deg / 180, w 0)
//! ccEntryCtrl::entryObject       npcTbl's esize (1): land still -1, so pos.z =
//!   0x00430c90 / 0x004307f0      ccLandHitCheck(pos, 0x20000002); entryNpc
//! entryNpc 0x004317b0            entry.ep = &param; ccEntryRtownMerchant
//!                                (0x00505a50): new ccMerchan; initObject; onto
//!                                the NPC list (after the gimmicks, run last)
//! ccMerchan::ccMerchan 0x00505aa0
//!   entParam, pos, dirc and defaultDirc (+0x1e0) from the param; base =
//!   npcTbl[id] (height 180, width 100); bodyHit: pos, radius 35, height 120,
//!   type 2; the CMP_trall clump; anmTbl (+0x204) = merchanAnmPtr[town]
//!   (0x005ed870: ANM_ctrNnut0, ANM_ctrNact0, ANM_ctrNact2 of CTRN), the
//!   administrator's (29) and the quiz man's (158) their own; ids 10, 16, 22,
//!   28, 29 and 158 bodyHit.HitEnable and affectFunc = breederInfluence, the
//!   rest HitDisable (the affectFunc stays ccGimmick's ccGimmickAffect);
//!   SetAnm(anmTbl[0]); actNum, actProcess, anmOld 0; transrate 1
//! ccEntryCtrl::initObject        its area is the entry control's: objFlag,
//!   0x00430ec0                   initFlag, bodyHit.SetHitSW(1) (on the
//!                                character hit list, after all); land -1
//!                                again: pos.z = ccLandHitCheck(pos, 0x20000002);
//!                                entParam.pos = pos; posP = W2PPos(pos)
//! ```
//!
//! No palette is swapped (the rows' `clut` is empty and `ccMerchan` never
//! calls `ccEntryChangeCLUT`). Each frame `ccThEntryCtrl` (priority 64, after
//! the player) runs, for each NPC in list order, [`EntryObj::routine`]
//! (`ccEntryObj::routine` 0x0042fa60) then `ccMerchan::main` (0x00505e20):
//!
//! ```text
//! posP = W2PPos(pos); d = |posP| (x, y less Kite's, the merchant's own z)
//! cam = ccCheckCameraDeg(pos, 12288)       main 0x001da710
//! npcID 29 or 158: sysopeAct 0x00506170 (the Administrator: comes and
//! goes through the gate's transfer, see [`Merchant::sysope_act`]); else
//! breederAct 0x00505fc0
//! bodyHit.pos = pos; CollisionDetection: bodyHitFlag, bodyHitCnt
//! if d < 4600 and cam:
//!     _AnimateForward(frameSpd); SetMatrix_PosRotZYX(pos, dirc)
//!     transparency = setTransparency = transrate; ccChar::Draw
//! ```
//!
//! So a merchant steps and draws only within 4600 of Kite and inside 67.5
//! degrees either side of the camera's line of sight (measured on the
//! ground from the camera's eye); `ccChar::Draw` then fades it near the
//! camera and beyond 4000 ([`Char::fade`]).
//!
//! Facing: in Mac Anu nothing turns a merchant. `breederAct` turns one
//! toward Kite (`plDirc`, act 1, `ccSetDirc(.., 64)`, playing anmTbl[2] for
//! 120 frames) or back to `defaultDirc` (act 2, `ccSetDirc(.., 128)`,
//! anmTbl[0]) only when `breederInfluence` (0x005065d0) set the act: the
//! menu's `EntryAffect` of command 14 (the shop opening) or 15 gives act 1,
//! command 0 (closing) act 2. Only the Grunt Shops (10, 16, 22, 28), the
//! administrator and the quiz man have it; Mac Anu's five keep
//! `ccGimmickAffect` (0x00453400), which only sets `affectFlag`. The event
//! scripts' turn (`grotDeg`/`grotSpd`, `ccSetDirc` in `routine` until within
//! 0.003) and fades (`fadeFlag`/`fadeCnt`) are in [`EntryObj`].

use std::rc::Rc;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::body::{Body, TRALL};
use crate::camera::Cam;
use crate::char::{Char, View, w2p};
use crate::ee::{self, F, ONE, V4};
use crate::entry::{Npc, NpcCtx};
use crate::hit::{self, Hits};
use crate::npc::NpcRow;
use crate::town::TownLights;

/// `ccMerchan::main` steps and draws only nearer than this (4600).
pub const NEAR: F = 0x458f_c000;
/// `ccCheckCameraDeg(pos, 12288)`: 67.5 degrees either side.
pub const VIEW_DEG: i16 = 12288;
/// `ccEntryObj::routine`: `dispSW` within 7000, frozen (and off the command
/// lists) beyond 10000.
pub const DISP_DIST: F = 0x45da_c000;
pub const FREEZE_DIST: F = 0x461c_4000;
/// The merchant's `bodyHit`: radius 35, height 120, type 2; both masks all
/// ones (every character type; no hit polygon has every bit).
pub const BODY_RADIUS: F = 0x420c_0000;
pub const BODY_HEIGHT: F = 0x42f0_0000;
pub const BODY_TYPE: u32 = 2;
pub const BODY_MASK: u32 = 0xffff_ffff;
/// The merchants' bodies' keys in [`Hits::chars`]: this plus the row (the
/// walking PCs' are [`crate::rtownpc::BODY_ID`] on, Kite's
/// [`hit::PLAYER_ID`]).
pub const BODY_ID: u32 = 0x80;
/// `ccLandHitCheck`'s mask for entries: floors with bit 1.
pub const ENTRY_LAND_MASK: u32 = 0x2000_0002;
/// `merchan1AnmTbl`..`merchan4AnmTbl` (0x005ed810..), `merchanAnmPtr`
/// (0x005ed870) by town: idle, act0 (unused), act2 (the breeders' talk).
pub const ANM_TBL: [[&str; 3]; 4] = [
    ["ANM_ctr1nut0", "ANM_ctr1act0", "ANM_ctr1act2"],
    ["ANM_ctr2nut0", "ANM_ctr2act0", "ANM_ctr2act2"],
    ["ANM_ctr3nut0", "ANM_ctr3act0", "ANM_ctr3act2"],
    ["ANM_ctr4nut0", "ANM_ctr4act0", "ANM_ctr4act2"],
];
/// `ccMerchan::breederAct`'s talk ends when the anm's `frameNow` reaches
/// this.
pub const TALK_FRAMES: u32 = 120;
/// The ids `ccMerchan::ccMerchan` gives `breederInfluence` (and a live
/// `bodyHit`): the Grunt Shops, the administrator (29), the quiz man (158).
pub const INFLUENCED: [i32; 6] = [158, 29, 28, 22, 16, 10];
/// The ids `ccMerchan::main` runs `sysopeAct` for (the Administrator).
pub const SYSOPE: [i32; 2] = [29, 158];
/// `ccSetMerchant(0)`'s rows by town.
pub const TOWN_ROWS: [std::ops::RangeInclusive<usize>; 5] = [0..=4, 5..=10, 11..=16, 17..=22, 23..=28];

/// `setMerchant`'s jump table (0x006ac890): the `DMY_merchantN` of each id
/// below 29.
pub fn dummy_of(id: i32) -> Option<String> {
    const N: [u8; 6] = [1, 4, 3, 2, 5, 6];
    let k = match id {
        0..=4 => id as usize,
        5..=28 => ((id - 5) % 6) as usize,
        _ => return None,
    };
    Some(format!("DMY_merchant{}", N[k]))
}

/// `ccMerchan::ccMerchan`'s `anmTbl` for id `id` in town `town`: the
/// Administrator's `sysopeAnmTbl` (0x005ed850) and the quiz man's
/// `quizmanAnmTbl` (0x005ed860) name the first two towns' clips anywhere.
pub fn anm_tbl(id: i32, town: i32) -> Option<[&'static str; 3]> {
    match (id, town) {
        (29, _) => Some(ANM_TBL[0]),
        (158, _) => Some(ANM_TBL[1]),
        (_, 0..=3) => Some(ANM_TBL[town as usize]),
        (23..=28, _) => Some(ANM_TBL[1 + ((id - 23) / 2) as usize]),
        _ => None,
    }
}

/// `ccGetDircChg(cur, tgt, spd)` (main 0x001d9eb0): the step from `cur`
/// toward `tgt` (16-bit angles) the shorter way round, `16 (tgt - cur) /
/// (spd & 0xffff)` and at least 1 in mode 0 (`spd & 0xf0000`); mode 0x10000
/// the same but nothing when `16 |tgt - cur|` is under 1024, mode 0x20000
/// the whole difference then; other modes `16 (tgt - cur)`; at most 24576.
pub fn dirc_chg(cur: i16, tgt: i16, spd: i32) -> i32 {
    let d = i32::from(tgt) - i32::from(cur);
    if d == 0 {
        return 0;
    }
    let mut a = i64::from((d as u32) & 0xffff) << 4;
    let neg = a >= 0x8_0001;
    if neg {
        a = 0x10_0000 - a;
    }
    let div = i64::from(spd & 0xffff);
    let by = |a: i64| if div == 0 { a } else { (a / div).max(1) };
    match spd & 0xf_0000 {
        0 => a = by(a),
        0x1_0000 => a = if a < 1024 { 0 } else { by(a) },
        0x2_0000 => a = if a < 1024 { a >> 4 } else { by(a) },
        _ => {}
    }
    let a = a.min(24576) as i32;
    if neg { -a } else { a }
}

/// `ccSetDirc(&angle, target, spd)` (main 0x001da0b0): `angle` turned toward
/// `target` by [`dirc_chg`] in 16-bit angles.
pub fn set_dirc(angle: F, target: F, spd: i32) -> F {
    let cur = ee::rad2deg(angle);
    let chg = dirc_chg(cur, ee::rad2deg(target), spd);
    ee::deg2rad((i32::from(cur) + chg) as i16)
}

/// `ccGetDirc(from, to)` (main 0x001d9ce0): the heading that faces `to`
/// from `from` on the ground (0 faces -y), within -pi..pi.
pub fn get_dirc(from: V4, to: V4) -> F {
    let a = ee::add(0x3fc9_0fdb, ee::atan2f(ee::sub(to[1], from[1]), ee::sub(to[0], from[0])));
    wrap_pi(a)
}

/// Into -pi..pi by one turn, as `ccGetDirc` and `routine` do it.
fn wrap_pi(mut a: F) -> F {
    if !ee::le(a, ee::PI) {
        a = ee::sub(a, 0x40c9_0fdb);
    }
    if ee::lt(a, ee::neg(ee::PI)) {
        a = ee::add(a, 0x40c9_0fdb);
    }
    a
}

/// `ccCheckCameraDeg(pos, deg)` (main 0x001da710) with the field camera
/// (`activeCamPtr` is `tcam`) and Kite at `player`: is `pos` within `deg`
/// either side of the line from the eye to the point looked at, on the
/// ground (all three through `W2PPos`)? The bounds are open.
pub fn check_camera_deg(pos: V4, player: V4, cam: &Cam, deg: i16) -> bool {
    let p = w2p(pos, player);
    let c = w2p(cam.pos, player);
    let v = w2p(cam.view, player);
    let a = ee::rad2deg(ee::atan2f(ee::sub(p[1], c[1]), ee::sub(p[0], c[0])));
    let b = ee::rad2deg(ee::atan2f(ee::sub(v[1], c[1]), ee::sub(v[0], c[0])));
    let d = (i32::from(deg) + (i32::from(a) - i32::from(b))) as i16;
    d > 0 && i32::from(d) < 2 * i32::from(deg)
}

/// `ccEntryObj`'s state beyond `ccChar` (+0xe0..+0x160) as `routine`
/// keeps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryObj {
    /// +0xe0 bits: `freezeFlag` (2), `affectFlag` (3), `dispSW` (4),
    /// `cmndFlag` (6: kept off the command lists for good).
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
    /// `entParam.entRoot` (+0x140): -1 for merchants.
    pub ent_root: i32,
}

impl Default for EntryObj {
    /// `ccEntryObj::ccEntryObj`: every flag clear but `dispSW`.
    fn default() -> Self {
        EntryObj {
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
            ent_root: -1,
        }
    }
}

impl EntryObj {
    /// `ccEntryObj::routine` (gcmn 0x0042fa60) for a character `ch` with
    /// Kite at `player`: the event's turn, `plDist`/`plDirc`, `dispSW`, the
    /// command lists (`ccEntryCmnd` within 10000, `deleteCmnd` beyond), the
    /// fade.
    pub fn routine(&mut self, ch: &mut Char, player: V4) {
        if self.grot_spd != 0 {
            let r = ee::deg2rad(self.grot_deg);
            let z = set_dirc(ch.dirc[2], r, i32::from(self.grot_spd));
            ch.dirc[2] = z;
            // fabs((double) (z - r)) < (double) 0.003f
            if f64::from(ee::f(ee::sub(z, r))).abs() < f64::from_bits(0x3f68_9374_c000_0000) {
                self.grot_spd = 0;
            }
        }
        let mut p = w2p(ch.pos, player);
        p[0] = ee::mul(p[0], 0xbf80_0000);
        p[1] = ee::mul(p[1], 0xbf80_0000);
        p[2] = 0;
        p[3] = ONE;
        self.pl_dist = ee::sqrtf(ee::dot(p, p));
        self.pl_dirc = wrap_pi(ee::add(0x3fc9_0fdb, ee::atan2f(p[1], p[0])));
        self.disp_sw = ee::le(self.pl_dist, DISP_DIST);
        if ee::le(self.pl_dist, FREEZE_DIST) {
            self.freeze = false;
            if !self.cmnd_flag {
                self.listed = true;
            }
        } else {
            self.freeze = true;
            self.delete_cmnd(self.ent_root != -1 && self.ent_root != 0);
        }
        match self.fade_flag {
            1 => {
                let mut t = ee::add(ch.set_transparency, ee::div(ONE, ee::from_int(i32::from(self.fade_cnt))));
                if !ee::le(t, ONE) {
                    t = ONE;
                    self.fade_flag = 0;
                }
                ch.transparency = t;
                ch.set_transparency = t;
            }
            2 => {
                let mut t = ee::sub(ch.set_transparency, ee::div(ONE, ee::from_int(i32::from(self.fade_cnt))));
                if ee::lt(t, 0) {
                    t = 0;
                    self.fade_flag = 0;
                }
                ch.transparency = t;
                ch.set_transparency = t;
            }
            _ => {}
        }
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

/// What a merchant's `main` did in a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MerchantFrame {
    /// Nearer than 4600 to Kite.
    pub near: bool,
    /// Inside `ccCheckCameraDeg`'s cone.
    pub in_view: bool,
    /// Both: the anm stepped and `ccChar::Draw` ran.
    pub stepped: bool,
    /// `ccChar::Draw` drew it.
    pub drawn: bool,
}

/// What the Administrator's `sysopeAct` starts, for the town to carry out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SysopEvent {
    /// `effTransfer(this)`: coming or going through the gate.
    Transfer,
    /// `ccSeOn(217)`, `effSkillExecForceRing(this, 4, 1)`,
    /// `effSkillExecForceRing2(this, 4, 1)`, `effSkillTornadeRings(this, 4,
    /// 3)`: act -5's vanishing.
    Vanish,
    /// `ccMenu.interNoiz = 2` (the events' `noise2`), 31 frames after.
    Noise,
}

/// One merchant.
#[derive(Clone)]
pub struct Merchant {
    /// `npcID` (+0x1f0), the `npcTbl` row; the base parameters' name, type
    /// flags and message table (`base->msg`: one `ccEvMsgData` list per
    /// server).
    pub id: i32,
    pub name: String,
    pub flags: u32,
    pub msg: u32,
    pub ch: Char,
    pub entry: EntryObj,
    /// +0x1e0 `defaultDirc`: the dummy's rotation.
    pub default_dirc: V4,
    /// +0x1f4 `actNum`, +0x1f8 `actProcess`, +0x1fc `anmOld`.
    pub act_num: i32,
    pub act_process: i32,
    pub anm_old: i32,
    /// +0x200 `bodyHitFlag`, +0x202 `bodyHitCnt`: touching another
    /// character, and for how many frames.
    pub body_hit_flag: i16,
    pub body_hit_cnt: i16,
    /// +0x204 `anmTbl`.
    pub anm_tbl: [&'static str; 3],
    /// +0x208 `transrate`: what `main` sets the transparency to (1).
    pub transrate: F,
    /// +0x9c `affectType`: the command `EntryAffect` last passed.
    pub affect_type: i16,
    /// The last frame's.
    pub frame: MerchantFrame,
    /// +0x1d8, the Administrator's frame count within an act.
    pub act_cnt: i32,
    /// What `sysopeAct` started since the town last took it.
    pub sysop_events: Vec<SysopEvent>,
    /// `sysopeAct` is done with it (act 5's end: `main` returns 1 and the
    /// entry control deletes it).
    pub gone: bool,
}

impl Merchant {
    /// `setMerchant(row.id)` in town `town`, through `entryObject`,
    /// `ccMerchan::ccMerchan` and `initObject`: at its dummy in the town's
    /// file `town_file`, set on the ground of `hits` twice, with `body` (the
    /// row's file's `CMP_trall`) playing anmTbl[0], Kite at `player`.
    pub fn new(
        row: &NpcRow,
        body: Rc<Body>,
        town_file: &SceneFile,
        hits: &mut Hits,
        town: i32,
        player: V4,
    ) -> Result<Merchant> {
        let id = i32::from(row.id);
        let dummy_name = dummy_of(id).ok_or_else(|| Error::NotFound(format!("merchant {id}: no dummy")))?;
        let dummy = town_file
            .ccs
            .find_object(&dummy_name)
            .and_then(|d| town_file.scene.dummies.get(&d))
            .ok_or_else(|| Error::NotFound(dummy_name.clone()))?;
        let rot = dummy.rot.ok_or_else(|| Error::NotFound(format!("{dummy_name}: no rotation")))?;
        let pos = [dummy.pos.x.to_bits(), dummy.pos.y.to_bits(), dummy.pos.z.to_bits(), ONE];
        let r = piney_data::anim::const_radians([rot.x.to_bits(), rot.y.to_bits(), rot.z.to_bits()]);
        Merchant::at(row, body, pos, [r[0], r[1], r[2], 0], hits, town, player)
    }

    /// `ccMerchan::ccMerchan` over an entry at `pos` facing `dirc` (what
    /// `setMerchant` leaves for ids 29 and 158: the origin, for an event to
    /// place), through `entryObject` and `initObject` as [`Merchant::new`].
    pub fn at(
        row: &NpcRow,
        body: Rc<Body>,
        pos: V4,
        dirc: V4,
        hits: &mut Hits,
        town: i32,
        player: V4,
    ) -> Result<Merchant> {
        let id = i32::from(row.id);
        let anm_tbl = anm_tbl(id, town).ok_or_else(|| Error::NotFound(format!("merchant {id}: not ported")))?;
        let mut pos = pos;
        // entryObject(ep, 1): land -1.
        pos[2] = hits.land(pos, ENTRY_LAND_MASK);
        let mut ch = Char::new(body, anm_tbl[0], pos, dirc, row.height, row.width)
            .ok_or_else(|| Error::NotFound(anm_tbl[0].into()))?;
        ch.hit = hit::Body {
            pos,
            radius: BODY_RADIUS,
            height: BODY_HEIGHT,
            mask: BODY_MASK,
            mask2: BODY_MASK,
            kind: BODY_TYPE,
            id: BODY_ID + id as u32,
            ..hit::Body::default()
        };
        // initObject: SetHitSW(1) (onto the character list's tail), the
        // ground again, posP.
        hits.hit_enable(&mut ch.hit);
        pos[2] = hits.land(pos, ENTRY_LAND_MASK);
        ch.pos = pos;
        ch.pos_p = w2p(pos, player);
        Ok(Merchant {
            id,
            name: row.name.clone(),
            flags: row.flags,
            msg: row.msg,
            ch,
            entry: EntryObj::default(),
            default_dirc: dirc,
            act_num: 0,
            act_process: 0,
            anm_old: 0,
            body_hit_flag: 0,
            body_hit_cnt: 0,
            anm_tbl,
            transrate: ONE,
            affect_type: 0,
            frame: MerchantFrame::default(),
            act_cnt: 0,
            sysop_events: Vec::new(),
            gone: false,
        })
    }

    /// Whether `affectFunc` is `breederInfluence`.
    pub fn is_breeder(&self) -> bool {
        INFLUENCED.contains(&self.id)
    }

    /// `ccAnm::SetAnm(anmTbl[k])`.
    fn set_anm(&mut self, k: usize) {
        self.ch.set_anim(self.anm_tbl[k]);
    }

    /// `ccMerchan::sysopeAct` (gcmn 0x00506170): the Administrator's acts,
    /// which the events' `npc_act 29 N` sets (`actNum` N, `actProcess` 0);
    /// true when it is done with (act 5's end). `transrate` is what `main`
    /// draws him at.
    ///
    /// ```text
    /// 2, other   anmTbl[0], transrate 1
    /// 3          anmTbl[0], transrate 0; effTransfer; then transrate + 0.02 a
    ///            frame to 1, then act 2
    /// 4, 5       transrate 1, off the command list, effTransfer; then
    ///            transrate - 0.02 a frame to 0; after 141 frames act 4 stays,
    ///            act 5 ends (deleted)
    /// 6          transrate 0 (hidden)
    /// 7          transrate 1, act 2
    /// -3         anmTbl[2], turning toward Kite (ccSetDirc 64) until its frame
    ///            reaches 120, then act 0
    /// -5         transrate 1, anmTbl[2] at speed 384, sound 217, the force
    ///            rings and the tornado's rings; 31 frames on, interNoiz 2
    /// ```
    pub fn sysope_act(&mut self) -> bool {
        // 0.02.
        const STEP: F = 0x3ca3_d70a;
        match self.act_num {
            -5 => match self.act_process {
                0 => {
                    self.transrate = ONE;
                    self.set_anm(2);
                    self.ch.play.frame_spd = 384;
                    self.sysop_events.push(SysopEvent::Vanish);
                    self.act_process += 1;
                    self.act_cnt = 0;
                }
                1 => {
                    let c = self.act_cnt;
                    self.act_cnt += 1;
                    if c >= 31 {
                        self.sysop_events.push(SysopEvent::Noise);
                        self.act_process += 1;
                        self.act_cnt = 0;
                    }
                }
                _ => {}
            },
            3 => match self.act_process {
                0 => {
                    self.set_anm(0);
                    self.act_process += 1;
                    self.transrate = 0;
                }
                1 => {
                    self.sysop_events.push(SysopEvent::Transfer);
                    self.act_process += 1;
                }
                2 => {
                    let t = ee::add(self.transrate, STEP);
                    self.transrate = t;
                    if !ee::le(t, ONE) {
                        self.transrate = ONE;
                        self.act_process += 1;
                        self.act_cnt = 0;
                        self.act_num = 2;
                    }
                }
                _ => {}
            },
            7 => {
                self.transrate = ONE;
                self.act_num = 2;
            }
            6 => {
                if self.act_process == 0 {
                    self.transrate = 0;
                }
            }
            4 | 5 => match self.act_process {
                0 => {
                    self.transrate = ONE;
                    // deleteCmnd(this).
                    self.entry.listed = false;
                    self.sysop_events.push(SysopEvent::Transfer);
                    self.act_process += 1;
                    self.act_cnt = 0;
                }
                1 => {
                    let t = ee::sub(self.transrate, STEP);
                    self.transrate = if ee::lt(t, 0) { 0 } else { t };
                    let c = self.act_cnt;
                    self.act_cnt += 1;
                    if c >= 141 {
                        if self.act_num == 5 {
                            return true;
                        }
                        self.act_process += 1;
                    }
                }
                _ => {}
            },
            -3 => {
                if self.act_process == 0 {
                    self.set_anm(2);
                    self.act_process += 1;
                }
                self.ch.dirc[2] = set_dirc(self.ch.dirc[2], self.entry.pl_dirc, 64);
                if self.ch.play.time >> 8 >= 120 {
                    self.act_process = 0;
                    self.act_num = 0;
                }
            }
            _ => {
                if self.act_process == 0 {
                    self.transrate = ONE;
                    self.set_anm(0);
                    self.act_process += 1;
                }
            }
        }
        false
    }

    /// `ccMerchan::breederAct` (0x00505fc0): act 0 idles, act 1 plays
    /// anmTbl[2] turning toward Kite (`ccSetDirc(dirc.z, plDirc, 64)`) until
    /// the anm's frame reaches 120, act 2 idles turning back to
    /// `defaultDirc` (`ccSetDirc(.., 128)`).
    pub fn breeder_act(&mut self) {
        match self.act_num {
            0 => {
                if self.act_process == 0 {
                    if self.anm_old != 0 {
                        self.set_anm(0);
                    }
                    self.anm_old = 0;
                    self.act_process += 1;
                }
            }
            1 => {
                if self.act_process == 0 {
                    if self.anm_old != 2 {
                        self.set_anm(2);
                    }
                    self.anm_old = 2;
                    self.act_process += 1;
                }
                self.ch.dirc[2] = set_dirc(self.ch.dirc[2], self.entry.pl_dirc, 64);
                if self.ch.play.time >> 8 >= TALK_FRAMES {
                    self.act_process = 0;
                    self.act_num = 0;
                }
            }
            2 => {
                if self.act_process == 0 {
                    if self.anm_old != 0 {
                        self.set_anm(0);
                    }
                    self.anm_old = 0;
                    self.act_process += 1;
                }
                self.ch.dirc[2] = set_dirc(self.ch.dirc[2], self.default_dirc[2], 128);
            }
            _ => {}
        }
    }

    /// `breederInfluence` (0x005065d0) of the menu's command: 14 (the shop
    /// opens) or 15, act 1 (face Kite); 0 (it closes), act 2 (face the booth
    /// again); others nothing.
    pub fn breeder_influence(&mut self, cmd: i16) {
        self.affect_type = cmd;
        match cmd {
            14 | 15 => (self.act_num, self.act_process) = (1, 0),
            0 => (self.act_num, self.act_process) = (2, 0),
            _ => {}
        }
    }

    /// `ccMerchan::main` (0x00505e20) after `routine`, with Kite at
    /// `player`, the camera `cam` and `view`, the town's `hits`.
    pub fn main(&mut self, player: V4, cam: &Cam, view: &View, hits: &mut Hits) -> MerchantFrame {
        self.ch.pos_p = w2p(self.ch.pos, player);
        let p = self.ch.pos_p;
        // x x + y y (adda), + z z (madd); fptodp, sqrt, dptofp.
        let d2 = ee::add(ee::add(ee::mul(p[0], p[0]), ee::mul(p[1], p[1])), ee::mul(p[2], p[2]));
        let dist = ee::sqrtf(d2);
        let in_view = check_camera_deg(self.ch.pos, player, cam, VIEW_DEG);
        if SYSOPE.contains(&self.id) {
            if self.sysope_act() {
                // main returns 1 at once: the object is deleted.
                self.gone = true;
                return MerchantFrame::default();
            }
        } else {
            self.breeder_act();
        }
        self.ch.hit.pos = self.ch.pos;
        if hits.collision_detection(&mut self.ch.hit) != 0 {
            self.body_hit_flag = 1;
            self.body_hit_cnt = self.body_hit_cnt.wrapping_add(1);
        } else {
            self.body_hit_flag = 0;
            self.body_hit_cnt = 0;
        }
        let near = ee::lt(dist, NEAR);
        let mut f = MerchantFrame { near, in_view, ..MerchantFrame::default() };
        if near && in_view {
            self.ch.forward();
            self.ch.transparency = self.transrate;
            self.ch.set_transparency = self.transrate;
            f.stepped = true;
            f.drawn = self.ch.fade(view);
        } else {
            self.ch.drawn = false;
        }
        self.frame = f;
        f
    }

    /// `SetMatrix_PosRotZYX(pos, dirc)` as the anm stores it.
    pub fn root(&self) -> [V4; 4] {
        let d = self.ch.dirc;
        crate::town::pos_rot_zyx(self.ch.pos, [d[0], d[1], d[2]])
    }

    /// The last step's draw: `ccAnm::Draw` of the body on the character
    /// layer, lit by the draw environment's `lights`.
    pub fn draw(&self, layers: &mut Layers, to_screen: glam::Mat4, lights: &TownLights) {
        self.ch.draw(layers, to_screen, lights);
    }
}

impl Npc for Merchant {
    fn code(&self) -> i32 {
        self.id
    }

    fn flags(&self) -> u32 {
        self.flags
    }

    fn char(&self) -> &Char {
        &self.ch
    }

    fn char_mut(&mut self) -> &mut Char {
        &mut self.ch
    }

    /// `ccEntryObj::routine` then `ccMerchan::main`, its body against the
    /// character list as it stands (the merchants before it already moved
    /// this frame).
    fn step(&mut self, ctx: &mut NpcCtx) {
        self.entry.routine(&mut self.ch, ctx.player);
        self.main(ctx.player, ctx.cam, &ctx.view, ctx.hits);
    }

    fn listed(&self) -> bool {
        self.entry.listed
    }

    /// `affectFunc` (+0x94) as `ccChar::EntryAffect` calls it with
    /// `affectType` `cmd`: [`Merchant::breeder_influence`] for the ids that
    /// have it; Mac Anu's merchants keep `ccGimmickAffect`, which only sets
    /// `affectFlag`.
    fn influence(&mut self, cmd: i16) {
        if self.is_breeder() {
            self.breeder_influence(cmd);
        } else {
            self.affect_type = cmd;
            self.entry.affect = true;
        }
    }

    /// `npc_turn` and `npc_face` with a rate (`ccEvent::Execute`, main
    /// 0x001ae5d0 and 0x001ae684): `grotDeg` the heading (for a face,
    /// `RAD2DEG(ccGetDirc(pos, target))`), `grotSpd` the rate (1 means 64),
    /// which [`EntryObj::routine`] turns by until within 0.003. Events
    /// reach merchants only by codes 29 and 158; Mac Anu's shops never.
    /// `npc_act` (main 0x001acbc4) on those two: `actNum` the parameter,
    /// `actProcess` 0 ([`Merchant::sysope_act`]).
    fn command(&mut self, c: &piney_event::host::NpcCommand, _marker: Option<(V4, F)>, target: Option<V4>) -> bool {
        use piney_event::host::NpcCommand;
        let rate = |chg: i16| if chg == 1 { 64 } else { chg };
        match *c {
            NpcCommand::Act { param, .. } if SYSOPE.contains(&self.id) => {
                self.act_num = i32::from(param);
                self.act_process = 0;
                true
            }
            NpcCommand::Turn { dirc, chg, .. } if chg != 0 => {
                self.entry.grot_deg = dirc;
                self.entry.grot_spd = rate(chg);
                true
            }
            NpcCommand::Face { chg, .. } if chg != 0 => match target {
                Some(t) => {
                    self.entry.grot_deg = ee::rad2deg(get_dirc(self.ch.pos, t));
                    self.entry.grot_spd = rate(chg);
                    true
                }
                None => false,
            },
            _ => false,
        }
    }

    /// The shop menus' greeting: `ccMessage::Open(base->msg[game.server],
    /// base->name, -1, -1)` (`VenderMenu` 0x00542840 and its kin): the
    /// table and the server's index into it (0 for Mac Anu).
    fn talk_line(&self) -> Option<(u32, i32)> {
        Some((self.msg, 0))
    }
}

/// `ccSetMerchant(0)` (0x005057f0) in area 0 (a Root Town): town `town`'s
/// merchants from the `volume`'s `npcTbl`, each its row's file's clump, at the
/// dummies of `town_file`, on the ground of `hits`, Kite at `player`.
pub fn set_merchants(
    archive: &Arc<Archive>,
    volume: Volume,
    town_file: &SceneFile,
    hits: &mut Hits,
    town: i32,
    player: V4,
) -> Result<Vec<Merchant>> {
    let rows = TOWN_ROWS.get(town as usize).ok_or_else(|| Error::NotFound(format!("town {town}")))?.clone();
    let mut bodies: Vec<(String, Rc<Body>)> = Vec::new();
    let mut out = Vec::new();
    for r in rows {
        let row = NpcRow::of(volume, r)?;
        let stem = row.stem();
        let body = match bodies.iter().find(|(s, _)| *s == stem) {
            Some((_, b)) => b.clone(),
            None => {
                let b = Rc::new(Body::read(archive, &stem, TRALL)?);
                bodies.push((stem, b.clone()));
                b
            }
        };
        out.push(Merchant::new(&row, body, town_file, hits, town, player)?);
    }
    Ok(out)
}

/// One frame of `ccThEntryCtrl` for the merchants on its NPC list, in
/// order: each one's `CollisionDetection` meets the bodies on the character
/// list ([`Hits::chars`]: the other merchants as they stand, the earlier
/// ones already moved, then whoever was enabled after them - the walking
/// PCs, and Kite once his arrival ends).
pub fn step_all(merchants: &mut [Merchant], ctx: &mut NpcCtx) {
    for m in merchants {
        m.step(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummies_and_tables() {
        let names: Vec<String> = (0..5).map(|i| dummy_of(i).unwrap()).collect();
        assert_eq!(names, ["DMY_merchant1", "DMY_merchant4", "DMY_merchant3", "DMY_merchant2", "DMY_merchant5"]);
        assert_eq!(dummy_of(10).unwrap(), "DMY_merchant6");
        assert_eq!(dummy_of(28).unwrap(), "DMY_merchant6");
        assert_eq!(dummy_of(23).unwrap(), "DMY_merchant1");
        assert_eq!(dummy_of(29), None);
        assert_eq!(anm_tbl(4, 0).unwrap()[0], "ANM_ctr1nut0");
        assert_eq!(anm_tbl(24, 4).unwrap()[2], "ANM_ctr2act2");
        assert_eq!(anm_tbl(28, 4).unwrap()[0], "ANM_ctr4nut0");
        assert_eq!(anm_tbl(29, 2).unwrap()[2], "ANM_ctr1act2");
        assert_eq!(anm_tbl(158, 0).unwrap()[0], "ANM_ctr2nut0");
    }

    #[test]
    fn turning() {
        // A quarter turn at 64: a sixteenth of it (16384 * 16 / 64).
        assert_eq!(dirc_chg(0, 16384, 64), 4096);
        assert_eq!(dirc_chg(0, -16384, 64), -4096);
        // The shorter way round, and the cap.
        assert_eq!(dirc_chg(30000, -30000, 64), (65536 - 60000) * 16 / 64);
        assert_eq!(dirc_chg(0, 16384, 1), 24576);
        // Small steps: at least 1; mode 1 drops them, mode 2 snaps.
        assert_eq!(dirc_chg(0, 1, 128), 1);
        assert_eq!(dirc_chg(0, 10, 0x1_0000 | 64), 0);
        assert_eq!(dirc_chg(0, 10, 0x2_0000 | 64), 10);
        assert_eq!(dirc_chg(5, 5, 64), 0);
        // RAD2DEG truncates: 2048 comes back as 2047, and DEG2RAD of that
        // as 2046.
        let z = set_dirc(0, ee::deg2rad(8192), 64);
        assert_eq!(ee::rad2deg(z), 2046);
    }

    #[test]
    fn the_view_cone() {
        let cam = Cam { pos: [0, 0, 0, ONE], view: [0, ee::k(-100.0), 0, ONE], ..Cam::default() };
        let kite = [0, ee::k(-50.0), 0, ONE];
        let at = |x: f32, y: f32| check_camera_deg([ee::k(x), ee::k(y), 0, ONE], kite, &cam, VIEW_DEG);
        assert!(at(0.0, -1000.0));
        assert!(at(1000.0, -1000.0));
        assert!(at(-1000.0, -1000.0));
        assert!(!at(0.0, 1000.0));
        assert!(!at(1000.0, 100.0));
    }

    fn mac_anu() -> Option<(Vec<Merchant>, Hits)> {
        let archive = crate::town::tests::archive()?;
        let town = crate::town::Town::open(&archive, 0, false).unwrap();
        let mut hits = town.base.hits.clone();
        let m = set_merchants(&archive, Volume::Inf, &town.base.file, &mut hits, 0, crate::START_POS).unwrap();
        Some((m, hits))
    }

    /// `ccSetMerchant(0)` in Mac Anu: the five shops at their dummies, on
    /// the ground, facing out of their booths, idling.
    #[test]
    fn five_shops_in_their_booths() {
        let Some((m, _)) = mac_anu() else { return };
        let got: Vec<(&str, [f32; 3], i16)> = m
            .iter()
            .map(|m| (m.name.as_str(), [0, 1, 2].map(|i| ee::f(m.ch.pos[i])), ee::rad2deg(m.ch.dirc[2])))
            .collect();
        assert_eq!(
            got,
            [
                ("Weapon Shop", [2400.0, -2450.0, 0.0], -32767),
                ("Elf's Haven", [2400.0, 2450.0, 0.0], 0),
                ("Item Shop", [-2400.0, 2450.0, 0.0], 0),
                ("Magic Shop", [-2400.0, -2450.0, 0.0], -32767),
                ("Recorder", [-850.0, 3600.0, 300.0], 16383),
            ]
        );
        for x in &m {
            assert_eq!(x.ch.anim_name(), Some("ANM_ctr1nut0"));
            assert_eq!((ee::f(x.ch.height), ee::f(x.ch.width)), (180.0, 100.0));
            assert!(x.ch.clut_swaps.is_empty() && x.ch.hit.sw && !x.is_breeder());
            assert_eq!(x.talk_line().unwrap().1, 0);
        }
        assert_eq!(m[4].flags, 0x1000);
    }

    /// Near the Recorder with the camera behind Kite looking at it: it steps
    /// and draws; turned away, or 4600 off, it does neither; beyond 10000
    /// it leaves the command lists.
    #[test]
    fn near_and_in_view() {
        let Some((mut m, mut hits)) = mac_anu() else { return };
        let kite = [ee::k(-300.0), ee::k(3600.0), ee::k(300.0), ONE];
        let cam = Cam {
            pos: [ee::k(500.0), ee::k(3600.0), ee::k(600.0), ONE],
            view: [ee::k(-300.0), ee::k(3600.0), ee::k(420.0), ONE],
            deg: [0, 1512],
            kind: crate::camera::kind::FOLLOW,
            ..Cam::default()
        };
        let view = View { player: kite, cam: cam.pos, deg1: 1512, eye: false };
        let mut rand = crate::Rand(1);
        let mut ctx = NpcCtx { player: kite, player_dirc: [0; 4], view, cam: &cam, hits: &mut hits, rand: &mut rand };
        step_all(&mut m, &mut ctx);
        let r = &m[4];
        assert_eq!(r.frame, MerchantFrame { near: true, in_view: true, stepped: true, drawn: true });
        assert!(r.listed() && r.entry.disp_sw && !r.entry.freeze);
        assert_eq!(ee::f(r.entry.pl_dist), 550.0);
        assert_eq!(r.ch.play.time, 256);
        // The Weapon Shop is 6000 away.
        assert!(!m[0].frame.near && !m[0].frame.stepped);
        // Turned away.
        let back = Cam { view: [ee::k(1500.0), ee::k(3600.0), ee::k(420.0), ONE], ..cam };
        ctx.cam = &back;
        step_all(&mut m, &mut ctx);
        assert_eq!(m[4].frame, MerchantFrame { near: true, in_view: false, stepped: false, drawn: false });
        assert_eq!(m[4].ch.play.time, 256);
        // Far off: off the command lists.
        ctx.player = [ee::k(-850.0), ee::k(-7000.0), 0, ONE];
        step_all(&mut m, &mut ctx);
        assert!(!m[4].listed() && m[4].entry.freeze && !m[4].entry.disp_sw);
    }

    /// Kite walking into a merchant's body: `bodyHitFlag` and its count.
    #[test]
    fn touching_kite() {
        let Some((mut m, mut hits)) = mac_anu() else { return };
        let cam = Cam::default();
        let kite = [ee::k(-800.0), ee::k(3600.0), ee::k(300.0), ONE];
        let view = View { player: kite, cam: cam.pos, deg1: 1512, eye: false };
        let mut rand = crate::Rand(1);
        let mut body = hit::Body { pos: kite, radius: ee::k(45.0), ..hit::Body::default() };
        hits.hit_enable(&mut body);
        let mut ctx = NpcCtx { player: kite, player_dirc: [0; 4], view, cam: &cam, hits: &mut hits, rand: &mut rand };
        step_all(&mut m, &mut ctx);
        step_all(&mut m, &mut ctx);
        assert_eq!((m[4].body_hit_flag, m[4].body_hit_cnt), (1, 2));
        // 80 - 50 along -x (sceVu0Normalize rounds it short); w 1 from
        // ccModelHitCheckQ.
        assert_eq!(m[4].ch.hit.offset.map(ee::f), [-29.999998, 0.0, 0.0, 1.0]);
        assert_eq!((m[3].body_hit_flag, m[3].body_hit_cnt), (0, 0));
        hits.hit_disable(&mut body);
        let mut ctx = NpcCtx { player: kite, player_dirc: [0; 4], view, cam: &cam, hits: &mut hits, rand: &mut rand };
        step_all(&mut m, &mut ctx);
        assert_eq!((m[4].body_hit_flag, m[4].body_hit_cnt), (0, 0));
    }

    /// Mac Anu's shops keep `ccGimmickAffect`: talking to them only sets
    /// `affectFlag`. `breederInfluence` turns a merchant to Kite over the
    /// talk animation and back when the menu closes.
    #[test]
    fn influence_and_the_breeders_turn() {
        let Some((mut m, mut hits)) = mac_anu() else { return };
        let kite = [ee::k(-850.0), ee::k(3300.0), ee::k(300.0), ONE];
        let cam = Cam {
            pos: [ee::k(-850.0), ee::k(2900.0), ee::k(600.0), ONE],
            view: [ee::k(-850.0), ee::k(3600.0), ee::k(420.0), ONE],
            ..Cam::default()
        };
        let view = View { player: kite, cam: cam.pos, deg1: 1512, eye: false };
        let mut rand = crate::Rand(1);
        let mut ctx = NpcCtx { player: kite, player_dirc: [0; 4], view, cam: &cam, hits: &mut hits, rand: &mut rand };
        let r = &mut m[4];
        r.influence(14);
        assert!(r.entry.affect && r.act_num == 0);
        r.breeder_influence(14);
        let mut turned = Vec::new();
        for i in 0..130 {
            r.step(&mut ctx);
            turned.push(ee::rad2deg(r.ch.dirc[2]));
            if i == 60 {
                assert_eq!(r.ch.anim_name(), Some("ANM_ctr1act2"));
            }
        }
        // Kite is due south: heading 0. From 16383 a quarter of the way
        // (sixteenths / 64) each frame.
        assert_eq!(&turned[..3], &[12287, 9215, 6911]);
        // The talk ended at frame 120 and the idle came back.
        assert_eq!(r.ch.anim_name(), Some("ANM_ctr1nut0"));
        assert_eq!((r.act_num, r.act_process), (0, 1));
        assert!(turned[125].abs() < 4);
        r.breeder_influence(0);
        for _ in 0..100 {
            r.step(&mut ctx);
        }
        assert_eq!(r.ch.anim_name(), Some("ANM_ctr1nut0"));
        // Back toward the booth's 16383 an eighth at a time, until the steps
        // are one unit, which RAD2DEG's truncation takes back: 15 short.
        assert_eq!(ee::rad2deg(r.ch.dirc[2]), 16368);
    }
}
