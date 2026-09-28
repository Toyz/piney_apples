//! The field's effects (`docs/engine/effects.md`): what `ccThEffect`
//! (priority 80) and `ccThParticle` (98) run and draw each frame, started by
//! the rest of the game - Kite's arrival, hits, the spells, deaths, level
//! ups, Data Drain.
//!
//! ```text
//! ccSetupGameCtrl (main 0x00168960) starts both tasks with the field's:
//! ccThEffect   (main 0x001c2e60, 80)  new ccEffectCtrl(0) (effc: 500 ccEffect
//!                                     in effWork, 100 ccEffect2 in effWork2,
//!                                     the effect files' objects by effectTbl);
//!                                     then each frame ccEffectCtrl::Main
//!                                     (0x001c38a0) - every live ccEffect's
//!                                     Main on the effect layer (20) or its
//!                                     own - and ccEffectElementManager::Main
//! ccThParticle (main 0x001bc2f0, 98)  new ccParticleCtrl; each frame its Main:
//!                                     the generators, then the particles
//! ```
//!
//! The port keeps the game's split between starting an effect (the `eff*`
//! functions, called by whoever the effect is for), running them (one
//! [`Effects::step`] a frame for `ccThEffect`, [`Effects::step_particles`]
//! for `ccThParticle`), and drawing (each step records what `Main` sends to
//! the GS - models, clumps, sprites - with the layer it is sent on;
//! [`Effects::draw`] turns the records into the frame's layers).
//!
//! Everything the effects read of the world (characters' positions and
//! sizes, the camera, the player's map wrap, `rand()`) comes through
//! [`Host`], and what they ask of it (sounds, damage calls) goes out as
//! [`Event`]s.

pub mod ability;
pub mod arrival;
pub mod boss;
pub mod convergence;
pub mod damupr;
pub mod debris;
pub mod dmath;
pub mod drain;
pub mod draw;
pub mod drawelm;
pub mod dust;
pub mod eff;
pub mod effect;
pub mod element;
pub mod fall;
pub mod files;
pub mod flyfont;
pub mod gimmick;
pub mod heal;
pub mod hit;
pub mod host;
pub mod levelup;
pub mod nodes;
pub mod particle;
pub mod pfx;
pub mod pg;
pub mod portal;
pub mod ring;
pub mod shield;
pub mod shockwave;
pub mod skillstart;
pub mod space;
pub mod spell;
pub mod sprite;
pub mod strfx;
pub mod summoned;
pub mod summons;
pub mod thunder;
pub mod tornado;
pub mod upheaval;
pub mod vu;

use piney_data::Result;
use piney_data::archive::Archive;
use piney_data::volume::Volume;
use piney_desktop::layers::Layers;

pub use piney_world::ee::{self, F, ONE, V4};

use crate::draw::{Camera, DrawRec};
use crate::effect::EffectCtrl;
use crate::files::Assets;
use crate::particle::Particles;

/// What `ccLandHitCheck2` answers for no hit: 9900.0.
pub const NO_HIT: F = 0x461a_b000;
/// `ccSpcChar` +0x160 `weaponEffPos[4]` and +0xe4 `armsEffectSW`: the
/// weapon's points and particles' switch ([`VecRef::CharAt`],
/// [`IntRef::CharAt`]).
pub const WEAPON_POS: u16 = 0x160;
pub const ARMS_SW: u16 = 0xe4;

/// A character as the host names it: the game's `ccChar *`.
pub type CharRef = u32;

/// A vector the game points an effect at by address (`posPtr`, `rotPtr`,
/// a generator's `syncPos`): read afresh each time it is used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VecRef {
    /// `ccChar` +0x40 `pos`.
    CharPos(CharRef),
    /// `ccChar` +0x60 `dirc`.
    CharDirc(CharRef),
    /// Another effect's +0x00 `pos`, by slot.
    EffectPos(usize),
    /// Another effect's +0x20 `rot`, by slot.
    EffectRot(usize),
    /// Another effect's +0x50 `posT`, by slot.
    EffectPosT(usize),
    /// A vector `off` bytes into a character (a `ccSpcChar`'s weapon points
    /// for `startParticleEffect2`): [`Host::char_vec`].
    CharAt(CharRef, u16),
    /// A vector another system owns and publishes each frame
    /// ([`effect::EffectCtrl::anchors`]): a boss effect's photon.
    Anchor(u32),
}

/// An int the game points a particle generator at (`syncSW`): read each
/// frame, the generator stops when it is 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IntRef {
    /// An int `off` bytes into a character (a `ccSpcChar`'s +0xe4 for its
    /// arms' effect): [`Host::char_int`].
    CharAt(CharRef, u16),
    /// An effect's `temp[k]`, by (slot, k): `effDrain`'s orbs switch their
    /// trails off by clearing `temp[1]`. The particle system reads it from
    /// the effect slots.
    EffectTemp(usize, usize),
    /// An effect's `flags` (+0x68), by slot: the meteors' smoke stops as
    /// they land.
    EffectFlags(usize),
}

/// What the effects read of the rest of the game.
pub trait Host {
    /// newlib `rand()` (`INF SLUS_202.67:0x00133a38`): the one sequence the
    /// whole game draws from.
    fn rand(&mut self) -> i32;
    /// `plw`'s position (`ccPlayer` +0x40): what `W2PPos` measures from.
    fn player_pos(&self) -> V4;
    /// `WORLD_MAN` +0x420: the map's x min, y min, x max, y max (a town's
    /// are -24000 and 24000).
    fn bounds(&self) -> [F; 4] {
        space::TOWN_BOUNDS
    }
    /// The camera the effects face and fade by: `cameraGetPos(camID)`'s eye,
    /// `activeCamPtr`'s eye and the point it looks at, and the view the
    /// frame is drawn through.
    fn camera(&self) -> Camera;
    /// `ccChar` +0x40 `pos`.
    fn char_pos(&self, c: CharRef) -> V4;
    /// `ccChar` +0x60 `dirc`.
    fn char_dirc(&self, c: CharRef) -> V4;
    /// The character's parameter row (`ccChar` +0x00): +0x18 `height`.
    fn char_height(&self, c: CharRef) -> F;
    /// ... and +0x1c `width`.
    fn char_width(&self, c: CharRef) -> F;
    /// `ccCheckTarget(c)` (gcmn 0x00519920): is `c` on the command lists
    /// (`cmndPcRoot`, `cmndEneRoot`, `cmndObjRoot`) - a character the
    /// battle can name.
    fn check_target(&self, _c: CharRef) -> bool {
        true
    }
    /// The parameter row's +0x08 `type` (bit 2: a party member).
    fn char_type(&self, _c: CharRef) -> i32 {
        0
    }
    /// `checkPartyMenberNum(base->id)` (gcmn 0x0059d100): the member's
    /// slot in `ccPartyManager`, -1 none.
    fn party_slot(&self, _c: CharRef) -> i32 {
        -1
    }
    /// `ccConditionIconNum(c)` (gcmn 0x0051bcb0): the condition icons the
    /// member's panel shows.
    fn condition_icon_num(&self, _c: CharRef) -> i32 {
        0
    }
    /// `checkCameraID()` (main 0x00160ca0, `camID`) and `checkCameraType()`
    /// (0x00160cb0, `activeCamPtr` +0x5c).
    fn camera_id(&self) -> i16 {
        0
    }
    fn camera_type(&self) -> i32 {
        0
    }
    /// `ccCheckObjectSize(c)` (gcmn 0x0042e120): the size class of an
    /// enemy, gimmick or NPC's row (+0x70 / +0x30).
    fn object_size(&self, _c: CharRef) -> i32 {
        0
    }
    /// `ccChar::CheckCharAttribute(flag)` (gcmn 0x005701d0): the
    /// character's element by its row (flag 1: the one it is weak to, as
    /// the attribute critical uses it), -1 for none.
    fn char_attribute(&self, _c: CharRef, _flag: i32) -> i32 {
        -1
    }
    /// `ccChar` +0x08 `condition.dead`: 0 alive, 1 on the way down, more
    /// when down.
    fn char_dead(&self, _c: CharRef) -> i16 {
        0
    }
    /// `genrand()` (main 0x001d9620): the next word of the game's second
    /// generator, the Mersenne Twister behind `ccRand` and `ccRandF`
    /// (`piney_world::mt::Mt`).
    fn genrand(&mut self) -> u32 {
        0
    }
    /// The vector `off` bytes into a character ([`VecRef::CharAt`]): its
    /// pos and dirc by default, (0, 0, 0, 1) for others.
    fn char_vec(&self, c: CharRef, off: u16) -> V4 {
        match off {
            0x40 => self.char_pos(c),
            0x60 => self.char_dirc(c),
            _ => [0, 0, 0, ONE],
        }
    }
    /// The int `off` bytes into a character ([`IntRef::CharAt`]); 1 (on)
    /// by default.
    fn char_int(&self, _c: CharRef, _off: u16) -> i32 {
        1
    }
    /// `ccChar` +0x50 `posP`: the position in the player's frame
    /// (`W2PPos` of `pos` by default).
    fn char_pos_p(&self, c: CharRef) -> V4 {
        space::w2p(self.char_pos(c), self.player_pos(), self.bounds())
    }
    /// `ccChar` +0x98 `affectPerson`: who last affected the character (the
    /// attacker whose blow it took); None for a null pointer.
    fn affect_person(&self, _c: CharRef) -> Option<CharRef> {
        None
    }
    /// `ccLandHitCheck(pos, 0x20000000)` then `checkHitResultAttlibute()`:
    /// the ground's attribute under `pos` (0 by default: plain ground).
    fn ground_attribute(&mut self, _pos: V4) -> u32 {
        0
    }
    /// The int an [`IntRef`] names.
    fn int(&self, r: IntRef) -> i32 {
        match r {
            IntRef::CharAt(c, off) => self.char_int(c, off),
            // Read from the effect slots by the particle system, not here.
            IntRef::EffectTemp(..) | IntRef::EffectFlags(_) => 1,
        }
    }
    /// `game` +0x14 `area`, +0x28 `dungeon`, and `WORLD_MAN::GetFieldType`
    /// (+0x10): where the spell falls from (`FallSystem`).
    fn area(&self) -> (i32, i32, i32) {
        (1, 0, 0)
    }
    /// `ccLandHitCheck2(pos, offsetZ, mask)` (gcmn 0x00572100): where the
    /// segment from `pos` down (or up) by `offset_z` meets the land (the
    /// field's models through `ccHitCheckLM2`, the ground by
    /// `ccSetGroundHeight`), as a height; [`NO_HIT`] for none.
    fn land_hit_check2(&mut self, _pos: V4, _offset_z: F, _mask: u32) -> F {
        NO_HIT
    }
    /// `ccLandHitCheck(pos, mask)` (gcmn 0x00571e00): the land's height
    /// under `pos` (the segment from 105 above it to 1000 below), `pos.z`
    /// itself when none is met.
    fn land_hit_check(&mut self, pos: V4, _mask: u32) -> F {
        pos[2]
    }
    /// `ccHitCheckLM2(from, to, mask)` (main 0x00153900): the field's
    /// models (`ccModelHit`) met by the segment from `from` to `to`: the
    /// nearest hit written into `to` and its answer, or -1.0 (`to` left)
    /// for none.
    fn hit_check_lm2(&mut self, _from: V4, _to: &mut V4, _mask: u32) -> F {
        0xbf80_0000
    }
    /// `cameraGetRot(out, camID)` (main 0x00161610): the rotation of the
    /// camera the effects face, `out`'s lanes kept where the camera leaves
    /// them.
    fn camera_rot(&self, out: V4) -> V4 {
        out
    }
    /// Called with each event the moment the game would make the call, for
    /// a runtime that must act at once (the damage calls and
    /// `cameraShake` draw `rand()` in the game, between the effects' own
    /// draws); the events are queued for [`Effects::take_events`] as well.
    fn raise(&mut self, _e: &Event) {}
}

/// What the effects ask of the rest of the game, in the order they ask it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// `ccSeOn3D(se, pos)`: a sound effect placed in the world.
    Sound3d { se: i32, pos: V4 },
    /// `cameraShake(a, b, c, d)` (main 0x00162cd0): power, cycle, time,
    /// direction (2 draws `rand()`).
    CameraShake([i32; 4]),
    /// `ccSeOn3DNote(se, pos, note)` (main 0x00179f50): a sound effect at a
    /// pitch.
    Sound3dNote { se: i32, pos: V4, note: i32 },
    /// `ccSeOnNote(se, note)` (main 0x00179cb0): a sound effect at a pitch,
    /// not placed.
    SoundNote { se: i32, note: i32 },
    /// `ccMenu->noiz->SetNoizBs(bs)` (main 0x001bb410): the screen noise.
    Noise { bs: i32 },
    /// `ccSkillDamage(attacker, target, ccGetSkillParam(sid), &acFlag, sid)`
    /// (gcmn 0x00573e60): the spell's damage on its target (or the area
    /// around it). `spell` names the `ccSkill` whose `acFlag` it takes
    /// ([`spell::Spells`]).
    SkillDamage { spell: u32, attacker: Option<CharRef>, target: Option<CharRef>, sid: i32 },
    /// `ccSkillDamage(attacker, pos, tType, ccGetSkillParam(sid), sid)`
    /// (gcmn 0x005743c0): the area around a world position (the rule takes
    /// it through `ccTransPosW2P`).
    SkillDamageAt { attacker: Option<CharRef>, pos: V4, ttype: i32, sid: i32 },
    /// `ccSkillDamage2(attacker, target, pos, tType,
    /// ccGetSkillParam(sid), &acFlag, sid)` (gcmn 0x005746d0): the area
    /// around a world position, the aimed target taking the attribute
    /// critical.
    SkillDamage2 { spell: u32, attacker: Option<CharRef>, target: Option<CharRef>, pos: V4, ttype: i32, sid: i32 },
    /// A spell system clears its caster's `skillID` (+0x7c) and
    /// `skillStatus` (+0x7e): the caster may act again.
    SkillRelease { ch: CharRef },
    /// `scFadeDef->EntryFlash(time, color, x, y, w, h)` (main 0x00160240):
    /// the screen's rectangle (in 512 x 384) flashed in `color` (the GS's
    /// RGBA, alpha 0x80 full) for `time` frames.
    Flash { time: i32, color: u32, rect: [F; 4] },
}

/// The field's effects: `ccThEffect`'s and `ccThParticle`'s state.
pub struct Effects {
    pub assets: Assets,
    pub ctrl: EffectCtrl,
    pub particles: Particles,
    /// The hits' state ([`hit`]).
    pub hits: hit::Hits,
    /// `flyFontCtrl`, `ccDamUprStr`'s list, the `flyFont` they draw with and
    /// the executable's strings ([`flyfont`], [`damupr`]).
    pub fly: flyfont::FlyFonts,
    pub dam: damupr::DamUprStr,
    pub font: flyfont::Font,
    pub strings: flyfont::Strings,
    /// The spells' state: their `ccSkill`s as the effects see them, the
    /// effect elements, `ccEffect2`, and their tables ([`spell`]).
    pub spells: spell::Spells,
    /// The boss effect manager's effects ([`boss`]) and its last pass's
    /// draws.
    pub boss: boss::BossEffects,
    boss_draws: Vec<DrawRec>,
    events: Vec<Event>,
    /// The last [`Effects::step`]'s draws, in the order they were sent.
    draws: Vec<DrawRec>,
    /// The last [`Effects::step_particles`]'s.
    particle_draws: Vec<DrawRec>,
}

/// The stream demo's effects: `ccThEffectStr`'s `ccEffectCtrl(1)`
/// (`effcStr`, [`strfx`]) and `ccThParticle`'s second system
/// (`particleSystemStr`), which take what the streams' effect tasks start.
pub struct StreamEffects {
    pub assets: Assets,
    pub ctrl: EffectCtrl,
    pub particles: Particles,
    hits: hit::Hits,
    spells: spell::Spells,
    events: Vec<Event>,
    draws: Vec<DrawRec>,
    particle_draws: Vec<DrawRec>,
}

impl StreamEffects {
    /// Both systems made, empty but for effect.cpp's static generators.
    pub fn new(archive: &Archive, volume: Volume) -> Result<StreamEffects> {
        let assets = Assets::read(archive, volume)?;
        let mut particles = Particles::stream();
        particles.install_statics(&assets);
        Ok(StreamEffects {
            assets,
            ctrl: EffectCtrl::stream(),
            particles,
            hits: hit::Hits::read(volume),
            spells: spell::Spells::read(volume),
            events: Vec::new(),
            draws: Vec::new(),
            particle_draws: Vec::new(),
        })
    }

    fn cx<'a>(&'a mut self, host: &'a mut dyn Host) -> (&'a mut EffectCtrl, Cx<'a>) {
        let cx = Cx {
            host,
            assets: &self.assets,
            particles: &mut self.particles,
            hits: &mut self.hits,
            spells: &mut self.spells,
            events: &mut self.events,
            draws: &mut self.draws,
        };
        (&mut self.ctrl, cx)
    }

    /// `effHitMarkStr(pos, rot, layer)`.
    pub fn hit_mark(&mut self, host: &mut dyn Host, pos: V4, rot: V4, layer: Option<i16>) {
        let (ctrl, mut cx) = self.cx(host);
        strfx::eff_hit_mark_str(ctrl, &mut cx, pos, rot, layer);
    }

    /// `effTransferStr(pos, height, layer)`.
    pub fn transfer(&mut self, host: &mut dyn Host, pos: V4, height: F, layer: Option<i16>) {
        let (ctrl, mut cx) = self.cx(host);
        strfx::eff_transfer_str(ctrl, &mut cx, pos, height, layer);
    }

    /// One frame of `ccThEffectStr` (80): `ccEffectCtrl::MainStr`.
    pub fn step(&mut self, host: &mut dyn Host) {
        let (ctrl, mut cx) = self.cx(host);
        cx.draws.clear();
        ctrl.main(&mut cx);
    }

    /// One frame of the second particle system (`ccThParticle`, 98).
    pub fn step_particles(&mut self, host: &mut dyn Host) {
        self.particle_draws.clear();
        let mut step =
            particle::Step { host, assets: &self.assets, effects: &self.ctrl, draws: &mut self.particle_draws };
        self.particles.main(&mut step);
    }

    /// The last steps' draws into `layers` through `camera`'s view.
    pub fn draw(&self, layers: &mut Layers, camera: &Camera) {
        for d in self.draws.iter().chain(&self.particle_draws) {
            d.render(&self.assets, layers, camera);
        }
    }

    /// The last effect and particle steps' draws, as recorded.
    pub fn draws(&self) -> (&[DrawRec], &[DrawRec]) {
        (&self.draws, &self.particle_draws)
    }

    /// Whether anything is still running.
    pub fn busy(&self) -> bool {
        self.ctrl.effects.iter().any(|e| e.status != 0)
            || !self.particles.gens.is_empty()
            || self.particles.slots.iter().any(|p| p.tex_id != -1)
    }
}

/// One frame's context for the effects' code: the world, the files, and
/// where events and draws go.
pub struct Cx<'a> {
    pub host: &'a mut dyn Host,
    pub assets: &'a Assets,
    pub particles: &'a mut Particles,
    pub hits: &'a mut hit::Hits,
    pub spells: &'a mut spell::Spells,
    pub events: &'a mut Vec<Event>,
    pub draws: &'a mut Vec<DrawRec>,
}

impl Cx<'_> {
    /// An event, in the order the game makes the call: to the host at once
    /// ([`Host::raise`]) and onto the queue.
    pub fn raise(&mut self, e: Event) {
        self.host.raise(&e);
        self.events.push(e);
    }
}

impl Effects {
    /// `ccThEffect`'s set-up: the effect files (`effectCCSTbl`) and
    /// `effectTbl`, the volume's, the slots empty; `ccThParticle`'s: no
    /// generators but effect.cpp's static ones.
    pub fn new(archive: &Archive, volume: Volume) -> Result<Effects> {
        let assets = Assets::read(archive, volume)?;
        let mut particles = Particles::default();
        particles.install_statics(&assets);
        Ok(Effects {
            assets,
            ctrl: EffectCtrl::new(),
            particles,
            hits: hit::Hits::read(volume),
            fly: flyfont::FlyFonts::default(),
            dam: damupr::DamUprStr::default(),
            font: flyfont::Font::fly(),
            strings: flyfont::Strings::read(volume),
            spells: spell::Spells::read(volume),
            boss: boss::BossEffects::default(),
            boss_draws: Vec::new(),
            events: Vec::new(),
            draws: Vec::new(),
            particle_draws: Vec::new(),
        })
    }

    /// The context the effects' code runs in, and the slots apart from it.
    pub fn split<'a>(&'a mut self, host: &'a mut dyn Host) -> (&'a mut EffectCtrl, Cx<'a>) {
        let cx = Cx {
            host,
            assets: &self.assets,
            particles: &mut self.particles,
            hits: &mut self.hits,
            spells: &mut self.spells,
            events: &mut self.events,
            draws: &mut self.draws,
        };
        (&mut self.ctrl, cx)
    }

    /// `ccThBossEffect` (66): `ccBossEffManager::Draw`, each boss effect's
    /// Draw ([`boss::BossEffects::draw`]); its draws replace the last
    /// pass's.
    pub fn boss_step(&mut self, host: &mut dyn Host) {
        self.boss_draws.clear();
        let mut cx = Cx {
            host,
            assets: &self.assets,
            particles: &mut self.particles,
            hits: &mut self.hits,
            spells: &mut self.spells,
            events: &mut self.events,
            draws: &mut self.boss_draws,
        };
        self.boss.draw(&mut self.ctrl, &mut cx, &self.assets.boss);
    }

    /// A `ccBossEff*Create` the boss's task makes: its slot (or -1).
    pub fn boss_create(&mut self, host: &mut dyn Host, make: boss::Make) -> i32 {
        let mut none = Vec::new();
        let mut cx = Cx {
            host,
            assets: &self.assets,
            particles: &mut self.particles,
            hits: &mut self.hits,
            spells: &mut self.spells,
            events: &mut self.events,
            draws: &mut none,
        };
        self.boss.create(&mut cx, &self.assets.boss, make)
    }

    /// The boss effects' last pass's draws.
    pub fn boss_draws(&self) -> &[DrawRec] {
        &self.boss_draws
    }

    /// One frame of `ccThEffect` (80): `ccEffectCtrl::Main` (the 500
    /// `ccEffect`s, then the 100 `ccEffect2`s), then
    /// `ccEffectElementManager::Main` (the spells' elements).
    pub fn step(&mut self, host: &mut dyn Host) {
        let (ctrl, mut cx) = self.split(host);
        cx.draws.clear();
        ctrl.main(&mut cx);
        spell::after_effects(ctrl, &mut cx);
    }

    /// One frame of `ccThParticle` (98): `ccParticleCtrl::Main`.
    pub fn step_particles(&mut self, host: &mut dyn Host) {
        self.particle_draws.clear();
        let mut step =
            particle::Step { host, assets: &self.assets, effects: &self.ctrl, draws: &mut self.particle_draws };
        self.particles.main(&mut step);
    }

    /// The draws of the last steps into `layers`, through `camera`'s view:
    /// the effects', then the particles'.
    pub fn draw(&self, layers: &mut Layers, camera: &Camera) {
        for d in self.boss_draws.iter().chain(&self.draws).chain(&self.particle_draws) {
            d.render(&self.assets, layers, camera);
        }
    }

    /// The last particle step's draws, as recorded.
    pub fn particle_draws(&self) -> &[DrawRec] {
        &self.particle_draws
    }

    /// The last step's draws, as recorded.
    pub fn draws(&self) -> &[DrawRec] {
        &self.draws
    }

    /// What the effects asked for since the last call.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// `effTransfer(ch)` (main 0x001ce020): the arrival's rings around `ch`.
    pub fn transfer(&mut self, host: &mut dyn Host, ch: CharRef) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        arrival::eff_transfer(ctrl, &mut cx, ch)
    }

    /// `effWarpTransfer(ch)` (main 0x001ce120): the arrival's sparks alone.
    pub fn warp_transfer(&mut self, host: &mut dyn Host, ch: CharRef) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        arrival::eff_warp_transfer(ctrl, &mut cx, ch)
    }

    /// `effLevelUp(ch)` (main 0x001cdde0): the level up's word and sparks.
    pub fn level_up(&mut self, host: &mut dyn Host, ch: CharRef) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        levelup::eff_level_up(ctrl, &mut cx, ch)
    }

    /// `effDrainCtrl(ap, bp, type, time, num)` (main 0x001d79f0): `num`
    /// orbs of HP (`kind` 0) or SP (1) from `bp` to `ap` over `time`
    /// frames, and the word over `bp`.
    pub fn drain_ctrl(
        &mut self,
        host: &mut dyn Host,
        ap: CharRef,
        bp: CharRef,
        kind: i32,
        time: i32,
        num: i32,
    ) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        drain::eff_drain_ctrl(ctrl, &mut cx, ap, bp, kind, time, num)
    }

    /// `effAfterDrain(ch, size)` (main 0x001cd380): the wave after a Data
    /// Drain; `size` 0-2 (small to large), or -1 for the character's own
    /// ([`Host::object_size`]).
    pub fn after_drain(&mut self, host: &mut dyn Host, ch: CharRef, size: i32) -> Option<usize> {
        let size = if size == -1 { drain::size_of_object(host.object_size(ch))? } else { size };
        let (ctrl, mut cx) = self.split(host);
        drain::eff_after_drain(ctrl, &mut cx, ch, size)
    }

    /// `ccHitMarkDisp(ch, attacker)` (gcmn 0x005714e0): the spark, ring and
    /// photons where `attacker`'s blow meets `ch` (piney-battle's
    /// `Event::HitMark { on: ch, by: attacker }`).
    pub fn hit_mark(&mut self, host: &mut dyn Host, ch: CharRef, attacker: CharRef) {
        let (ctrl, mut cx) = self.split(host);
        hit::hit_mark_disp(ctrl, &mut cx, ch, attacker);
    }

    /// `effProtect(ch, broken, kind)` (main 0x001cf250; `Event::Protect`).
    pub fn protect(&mut self, host: &mut dyn Host, ch: CharRef, broken: i32, kind: i32) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        hit::eff_protect(ctrl, &mut cx, ch, broken, kind)
    }

    /// `ccParticleAttributeCritical(ch)` (main 0x001bca90;
    /// `Event::AttributeCriticalParticle(ch)`).
    pub fn attribute_critical(&mut self, host: &mut dyn Host, ch: CharRef) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        hit::particle_attribute_critical(ctrl, &mut cx, ch)
    }

    /// `ccParticleCritical(ch)` (main 0x001bc540; `Event::Critical(ch)`).
    pub fn critical(&mut self, host: &mut dyn Host, ch: CharRef) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        hit::particle_critical(ctrl, &mut cx, ch)
    }

    /// `ccParticleDying(ch)` (main 0x001bc6d0; `Event::Dying(ch)`).
    pub fn dying(&mut self, host: &mut dyn Host, ch: CharRef) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        hit::particle_dying(ctrl, &mut cx, ch)
    }

    /// `ccParticleNoDamage(ch)` (main 0x001bc8c0; `Event::NoDamage(ch)`).
    pub fn no_damage(&mut self, host: &mut dyn Host, ch: CharRef) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        hit::particle_no_damage(ctrl, &mut cx, ch)
    }

    /// `ccParticleAttributeGuard(ch, attacker)` (main 0x001bcc40;
    /// `Event::AttributeGuard { on: ch, by: attacker }`).
    pub fn attribute_guard(&mut self, host: &mut dyn Host, ch: CharRef, attacker: CharRef) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        hit::particle_attribute_guard(ctrl, &mut cx, ch, attacker)
    }

    /// `effSkillStart(ch, sid, a, b)` (main 0x001d35f0): what shows on `ch`
    /// as a skill starts - the sparks, the exec ring, the force rings
    /// unless `a`, and at count 7 the circle; `b` puts the sparks over its
    /// head. piney-battle's skill request (`Request::skill_start`), Kite's
    /// and a party member's `Out::SkillStart` and `ccSkillModifyCondition`'s
    /// starts. The controller -12 (-13).
    pub fn skill_start(&mut self, host: &mut dyn Host, ch: CharRef, sid: i32, a: i32, b: i32) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        skillstart::eff_skill_start(ctrl, &mut cx, ch, sid, a, b)
    }

    /// `effSkillStart(ch, sp, a, b)` (main 0x001d3650) for a
    /// `ccSkillParam` outside `skillTbl` (an enemy's own attacks, piney-
    /// battle's `enemy_ai::Out::SkillStart`): its `type` (+0x2c), and
    /// whether it is `skillTbl[1]` (the normal attack).
    pub fn skill_start_param(
        &mut self,
        host: &mut dyn Host,
        ch: CharRef,
        ty: i32,
        normal_attack: bool,
        a: i32,
        b: i32,
    ) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        skillstart::eff_skill_start_param(ctrl, &mut cx, ch, ty, normal_attack, a, b)
    }

    /// `effSkillStartEffect(ch, attr, b)` (main 0x001d39d0): the Data Drain
    /// menu's side effects (piney-battle's `drain` `shown`, `(ch, 0, 1)`).
    pub fn skill_start_effect(&mut self, host: &mut dyn Host, ch: CharRef, attr: i32, b: i32) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        skillstart::eff_skill_start_effect(ctrl, &mut cx, ch, attr, b)
    }

    /// `effPhysicalSkillHitShockWave(pos, attr)` (main 0x001d2790): a
    /// physical skill's 0x8002 note (piney-battle's `MainOut::shock_waves`)
    /// at the skill's target position, `attr` its `type & 0xfc`. The last
    /// wave.
    pub fn shock_wave(&mut self, host: &mut dyn Host, pos: V4, attr: i32) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        shockwave::eff_physical_skill_hit_shock_wave(ctrl, &mut cx, pos, attr)
    }

    /// `effHeal(ch, n)` (main 0x001ccd20): the healing ring, sparks `n`
    /// times and the sound (the items' and the drain menu's `effHeal(ch,
    /// 1)`). The controller -19 (the game answers 0).
    pub fn heal(&mut self, host: &mut dyn Host, ch: CharRef, n: i32) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        heal::eff_heal(ctrl, &mut cx, ch, n)
    }

    /// `effHealSkill(ch, sid)` (main 0x001cccf0; `Event::HealSkill`).
    pub fn heal_skill(&mut self, host: &mut dyn Host, ch: CharRef, sid: i32) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        heal::eff_heal_skill(ctrl, &mut cx, ch, sid)
    }

    /// `effCure(ch)` (main 0x001cef50; `Event::Cure`).
    pub fn cure(&mut self, host: &mut dyn Host, ch: CharRef) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        heal::eff_cure(ctrl, &mut cx, ch)
    }

    /// `effSanity(ch)` (main 0x001cf0d0; `Event::Sanity`).
    pub fn sanity(&mut self, host: &mut dyn Host, ch: CharRef) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        heal::eff_sanity(ctrl, &mut cx, ch)
    }

    /// `effResurrect(ch)` (main 0x001cedd0; `Event::Resurrect`).
    pub fn resurrect(&mut self, host: &mut dyn Host, ch: CharRef) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        heal::eff_resurrect(ctrl, &mut cx, ch)
    }

    /// `effOpenBox(pos)` (main 0x001ced40): the sparks at `pos` (Kite's
    /// and a party member's `Out::OpenBox`, a box, fountain, food or idol
    /// opening).
    pub fn open_box(&mut self, host: &mut dyn Host, pos: V4) {
        let (_, mut cx) = self.split(host);
        heal::eff_open_box(&mut cx, pos);
    }

    /// `effEvolvePG(ch)` (main 0x001d0f00): a Grunty growing up; the
    /// slot, None with none free.
    pub fn evolve_pg(&mut self, host: &mut dyn Host, ch: CharRef) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        pg::eff_evolve_pg(ctrl, &mut cx, ch)
    }

    /// `effGrowPG(ch)` (main 0x001d0fd0) on a character at `pos` of
    /// `height`: a young Grunty becoming a grown one.
    pub fn grow_pg(&mut self, host: &mut dyn Host, pos: V4, height: F) {
        let (_, mut cx) = self.split(host);
        pg::eff_grow_pg(&mut cx, pos, height);
    }

    /// `effRemoveTrap(pos, a, b)` (main 0x001d0980): textures `a` and `b`
    /// (-1: 101 and 118) - an item's trap removal `(pos, -1, -1)`, a dead
    /// enemy's treasure box `(pos, 103, 121)`. The controller -6.
    pub fn remove_trap(&mut self, host: &mut dyn Host, pos: V4, a: i32, b: i32) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        heal::eff_remove_trap(ctrl, &mut cx, pos, a, b)
    }

    /// `effSmoke(pos, v, s, life, t, in, out)` (main 0x001ce300): one puff
    /// of smoke; whether a particle was free.
    #[allow(clippy::too_many_arguments)]
    pub fn smoke(
        &mut self,
        host: &mut dyn Host,
        pos: V4,
        v: V4,
        s: F,
        life: i32,
        t: i32,
        fade_in: i16,
        fade_out: i16,
    ) -> bool {
        let (_, mut cx) = self.split(host);
        dust::eff_smoke(&mut cx, pos, v, s, life, t, fade_in, fade_out)
    }

    /// `effSmokeN(pos, v, s, life, t, in, out)` (main 0x001ce490): a puff
    /// not faded by distance; whether a particle was free.
    #[allow(clippy::too_many_arguments)]
    pub fn smoke_n(
        &mut self,
        host: &mut dyn Host,
        pos: V4,
        v: V4,
        s: F,
        life: i32,
        t: i32,
        fade_in: i16,
        fade_out: i16,
    ) -> bool {
        let (_, mut cx) = self.split(host);
        dust::eff_smoke_n(&mut cx, pos, v, s, life, t, fade_in, fade_out)
    }

    /// `ccEffPawSmoke(ch, speed)` (main 0x001ce640): a puff at the lower of
    /// a runner's feet (`feet`: its "OBJ_t0 l foot" and "OBJ_t0 r foot"
    /// nodes' places), thrown back along its heading `dirc_z`; whether one
    /// was made.
    pub fn paw_smoke(&mut self, host: &mut dyn Host, feet: [V4; 2], dirc_z: F, speed: F) -> bool {
        let (_, mut cx) = self.split(host);
        dust::eff_paw_smoke(&mut cx, feet, dirc_z, speed)
    }

    /// `ccEnemyEffDust(p, n, s, life, t)` (gcmn 0x0043a3e0): n puffs of
    /// dust about `p`.
    pub fn enemy_dust(&mut self, host: &mut dyn Host, p: V4, n: i32, s: F, life: i32, t: i32) {
        let (_, mut cx) = self.split(host);
        dust::enemy_eff_dust(&mut cx, p, n, s, life, t);
    }

    /// `ccEnemyEffDustRing(p, s, r, n, life, t)` (gcmn 0x0043a590): a ring
    /// of n puffs about `p` (under 4, `ccEnemyEffDust`).
    #[allow(clippy::too_many_arguments)]
    pub fn dust_ring(&mut self, host: &mut dyn Host, p: V4, s: F, r: F, n: i32, life: i32, t: i32) {
        let (_, mut cx) = self.split(host);
        dust::enemy_eff_dust_ring(&mut cx, p, s, r, n, life, t);
    }

    /// `ccEnemyEffDustRing(ch, ofs, s, r, n, life, t)` (gcmn 0x0043a7a0)
    /// for a character at `pos` facing `dirc`: a ring of dust `ofs` from
    /// it (an enemy's feet, an idol opening).
    #[allow(clippy::too_many_arguments)]
    pub fn enemy_dust_ring(
        &mut self,
        host: &mut dyn Host,
        pos: V4,
        dirc: V4,
        ofs: V4,
        s: F,
        r: F,
        n: i32,
        life: i32,
        t: i32,
    ) {
        let (_, mut cx) = self.split(host);
        dust::enemy_eff_dust_ring_at(&mut cx, pos, dirc, ofs, s, r, n, life, t);
    }

    /// `effVirusCrystal(pos)` (main 0x001cf420): a virus core's crystal
    /// broken. The controller -4.
    pub fn virus_crystal(&mut self, host: &mut dyn Host, pos: V4) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        gimmick::eff_virus_crystal(ctrl, &mut cx, pos)
    }

    /// `effCrushBarrel`, `effCrushPot`, `effCrushCorpse`, `effCrushEgg`
    /// (pos, x) (main 0x001d0300-0x001d0770): the breakable's pieces and
    /// dust. `ccGimBox::breakObject` passes x 0 and picks the kind with
    /// [`gimmick::Fragment::of_crush`].
    pub fn crush(&mut self, host: &mut dyn Host, kind: gimmick::Fragment, pos: V4, x: i32) {
        let (ctrl, mut cx) = self.split(host);
        gimmick::eff_crush(ctrl, &mut cx, kind, pos, x);
    }

    /// `effOpenTrapBox(pos, kind, trap)` (main 0x001d0780): a trapped box
    /// going off. The controller -5.
    pub fn open_trap_box(&mut self, host: &mut dyn Host, pos: V4, kind: i32, trap: i32) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        gimmick::eff_open_trap_box(ctrl, &mut cx, pos, kind, trap)
    }

    /// `effStatueOfGod(pos, &idol->effsw)` (main 0x001d0e60): the glow over
    /// a Statue of God, while the idol's `effsw` ([`Host::char_int`] at
    /// +0x1e0) holds.
    /// `effUseSymbol(pos)` (main 0x001d0d70): a symbol cast.
    pub fn use_symbol(&mut self, host: &mut dyn Host, pos: V4) {
        let (_, mut cx) = self.split(host);
        gimmick::eff_use_symbol(&mut cx, pos);
    }

    /// `ccParticleExplode(pos, speed, scale, kind)` (main): one burst of
    /// particles (a symbol's spark, kind 1).
    pub fn particle_explode(&mut self, host: &mut dyn Host, pos: V4, speed: V4, scale: F, kind: i32) {
        let (_, mut cx) = self.split(host);
        particle::cc_particle_explode(&mut cx, pos, speed, scale, kind);
    }

    /// `startParticleEffect2(&weaponEffPos[a], &weaponEffPos[b], row,
    /// &armsEffectSW)` from `ccSpcChar::StartArmsEffect` (gcmn 0x0059e530):
    /// an art's element particles (`particleEffectTbl` rows 40-45) along
    /// the weapon of `ch`, between its weapon points `a` and `b`
    /// ([`Host::char_vec`] at +0x160 + 16 k), while its `armsEffectSW`
    /// (+0xe4, [`Host::char_int`]) holds.
    pub fn arms_particles(&mut self, host: &mut dyn Host, ch: CharRef, a: i32, b: i32, row: i32) {
        let Ok(row) = usize::try_from(row) else { return };
        if row >= self.assets.particle.effects.len() {
            return;
        }
        let point = |k: i32| VecRef::CharAt(ch, WEAPON_POS + 16 * (k & 3) as u16);
        let (ctrl, mut cx) = self.split(host);
        particle::start_particle_effect2(ctrl, &mut cx, point(a), point(b), row, Some(IntRef::CharAt(ch, ARMS_SW)));
    }

    /// `effFountain(pos, &spring->effsw)` (main 0x001d0ae0): the spring's
    /// mist while its `effsw` ([`Host::char_int`] at +0x1e0) holds.
    pub fn fountain(&mut self, host: &mut dyn Host, pos: V4, spring: CharRef) {
        let (_, mut cx) = self.split(host);
        gimmick::eff_fountain(&mut cx, pos, spring);
    }

    /// `effBossRoomEntrance(pos, &warning->effsw)` (main 0x001d0be0).
    pub fn boss_room_entrance(&mut self, host: &mut dyn Host, pos: V4, warning: CharRef) {
        let (_, mut cx) = self.split(host);
        gimmick::eff_boss_room_entrance(&mut cx, pos, warning);
    }

    /// `effDungeonEntrance(pos, &entrance->effsw, kind)` (main 0x001d0c90).
    pub fn dungeon_entrance(&mut self, host: &mut dyn Host, pos: V4, entrance: CharRef, kind: usize) {
        let (_, mut cx) = self.split(host);
        gimmick::eff_dungeon_entrance(&mut cx, pos, entrance, kind);
    }

    pub fn statue_of_god(&mut self, host: &mut dyn Host, pos: V4, idol: CharRef) {
        let (_, mut cx) = self.split(host);
        gimmick::eff_statue_of_god(&mut cx, pos, idol);
    }

    /// `effAbilityUp(ch, num)` (main 0x001d7380): condition 20-34's sparks
    /// until the condition ends them. The controller -21.
    pub fn ability_up(&mut self, host: &mut dyn Host, ch: CharRef, num: i32) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        ability::eff_ability_up(ctrl, &mut cx, ch, num)
    }

    /// `effAbilityDown(ch, num)` (main 0x001d7630): condition 0-18's. The
    /// controller -22.
    pub fn ability_down(&mut self, host: &mut dyn Host, ch: CharRef, num: i32) -> Option<usize> {
        let (ctrl, mut cx) = self.split(host);
        ability::eff_ability_down(ctrl, &mut cx, ch, num)
    }

    /// `setConditionEffect(ch)` (main 0x001c24a0) with `num` the
    /// character's `conditionNum` (+0x30): `new ccConditionEffect(ch, num)`
    /// with its stat change's effect ([`ability::eff_ability`]); what
    /// `ccChar::DispConditionEffect` (gcmn 0x0056f950) starts.
    pub fn condition_effect(&mut self, host: &mut dyn Host, ch: CharRef, num: i32) -> particle::ConditionEffect {
        let (ctrl, mut cx) = self.split(host);
        particle::set_condition_effect(ctrl, &mut cx, ch, num, ability::eff_ability)
    }

    /// `killConditionEffect(ep)` (main 0x001c24f0): the particles fade out,
    /// the stat change's effect ends.
    pub fn kill_condition_effect(&mut self, host: &mut dyn Host, ep: particle::ConditionEffect) {
        let (ctrl, mut cx) = self.split(host);
        particle::kill_condition_effect(ctrl, &mut cx, ep);
    }

    /// `deleteConditionEffect(ep)` (main 0x001c2520): the particles end at
    /// once, and the effect.
    pub fn delete_condition_effect(&mut self, host: &mut dyn Host, ep: particle::ConditionEffect) {
        let (ctrl, mut cx) = self.split(host);
        particle::delete_condition_effect(ctrl, &mut cx, ep);
    }

    /// `effResistantShield(ch, magic, n)` (gcmn 0x00501ab0;
    /// `Event::ResistantShield { on: ch, magic }` with `n` -1): the shield
    /// on an enemy or boss a blow met an immunity of, turned to face its
    /// `affectPerson` ([`Host::affect_person`]). The element's slot.
    pub fn resistant_shield(&mut self, host: &mut dyn Host, ch: CharRef, magic: i32, n: i32) -> Option<usize> {
        let (_, mut cx) = self.split(host);
        shield::eff_resistant_shield(&mut cx, ch, magic, n)
    }

    /// `ccEntryFlyFontNew(kind, value, pos, ch, sx, sy)` (gcmn 0x0051afc0;
    /// `Event::FlyFont { on: ch, kind, value }`): a number (-1: MISS)
    /// stacked over `ch`.
    pub fn fly_font(&mut self, ch: Option<CharRef>, kind: i32, value: i32) -> usize {
        self.dam.entry_new(&self.strings, kind, value, ch, &mut self.font)
    }

    /// `ccEntryFlyFontNewExp(kind, value, pos, ch)` (gcmn 0x0051aea0).
    pub fn fly_font_exp(&mut self, ch: Option<CharRef>, kind: i32, value: i32) -> usize {
        self.dam.entry_exp(&self.strings, kind, value, ch, &mut self.font)
    }

    /// `ccEntryFlyFontNewLevelDown(kind, pos, ch)` (gcmn 0x0051af20).
    pub fn fly_font_level_down(&mut self, ch: Option<CharRef>, kind: i32) -> usize {
        self.dam.entry_level_down(&self.strings, kind, ch, &mut self.font)
    }

    /// `ccEntryFlyFontNewMiss(pos, ch)` (gcmn 0x0051af70).
    pub fn fly_font_miss(&mut self, ch: Option<CharRef>) -> usize {
        self.dam.entry_miss(&self.strings, ch, &mut self.font)
    }

    /// `ccEntryFlyFontNum(color, num, pos, sx, sy, ofsX)` (gcmn 0x0051a900):
    /// a rising number (the bosses' own).
    #[allow(clippy::too_many_arguments)]
    pub fn fly_font_num(&mut self, color: i32, num: i32, pos: V4, sx: F, sy: F, ofs_x: i32) -> Option<usize> {
        self.fly.entry_num(&self.strings, color, num, pos, sx, sy, ofs_x)
    }

    /// `ccMenuCtrl::Disp`'s numbers (gcmn 0x0052170c-0x00521754):
    /// `ccCtrlFlyFont(pause)`, then unless `pause` `ccDamUprStr::CtrlAll()`
    /// and unless `game_over` (`compulsionGameOver`) `DrawAll()`, into
    /// `flyFont`'s queue after whatever the menu queued on it this frame
    /// (`font.packet_num` counts those against its 160). The quads are
    /// [`flyfont::Font::take`]n when the menu sends `flyFont`.
    pub fn fly_fonts(&mut self, host: &dyn Host, pause: bool, game_over: bool) {
        let ws = host.camera().world_screen;
        let view = piney_desktop::message::menu_view();
        self.fly.ctrl(pause, &mut self.font, &ws, &view);
        if !pause {
            self.dam.ctrl_all(host, &ws);
            if !game_over {
                self.dam.draw_all(&mut self.font, &view);
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    const ISO: &str = "../../work/infection/infection.iso";

    /// The effects read from the disc, or None (the test skips) without it.
    pub fn effects() -> Option<Effects> {
        if !std::path::Path::new(ISO).exists() {
            eprintln!("skipped: no {ISO}");
            return None;
        }
        let mut iso = piney_data::iso::Iso::open(ISO).unwrap();
        let archive = Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap();
        Some(Effects::new(&archive, iso.volume().unwrap()).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::Kind;

    /// The effect files and tables as `ccEffectCtrl` finds them: twelve
    /// files, 173 rows, the arrival's ring the one object a town resolves.
    #[test]
    fn the_effect_files_and_tables() {
        let Some(fx) = testing::effects() else { return };
        let a = &fx.assets;
        let stems: Vec<&str> = a.files.iter().map(|f| f.stem.as_str()).collect();
        assert_eq!(
            stems,
            ["drain", "particle", "x104", "x204", "x304", "x404", "x504", "x604", "x703", "x704", "x705", "x706"]
        );
        assert_eq!(a.tbl.len(), 173);
        assert_eq!(
            (a.tbl[3].ccs.as_str(), a.tbl[3].name.as_str(), a.tbl[3].kind),
            ("particle", "CMP_x032", Kind::Clump)
        );
        assert_eq!(a.tbl[4].kind, Kind::None);
        let resolved: Vec<i16> = (0..173).filter(|&i| a.adrs(i, true).is_some()).collect();
        assert_eq!(resolved, [3]);
        let field = (0..173).filter(|&i| a.adrs(i, false).is_some()).count();
        assert_eq!(field, 171);
        // PARTICLE.CCS's 49 Eff chunks, EFF_x000 first: 46 patterns each the
        // whole texture (4096 is all of it), opaque.
        let p = a.file_index("particle").unwrap();
        assert_eq!(a.effs[p].len(), 49);
        let e = &a.effs[p][0];
        assert_eq!(
            (a.name(files::ObjRef { file: p, object: e.object }), e.pat_num(), e.w, e.h),
            ("EFF_x000", 46, 4096, 4096)
        );
        assert_eq!(e.pats[0].transparency, 4096);
    }
}
