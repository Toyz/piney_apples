//! The Root Towns' merchants: `ccMerchan` (gcmn merchan.cpp, 0x210 bytes, a
//! `ccGimmick` and so a `ccEntryObj`), the shopkeepers in their booths, placed
//! by `ccSetMerchant(0)` (0x005057f0) at the dummies of the jump table
//! 0x006ac890 and dropped on the floor. Each frame `ccMerchan::main`
//! (0x00505e20) steps and draws one only within 4600 of Kite and 67.5 degrees
//! of the camera's line of sight. Only the Grunt shops, the administrator and
//! the quiz man turn toward Kite (`breederInfluence`); Mac Anu's five never
//! turn (docs/engine/field-game.md, "The merchants").

use std::rc::Rc;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::camera::CamList;
use crate::chara::body::{Body, TRALL};
use crate::chara::char::{Char, check_camera_deg, w2p};
use crate::chara::npc::{Core, NpcRow};
use crate::ee::{self, F, ONE, V4};
use crate::hit::{self, Hits};
use crate::systems::npcs::NpcCtx;
use crate::town::TownLights;
use piney_battle::geom::{get_dirc, set_dirc};

/// `ccMerchan::main` steps and draws only nearer than this (4600).
pub const NEAR: F = 0x458f_c000;
/// `ccCheckCameraDeg(pos, 12288)`: 67.5 degrees either side.
pub const VIEW_DEG: i16 = 12288;
/// The merchant's `bodyHit`: radius 35, height 120; both masks all ones
/// (every character type; no hit polygon has every bit).
pub const BODY_RADIUS: F = 0x420c_0000;
pub const BODY_HEIGHT: F = 0x42f0_0000;
pub const BODY_MASK: u32 = 0xffff_ffff;

/// The merchant's body's type: 2 on Infection, 16 from Mutation on (MUT
/// gcmn 0x00520d50).
pub fn body_kind(volume: Volume) -> u32 {
    if volume == Volume::Inf { 2 } else { 16 }
}

/// From Mutation on, the "Event NPC" (`ITEMCOMPLETE`), row 175 + town,
/// which `ccSetMerchant(0)` makes while `saveData.eventStatus[52]`
/// (+0x652c) is set: event 317 ("ITEM COMPLETE") sets it once mail 328 is
/// read. Its action button opens menu 90.
pub const EVENT_NPC: std::ops::RangeInclusive<i32> = 175..=179;
pub const EVENT_NPC_STATUS: usize = 52;
/// The merchants' bodies' keys in [`Hits::chars`]: this plus the row (the
/// walking PCs' are [`crate::town::pc::BODY_ID`] on, Kite's
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
/// The ids `ccMerchan::main` runs `sysopeAct` for (the Administrator), and
/// the only ones `ccSetMerchant(code)` makes for an event's `entry 4`.
pub const SYSOPE: [i32; 2] = [29, 158];
/// `ccSetMerchant(0)`'s rows by town.
pub const TOWN_ROWS: [std::ops::RangeInclusive<usize>; 5] = [0..=4, 5..=10, 11..=16, 17..=22, 23..=28];

/// `setMerchant`'s jump table (0x006ac890): the `DMY_merchantN` of each id
/// below 29; from Mutation on (MUT gcmn 0x005207b4) the Event NPC's marker
/// dummies.
pub fn dummy_of(id: i32) -> Option<String> {
    const N: [u8; 6] = [1, 4, 3, 2, 5, 6];
    let k = match id {
        0..=4 => id as usize,
        5..=28 => ((id - 5) % 6) as usize,
        175 | 176 | 178 => return Some("DMY_marker_ev04".into()),
        177 => return Some("DMY_marker_ev01".into()),
        179 => return Some("DMY_marker_ev02".into()),
        _ => return None,
    };
    Some(format!("DMY_merchant{}", N[k]))
}

/// What `setMerchant` writes into the Event NPC's dummy before it reads
/// it (the town's dummy itself, for the rest of the visit): 177's turned
/// to `DEG2RAD(24576)`, 179's x and y 0 and z `DEG2RAD(-24576)`.
pub fn dummy_turn(id: i32, rot: V4) -> Option<V4> {
    match id {
        177 => Some([rot[0], rot[1], ee::deg2rad(24576), rot[3]]),
        179 => Some([0, 0, ee::deg2rad(-24576), rot[3]]),
        _ => None,
    }
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
        // From Mutation on, Lia Fail's Event NPC.
        (179, _) => Some(ANM_TBL[1]),
        _ => None,
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
    /// Its row (`npcID`, +0x1f0, the base parameters' name, type flags and
    /// message table: one `ccEvMsgData` list per server), character,
    /// `ccEntryObj`, acts (+0x1f4 `actNum`, +0x1f8 `actProcess`; `actCnt`
    /// is `ccGimmick`'s +0x1d8, the Administrator's count within an act)
    /// and body touch (+0x200 `bodyHitFlag`, +0x202 `bodyHitCnt`).
    pub core: Core,
    /// +0x1e0 `defaultDirc`: the dummy's rotation.
    pub default_dirc: V4,
    /// +0x1fc `anmOld`.
    pub anm_old: i32,
    /// +0x204 `anmTbl`.
    pub anm_tbl: [&'static str; 3],
    /// +0x208 `transrate`: what `main` sets the transparency to (1).
    pub transrate: F,
    /// The last frame's.
    pub frame: MerchantFrame,
    /// What `sysopeAct` started since the town last took it.
    pub sysop_events: Vec<SysopEvent>,
    /// `sysopeAct` is done with it (act 5's end: `main` returns 1 and the
    /// entry control deletes it).
    pub gone: bool,
    /// For the Event NPC: `saveData.eventStatus[52]` as the frame starts
    /// (the world's); once clear it goes.
    pub status_on: bool,
}

impl Merchant {
    /// `setMerchant(row.id)` in town `town`, through `entryObject`,
    /// `ccMerchan::ccMerchan` and `initObject`: at its dummy in the town's
    /// file `town_file` (as `written` has left the dummies' rotations, and
    /// with the Event NPC's own turn written there), set on the ground of
    /// `hits` twice, with `body` (the row's file's `CMP_trall`) playing
    /// anmTbl[0], Kite at `player`.
    pub fn new(
        row: &NpcRow,
        body: Rc<Body>,
        town_file: &SceneFile,
        written: &mut Vec<(String, V4)>,
        hits: &mut Hits,
        town: i32,
        player: V4,
    ) -> Result<Merchant> {
        let id = i32::from(row.id);
        let name = dummy_of(id).ok_or_else(|| Error::NotFound(format!("merchant {id}: no dummy")))?;
        let (pos, mut rot) = crate::event::dummy_in(town_file, &name).ok_or_else(|| Error::NotFound(name.clone()))?;
        if let Some(&(_, r)) = written.iter().find(|(n, _)| *n == name) {
            rot = r;
        }
        if let Some(r) = dummy_turn(id, rot) {
            rot = r;
            written.retain(|(n, _)| *n != name);
            written.push((name, r));
        }
        Merchant::at(row, body, pos, rot, hits, town, player)
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
            kind: body_kind(hits.volume),
            id: BODY_ID + id as u32,
            ..hit::Body::default()
        };
        // initObject: SetHitSW(1) (onto the character list's tail), the
        // ground again, posP.
        hits.hit_enable(&mut ch.hit);
        pos[2] = hits.land(pos, ENTRY_LAND_MASK);
        ch.pos = pos;
        ch.pos_p = w2p(pos, player);
        let mut core = Core::new(row.clone(), ch);
        core.entry.obj_flag = true;
        Ok(Merchant {
            core,
            default_dirc: dirc,
            anm_old: 0,
            anm_tbl,
            transrate: ONE,
            frame: MerchantFrame::default(),
            sysop_events: Vec::new(),
            gone: false,
            status_on: true,
        })
    }

    /// Whether `affectFunc` is `breederInfluence`.
    pub fn is_breeder(&self) -> bool {
        INFLUENCED.contains(&self.core.code())
    }

    /// `ccAnm::SetAnm(anmTbl[k])`.
    fn set_anm(&mut self, k: usize) {
        self.core.ch.set_anim(self.anm_tbl[k]);
    }

    /// `ccMerchan::sysopeAct` (gcmn 0x00506170): the Administrator's acts, which
    /// the events' `npc_act 29 N` sets (`actNum` N, `actProcess` 0): shown,
    /// transferring in (3) or out (4, 5), hidden (6), turning toward Kite (-3),
    /// the force rings (-5); the table is in docs/engine/field-game.md. True when
    /// it is done with (act 5's end). `transrate` is what `main` draws him at.
    pub fn sysope_act(&mut self) -> bool {
        // 0.02.
        const STEP: F = 0x3ca3_d70a;
        match self.core.acts.num {
            -5 => match self.core.acts.process {
                0 => {
                    self.transrate = ONE;
                    self.set_anm(2);
                    self.core.ch.play.frame_spd = 384;
                    self.sysop_events.push(SysopEvent::Vanish);
                    self.core.acts.process += 1;
                    self.core.acts.cnt = 0;
                }
                1 => {
                    let c = self.core.acts.cnt;
                    self.core.acts.cnt += 1;
                    if c >= 31 {
                        self.sysop_events.push(SysopEvent::Noise);
                        self.core.acts.process += 1;
                        self.core.acts.cnt = 0;
                    }
                }
                _ => {}
            },
            3 => match self.core.acts.process {
                0 => {
                    self.set_anm(0);
                    self.core.acts.process += 1;
                    self.transrate = 0;
                }
                1 => {
                    self.sysop_events.push(SysopEvent::Transfer);
                    self.core.acts.process += 1;
                }
                2 => {
                    let t = ee::add(self.transrate, STEP);
                    self.transrate = t;
                    if !ee::le(t, ONE) {
                        self.transrate = ONE;
                        self.core.acts.process += 1;
                        self.core.acts.cnt = 0;
                        self.core.acts.num = 2;
                    }
                }
                _ => {}
            },
            7 => {
                self.transrate = ONE;
                self.core.acts.num = 2;
            }
            6 => {
                if self.core.acts.process == 0 {
                    self.transrate = 0;
                }
            }
            4 | 5 => match self.core.acts.process {
                0 => {
                    self.transrate = ONE;
                    // deleteCmnd(this).
                    self.core.entry.listed = false;
                    self.sysop_events.push(SysopEvent::Transfer);
                    self.core.acts.process += 1;
                    self.core.acts.cnt = 0;
                }
                1 => {
                    let t = ee::sub(self.transrate, STEP);
                    self.transrate = if ee::lt(t, 0) { 0 } else { t };
                    let c = self.core.acts.cnt;
                    self.core.acts.cnt += 1;
                    if c >= 141 {
                        if self.core.acts.num == 5 {
                            return true;
                        }
                        self.core.acts.process += 1;
                    }
                }
                _ => {}
            },
            -3 => {
                if self.core.acts.process == 0 {
                    self.set_anm(2);
                    self.core.acts.process += 1;
                }
                self.core.ch.dirc[2] = set_dirc(self.core.ch.dirc[2], self.core.entry.pl_dirc, 64);
                if self.core.ch.play.time >> 8 >= 120 {
                    self.core.acts.process = 0;
                    self.core.acts.num = 0;
                }
            }
            _ => {
                if self.core.acts.process == 0 {
                    self.transrate = ONE;
                    self.set_anm(0);
                    self.core.acts.process += 1;
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
        match self.core.acts.num {
            0 => {
                if self.core.acts.process == 0 {
                    if self.anm_old != 0 {
                        self.set_anm(0);
                    }
                    self.anm_old = 0;
                    self.core.acts.process += 1;
                }
            }
            1 => {
                if self.core.acts.process == 0 {
                    if self.anm_old != 2 {
                        self.set_anm(2);
                    }
                    self.anm_old = 2;
                    self.core.acts.process += 1;
                }
                self.core.ch.dirc[2] = set_dirc(self.core.ch.dirc[2], self.core.entry.pl_dirc, 64);
                if self.core.ch.play.time >> 8 >= TALK_FRAMES {
                    self.core.acts.process = 0;
                    self.core.acts.num = 0;
                }
            }
            2 => {
                if self.core.acts.process == 0 {
                    if self.anm_old != 0 {
                        self.set_anm(0);
                    }
                    self.anm_old = 0;
                    self.core.acts.process += 1;
                }
                self.core.ch.dirc[2] = set_dirc(self.core.ch.dirc[2], self.default_dirc[2], 128);
            }
            _ => {}
        }
    }

    /// From Mutation on, the Event NPC's act (MUT gcmn 0x00521a00): the
    /// breeders' acts 0-2 (idle, facing Kite, facing its way again), and
    /// once `eventStatus[52]` is clear act 3, going: idle, `effTransfer`
    /// and off the command lists, fading out by 0.02. True when gone.
    pub fn event_npc_act(&mut self) -> bool {
        // 0.02.
        const STEP: F = 0x3ca3_d70a;
        if !self.status_on && self.core.acts.num != 3 {
            self.core.acts.num = 3;
            self.core.acts.process = 0;
        }
        if self.core.acts.num != 3 {
            self.breeder_act();
            return false;
        }
        match self.core.acts.process {
            0 => {
                self.core.acts.process = 1;
                self.transrate = ONE;
                if self.anm_old != 0 {
                    self.set_anm(0);
                }
                self.anm_old = 0;
            }
            1 => {
                self.sysop_events.push(SysopEvent::Transfer);
                self.core.acts.process += 1;
                self.core.entry.delete_cmnd(true);
            }
            2 => {
                let t = ee::sub(self.transrate, STEP);
                self.transrate = t;
                if ee::lt(t, 0) {
                    self.transrate = 0;
                    self.core.acts.process += 1;
                    self.core.acts.cnt = 0;
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    /// From Mutation on, the Event NPC's `affectFunc` (MUT gcmn
    /// 0x00521990): 15 (Talk) faces Kite from the talk's start, 14 (its
    /// list opening) only turns it, 0 (closing) faces its way again.
    pub fn event_npc_influence(&mut self, cmd: i16) {
        self.core.affect_type = cmd;
        match cmd {
            15 => (self.core.acts.num, self.core.acts.process) = (1, 0),
            14 => (self.core.acts.num, self.core.acts.process) = (1, 1),
            0 => (self.core.acts.num, self.core.acts.process) = (2, 0),
            _ => {}
        }
    }

    /// `breederInfluence` (0x005065d0) of the menu's command: 14 (the shop
    /// opens) or 15, act 1 (face Kite); 0 (it closes), act 2 (face the booth
    /// again); others nothing.
    pub fn breeder_influence(&mut self, cmd: i16) {
        self.core.affect_type = cmd;
        match cmd {
            14 | 15 => (self.core.acts.num, self.core.acts.process) = (1, 0),
            0 => (self.core.acts.num, self.core.acts.process) = (2, 0),
            _ => {}
        }
    }

    /// `ccMerchan::main` (0x00505e20) after `routine`, with Kite at
    /// `player`, camera.cpp's list `cams` (the active camera's view cone
    /// and fade), the town's `hits`.
    pub fn main(&mut self, player: V4, cams: &CamList, hits: &mut Hits) -> MerchantFrame {
        self.core.ch.pos_p = w2p(self.core.ch.pos, player);
        let p = self.core.ch.pos_p;
        // x x + y y (adda), + z z (madd); fptodp, sqrt, dptofp (sqrt.s
        // from Outbreak on, OUT gcmn 0x0051beec).
        let d2 = ee::add(ee::add(ee::mul(p[0], p[0]), ee::mul(p[1], p[1])), ee::mul(p[2], p[2]));
        let dist = ee::dsqrt_on(hits.volume, d2);
        let in_view = check_camera_deg(self.core.ch.pos, VIEW_DEG, cams.active(), player);
        // Infection's acts and collides for every one; from Mutation on
        // (MUT gcmn 0x00521134) only the Administrator, the quiz man, the
        // Grunt Shops and the Event NPC do.
        let event_npc = hits.volume != Volume::Inf && EVENT_NPC.contains(&self.core.code());
        let acts = SYSOPE.contains(&self.core.code())
            || hits.volume == Volume::Inf
            || INFLUENCED.contains(&self.core.code())
            || event_npc;
        let gone = if SYSOPE.contains(&self.core.code()) {
            self.sysope_act()
        } else if event_npc {
            self.event_npc_act()
        } else {
            if acts {
                self.breeder_act();
            }
            false
        };
        if gone {
            // main returns 1 at once: the object is deleted.
            self.gone = true;
            return MerchantFrame::default();
        }
        if acts {
            self.core.touch.collide(&mut self.core.ch, hits, false);
        }
        let near = ee::lt(dist, NEAR);
        let mut f = MerchantFrame { near, in_view, ..MerchantFrame::default() };
        if near && in_view {
            self.core.ch.forward();
            self.core.ch.transparency = self.transrate;
            self.core.ch.set_transparency = self.transrate;
            f.stepped = true;
            f.drawn = self.core.ch.fade(cams, player);
        } else {
            self.core.ch.drawn = false;
        }
        self.frame = f;
        f
    }

    /// `SetMatrix_PosRotZYX(pos, dirc)` as the anm stores it.
    pub fn root(&self) -> [V4; 4] {
        let d = self.core.ch.dirc;
        crate::town::pos_rot_zyx(self.core.ch.pos, [d[0], d[1], d[2]])
    }

    /// The last step's draw: `ccAnm::Draw` of the body on the character
    /// layer, lit by the draw environment's `lights`.
    pub fn draw(&self, layers: &mut Layers, to_screen: glam::Mat4, lights: &TownLights) {
        self.core.ch.draw(layers, to_screen, lights);
    }
}

impl Merchant {
    /// `ccEntryObj::routine` then `ccMerchan::main`, its body against the
    /// character list as it stands (the merchants before it already moved
    /// this frame).
    pub fn step(&mut self, ctx: &mut NpcCtx) {
        self.core.entry.routine(ctx.hits.volume, &mut self.core.ch, ctx.player);
        self.main(ctx.player, ctx.cams, ctx.hits);
        // main returned 1: the entry control deletes it at once, and
        // `~ccCharHit` takes its body off the character list.
        if self.gone {
            ctx.hits.hit_disable(&mut self.core.ch.hit);
        }
    }

    /// `affectFunc` (+0x94) as `ccChar::EntryAffect` calls it with
    /// `affectType` `cmd`: [`Merchant::breeder_influence`] for the ids that
    /// have it; Mac Anu's merchants keep `ccGimmickAffect`, which only sets
    /// `affectFlag`.
    pub fn influence(&mut self, cmd: i16) {
        if self.is_breeder() {
            self.breeder_influence(cmd);
        } else if EVENT_NPC.contains(&self.core.code()) {
            self.event_npc_influence(cmd);
        } else {
            self.core.affect_type = cmd;
            self.core.entry.affect = true;
        }
    }

    /// `npc_turn` and `npc_face` with a rate (`ccEvent::Execute`, main
    /// 0x001ae5d0 and 0x001ae684): `grotDeg` the heading (for a face,
    /// `RAD2DEG(ccGetDirc(pos, target))`), `grotSpd` the rate (1 means 64),
    /// which [`EntryObj::routine`] turns by until within 0.003. Events
    /// reach merchants only by codes 29 and 158; Mac Anu's shops never.
    /// `npc_act` (main 0x001acbc4) on those two: `actNum` the parameter,
    /// `actProcess` 0 ([`Merchant::sysope_act`]).
    pub fn command(&mut self, c: &piney_event::host::NpcCommand, _marker: Option<(V4, F)>, target: Option<V4>) -> bool {
        use piney_event::host::NpcCommand;
        let rate = |chg: i16| if chg == 1 { 64 } else { chg };
        match *c {
            NpcCommand::Act { param, .. } if SYSOPE.contains(&self.core.code()) => {
                self.core.acts.num = i32::from(param);
                self.core.acts.process = 0;
                true
            }
            NpcCommand::Turn { dirc, chg, .. } if chg != 0 => {
                self.core.entry.grot_deg = dirc;
                self.core.entry.grot_spd = rate(chg);
                true
            }
            NpcCommand::Face { chg, .. } if chg != 0 => match target {
                Some(t) => {
                    self.core.entry.grot_deg = ee::rad2deg(get_dirc(self.core.ch.pos, t));
                    self.core.entry.grot_spd = rate(chg);
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
    pub fn talk_line(&self) -> Option<(u32, i32)> {
        Some((self.core.row.msg, 0))
    }
}

/// `ccSetMerchant(0)` (0x005057f0; MUT 0x00520910) in area 0 (a Root
/// Town): town `town`'s merchants from the `volume`'s `npcTbl`, each its
/// row's file's clump, at the dummies of `town_file`, on the ground of
/// `hits`, Kite at `player`; from Mutation on, with `event_npc`
/// (`eventStatus[52]`), the Event NPC after them. `written` gathers the
/// dummies whose rotation `setMerchant` wrote.
#[allow(clippy::too_many_arguments)]
pub fn set_merchants(
    archive: &Arc<Archive>,
    volume: Volume,
    town_file: &SceneFile,
    written: &mut Vec<(String, V4)>,
    hits: &mut Hits,
    town: i32,
    player: V4,
    event_npc: bool,
) -> Result<Vec<Merchant>> {
    let rows = TOWN_ROWS.get(town as usize).ok_or_else(|| Error::NotFound(format!("town {town}")))?.clone();
    let event_row = (volume != Volume::Inf && event_npc).then(|| *EVENT_NPC.start() as usize + town as usize);
    let mut bodies: Vec<(String, Rc<Body>)> = Vec::new();
    let mut out = Vec::new();
    for r in rows.chain(event_row) {
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
        out.push(Merchant::new(&row, body, town_file, written, hits, town, player)?);
    }
    Ok(out)
}

/// An event's merchant in town `town` (`entry 4 29`, or `entry 3` of a
/// merchant's row): `ccSetMerchant(code)` or `ccSetRtownPC(code, -1)` makes
/// it at the origin (`setMerchant` gives rows 29 and 158 no dummy), then
/// `ccEntryEventMng` writes its marker's position and rotation `at`
/// ([`crate::World::event_marker`]) into its `pos` and `dirc` alone.
pub fn event_merchant(
    archive: &Arc<Archive>,
    row: &NpcRow,
    hits: &mut Hits,
    town: i32,
    player: V4,
    at: Option<(V4, V4)>,
) -> Result<Merchant> {
    let body = Rc::new(Body::read(archive, &row.stem(), TRALL)?);
    let mut m = Merchant::at(row, body, ee::VF0, ee::VF0, hits, town, player)?;
    if let Some((pos, dirc)) = at {
        (m.core.ch.pos, m.core.ch.dirc) = (pos, dirc);
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Cam;

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
    fn the_view_cone() {
        let cam = Cam { pos: [0, 0, 0, ONE], view: [0, ee::k(-100.0), 0, ONE], ..Cam::default() };
        let kite = [0, ee::k(-50.0), 0, ONE];
        let at = |x: f32, y: f32| check_camera_deg([ee::k(x), ee::k(y), 0, ONE], VIEW_DEG, &cam, kite);
        assert!(at(0.0, -1000.0));
        assert!(at(1000.0, -1000.0));
        assert!(at(-1000.0, -1000.0));
        assert!(!at(0.0, 1000.0));
        assert!(!at(1000.0, 100.0));
    }

    fn mac_anu() -> Option<(Vec<Merchant>, Hits)> {
        let archive = crate::town::tests::archive()?;
        let town = crate::town::Town::open(&archive, piney_data::volume::Volume::Inf, 0, false).unwrap();
        let mut hits = town.core.hits.clone();
        let m = set_merchants(
            &archive,
            Volume::Inf,
            &town.core.file,
            &mut Vec::new(),
            &mut hits,
            0,
            crate::START_POS,
            false,
        )
        .unwrap();
        Some((m, hits))
    }

    /// `ccSetMerchant(0)` in Mac Anu: the five shops at their dummies, on
    /// the ground, facing out of their booths, idling.
    #[test]
    fn five_shops_in_their_booths() {
        let Some((m, _)) = mac_anu() else { return };
        let got: Vec<(&str, [f32; 3], i16)> = m
            .iter()
            .map(|m| {
                (m.core.row.name.as_str(), [0, 1, 2].map(|i| ee::f(m.core.ch.pos[i])), ee::rad2deg(m.core.ch.dirc[2]))
            })
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
            assert_eq!(x.core.ch.anim_name(), Some("ANM_ctr1nut0"));
            assert_eq!((ee::f(x.core.ch.height), ee::f(x.core.ch.width)), (180.0, 100.0));
            assert!(x.core.ch.clut_swaps.is_empty() && x.core.ch.hit.sw && !x.is_breeder());
            assert_eq!(x.talk_line().unwrap().1, 0);
        }
        assert_eq!(m[4].core.row.flags, 0x1000);
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
        let mut cams = CamList::new(Volume::Inf, cam);
        let mut rand = crate::Rand(1);
        let mut cc = piney_battle::rand::Genrand::default();
        let mut save = piney_data::save::SaveData::default();
        let mut ctx = NpcCtx {
            player: kite,
            player_dirc: [0; 4],
            cams: &mut cams,
            hits: &mut hits,
            rand: &mut rand,
            cc: &mut cc,
            save: &mut save,
            race: None,
        };
        for x in &mut m {
            x.step(&mut ctx);
        }
        let r = &m[4];
        assert_eq!(r.frame, MerchantFrame { near: true, in_view: true, stepped: true, drawn: true });
        assert!(r.core.entry.listed && r.core.entry.disp_sw && !r.core.entry.freeze);
        assert_eq!(ee::f(r.core.entry.pl_dist), 550.0);
        assert_eq!(r.core.ch.play.time, 256);
        // The Weapon Shop is 6000 away.
        assert!(!m[0].frame.near && !m[0].frame.stepped);
        // Turned away.
        ctx.cams.active_mut().view = [ee::k(1500.0), ee::k(3600.0), ee::k(420.0), ONE];
        for x in &mut m {
            x.step(&mut ctx);
        }
        assert_eq!(m[4].frame, MerchantFrame { near: true, in_view: false, stepped: false, drawn: false });
        assert_eq!(m[4].core.ch.play.time, 256);
        // Far off: off the command lists.
        ctx.player = [ee::k(-850.0), ee::k(-7000.0), 0, ONE];
        for x in &mut m {
            x.step(&mut ctx);
        }
        assert!(!m[4].core.entry.listed && m[4].core.entry.freeze && !m[4].core.entry.disp_sw);
    }

    /// Kite walking into a merchant's body: `bodyHitFlag` and its count.
    #[test]
    fn touching_kite() {
        let Some((mut m, mut hits)) = mac_anu() else { return };
        let cam = Cam { deg: [0, 1512], ..Cam::default() };
        let kite = [ee::k(-800.0), ee::k(3600.0), ee::k(300.0), ONE];
        let mut cams = CamList::new(Volume::Inf, cam);
        let mut rand = crate::Rand(1);
        let mut cc = piney_battle::rand::Genrand::default();
        let mut save = piney_data::save::SaveData::default();
        let mut body = hit::Body { pos: kite, radius: ee::k(45.0), ..hit::Body::default() };
        hits.hit_enable(&mut body);
        let mut ctx = NpcCtx {
            player: kite,
            player_dirc: [0; 4],
            cams: &mut cams,
            hits: &mut hits,
            rand: &mut rand,
            cc: &mut cc,
            save: &mut save,
            race: None,
        };
        for x in &mut m {
            x.step(&mut ctx);
        }
        for x in &mut m {
            x.step(&mut ctx);
        }
        assert_eq!((m[4].core.touch.flag, m[4].core.touch.cnt), (1, 2));
        // 80 - 50 along -x (sceVu0Normalize rounds it short); w 1 from
        // ccModelHitCheckQ.
        assert_eq!(m[4].core.ch.hit.offset.map(ee::f), [-29.999998, 0.0, 0.0, 1.0]);
        assert_eq!((m[3].core.touch.flag, m[3].core.touch.cnt), (0, 0));
        hits.hit_disable(&mut body);
        let mut ctx = NpcCtx {
            player: kite,
            player_dirc: [0; 4],
            cams: &mut cams,
            hits: &mut hits,
            rand: &mut rand,
            cc: &mut cc,
            save: &mut save,
            race: None,
        };
        for x in &mut m {
            x.step(&mut ctx);
        }
        assert_eq!((m[4].core.touch.flag, m[4].core.touch.cnt), (0, 0));
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
            deg: [0, 1512],
            ..Cam::default()
        };
        let mut cams = CamList::new(Volume::Inf, cam);
        let mut rand = crate::Rand(1);
        let mut cc = piney_battle::rand::Genrand::default();
        let mut save = piney_data::save::SaveData::default();
        let mut ctx = NpcCtx {
            player: kite,
            player_dirc: [0; 4],
            cams: &mut cams,
            hits: &mut hits,
            rand: &mut rand,
            cc: &mut cc,
            save: &mut save,
            race: None,
        };
        let r = &mut m[4];
        r.influence(14);
        assert!(r.core.entry.affect && r.core.acts.num == 0);
        r.breeder_influence(14);
        let mut turned = Vec::new();
        for i in 0..130 {
            r.step(&mut ctx);
            turned.push(ee::rad2deg(r.core.ch.dirc[2]));
            if i == 60 {
                assert_eq!(r.core.ch.anim_name(), Some("ANM_ctr1act2"));
            }
        }
        // Kite is due south: heading 0. From 16383 a quarter of the way
        // (sixteenths / 64) each frame.
        assert_eq!(&turned[..3], &[12287, 9215, 6911]);
        // The talk ended at frame 120 and the idle came back.
        assert_eq!(r.core.ch.anim_name(), Some("ANM_ctr1nut0"));
        assert_eq!((r.core.acts.num, r.core.acts.process), (0, 1));
        assert!(turned[125].abs() < 4);
        r.breeder_influence(0);
        for _ in 0..100 {
            r.step(&mut ctx);
        }
        assert_eq!(r.core.ch.anim_name(), Some("ANM_ctr1nut0"));
        // Back toward the booth's 16383 an eighth at a time, until the steps
        // are one unit, which RAD2DEG's truncation takes back: 15 short.
        assert_eq!(ee::rad2deg(r.core.ch.dirc[2]), 16368);
    }
}
