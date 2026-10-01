//! `ccPlayer` (gcmn `player.cpp`, 0x005977d0-0x0059ce00): Kite, moved by the
//! pad as `ccPlayer::Main` (0x00598310) moves him once a frame on
//! `ccThPlayer` (priority 49): `ControlMove`, `HitCheck` along the walls,
//! `CollisionTest` onto the floor, `MapLoopAdjustPos`, the camera, `AnimCtrl`
//! (motion.rs), the matrix and the draw. Only what a player reaches is
//! ported: the AI only in manual mode, no conditions, skills, targets or gate
//! hacking. The area's [`Hits`] decide the ground, the map's wrap, the
//! camera's floor rule and the stopping act (docs/engine/field-game.md).

use piney_data::anim::Animation;

use crate::ai::{self, Ai, SpcRef};
use crate::camera::{CamPad, Camera, kind};
use crate::ee::{self, F, ONE, V4, add, div, mul};
use crate::hit::{self, Hits};
use crate::motion::{ActEvent, ActInput, Acts, anim_ctrl};

/// `tsp` (`INF SLUS_202.67:0x003782b0`): the walking speed, 3.1 units a
/// frame at full lean.
pub const TSP: F = 0x4046_6666;
/// 255.0.
const K255: F = 0x437f_0000;
/// Leaning past this runs (`c.le.s power, 230.0`).
const RUN_POWER: F = 0x4366_0000;
/// Walking's `speedRate = power / 140`, at most 1.3.
const WALK_DIV: F = 0x430c_0000;
const WALK_MAX: F = 0x3fa6_6666;
/// The camera looks at a point this far above his feet (`CameraPosCalc`).
pub const HEAD: F = 0x430c_0000; // 140.0
/// `ccGetCameraTransparency` as `ccChar::Draw` asks it: in a town he fades
/// out from 4000 away over 400 more, in a field or dungeon from 7000 over
/// 600 (never, in practice, for him), and near the camera.
const FADE_TOWN: (F, F) = (0x457a_0000, 0x43c8_0000);
const FADE_AREA: (F, F) = (0x45da_c000, 0x4416_0000);
/// Below this transparency he is not drawn.
const MIN_DRAWN: F = 0x3d4c_cccd; // 0.05

/// What `ControlMove` reads of the rest of the game.
#[derive(Clone, Copy, Debug, Default)]
pub struct MoveInput {
    /// `ccSys->pad[0].powL` and `.dircL`.
    pub pow_l: u8,
    pub dirc_l: F,
    /// `cmndTarget != 0`: something to talk to or examine is targeted.
    pub cmnd_target: bool,
    /// `activeCamPtr->resetDirc` (the flag is passed on its own: leaning
    /// against it clears it).
    pub cam_reset_dirc: F,
    /// z of `cameraGetRot(camID)`: the camera's heading (`tcam.rot.z` for
    /// camera 1, else from the active camera's eye and target).
    pub cam_rot_z: F,
    /// `checkCameraType()`: `activeCamPtr->type`.
    pub cam_type: i32,
}

/// The members of `ccPlayer` the movement uses (offsets in the class).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Body {
    /// +0x40 `pos`.
    pub pos: V4,
    /// +0x60 `dirc`: z is the heading, radians; 0 faces -y.
    pub dirc: V4,
    /// +0x28 `condition.speedValue` (1.0).
    pub speed_value: F,
    /// +0x104 `speed`: `personality->velocity`, the running speed.
    pub speed: F,
    /// +0x108 `speedRate`, +0x10c `nowSpeed`.
    pub speed_rate: F,
    pub now_speed: F,
    /// +0xe0 bit 3 `moveFlag`, bit 5 `runFlag`.
    pub move_flag: bool,
    pub run_flag: bool,
    /// +0xe0 bits 0 `pauseSW` and 2 `restraintSW` hold him still.
    pub pause: bool,
    pub restraint: bool,
    /// +0x200 bit 1 `inactiveSW`.
    pub inactive: bool,
    /// +0x7c `skillID`, +0xfe `attack`.
    pub skill_id: i16,
    pub attack: i16,
    /// +0x2e4 `targetCount`: frames the stick has been leant (to 4).
    pub target_count: i16,
    /// +0x290 `movePos`.
    pub move_pos: V4,
}

/// `ccPlayer::PadLeverPower(p)` (0x005992b0): the stick's strength as the
/// player moves by it. Below 64 nothing; 64-127 eased to 32-126; from 128
/// as it is. The first four frames of a lean give nothing while a target
/// is up.
pub fn pad_lever_power(body: &mut Body, p: F, cmnd_target: bool) -> F {
    let v = ee::to_int(p) as i16;
    let r: i32 = if v < 64 {
        body.target_count = 0;
        0
    } else {
        let mut r = if v < 128 { (i32::from(v) - 64) * 3 / 2 + 32 } else { i32::from(v) };
        if body.target_count < 4 {
            body.target_count += 1;
            if cmnd_target {
                r = 0;
            }
        }
        r
    };
    ee::from_int(r)
}

/// `ccPlayer::ControlMove` (0x00598af0) for `ctrlType` 0 with no condition
/// holding him: the heading and `movePos` from the left stick relative to
/// the camera. `cam_reset` is `activeCamPtr->resetFlag`, which leaning
/// away from the reset's direction clears. Returns the heading as a 16-bit
/// angle.
pub fn control_move(body: &mut Body, input: &MoveInput, cam_reset: &mut bool) -> i16 {
    // RAD2DEG(pi + heading) - 32768, kept as a short: the heading itself.
    let mut s0 = (i32::from(ee::rad2deg(add(ee::PI, body.dirc[2]))) - 32768) as i16;
    let mut power = pad_lever_power(body, ee::from_int(i32::from(input.pow_l)), input.cmnd_target);
    if body.inactive {
        power = 0;
    }
    body.now_speed = 0;
    body.speed_rate = div(power, K255);
    let held = body.skill_id >= 2
        || (body.skill_id == 1 && body.attack == 1)
        || (body.skill_id == 0 && (body.pause || body.restraint));
    if !held {
        if ee::eq(power, 0) {
            body.move_flag = false;
        } else {
            body.move_flag = true;
            body.run_flag = !ee::le(power, RUN_POWER);
            let mut dirc = input.dirc_l;
            if *cam_reset {
                let d = ee::rad2deg(input.cam_reset_dirc).wrapping_sub(ee::rad2deg(dirc));
                if (-2047..2048).contains(&d) {
                    dirc = ee::PI;
                } else {
                    *cam_reset = false;
                }
            }
            let s = ee::rad2deg(add(ee::PI, dirc));
            s0 = ee::rad2deg(input.cam_rot_z).wrapping_sub(s);
            let heading = ee::deg2rad(s0);
            let speed = if !body.run_flag {
                let mut rate = div(power, WALK_DIV);
                if !ee::le(rate, WALK_MAX) {
                    rate = WALK_MAX;
                }
                body.speed_rate = rate;
                mul(rate, mul(body.speed_value, TSP))
            } else {
                mul(body.speed_rate, mul(body.speed_value, body.speed))
            };
            body.now_speed = speed;
            body.move_pos[0] = mul(speed, ee::sinf(heading));
            body.move_pos[1] = mul(ee::neg(speed), ee::cosf(heading));
        }
    }
    if input.cam_type != 1 && !ee::eq(power, 0) {
        body.dirc[2] = ee::deg2rad(s0);
    }
    s0
}

/// `ccGetCameraTransparency(pos, width, height, far, fadeLen, &hide)`
/// (`INF SLUS_202.67:0x001da8b0`) for a camera at `cam` and its pitch
/// `deg1`: 1 in the open; toward 0 as the camera closes in within about
/// 1.25 times his size (at least 180), and beyond `fade.0` over `fade.1`.
#[allow(clippy::too_many_arguments)]
pub fn camera_transparency(
    volume: piney_data::volume::Volume,
    pos: V4,
    cam: V4,
    deg1: i16,
    width: F,
    height: F,
    fade: (F, F),
    hide: &mut bool,
) -> F {
    let (far, len) = fade;
    // ccTransPosW2P of both: relative to the player, so pos - pos is zero.
    let p = [0, 0, pos[2], ONE];
    let c = [ee::sub(cam[0], pos[0]), ee::sub(cam[1], pos[1]), cam[2], ONE];
    let d = ee::vsub(c, p);
    let dist = ee::sqrtf_on(volume, ee::dot(d, d));
    let t = div(ee::from_int(8192 - i32::from(deg1)), 0x4600_0000);
    let u = ee::sub(ONE, t);
    let mut r = add(mul(width, t), mul(height, u));
    if ee::lt(r, 0x4334_0000) {
        r = 0x4334_0000;
    }
    let r2 = mul(0x3fa0_0000, r);
    if !*hide && ee::lt(dist, r2) {
        *hide = true;
        let near = mul(0x3f4c_cccd, r);
        let f = ee::sub(dist, near);
        return if ee::le(f, 0) { 0 } else { div(f, ee::sub(r2, near)) };
    }
    *hide = false;
    if ee::le(far, 0) || ee::le(dist, far) {
        return ONE;
    }
    let over = ee::sub(dist, far);
    if ee::le(over, len) { div(ee::sub(len, over), len) } else { 0 }
}

/// Kite.
#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    pub body: Body,
    pub acts: Acts,
    /// +0x1a0 `bodyHit`.
    pub hit_body: hit::Body,
    /// +0x18 / +0x1c of `base`: his height (160) and width (45).
    pub height: F,
    pub width: F,
    /// +0x240 `posView`, +0x260 `posEye`, +0x270 `angle`.
    pub pos_view: V4,
    pub pos_eye: V4,
    pub angle: V4,
    /// +0x80 `hitAttribute`: the ground under him (footsteps, shade).
    pub hit_attribute: u32,
    /// +0xe1 bit 5 `lostHeadFlag`: the eye view hides him.
    pub lost_head: bool,
    /// +0x88 `transparency` (as drawn), +0x8c `setTransparency`.
    pub transparency: F,
    pub set_transparency: F,
    /// The transparency his shadow's alpha comes from (`ccChar::Draw`:
    /// `setTransparency` while the camera fade is off).
    pub shadow_t: F,
    /// Whether he was drawn this frame (`ccChar::Draw`'s return).
    pub drawn: bool,
    /// +0xfc `stopCnt`, +0x11c `cycle`, +0x2e8 `stressMeter`.
    pub stop_cnt: i16,
    pub cycle: i32,
    pub stress: i16,
    /// What `AnimCtrl` asked for this frame.
    pub events: Vec<ActEvent>,
    // --- what the event scripts drive (ai.rs, party.rs) ---
    /// +0x128 `ai`: none until `ccSPC::Reboot` makes it ([`Player::build`]).
    pub ai: Option<Ai>,
    /// +0x08 `condition.dead` (0: alive), +0x7e `skillStatus`.
    pub dead: i16,
    pub skill_status: i16,
    /// +0xe0 bit 1 `dispSW` (the ccSpcChar constructor sets it), bit 7
    /// `noDeathFlag`; +0xe1 bit 4 `recallFlag`; bits 14-16 `partyFlag`
    /// (the registry's, 1 for Kite).
    pub disp: bool,
    pub no_death: bool,
    pub recall: bool,
    pub party_flag: i8,
    /// +0x90 `transDist`: `ccChar::Draw`'s near fade on (`menu_ban` turns
    /// it off).
    pub trans_dist: bool,
    /// On the party's command list (`ccEntryCmnd`).
    pub listed: bool,
    /// +0xe8 `SpcListNum`: his registry slot.
    pub list_num: i32,
    /// What his AI's remote command 5 asked of the party this frame
    /// ([`crate::party::Spcs::leave`]).
    pub leave: Option<ai::PartyLeave>,
    /// `WORLD_MAN::Enter(pos)` was called this frame (`CollisionTest` or
    /// `MapLoopAdjustPos` landed on ground whose attribute has bit 0x80000:
    /// a field's dungeon entrance, a dungeon's doors and stairs).
    pub enter: bool,
    /// `dneFlag` (0x00378cd8): the entrances are shut.
    pub dne: bool,
    /// [`Hits::attribute_qualified`] as `hit_attribute` was taken.
    pub attribute_qualified: bool,
    /// The disc's volume: how the AI numbers its modes.
    pub volume: piney_data::volume::Volume,
}

/// The ground attribute bit that walks into `WORLD_MAN::Enter`.
pub const ENTER_BIT: u32 = 0x8_0000;

/// What `Player::main` reads of the rest of the game.
pub struct Frame<'a> {
    pub pad: &'a CamPad,
    pub camera: &'a mut Camera,
    pub hits: &'a mut Hits,
    /// Kite's animations by act (`playerAnimTbl`).
    pub anims: &'a [&'a Animation],
    /// newlib `rand()` (the fidget).
    pub rand: &'a mut dyn FnMut() -> i32,
    /// `cmndTarget != 0`.
    pub cmnd_target: bool,
    /// `checkPartyAnnihilation()` (the AI's paths read it).
    pub annihilated: bool,
}

/// `ccAI::arrivalChatCnt` as the AI's constructor sets it in a town.
pub const ARRIVAL_CHAT_TOWN: i16 = 150;
/// `ccPlayer::Main`'s speeds under the AI: `moveFlag` walking 140 and
/// running 100, over which `speedRate` is taken.
const MANUAL_WALK: F = 0x430c_0000;
const MANUAL_RUN: F = 0x42c8_0000;

/// `ccPlayer::Main`'s move under the AI (gcmn 0x00598608-0x005986f0): from
/// `moveFlag` and `runFlag` and the heading, as `ControlMove` would at full
/// lean - walking `speedRate` 1 (140 / 140) at `speedValue * tsp`, running 1
/// (100 / 100) at `speedValue * speed`, standing 0 - and `movePos =
/// (v sinf(dirc.z), -v cosf(dirc.z))` (signed zeros standing).
pub fn manual_move(b: &mut Body) {
    let f1 = if b.move_flag { if b.run_flag { MANUAL_RUN } else { MANUAL_WALK } } else { 0 };
    let z = b.dirc[2];
    let speed = if !b.run_flag {
        b.speed_rate = div(f1, MANUAL_WALK);
        mul(b.speed_rate, mul(b.speed_value, TSP))
    } else {
        b.speed_rate = div(f1, MANUAL_RUN);
        mul(b.speed_rate, mul(b.speed_value, b.speed))
    };
    b.now_speed = speed;
    b.move_pos[0] = mul(speed, ee::sinf(z));
    b.move_pos[1] = mul(ee::neg(speed), ee::cosf(z));
}

impl Player {
    /// `ccPlayer::ccPlayer(0)` arriving in a town: at the start position,
    /// act 13 (the fade-in), with `speed` his `velocity` (27.5 for Kite).
    pub fn new(pos: V4, dirc: V4, speed: F, width: F, height: F) -> Self {
        let acts = Acts::arriving();
        Player {
            body: Body {
                pos,
                dirc,
                speed_value: ONE,
                speed,
                speed_rate: ONE,
                restraint: acts.restraint,
                move_pos: ee::VF0,
                ..Body::default()
            },
            acts,
            hit_body: hit::Body { pos, radius: width, ..hit::Body::default() },
            height,
            width,
            pos_view: [0; 4],
            pos_eye: [0; 4],
            angle: ee::VF0,
            hit_attribute: 0,
            lost_head: false,
            transparency: ONE,
            set_transparency: ONE,
            shadow_t: ONE,
            drawn: false,
            stop_cnt: 0,
            cycle: 0,
            stress: 0,
            events: Vec::new(),
            ai: None,
            dead: 0,
            skill_status: 0,
            disp: true,
            no_death: false,
            recall: false,
            party_flag: 1,
            trans_dist: true,
            listed: true,
            list_num: 0,
            leave: None,
            enter: false,
            dne: false,
            attribute_qualified: true,
            volume: piney_data::volume::Volume::Inf,
        }
    }

    /// `CollisionTest`'s and `MapLoopAdjustPos`'s tail: landed on ground
    /// with [`ENTER_BIT`], unless `dneFlag`, `WORLD_MAN::Enter(pos)`.
    fn check_enter(&mut self, hits: &Hits) {
        if !self.dne && hits.num != 0 && hits.nearest.att & ENTER_BIT != 0 {
            self.enter = true;
        }
    }

    /// `ccPlayer::ccPlayer(0)` (gcmn 0x00597910) in a town with the registry's
    /// `bootParam`, then `ccSPC::Reboot`'s AI (0x005a00d0): the arrival of
    /// [`Player::new`], off the command list when `boot` has bit 2,
    /// `SetBootStatus(boot)`, `restraintSW` unless standing; then a `ccAI` in mode
    /// 1, manual when `boot` has bit 2. Event 2's `pc_mode -3 6` makes him start
    /// out of sight (act 14) under manual control.
    pub fn build(pos: V4, dirc: V4, speed: F, width: F, height: F, boot: i32, hits: &mut Hits) -> Self {
        let mut p = Player::new(pos, dirc, speed, width, height);
        p.listed = boot & 4 == 0;
        // The arrival's restraintSW is the constructor's for act 13; the
        // ccSpcChar constructor leaves it 0 for one standing.
        p.acts.restraint = false;
        p.spc().set_boot_status(boot, hits);
        if p.acts.act != 2 {
            p.acts.restraint = true;
        }
        p.body.restraint = p.acts.restraint;
        let mut ai = Ai::new(ARRIVAL_CHAT_TOWN, 0);
        ai.change_mode(1);
        if boot & 4 != 0 {
            ai.manual_mode(&p.body.dirc);
        }
        p.ai = Some(ai);
        p
    }

    /// The `ccSpcChar` members the AI and the event instructions touch.
    pub fn spc(&mut self) -> SpcRef<'_> {
        SpcRef {
            ai: self.ai.as_mut(),
            pos: &mut self.body.pos,
            dirc: &mut self.body.dirc,
            dead: &mut self.dead,
            skill_id: &mut self.body.skill_id,
            skill_status: &mut self.skill_status,
            transparency: &mut self.transparency,
            set_transparency: &mut self.set_transparency,
            trans_dist: &mut self.trans_dist,
            disp: &mut self.disp,
            restraint: &mut self.acts.restraint,
            move_flag: &mut self.body.move_flag,
            stop_flag: &mut self.acts.stop_flag,
            run_flag: &mut self.body.run_flag,
            ghost: &mut self.acts.ghost,
            no_death: &mut self.no_death,
            recall: &mut self.recall,
            party_flag: &mut self.party_flag,
            act: &mut self.acts.act,
            act_old: &mut self.acts.act_old,
            anm_flag: &mut self.acts.anm_flag,
            act_cnt: &mut self.acts.act_cnt,
            transfer_lag: &mut self.acts.transfer_lag,
            cloak: &mut self.acts.cloak,
            hit: &mut self.hit_body,
            listed: &mut self.listed,
            velocity: self.body.speed,
        }
    }

    /// `ccSpcChar::CheckControlMode`: his AI in manual mode.
    pub fn manual(&self) -> bool {
        self.ai.as_ref().is_some_and(|a| a.manual_sw)
    }

    /// One frame of `ccPlayer::Main`.
    pub fn main(&mut self, f: &mut Frame) {
        self.events.clear();
        self.leave = None;
        let manual = self.manual();
        if self.ai.is_some() {
            self.hit_body.manual = manual;
        }
        self.enter = false;
        let b = &mut self.body;
        b.move_pos = ee::VF0;
        // Restraint lives on the act side; the move reads it.
        b.restraint = self.acts.restraint;
        b.pause = self.acts.pause;
        b.attack = self.acts.attack;
        if manual {
            // Under the AI: Brains, then the move from the heading and the
            // flags ManualControl left (0x005985bc-0x005986f0).
            let input = ai::BrainsInput {
                annihilated: f.annihilated,
                list_num: self.list_num,
                in_battle: 0,
                volume: self.volume,
            };
            self.leave = ai::brains(&mut self.spc(), f.hits, &input);
            manual_move(&mut self.body);
        } else {
            // ControlMove reads the active camera (activeCamPtr) and
            // cameraGetRot(camID).
            let input = MoveInput {
                pow_l: f.pad.pow_l,
                dirc_l: f.pad.dirc_l,
                cmnd_target: f.cmnd_target,
                cam_reset_dirc: f.camera.active().reset_dirc,
                cam_rot_z: f.camera.rot()[2],
                cam_type: f.camera.active().kind,
            };
            control_move(&mut self.body, &input, &mut f.camera.active_mut().reset_flag);
        }
        // ccSpcChar::HitCheck takes the body off the character list while
        // out of sight (act 14) or dead.
        if self.acts.act == crate::motion::act::HIDDEN || self.dead == 4 {
            f.hits.hit_disable(&mut self.hit_body);
        }
        let b = &mut self.body;
        let (r, mv) = f.hits.hit_check(&mut self.hit_body, b.pos, b.move_pos, self.width, b.now_speed, b.run_flag);
        b.move_pos = mv;
        if r & 3 == 3 {
            self.stress += 2;
            if self.stress > 100 {
                self.stress = 50;
            }
        } else {
            self.stress = (self.stress - 8).max(0);
        }
        b.pos[0] = add(b.pos[0], b.move_pos[0]);
        b.pos[1] = add(b.pos[1], b.move_pos[1]);
        // CollisionTest.
        b.pos[2] = f.hits.land(b.pos, hit::LAND_MASK);
        let b = &mut self.body;
        let hits = &*f.hits;
        if !self.dne && hits.num != 0 && hits.nearest.att & ENTER_BIT != 0 {
            self.enter = true;
        }
        self.hit_attribute = f.hits.attribute();
        self.attribute_qualified = f.hits.attribute_qualified();
        // MapLoopAdjustPos: WORLD_MAN::AddCenter(movePos), W2MPos (w 1; a
        // town never wraps), and after a wrap the ground again.
        f.hits.add_center(b.move_pos[0], b.move_pos[1]);
        let (p, wrapped) = f.hits.w2m(b.pos);
        b.pos = p;
        if wrapped {
            b.pos[2] = f.hits.land(b.pos, hit::LAND_MASK);
            self.check_enter(f.hits);
        }
        let b = &mut self.body;
        // CameraPosCalc.
        let mut v = b.pos;
        v[2] = add(b.pos[2], HEAD);
        v[3] = ONE;
        self.pos_view = v;
        self.pos_eye = v;
        self.angle[2] = b.dirc[2];
        // CameraPosSet (gcmn 0x0059bac0), by the active camera's type: the
        // eye view places tcam only while camera 1 is active (and only then
        // hides him); any other type goes to cameraSetManual, which moves
        // tcam only while camera 1 is active and follows.
        if f.camera.active().kind == kind::EYE {
            f.camera.eye_level(self.pos_eye, &mut self.angle, f.pad);
            self.lost_head = f.camera.cam_id == crate::camera::id::FIELD;
            if self.lost_head {
                b.dirc[2] = self.angle[2];
            }
        } else {
            let area = f.hits.area;
            f.camera.set_manual(self.pos_view, f.pad, area, f.hits);
            self.lost_head = false;
        }
        match f.hits.bounds {
            Some(bounds) => f.camera.set_wrapped(b.pos, bounds),
            None => f.camera.set(b.pos),
        }
        // AnimCtrl (checkCameraType: the active camera's).
        let manual = self.ai.as_ref().is_some_and(|a| a.manual_sw);
        let act_in =
            ActInput { area: f.hits.area, cam_type: f.camera.active().kind, warp: false, in_battle: 0, manual };
        let (mut mv_flag, mut run_flag) = (b.move_flag, b.run_flag);
        anim_ctrl(
            &mut self.acts,
            &mut mv_flag,
            &mut run_flag,
            b.speed_rate,
            b.speed_value,
            &act_in,
            f.anims,
            f.rand,
            &mut self.events,
        );
        b.move_flag = mv_flag;
        b.run_flag = run_flag;
        b.restraint = self.acts.restraint;
        // The arrival's end puts him on the command list unless the AI
        // holds him (the caller puts his body on the character list); the
        // departure's first frame takes his body off it.
        if self.events.contains(&ActEvent::Arrived) && !manual {
            self.listed = true;
        }
        if self.events.contains(&ActEvent::Leaving) {
            f.hits.hit_disable(&mut self.hit_body);
        }
        // Transparency and the draw test (ccChar::Draw, only while dispSW;
        // transDist 0 turns the near fade off).
        let t = if self.lost_head { 0 } else { self.acts.cloak };
        self.transparency = t;
        self.set_transparency = t;
        // ccChar::Draw, only while dispSW: hide starts set with transDist 0
        // or in the eye view (checkCameraType); ccGetCameraTransparency
        // measures from the active camera, fading him from game.area's
        // distance (+0x14: the town's, or the field's).
        self.drawn = false;
        if self.disp {
            let a = f.camera.active();
            let mut hide = !self.trans_dist || a.kind == kind::EYE;
            let far = if f.hits.area == 0 { FADE_TOWN } else { FADE_AREA };
            let fade =
                camera_transparency(self.volume, b.pos, a.pos, a.deg[1], self.width, self.height, far, &mut hide);
            self.transparency = mul(self.set_transparency, fade);
            self.shadow_t = if hide { self.set_transparency } else { self.transparency };
            self.drawn = !(ee::lt(self.transparency, MIN_DRAWN) && !hide);
        }
        // stopCnt, cycle.
        self.stop_cnt = if b.move_flag { 0 } else { self.stop_cnt.saturating_add(1) };
        self.cycle = self.cycle.wrapping_add(1);
    }
}
