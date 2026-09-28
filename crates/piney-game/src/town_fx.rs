//! The `ccEff`s a Root Town draws itself (the lens flares and clouds,
//! Lia Fail's glows: `piney_world::town::TownSprite`), drawn through
//! piney-effect's `ccEff::Draw` as the town's `Draw` asks for them: each
//! made once from its file as `ccEff::Init(chunk, 1)` leaves it, then for
//! the flares and clouds `SetRenderState(CCRS_ZENABLE, 0)` (no depth test)
//! and PRIM's fog bit off, as `LENSFLARE`'s and `CLOUD`'s constructors do;
//! its place, scale, turn and transparency set before each draw.
//!
//! And the town's `ccEffectCtrl(0)` with `ccThEffect` (80) and
//! `ccThParticle` (98): what the town's characters start in it, the
//! transfer through the Chaos Gate - `effTransfer` from `ccPlayer::AnimCtrl`
//! (Kite arriving, act 13, and leaving, act 12), `ccFellow::Action` (the
//! members), `ccRtownPC` (the walking players) and the Administrator's
//! `sysopeAct` (with his act -5's rings), `effWarpTransfer` for a warp -
//! each following the character's position, drawn after the
//! characters. The starters run after the town's tasks, then the effects
//! step, so their draws from `rand()` come after the frame's others.

use std::collections::HashMap;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_desktop::anm::Ctx;
use piney_effect::draw::Camera as FxCamera;
use piney_effect::eff::Eff;
use piney_effect::files::Assets;
use piney_effect::{CharRef, Effects, F, Host, ONE, V4};
use piney_world::camera::Camera;
use piney_world::town::TownSprite;

/// The effects' files and the sprites made so far, and the town's effects.
pub struct TownFx {
    assets: Assets,
    archive: Arc<Archive>,
    effs: HashMap<(&'static str, &'static str, bool), Option<Eff>>,
    fx: Effects,
}

/// A starter one of the town's tasks called this frame, on a character
/// as [`piney_world::World::fx_chars`] names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    Transfer(CharRef),
    WarpTransfer(CharRef),
    /// The Administrator's act -5: `effSkillExecForceRing(ch, 4, 1)`,
    /// `effSkillExecForceRing2(ch, 4, 1)` and `effSkillTornadeRings(ch, 4,
    /// 3)` (at its place: `ccCheckTarget` holds, he is still listed).
    Vanish(CharRef),
    /// `ccEffPawSmoke(this, speed)`: a running step's dust at Kite's feet,
    /// his heading and speed.
    PawSmoke([V4; 2], u32, u32),
    /// `effSmoke(pos, v, 3.0, 10, 1, 512, 32)`: a running dog's dust
    /// (`ccDog::effect`).
    Smoke(V4, V4),
    /// `effSmoke(pos, v, scale, life, type, 512, 32)`: a Grunty's burp,
    /// its growing up's puffs, a grown one's dust as it runs off and its
    /// last puff (`ccPGuso::burpEff`, `evoEff`, `effect`, `pgDead`).
    Puff {
        pos: V4,
        v: V4,
        scale: F,
        life: i32,
        kind: i32,
    },
    /// `effSmokeN(pos, v, scale, life, type, in, out)`: Carmina Gade's
    /// airship's chimneys (`AIRSHIP::Move`).
    PuffN {
        pos: V4,
        v: V4,
        scale: F,
        life: i32,
        kind: i32,
        fade: (i16, i16),
    },
    /// `effEvolvePG(ch)`: a Grunty growing up (following it).
    Evolve(CharRef),
    /// `effGrowPG(ch)`: a young Grunty becoming a grown one, at its place
    /// and half its height.
    Grow(V4, F),
}

/// What the town's effects read of the town.
struct TownHost<'a> {
    rand: &'a mut piney_world::Rand,
    chars: &'a [(u32, V4, F)],
    player: V4,
    camera: FxCamera,
}

impl TownHost<'_> {
    fn get(&self, c: CharRef) -> Option<&(u32, V4, F)> {
        self.chars.iter().find(|x| x.0 == c)
    }
}

impl Host for TownHost<'_> {
    fn rand(&mut self) -> i32 {
        self.rand.rand()
    }
    fn player_pos(&self) -> V4 {
        self.player
    }
    fn camera(&self) -> FxCamera {
        self.camera
    }
    fn char_pos(&self, c: CharRef) -> V4 {
        self.get(c).map_or([0, 0, 0, ONE], |x| x.1)
    }
    fn char_dirc(&self, _c: CharRef) -> V4 {
        [0; 4]
    }
    fn char_height(&self, c: CharRef) -> F {
        self.get(c).map_or(0, |x| x.2)
    }
    fn char_width(&self, _c: CharRef) -> F {
        0
    }
}

impl TownFx {
    /// The effect files (`effectCCSTbl`) and their tables, and an empty
    /// `ccEffectCtrl(0)` in a town.
    pub fn new(archive: &Arc<Archive>, volume: piney_data::volume::Volume) -> piney_data::Result<TownFx> {
        Ok(TownFx {
            assets: Assets::read(archive, volume)?,
            archive: archive.clone(),
            effs: HashMap::new(),
            fx: Effects::new(archive, volume)?,
        })
    }

    /// The starters of the frame, then `ccThEffect` and `ccThParticle`;
    /// the effects' sounds (`ccSeOn3D`: number and place) back.
    pub fn frame(
        &mut self,
        starts: &[Start],
        chars: &[(u32, V4, F)],
        player: V4,
        camera: &Camera,
        rand: &mut piney_world::Rand,
    ) -> Vec<(i32, V4)> {
        let mut h = TownHost { rand, chars, player, camera: FxCamera::from_field(camera) };
        for s in starts {
            match *s {
                Start::Transfer(c) => {
                    self.fx.transfer(&mut h, c);
                }
                Start::WarpTransfer(c) => {
                    self.fx.warp_transfer(&mut h, c);
                }
                Start::Vanish(c) => {
                    let pos = h.char_pos(c);
                    let (ctrl, mut cx) = self.fx.split(&mut h);
                    piney_effect::skillstart::eff_skill_exec_force_ring(ctrl, &mut cx, c, 4, 1);
                    piney_effect::skillstart::eff_skill_exec_force_ring2(ctrl, &mut cx, c, 4, 1);
                    piney_effect::tornado::eff_skill_tornade_rings_pos(ctrl, &mut cx, pos, 4, 3);
                }
                Start::PawSmoke(feet, dz, speed) => {
                    self.fx.paw_smoke(&mut h, feet, dz, speed);
                }
                Start::Smoke(pos, v) => {
                    self.fx.smoke(&mut h, pos, v, 0x4040_0000, 10, 1, 512, 32);
                }
                Start::Puff { pos, v, scale, life, kind } => {
                    self.fx.smoke(&mut h, pos, v, scale, life, kind, 512, 32);
                }
                Start::PuffN { pos, v, scale, life, kind, fade } => {
                    self.fx.smoke_n(&mut h, pos, v, scale, life, kind, fade.0, fade.1);
                }
                Start::Evolve(c) => {
                    self.fx.evolve_pg(&mut h, c);
                }
                Start::Grow(pos, height) => {
                    self.fx.grow_pg(&mut h, pos, height);
                }
            }
        }
        self.fx.step(&mut h);
        self.fx.step_particles(&mut h);
        self.fx
            .take_events()
            .into_iter()
            .filter_map(|e| match e {
                piney_effect::Event::Sound3d { se, pos } => Some((se, pos)),
                _ => None,
            })
            .collect()
    }

    /// The effects' draws of the frame.
    pub fn draw_effects(&self, camera: &Camera, ctx: &mut Ctx) {
        self.fx.draw(&mut ctx.layers, &FxCamera::from_field(camera));
    }

    /// The `ccEff` of a sprite as its owner's constructor left it.
    fn eff(&mut self, s: &TownSprite) -> Option<Eff> {
        let assets = &mut self.assets;
        let archive = &self.archive;
        self.effs
            .entry((s.file, s.eff, s.depth))
            .or_insert_with(|| {
                assets.add_file(archive, s.file).ok()?;
                let o = assets.find(s.file, s.eff)?;
                let (j, chunk) = assets.eff_chunk(o)?;
                let mut e = Eff::init(o.file, j, chunk, true, &assets.alpha_blend);
                if !s.depth {
                    e.test &= !0x1_0000;
                    e.prim &= !0x20;
                }
                Some(e)
            })
            .clone()
    }

    /// Each sprite's `ccEff::Draw(pos, pattern)` on its layer.
    pub fn draw(&mut self, sprites: &[TownSprite], camera: &Camera, ctx: &mut Ctx) {
        let cam = FxCamera::from_field(camera);
        for s in sprites {
            let Some(mut e) = self.eff(s) else { continue };
            e.pos = s.pos;
            e.scale_x = s.scale[0];
            e.scale_y = s.scale[1];
            e.rotate = s.rotate;
            e.transparency = s.transparency;
            e.render(&self.assets, &mut ctx.layers, s.layer, s.pattern, &cam);
        }
    }
}
