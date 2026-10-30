//! The field's effects (piney-effect's `ccThEffect` and `ccThParticle`) over a
//! field's or dungeon's battle, installed into the world
//! ([`piney_world::field::world::FieldFx`]) and run at their places in the frame
//! (`ccThEffect` 80, `ccThParticle` 98). Each task first starts what the tasks
//! before it asked for ([`Show`], Kite's arrival, a level up, a kill's
//! experience); here the starters run in a batch at the next effect task, so
//! their `rand()` draws come after that task's. Spells run from `ccThSkill`'s
//! walk ([`FxTasks::spell`]); docs/engine/effects.md.

mod ambient;

use std::sync::Arc;

use piney_battle::drain::SideStart as DrainStart;
use piney_battle::event::{Event as Rule, Who};
use piney_battle::scene::Scene;
use piney_battle::{enemy_ai, enemy_motion, entry, fellow, kite};
use piney_data::archive::Archive;
use piney_desktop::anm::Ctx;
use piney_effect::draw::{Camera as FxCamera, DrawRec};
use piney_effect::eff::Eff;
use piney_effect::portal::{CircleFrame, MagicCircle};
use piney_effect::{CharRef, Effects, Host, ONE, V4, flyfont, vu};
use piney_world::combat::{FxTasks, FxWorld, Look, Show, SpellDamage, SpellOut, SpellRun};
use piney_world::field::world::{FieldFx, FxView};
use piney_world::story_map::holy_ground::StorySprite;
use piney_world::town::merchant::SysopEvent;

/// An ambient sprite's `ccEff` by file, name, fog bit, depth test and
/// palette change (`piney_world::field::ambient::Sprite`).
type AmbientKey = (&'static str, &'static str, bool, bool, Option<(&'static str, &'static str)>);

/// The effects of an area and what they draw with.
pub struct AreaFx {
    fx: Effects,
    /// `flyFont`'s texture (`TEX_xasc00`).
    font: piney_draw::TexRef,
    /// A portal made only for its drawing (the palette swap of
    /// `ccEntryChangeCLUT`), and the spark's `ccEff::Init` as it starts.
    circle: Option<(MagicCircle, Eff)>,
    /// The effects' calls into the rest of the game since the runtime
    /// last took them (sounds, shakes, noise).
    pub events: Vec<piney_effect::Event>,
    /// A story map's `ccEff`s by file and name as its constructor made them
    /// (area 15's lens flare, an arena's fireflies, whose file is the
    /// stage's; a flare's render state is its own), and the disc their
    /// file is read from.
    story: std::collections::HashMap<(&'static str, &'static str, bool, bool), Option<Eff>>,
    /// A field's weather's `ccEff`s (and a dungeon's sparks and glows) by
    /// file, name, fog bit, depth test and palette change, and its smoke
    /// puffs waiting for the next effect task ([`ambient`]).
    ambient: std::collections::HashMap<AmbientKey, Option<Eff>>,
    ambient_smoke: Vec<piney_world::field::ambient::Op>,
    archive: Arc<Archive>,
    /// Each character's condition effect (`ccChar` +0x2c), by scene index.
    conditions: std::collections::HashMap<usize, piney_effect::particle::ConditionEffect>,
    /// The skills' own files (`ctu0ski4` ...), read once each, and each
    /// running skill's animation of its file, by the run's key.
    skill_files: std::collections::HashMap<String, Option<std::rc::Rc<piney_desktop::assets::SceneFile>>>,
    skill_anims:
        std::collections::HashMap<u32, (std::rc::Rc<piney_desktop::assets::SceneFile>, piney_world::chara::pose::Play)>,
    /// Quarantine's field staff roll: asked to start (the save's names), and
    /// running.
    ending_asked: Option<piney_desktop::kanji::Names>,
    ending: Option<Box<Ending>>,
    /// `cc3d`'s fog this frame ([`FieldFx::set_fog`]).
    fog: Option<piney_world::town::FogParams>,
}

/// `STFROLL_VOL4` as it runs, and what it draws with.
struct Ending {
    roll: piney_effect::stfroll::StfRoll,
    file: piney_desktop::assets::SceneFile,
    fonts: Option<piney_desktop::kanji::Fonts>,
    names: piney_desktop::kanji::Names,
    /// The logo's textures by name, found once each.
    logos: std::collections::HashMap<&'static str, Option<(piney_draw::TexRef, i32)>>,
    /// The last frame's draws, and what it asked since the world last took
    /// them.
    draws: Vec<piney_effect::stfroll::Draw>,
    asks: Vec<piney_world::field::world::EndingAsk>,
}

impl Ending {
    fn ask(&mut self, asks: Vec<piney_effect::stfroll::Ask>) {
        use piney_effect::stfroll::Ask;
        use piney_world::field::world::EndingAsk;
        self.asks.extend(asks.into_iter().map(|a| match a {
            Ask::Camera { pos, view } => EndingAsk::Camera { pos, view },
            Ask::Bgm(n) => EndingAsk::Bgm(n),
            Ask::BgmStop => EndingAsk::BgmStop,
        }));
    }

    /// The roll's draws as the game sends them: the credits' kanji on layer
    /// 129, the logo's mask on sysLayer, the ring and book (`ccAnm::Draw`
    /// with `ccLayer::active` sysLayer: Kite, hidden, makes no arms effect
    /// to leave it otherwise), all through sysLayer's view (`ccLayer::Init`'s).
    fn draw(&mut self, archive: &Archive, camera: &piney_world::camera::Camera, ctx: &mut Ctx) {
        use piney_effect::stfroll::{Draw, TEXT_LAYER};
        const SYS_LAYER: i16 = 0;
        // sysLayer's `SetFrame(0, 0, 512, 384, 256, 192, 1, 1)`, which the
        // roll's text layer takes too.
        let view = piney_desktop::view::LayerView::default_layer();
        let to_screen = piney_world::draw::screen(&camera.world_screen);
        for d in &self.draws {
            match d {
                Draw::Line { x, y, text, .. } => {
                    let Some(fonts) = &self.fonts else { continue };
                    let mut k = piney_desktop::kanji::Kanji::init(3, 16);
                    k.kt = piney_desktop::kanji::Kt::LargeFixed;
                    let c = piney_data::tables::kanji::shared().sprite_color_table[7];
                    k.colour = [c[0], c[1], c[2], 0x80];
                    k.dx = f32::from_bits(*x);
                    k.dy = f32::from_bits(*y);
                    let raw = piney_data::tables::sjis::encode(text);
                    ctx.disp(fonts, &mut k, TEXT_LAYER, &view, &raw, &self.names);
                }
                Draw::Logo { tex, transp } => {
                    let found = self.logos.entry(tex).or_insert_with(|| {
                        piney_desktop::staffroll::picture_tex(archive, piney_effect::stfroll::CCS, tex)
                    });
                    let Some((t, h)) = found else { continue };
                    let mut m = piney_desktop::sprite::Sprite::mask(SYS_LAYER, t.clone(), *h, 1);
                    m.transp = f32::from_bits(*transp);
                    (m.dx, m.dy, m.sx, m.sy) = (0.0, 0.0, 512.0, 384.0);
                    (m.su, m.sv, m.wu, m.wv, m.wi) = (512, 512, 0, 0, 1);
                    m.make_packet(0, &view);
                    m.send(&mut ctx.layers);
                }
                Draw::Anm { anim, time, matrix, transparency } => {
                    let Some(mut play) = piney_world::chara::pose::Play::new(&self.file, anim.clip()) else { continue };
                    (play.time, play.posed) = (*time, *time);
                    piney_world::town::anim_draw_blended(
                        &mut ctx.layers,
                        SYS_LAYER,
                        to_screen,
                        &self.file,
                        &play,
                        piney_world::draw::mat(matrix),
                        &[],
                        f32::from_bits(*transparency),
                        None,
                        None,
                        piney_world::draw::Fogging::None,
                        None,
                    );
                }
            }
        }
    }
}

impl AreaFx {
    /// `ccThEffect`'s and `ccThParticle`'s set-up, the portal's file added.
    pub fn new(archive: &Arc<Archive>, volume: piney_data::volume::Volume) -> piney_data::Result<AreaFx> {
        let mut fx = Effects::new(archive, volume)?;
        // ccEffectCtrl(0) outside a Root Town (INF 0x001c31bc: game+0x14,
        // the area, not 0) looks its ids up in effectTbl, not effectTbl2.
        fx.ctrl.town = false;
        // The boss effects' file (the WaveShock's animation); on
        // Quarantine Corbenik's file too (its effects' clumps).
        fx.assets.add_file(archive, "xeffect")?;
        if volume == piney_data::volume::Volume::Qua {
            fx.assets.add_file(archive, piney_battle::boss::corbenik::FILE)?;
        }
        let font = flyfont::font_tex(archive)?;
        let circle = fx.assets.add_file(archive, piney_effect::portal::FILE).ok().and_then(|_| {
            let c =
                MagicCircle::new(&fx.assets, [0, 0, 0, ONE], [0; 4], [0, 0, 0, ONE], piney_effect::space::TOWN_BOUNDS)?;
            let spark = fx.assets.find(piney_effect::portal::FILE, piney_effect::portal::SPARK)?;
            let (j, chunk) = fx.assets.eff_chunk(spark)?;
            let eff = Eff::init(spark.file, j, chunk, false, &fx.assets.alpha_blend);
            Some((c, eff))
        });
        Ok(AreaFx {
            fx,
            font,
            circle,
            events: Vec::new(),
            story: Default::default(),
            ambient: Default::default(),
            ambient_smoke: Vec::new(),
            conditions: std::collections::HashMap::new(),
            archive: archive.clone(),
            skill_files: Default::default(),
            skill_anims: Default::default(),
            ending_asked: None,
            ending: None,
            fog: None,
        })
    }

    /// A story map's `ccEff` for a sprite: `Init(chunk, 1)`; for a flare
    /// then `SetRenderState(CCRS_ZENABLE, 0)` (no depth test) and PRIM's
    /// fog bit cleared, as `EVENTAREA02`'s constructor leaves each.
    fn story_eff(&mut self, s: &StorySprite) -> Option<Eff> {
        let assets = &mut self.fx.assets;
        let archive = &self.archive;
        self.story
            .entry((s.file, s.eff, s.flare, s.fog))
            .or_insert_with(|| {
                assets.add_file(archive, s.file).ok()?;
                let o = assets.find(s.file, s.eff)?;
                let (j, chunk) = assets.eff_chunk(o)?;
                let mut e = Eff::init(o.file, j, chunk, true, &assets.alpha_blend);
                if s.flare {
                    e.test &= !0x1_0000;
                    e.prim &= !0x20;
                }
                if !s.fog {
                    e.prim &= !0x20;
                }
                Some(e)
            })
            .clone()
    }

    /// The starters for the shows, in order.
    fn start(&mut self, w: &mut FxWorld) {
        let shows = w.shows;
        let members = w.members;
        let mut host = BattleHost::of(w);
        for s in shows {
            // setConditionEffect, killConditionEffect, deleteConditionEffect
            // on the character's effect.
            if let Show::ConditionEffect { who, act, num } = *s {
                use piney_battle::chara::CondFx;
                let old = self.conditions.remove(&who);
                match (act, old) {
                    // Set leaves an effect it had running, unheld, as the
                    // game overwrites its pointer.
                    (CondFx::Set | CondFx::Replace, old) => {
                        if let (CondFx::Replace, Some(ep)) = (act, old) {
                            self.fx.kill_condition_effect(&mut host, ep);
                        }
                        let ep = self.fx.condition_effect(&mut host, cref(who), num);
                        self.conditions.insert(who, ep);
                    }
                    (CondFx::Kill, Some(ep)) => self.fx.kill_condition_effect(&mut host, ep),
                    (CondFx::Delete, Some(ep)) => self.fx.delete_condition_effect(&mut host, ep),
                    _ => {}
                }
                continue;
            }
            start(&mut self.fx, &mut host, members, s);
        }
        ambient::start_smoke(&mut self.fx, &mut host, &mut self.ambient_smoke);
    }
}

impl FxTasks for AreaFx {
    /// `STFROLL_VOL4` (gcmn 0x004f0950): its thread's first run makes the
    /// object (`Init(0)`); each run after it the main, until it is done
    /// (+0x108), when the tscb's +0x14 is set and it breathes for good.
    fn ending(&mut self, w: &mut FxWorld) {
        if let Some(names) = self.ending_asked.take() {
            let volume = self.fx.assets.volume;
            let file = match piney_desktop::assets::SceneFile::read(&self.archive, piney_effect::stfroll::CCS) {
                Ok(f) => f,
                Err(e) => {
                    tracing::warn!("the staff roll's scene file: {e}");
                    return;
                }
            };
            let t = piney_data::tables::stfroll::of(volume);
            let mut asks = Vec::new();
            let roll = piney_effect::stfroll::StfRoll::new(t, &file, &mut asks);
            let fonts = piney_desktop::assets::read_fonts(volume, &self.archive).ok();
            let mut e =
                Ending { roll, file, fonts, names, logos: Default::default(), draws: Vec::new(), asks: Vec::new() };
            e.ask(asks);
            self.ending = Some(Box::new(e));
            return;
        }
        let Some(e) = self.ending.as_mut() else { return };
        // Done, the thread only breathes: its main draws no more.
        if e.roll.done {
            e.draws.clear();
            return;
        }
        let t = piney_data::tables::stfroll::of(self.fx.assets.volume);
        let mut host = BattleHost::of(w);
        let (ctrl, mut cx) = self.fx.split(&mut host);
        let (draws, asks) = e.roll.main(ctrl, &mut cx, t, &e.file);
        e.draws = draws;
        e.ask(asks);
        self.events.extend(self.fx.take_events());
    }

    fn effect(&mut self, w: &mut FxWorld) {
        self.start(w);
        let mut host = BattleHost::of(w);
        self.fx.step(&mut host);
        self.events.extend(self.fx.take_events());
    }

    fn particle(&mut self, w: &mut FxWorld) {
        self.start(w);
        let mut host = BattleHost::of(w);
        self.fx.step_particles(&mut host);
        self.events.extend(self.fx.take_events());
    }

    /// The spell made at its first system call (`Effects::spell_request`),
    /// synced from the run, its system stepped (its damage calls made
    /// through [`BattleHost`]); the releases go back to the battle, the
    /// rest to the events.
    fn spell(&mut self, w: &mut FxWorld, run: &SpellRun) -> SpellOut {
        let (creator, target) = (run.creator.map(cref), run.target.map(cref));
        let mut host = BattleHost::of(w);
        if self.fx.spells.get(run.key).is_none() {
            self.fx.spell_request(&host, run.key, run.sid, run.stype, creator, target);
        }
        if let Some(s) = self.fx.spells.get_mut(run.key) {
            s.sync(run.count, run.hold, run.c_pos, run.c_dirc, run.t_pos, run.t_type, creator, target);
        }
        let ran = self.fx.spell_system(&mut host, run.key);
        let mut out = SpellOut { ran, ..SpellOut::default() };
        for e in self.fx.take_events() {
            match e {
                piney_effect::Event::SkillRelease { ch } => out.released.push(ch as usize),
                e => self.events.push(e),
            }
        }
        if let Some(s) = self.fx.spells.get(run.key) {
            out.status = s.status;
            out.hold = s.hold;
            out.level = s.level;
        }
        out
    }

    fn spell_remove(&mut self, key: u32) {
        self.fx.spell_remove(key);
        self.skill_anims.remove(&key);
    }

    /// The run's `ANM_<file>` from the skill's file (`GetCCSAdrs(file)`,
    /// `GetChunkAdrsF("ANM_" file)`, `SetAnm(chunk, 0)` at its first
    /// call), then `_AnimateForward(256)` and `NoteProcess` each frame.
    fn skill_anim(&mut self, key: u32, file: &str) -> Option<piney_battle::flow::AnimFrame> {
        if !self.skill_anims.contains_key(&key) {
            let archive = &self.archive;
            let f = self
                .skill_files
                .entry(file.to_string())
                .or_insert_with(|| piney_desktop::assets::SceneFile::read(archive, file).ok().map(std::rc::Rc::new))
                .clone()?;
            let play = piney_world::chara::pose::Play::new(&f, &format!("ANM_{file}"))?;
            self.skill_anims.insert(key, (f, play));
        }
        let (f, play) = self.skill_anims.get_mut(&key)?;
        Some(play.forward_notes(f))
    }

    fn boss_gens_alive(&mut self) -> Option<std::collections::HashSet<u32>> {
        Some(self.fx.corbenik_alive())
    }

    /// `ccThBossEffect`: the starters, then `ccBossEffManager::Draw`.
    fn boss_effects(&mut self, w: &mut FxWorld) {
        self.start(w);
        let mut host = BattleHost::of(w);
        self.fx.boss_step(&mut host);
        self.events.extend(self.fx.take_events());
    }

    /// The boss's `ccBossEff*Create`s (among the starters), then
    /// Fidchell's spells drawn as its rules' pass left them.
    fn boss_shows(&mut self, w: &mut FxWorld) {
        self.start(w);
        // Owned: the host borrows the scene mutably.
        let owned = boss_spells(w.scene);
        let spells: Vec<_> = owned.iter().map(|(k, fx)| (*k, fx)).collect();
        let mut host = BattleHost::of(w);
        self.fx.boss_sync(&mut host, &spells);
        let owned = innis_flights(host.w.scene);
        let flights: Vec<_> = owned.iter().map(|(k, el, no, f)| (*k, *el, *no, f)).collect();
        self.fx.innis_sync(&mut host, &flights);
        let k = kyvia_shows(host.w.scene);
        let meteors: Vec<_> = k.meteors.iter().map(|(s, f)| (*s, f)).collect();
        let bolts: Vec<_> = k.bolts.iter().map(|(s, b)| (*s, b)).collect();
        let balls: Vec<_> = k.balls.iter().map(|(s, b)| (*s, b)).collect();
        let hells: Vec<_> = k.hells.iter().map(|(s, h)| (*s, h)).collect();
        let rules = piney_effect::boss::KyviaRules { meteors: &meteors, bolts: &bolts, balls: &balls, hells: &hells };
        self.fx.kyvia_sync(&mut host, &rules, &k.members);
        let m = magus_shows(host.w.scene);
        let needles: Vec<_> = m.needles.iter().map(|(s, n)| (*s, n)).collect();
        self.fx.magus_sync(&mut host, &needles, &m.bursts, &m.markers);
        self.events.extend(self.fx.take_events());
    }
}

impl AreaFx {
    /// The object a draw drew (`eff N` for a sprite of chunk N).
    fn draw_name(&self, d: &DrawRec) -> String {
        match d {
            DrawRec::Clump { obj, .. } | DrawRec::Anm { obj, .. } => self.fx.assets.name(*obj).to_string(),
            DrawRec::Eff { eff, .. } => format!("eff {}", eff.chunk),
        }
    }
}

impl FieldFx for AreaFx {
    fn set_fog(&mut self, fog: Option<piney_world::town::FogParams>) {
        self.fog = fog;
    }
    fn tasks(&mut self) -> &mut dyn FxTasks {
        self
    }

    fn ending_start(&mut self, names: piney_desktop::kanji::Names) {
        self.ending_asked = Some(names);
    }

    fn ending_running(&self) -> bool {
        self.ending_asked.is_some() || self.ending.as_ref().is_some_and(|e| !e.roll.done)
    }

    fn take_ending_asks(&mut self) -> Vec<piney_world::field::world::EndingAsk> {
        self.ending.as_mut().map(|e| std::mem::take(&mut e.asks)).unwrap_or_default()
    }

    fn census(&self) -> piney_world::field::world::FxCensus {
        piney_world::field::world::FxCensus {
            effects: self.fx.ctrl.effects.iter().filter(|e| e.status != 0).map(|e| e.id).collect(),
            effect_targets: (self.fx.ctrl.effects.iter()).filter(|e| e.status != 0).map(|e| (e.id, e.target)).collect(),
            effects_unseen: (self.fx.ctrl.effects.iter())
                .filter(|e| e.status != 0 && (!e.disp_sw || e.cnt == 0))
                .map(|e| e.id)
                .collect(),
            shields: (self.fx.spells.elements.slots.iter().flatten())
                .filter_map(|e| match e {
                    piney_effect::element::Element::Shield(s) => {
                        Some((s.target, s.base.anim.scale, s.base.anim.offset))
                    }
                    _ => None,
                })
                .collect(),
            particles: self.fx.particles.live().count(),
            particle_draws: self.fx.particle_draws().len(),
            weapon_generators: (self.fx.particles.gens.iter())
                .filter(|g| matches!(g.sync_sw, Some(piney_effect::IntRef::CharAt(_, piney_effect::ARMS_SW))))
                .count(),
            effsw_generators: (self.fx.particles.gens.iter())
                .filter(|g| g.kill_flag < 2)
                .filter(|g| {
                    matches!(g.sync_sw, Some(piney_effect::IntRef::CharAt(_, piney_effect::gimmick::IDOL_EFFSW)))
                })
                .count(),
            char_generators: (self.fx.particles.gens.iter())
                .filter(|g| g.kill_flag < 2)
                .filter_map(|g| match g.sync_pos {
                    Some(piney_effect::VecRef::CharPos(c)) => Some(c),
                    _ => None,
                })
                .collect(),
            fly_fonts: self
                .fx
                .dam
                .nodes
                .iter()
                .flat_map(|n| n.uproll.lines.iter())
                .filter(|l| l.alpha > 0 && !l.text.is_empty())
                .map(|l| l.text.clone())
                .collect(),
            new_fly_fonts: (self.fx.dam.nodes.iter())
                .filter_map(|n| n.ch.map(|c| (c, &n.uproll)))
                .flat_map(|(c, u)| u.lines.iter().filter(|l| l.alpha_cnt == 23).map(move |l| (c, l.text.clone())))
                .collect(),
            anchored_generators: (self.fx.particles.gens.iter())
                .filter(|g| g.kill_flag < 2)
                .filter(|g| matches!(g.sync_pos, Some(piney_effect::VecRef::Anchor(_))))
                .count(),
            boss_draws: self.fx.boss_draws().iter().map(|d| self.draw_name(d)).collect(),
            effect_draws: (self.fx.draws().iter()).chain(self.fx.particle_draws()).map(|d| self.draw_name(d)).collect(),
        }
    }

    fn draws_circles(&self) -> bool {
        self.circle.is_some()
    }

    fn take_sounds(&mut self) -> Vec<(i32, Option<V4>, Option<i8>)> {
        let mut out = Vec::new();
        self.events.retain(|e| match *e {
            piney_effect::Event::Sound3d { se, pos } => {
                out.push((se, Some(pos), None));
                false
            }
            piney_effect::Event::Sound3dNote { se, pos, note } => {
                out.push((se, Some(pos), Some(note as i8)));
                false
            }
            piney_effect::Event::SoundNote { se, note } => {
                out.push((se, None, Some(note as i8)));
                false
            }
            piney_effect::Event::Sound { se } => {
                out.push((se, None, None));
                false
            }
            // The flashes, shakes and noise wait for their own takers.
            piney_effect::Event::Flash { .. }
            | piney_effect::Event::CameraShake(_)
            | piney_effect::Event::Noise { .. } => true,
            _ => false,
        });
        out
    }

    fn take_shakes(&mut self) -> Vec<[i32; 4]> {
        let mut out = Vec::new();
        self.events.retain(|e| match *e {
            piney_effect::Event::CameraShake(s) => {
                out.push(s);
                false
            }
            _ => true,
        });
        out
    }

    fn lights(&self) -> Vec<piney_world::town::Light> {
        self.fx.boss.omni_lights()
    }

    fn take_noises(&mut self) -> Vec<i32> {
        let mut out = Vec::new();
        self.events.retain(|e| match *e {
            piney_effect::Event::Noise { bs } => {
                out.push(bs);
                false
            }
            _ => true,
        });
        out
    }

    fn take_flashes(&mut self) -> Vec<(i32, u32)> {
        let mut out = Vec::new();
        self.events.retain(|e| match *e {
            piney_effect::Event::Flash { time, color, .. } => {
                out.push((time, color));
                false
            }
            _ => true,
        });
        out
    }

    /// The effects' draws of the frame, the portals, then the numbers
    /// (`ccMenuCtrl::Disp`: `ccCtrlFlyFont`, `ccDamUprStr`, sent on the
    /// menu layer before the menu's own).
    fn draw(&mut self, view: &FxView, ctx: &mut Ctx) {
        let camera = FxCamera::from_field(view.camera).fogged(self.fog);
        self.fx.draw(&mut ctx.layers, &camera);
        if let Some((mc, spark)) = &self.circle {
            let frame = circle_frame(view, mc, spark);
            mc.render(&frame, &self.fx.assets, &mut ctx.layers, &camera);
        }
        let host = ViewHost { view, camera };
        self.fx.fly_fonts(&host, false, false);
        flyfont::send(&self.fx.font.take(), &self.font, &mut ctx.layers);
        if let Some(e) = &mut self.ending {
            e.draw(&self.archive, view.camera, ctx);
        }
    }

    /// A story map's sprites (`EVENTAREA02::DrawLensFlare`,
    /// `EVENTAREAB0`'s fireflies): each `ccEff::Draw(pos, pat)` on its
    /// layer.
    fn story_sprites(&mut self, sprites: &[StorySprite], camera: &piney_world::camera::Camera, ctx: &mut Ctx) {
        let cam = FxCamera::from_field(camera).fogged(self.fog);
        for s in sprites {
            let Some(mut e) = self.story_eff(s) else { continue };
            e.pos = s.pos;
            e.scale_x = s.scale;
            e.scale_y = s.scale;
            e.rotate = s.rotate;
            e.transparency = s.transparency;
            e.render(&self.fx.assets, &mut ctx.layers, s.layer, s.pat, &cam);
        }
    }

    fn field_ambient(
        &mut self,
        ops: &[piney_world::field::ambient::Op],
        fresh: bool,
        camera: &piney_world::camera::Camera,
        ctx: &mut Ctx,
    ) {
        ambient::draw(self, ops, fresh, camera, ctx);
    }
}

/// What the portals drew this frame, as `ccMagicCircle::main` sent it
/// (piney-battle's `CircleDraw` and `PartDraw`): the circle's animation at
/// `SetMatrix_PosRotZYX(pos, dirc)`, then each live spark.
fn circle_frame(view: &FxView, mc: &MagicCircle, spark: &Eff) -> CircleFrame {
    let mut frame = CircleFrame::default();
    for who in view.ctrl.list(entry::Kind::Circle) {
        let Some(a) = view.cast.get(who).filter(|a| a.drawn && a.look == Look::Circle) else { continue };
        let Some(entry::Obj::Circle(c)) = view.ctrl.objs.get(who) else { continue };
        let pos = view.scene.chars[who].pos;
        let matrix = vu::trans(&vu::rot_zyx(&vu::UNIT, c.obj.dirc), pos);
        frame.draws.push(DrawRec::Anm {
            obj: mc.clump(),
            play: a.ch.play.clone(),
            matrix,
            alpha: a.alpha,
            layer: piney_effect::effect::EFFECT_LAYER,
            clut: Vec::new(),
            // ccMagicCircle's constructor: SetFogSw(0) on its clump.
            fog: false,
        });
        for p in c.parts.iter().filter(|p| p.status != 0) {
            let mut e = spark.clone();
            e.pos = p.eff.pos;
            e.scale_x = p.eff.scale_x;
            e.scale_y = p.eff.scale_y;
            e.rotate = p.eff.rotate;
            e.color = p.eff.color;
            e.transparency = p.eff.transparency;
            frame.draws.push(DrawRec::Eff {
                eff: Box::new(e),
                pat: p.eff_anm_pat,
                layer: piney_effect::effect::EFFECT_LAYER,
            });
        }
    }
    frame
}

/// Fidchell's spells in a boss's effect slots (a boss still in its task):
/// the slot and the rules' state.
fn boss_spells(scene: &Scene) -> Vec<(i32, piney_battle::boss::fidchell::eff::Fx)> {
    let mut out = Vec::new();
    for ch in &scene.chars {
        let Some(b) = ch.foe_state().and_then(|f| f.boss.as_ref()).filter(|b| b.base.exit == 0) else { continue };
        for (k, e) in b.base.effects.slots.iter().enumerate() {
            if let Some(fx) = e.as_ref().and_then(|e| e.fidchell.as_deref()) {
                out.push((k as i32, fx.clone()));
            }
        }
    }
    out
}

/// Innis's missiles in the fight's boss effects: their slot, element,
/// number and flight as the last `Draw` left it.
type Flight = (i32, piney_battle::boss::innis::Element, i32, piney_battle::boss::innis::Missile);

fn innis_flights(scene: &Scene) -> Vec<Flight> {
    let mut out = Vec::new();
    for ch in &scene.chars {
        let Some(b) = ch.foe_state().and_then(|f| f.boss.as_ref()).filter(|b| b.base.exit == 0) else { continue };
        for (k, e) in b.base.effects.slots.iter().enumerate() {
            let Some(e) = e.as_ref() else { continue };
            if let (piney_battle::boss::EffKind::Missile { element, no, .. }, Some(m)) = (e.kind, e.missile.as_deref())
            {
                out.push((k as i32, element, no, m.clone()));
            }
        }
    }
    out
}

/// What Kyvia's fight left this frame for its pictures: its meteors and
/// bolts by their slot, and the places its generators follow.
#[derive(Default)]
struct KyviaShows {
    meteors: Vec<(i32, piney_battle::boss::kyvia::Meteorite)>,
    bolts: Vec<(i32, piney_battle::boss::kyvia::thunder::Bolt)>,
    balls: Vec<(i32, piney_battle::boss::kyvia::magic_ball::MagicalBall)>,
    hells: Vec<(i32, piney_battle::boss::kyvia::hell::Hell)>,
    members: Vec<(piney_battle::boss::kyvia::Follow, V4)>,
}

fn kyvia_shows(scene: &Scene) -> KyviaShows {
    use piney_battle::boss::Class;
    use piney_battle::boss::kyvia::Follow;
    let mut out = KyviaShows::default();
    for (c, ch) in scene.chars.iter().enumerate() {
        let Some(b) = ch.foe_state().and_then(|f| f.boss.as_ref()).filter(|b| b.base.exit == 0) else { continue };
        match &b.class {
            Class::Kyvia(x) => {
                out.members.push((Follow::DeadSmoke, x.ded_dmg_vec));
                for k in 0..3u8 {
                    out.members.push((Follow::BeamHead(k), x.l_bpos[usize::from(k)]));
                    out.members.push((Follow::BeamTail(k), x.l_bpos_sub[usize::from(k)]));
                }
            }
            Class::Gomora(g) => out.members.push((Follow::Aura(c), g.aura_pos)),
            _ => {}
        }
        for (k, e) in b.base.effects.slots.iter().enumerate() {
            let Some(e) = e.as_ref() else { continue };
            if let Some(m) = e.meteorite.as_deref() {
                out.meteors.push((k as i32, m.clone()));
            }
            if let Some(bolt) = e.bolt.as_deref() {
                out.bolts.push((k as i32, bolt.clone()));
            }
            if let Some(ball) = e.ball.as_deref() {
                out.balls.push((k as i32, ball.clone()));
            }
            if let Some(h) = e.hell.as_deref() {
                out.hells.push((k as i32, h.clone()));
            }
        }
    }
    out
}

/// What Magus's fight left this frame for its pictures: its needles by
/// their slot, the bursts of the leaves bursting (act 21, their
/// `PreDrawAnm`), and each leaf's marker's place.
#[derive(Default)]
struct MagusShows {
    needles: Vec<(i32, piney_battle::boss::magus::Needle)>,
    bursts: Vec<piney_battle::boss::magus::leaf::Explode>,
    markers: Vec<(usize, V4)>,
}

fn magus_shows(scene: &Scene) -> MagusShows {
    use piney_battle::boss::Class;
    let mut out = MagusShows::default();
    for (c, ch) in scene.chars.iter().enumerate() {
        let Some(b) = ch.foe_state().and_then(|f| f.boss.as_ref()).filter(|b| b.base.exit == 0) else { continue };
        match &b.class {
            Class::MagusLeaf(l) => {
                out.markers.push((c, l.marker_pos));
                if b.base.act_num == piney_battle::boss::magus::leaf::act::FIRE {
                    out.bursts.push(l.exp);
                }
            }
            Class::Magus(_) => {
                for (k, e) in b.base.effects.slots.iter().enumerate() {
                    if let Some(n) = e.as_ref().and_then(|e| e.needle.as_deref()) {
                        out.needles.push((k as i32, n.clone()));
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Magus's laser's shock `k` as its `PreDrawAnm` drew it: where its beam
/// met the ground, at its transparency; None when it missed.
fn magus_shock(scene: &Scene, me: usize, k: usize) -> Option<(V4, piney_effect::ee::F)> {
    match &scene.chars[me].foe_state()?.boss.as_ref()?.class {
        piney_battle::boss::Class::Magus(x) => Some((*x.shock_at.get(k)?.as_ref()?, x.shocks.get(k)?.transparency)),
        _ => None,
    }
}

/// A scene index as the effects name a character.
fn cref(i: usize) -> CharRef {
    i as CharRef
}

/// One show's starter.
fn start(fx: &mut Effects, h: &mut BattleHost, members: &[(i32, usize)], s: &Show) {
    match s {
        Show::Rule(e) => event(fx, h, None, e),
        Show::Kite(k, kite::Out::Rule(e)) => event(fx, h, Some(*k), e),
        Show::Kite(k, kite::Out::Transfer) => {
            fx.transfer(h, cref(*k));
        }
        Show::Kite(k, kite::Out::WarpTransfer) => {
            fx.warp_transfer(h, cref(*k));
        }
        // ccFellow::Action's (a member arriving or leaving).
        Show::Member(m, fellow::Out::Transfer) => {
            fx.transfer(h, cref(*m));
        }
        Show::Member(m, fellow::Out::WarpTransfer) => {
            fx.warp_transfer(h, cref(*m));
        }
        Show::Kite(k, kite::Out::LevelUp) | Show::Member(k, fellow::Out::LevelUp) => {
            fx.level_up(h, cref(*k));
        }
        // ccEffPawSmoke(this, speed): Kite's and the members' running dust,
        // at the lower of the feet as the actor stands posed now.
        Show::Kite(k, kite::Out::PawSmoke { speed }) | Show::Member(k, fellow::Out::PawSmoke { speed }) => {
            if let Some(a) = h.w.cast.get(*k) {
                let worlds = a.ch.body.worlds(&a.ch.play, a.ch.root());
                let foot = |name: &str| {
                    let t = worlds.get(&a.ch.body.node(name)?)?.w_axis;
                    Some([t.x.to_bits(), t.y.to_bits(), t.z.to_bits(), ONE])
                };
                if let (Some(l), Some(r)) = (foot("OBJ_t0 l foot"), foot("OBJ_t0 r foot")) {
                    let dz = a.ch.dirc[2];
                    fx.paw_smoke(h, [l, r], dz, *speed);
                }
            }
        }
        Show::Member(m, fellow::Out::Rule(e)) => event(fx, h, Some(*m), e),
        // effSkillStart: _ccSkillRequest's (on the target, or an item's on
        // the caster), ccPlayer::AnimCtrl's, ccFellow::Action's and an
        // enemy's startSkill.
        Show::SkillStart { who, sid, a, b } => {
            fx.skill_start(h, cref(*who), *sid, *a, *b);
        }
        Show::ShockWave { pos, attr } => {
            fx.shock_wave(h, *pos, *attr);
        }
        Show::Kite(k, kite::Out::SkillStart { sid, item }) => {
            fx.skill_start(h, cref(*k), *sid, *item, 0);
        }
        Show::Member(m, fellow::Out::SkillStart { sid, flag }) => {
            fx.skill_start(h, cref(*m), i32::from(*sid), *flag, 0);
        }
        // ccSpcChar::StartArmsEffect's particles along the weapon.
        Show::Kite(who, kite::Out::ArmsParticles { a, b, attr: row }) | Show::ArmsParticles { who, a, b, row } => {
            fx.arms_particles(h, cref(*who), *a, *b, *row);
        }
        Show::Enemy(e, enemy_motion::Call::Rule(enemy_ai::Out::SkillStart(Some(r)))) => {
            let ene_id = h.w.foes.get(*e).and_then(Option::as_ref).map_or(-1, |f| f.ene_id);
            if let Some(p) = r.get(h.w.t, ene_id) {
                let normal = matches!(r, enemy_ai::SkillRef::Table(1));
                fx.skill_start_param(h, cref(*e), p.ty, normal, 0, 0);
            }
        }
        Show::Enemy(e, enemy_motion::Call::Rule(enemy_ai::Out::Rule(r))) => event(fx, h, Some(*e), r),
        Show::Entry(entry::Out::AfterDrain { who }) => {
            fx.after_drain(h, cref(*who), -1);
        }
        // A box or an idol opens (ccGimBox::boxMain, ccGimIdol::main).
        Show::Entry(entry::Out::OpenBox { pos }) => fx.open_box(h, *pos),
        // The Fortune Wire's trap taken off, the event's remove_trap, the
        // Zeit statue opened: effRemoveTrap(pos, -1, -1).
        Show::Entry(entry::Out::TrapRemoved { pos }) => {
            fx.remove_trap(h, *pos, -1, -1);
        }
        // A dead enemy's treasure box: effRemoveTrap(pos, 103, 121).
        Show::Entry(entry::Out::RemoveTrap { pos }) => {
            fx.remove_trap(h, *pos, 103, 121);
        }
        // ccGimBox::breakObject: the breakable's pieces and dust.
        Show::Entry(entry::Out::Crush { what, pos }) => {
            if let Some(kind) = piney_effect::gimmick::Fragment::of_crush(*what) {
                fx.crush(h, kind, *pos, 0);
            }
        }
        // ccGimBox::invokeTrap: the trapped box going off.
        Show::Entry(entry::Out::OpenTrapBox { pos, kind, trap }) => {
            fx.open_trap_box(h, *pos, *kind, *trap);
        }
        // ccGimBox::virusMain: the virus core's crystal broken.
        Show::Entry(entry::Out::VirusCrystal { pos }) => {
            fx.virus_crystal(h, *pos);
        }
        // ccGimIdol::main: the Statue of God's glow, on the idol's effsw.
        Show::Entry(entry::Out::StatueOfGod { who, pos }) => fx.statue_of_god(h, *pos, cref(*who)),
        // ccGimEtc::main's first frame: the spring's mist, the warning's
        // glow (row 21's entrance effect is never placed in Infection).
        Show::Entry(entry::Out::EtcEffect { who, row: 20, pos }) => fx.fountain(h, *pos, cref(*who)),
        Show::Entry(entry::Out::EtcEffect { who, row: 19, pos }) => fx.boss_room_entrance(h, *pos, cref(*who)),
        Show::Entry(entry::Out::EtcEffect { who, row: 21, pos }) => fx.dungeon_entrance(h, *pos, cref(*who), 2),
        // ctrlFountain's ccEnemyEffDustRing(pos, 3.0, 30.0, 4, 35, 110).
        Show::Entry(entry::Out::DustRingAt { pos, s, r, n, life, tex }) => {
            fx.dust_ring(h, *pos, *s, *r, *n, *life, *tex);
        }
        // ccGimSymbol::symMain: effUseSymbol at its light as it casts, and
        // invokeEff's spark each frame of the cast.
        Show::Entry(entry::Out::UseSymbol { pos }) => fx.use_symbol(h, *pos),
        Show::Entry(entry::Out::SymbolSpark { pos, v }) => fx.particle_explode(h, *pos, *v, 0x3fc0_0000, 1),
        // ccPucciguso::PawSmoke: the riding Grunty's dust at a leg.
        Show::Ride(piney_battle::ride::Out::Smoke { pos, v, s, life, t }) => {
            fx.smoke(h, *pos, *v, *s, *life, *t, 512, 32);
        }
        // ccGimSymbol::objMain: a lake symbol's puff every other frame.
        Show::Entry(entry::Out::SymbolSmoke { pos, v }) => {
            fx.smoke(h, *pos, *v, 0x3f80_0000, 8, 204, 512, 32);
        }
        // ccGimIdol::main from frame 100: ccEnemyEffDustRing(idol, ofs, 4.0,
        // 100.0, 8, 30, 132).
        Show::Entry(entry::Out::DustRing { who, ofs }) => {
            let pos = char_pos(h.w.scene, cref(*who));
            let dirc = h.w.ctrl.entry_obj(*who).map_or([0; 4], |o| o.dirc);
            fx.enemy_dust_ring(h, pos, dirc, *ofs, 0x4080_0000, 0x42c8_0000, 8, 30, 132);
        }
        // An enemy's ccEnemyDustCtrl: a ring of dust at its feet.
        Show::DustRing { pos, dirc, ofs, s, r, n, life, tex } => {
            fx.enemy_dust_ring(h, *pos, *dirc, *ofs, *s, *r, *n, *life, *tex);
        }
        // A gold goblin running: ccTransPosFW2LW, then ccEnemyEffDust(p, 1,
        // size, 6, eneSmoke).
        Show::Enemy(_, enemy_motion::Call::Dust { pos, size, smoke }) => {
            let p = piney_effect::space::fw2lw(*pos, h.player_pos(), h.bounds());
            fx.enemy_dust(h, p, 1, *size, 6, i32::from(*smoke));
        }
        // A scorpion spinning: ccEnemyEffDust(wheel, 2, 3, 20, eneSmoke).
        Show::Enemy(_, enemy_motion::Call::EffDust { pos, n, size, life, smoke }) => {
            fx.enemy_dust(h, *pos, *n, *size, *life, i32::from(*smoke));
        }
        Show::Exp(g) if g.in_party => {
            if let Some(&(_, m)) = members.iter().find(|m| m.0 == g.id) {
                fx.fly_font_exp(Some(cref(m)), 19, g.exp);
            }
        }
        // An item's recovery (ccUseItemRequest): effHeal(ch, n).
        Show::Heal { who, n } => {
            fx.heal(h, cref(*who), *n);
        }
        // Data Drain's side effect (DataDrainMenu step 10), as the menu
        // starts each.
        Show::DrainSide(st) => match *st {
            DrainStart::Heal(c) => {
                fx.heal(h, cref(c), 1);
            }
            DrainStart::Miss(c) => {
                fx.fly_font_miss(Some(cref(c)));
            }
            DrainStart::Effect(c) => {
                fx.skill_start_effect(h, cref(c), 0, 1);
            }
            DrainStart::Exp { who, value } => {
                fx.fly_font_exp(Some(cref(who)), 23, value);
            }
            DrainStart::AfterDrain(c) => {
                fx.after_drain(h, cref(c), 0);
            }
        },
        // Step 11's frame 30: LEVEL DOWN over Kite.
        Show::DrainLevelDown(k) => {
            fx.fly_font_level_down(Some(cref(*k)), 23);
        }
        // Magus's leaves' generators and what they do to those they keep;
        // the laser's shocks and their bursts where the beams met the
        // ground.
        Show::Boss(
            _,
            piney_battle::boss::Out::Magus(piney_battle::boss::magus::Pic::Particles { which, at, held }),
        ) => {
            fx.magus_particles(h, *which, *at, *held);
        }
        Show::Boss(_, piney_battle::boss::Out::Magus(piney_battle::boss::magus::Pic::Gen { held, op })) => {
            fx.magus_gen_op(h, *held, *op);
        }
        Show::Boss(me, piney_battle::boss::Out::Magus(piney_battle::boss::magus::Pic::Shocks { upto })) => {
            let shocks: Vec<_> = (0..=*upto).filter_map(|k| magus_shock(h.w.scene, *me, k)).collect();
            fx.magus_shocks(h, &shocks);
        }
        Show::Boss(me, piney_battle::boss::Out::Magus(piney_battle::boss::magus::Pic::ShockBurst { shock, turn })) => {
            if let Some((at, _)) = magus_shock(h.w.scene, *me, *shock) {
                fx.magus_shock_burst(h, at, *turn);
            }
        }
        // A leaf of Magus's lands: ccEnemyEffDust(pos, 30, 4.0).
        Show::Boss(_, piney_battle::boss::Out::Magus(piney_battle::boss::magus::Pic::Dust { pos })) => {
            let (life, tex) = (piney_effect::dust::DUST_LIFE, piney_effect::dust::DUST_TEX);
            fx.enemy_dust(h, *pos, 30, 0x4080_0000, life, tex);
        }
        // The boss's ccBossEff*Create (those the effects have).
        Show::Boss(_, piney_battle::boss::Out::Effect { id, kind, pos, dirc }) => {
            if let Some(m) = boss_make(*id, *kind, *pos, *dirc) {
                fx.boss_create(h, m);
            }
        }
        // Innis's and its images' particle generators.
        Show::Boss(_, piney_battle::boss::Out::Particles { which, pos }) => {
            fx.innis_particles(h, *which, *pos);
        }
        // Kyvia's fight's generators (the meteors' and bolts' own come with
        // their pictures, [`kyvia_shows`]), and what it does to those it
        // keeps.
        Show::Boss(_, piney_battle::boss::Out::KyviaParticles { which, at, held }) => {
            use piney_battle::boss::kyvia::Gen;
            if !matches!(which, Gen::Trail(_) | Gen::Burst(_)) {
                fx.kyvia_particles(h, *which, *at, *held);
            }
        }
        Show::Boss(_, piney_battle::boss::Out::KyviaGen { held, op }) => {
            fx.kyvia_gen_op(h, *held, *op);
        }
        // A boss's Affect on a blow it resists: effResistantShield(this,
        // magic, -1), a Gorre brother's reshaped.
        Show::Boss(me, piney_battle::boss::Out::Shield { magic, stretched }) => {
            if *stretched {
                fx.brother_shield(h, cref(*me), *magic);
            } else {
                fx.resistant_shield(h, cref(*me), *magic, -1);
            }
        }
        // What Fidchell's rules ask beside: the back dash's and a tower's
        // dust, a meteor's landing, a tower's and a strike's rocks.
        Show::Boss(_, piney_battle::boss::Out::Fidchell(p)) => fx.fidchell_calls(h, p),
        // Macha's lasers, blast and their bursts, the back dash's dust.
        Show::Boss(_, piney_battle::boss::Out::Macha(p)) => fx.macha_calls(h, p),
        // Tarvos's spikes, phantom, transfer, eruptions and shield.
        Show::Boss(me, piney_battle::boss::Out::Tarvos(p)) => fx.tarvos_calls(h, cref(*me), p),
        // Corbenik's generators, its seeds' charges, bursts, shields,
        // smoke and radiates.
        Show::Boss(_, piney_battle::boss::Out::Corbenik(p)) => fx.corbenik_calls(h, &cref, p),
        // Tarvos's spell's ccNoiz::SetNoizBs.
        Show::Boss(_, piney_battle::boss::Out::Noise(bs)) => fx.noise(h, *bs),
        // effSkillStart(who, sid, 0, 0): Fidchell's skill, a gomora's.
        Show::Boss(_, piney_battle::boss::Out::SkillStart { who, sid }) => {
            fx.skill_start(h, cref(*who), *sid, 0, 0);
        }
        // An event NPC's effTransfer as it comes or goes (a PC's act 4,
        // the Administrator's sysopeAct); his act -5's
        // effSkillExecForceRing(this, 4, 1),
        // effSkillExecForceRing2(this, 4, 1) and effSkillTornadeRings(this,
        // 4, 3) at his place.
        Show::Npc(who, SysopEvent::Transfer) => {
            fx.transfer(h, cref(*who));
        }
        Show::Npc(who, SysopEvent::Vanish) => {
            let c = cref(*who);
            let pos = char_pos(h.w.scene, c);
            let (ctrl, mut cx) = fx.split(h);
            piney_effect::skillstart::eff_skill_exec_force_ring(ctrl, &mut cx, c, 4, 1);
            piney_effect::skillstart::eff_skill_exec_force_ring2(ctrl, &mut cx, c, 4, 1);
            piney_effect::tornado::eff_skill_tornade_rings_pos(ctrl, &mut cx, pos, 4, 3);
        }
        _ => {}
    }
}

/// A `ccBossEff*Create` as the bosses call it (`docs/engine/boss.md`,
/// "Effects"): the rules name the effect, where and which way, in their
/// slot `id`. Fidchell's meteors, thunders and rock towers, Innis's
/// missiles, Kyvia's meteors and bolts and Magus's needles are drawn from
/// their rules' state ([`boss_spells`], [`innis_flights`], [`kyvia_shows`],
/// [`magus_shows`]); the rest, Gorre's flash among them, run their own.
fn boss_make(id: i32, kind: piney_battle::boss::EffKind, pos: V4, dirc: V4) -> Option<piney_effect::boss::Make> {
    use piney_battle::boss::EffKind;
    use piney_effect::boss::Make;
    const TEN: u32 = 0x4120_0000;
    Some(match kind {
        EffKind::WaveShock => Make::WaveShock { pos, dirc, scale: ONE },
        EffKind::WaveShockScaled { scale } => Make::WaveShock { pos, dirc, scale },
        EffKind::MagicSquare { n } => Make::MagicSquare { pos, n },
        EffKind::ForceGenerator { num, life, speed, r0, r1, clt } => {
            Make::ForceGenerator { p: pos, rot: dirc, speed, r0, r1, num, life, clt }
        }
        EffKind::AutoSamonRing { n } => Make::AutoSamonRing { pos, rot: dirc, param: [0, 0x3eaa_aaab, TEN, 0], n },
        EffKind::AutoSamonRingOf { n, param } => Make::AutoSamonRing { pos, rot: dirc, param, n },
        EffKind::IceBreak => Make::IceBreak { pos, scale: 0x4000_0000 },
        EffKind::Dead => Make::Dead { pos },
        // Magus's leaf's ring (model 195, DeadEffect's parameter).
        EffKind::LeafRing => Make::AutoSamonRing { pos, rot: dirc, param: [0x3f00_0000, ONE, 0, 0x4220_0000], n: 195 },
        EffKind::SamonRing { model, scale, spoint, tpoint } => {
            Make::SamonRing { pos, rot: dirc, scale, model, spoint, tpoint }
        }
        EffKind::Missile { element, no, .. } => Make::Missile { rules_slot: id, element, no },
        EffKind::Meteorite { count } => Make::Meteorite { rules_slot: id, n: count },
        EffKind::Thunderbolt { .. } => Make::Bolt { rules_slot: id },
        EffKind::MagicalBall => Make::Ball { rules_slot: id },
        EffKind::HellExpress => Make::Hell { rules_slot: id },
        EffKind::Needle { .. } => Make::Needles { rules_slot: id },
        EffKind::FinalPhotonFlash => Make::FinalPhotonFlash { pos },
        // Macha's lasers and blast, Tarvos's spikes, phantom and eruption,
        // Corbenik's own: drawn from their rules' state.
        EffKind::MeteoSworm { .. }
        | EffKind::ThunderStorm { .. }
        | EffKind::RockTower { .. }
        | EffKind::LaserShot
        | EffKind::LightBallShock
        | EffKind::SpikeRain
        | EffKind::Phantom
        | EffKind::Eruption
        | EffKind::CorbenikWave
        | EffKind::Flush
        | EffKind::StarDustFlash
        | EffKind::Transform
        | EffKind::Thunder3
        | EffKind::RootDrain
        | EffKind::TeraLaser
        | EffKind::RefrectionLaser => {
            return None;
        }
    })
}

/// A rule's event's starter (`docs/engine/effects.md`, "From
/// piney-battle's events"); `me` is whose code raised it.
fn event(fx: &mut Effects, h: &mut BattleHost, me: Option<usize>, e: &Rule) {
    let r = |w: Who| match w {
        Who::Char(c) => Some(cref(c)),
        Who::Me => me.map(cref),
        Who::Target | Who::Nobody => None,
    };
    match *e {
        Rule::HitMark { on, by } => {
            if let (Some(a), Some(b)) = (r(on), r(by)) {
                fx.hit_mark(h, a, b);
            }
        }
        Rule::FlyFont { on, kind, value } => {
            fx.fly_font(r(on), kind, value);
        }
        Rule::Protect { on, broken, kind } => {
            if let Some(c) = r(on) {
                fx.protect(h, c, broken, kind);
            }
        }
        Rule::AttributeGuard { on, by } => {
            if let (Some(a), Some(b)) = (r(on), r(by)) {
                fx.attribute_guard(h, a, b);
            }
        }
        Rule::AttributeCriticalParticle(on) => {
            if let Some(c) = r(on) {
                fx.attribute_critical(h, c);
            }
        }
        Rule::Critical(on) => {
            if let Some(c) = r(on) {
                fx.critical(h, c);
            }
        }
        Rule::Dying(on) => {
            if let Some(c) = r(on) {
                fx.dying(h, c);
            }
        }
        Rule::NoDamage(on) => {
            if let Some(c) = r(on) {
                fx.no_damage(h, c);
            }
        }
        Rule::DrainCtrl { from, to, kind, a, b } => {
            if let (Some(f), Some(t)) = (r(from), r(to)) {
                fx.drain_ctrl(h, f, t, kind, a, b);
            }
        }
        Rule::AfterDrain(on) => {
            if let Some(c) = r(on) {
                fx.after_drain(h, c, 0);
            }
        }
        // ccSkillRecovery's effHealSkill(ch, sid): a heal's light.
        Rule::HealSkill { on, sid } => {
            if let Some(c) = r(on) {
                fx.heal_skill(h, c, sid);
            }
        }
        // ccSkillModifyCondition's items: an Antidote's, a Restorative's,
        // a Resurrect's.
        Rule::Cure(on) => {
            if let Some(c) = r(on) {
                fx.cure(h, c);
            }
        }
        Rule::Sanity(on) => {
            if let Some(c) = r(on) {
                fx.sanity(h, c);
            }
        }
        Rule::Resurrect(on) => {
            if let Some(c) = r(on) {
                fx.resurrect(h, c);
            }
        }
        // effResistantShield(ch, magic, -1): an Exdefense immunity met.
        Rule::ResistantShield { on, magic } => {
            if let Some(c) = r(on) {
                fx.resistant_shield(h, c, magic, -1);
            }
        }
        _ => {}
    }
}

/// `weaponEffPos[k]` of character `c` at `off` (+0x160 + 16 k), when it
/// has a weapon's points.
fn weapon_point(cast: &piney_world::combat::Cast, c: CharRef, off: u16) -> Option<V4> {
    let k = usize::from(off.checked_sub(piney_effect::WEAPON_POS)? / 16);
    let w = cast.get(c as usize)?.weapon.as_ref()?;
    w.points.get(k).copied()
}

/// The characters as the effects read them.
fn char_pos(scene: &Scene, c: CharRef) -> V4 {
    scene.chars.get(c as usize).map_or([0, 0, 0, ONE], |ch| ch.pos)
}

/// The battle as [`Host`]: the scene's characters by index, the game's
/// two generators, the camera, the map's bounds and the ground.
struct BattleHost<'a, 'w> {
    w: &'a mut FxWorld<'w>,
    camera: FxCamera,
}

impl<'a, 'w> BattleHost<'a, 'w> {
    fn of(w: &'a mut FxWorld<'w>) -> Self {
        let camera = FxCamera::from_field(w.camera);
        BattleHost { w, camera }
    }

    fn kite_pos(&self) -> V4 {
        self.w.kite.map_or([0, 0, 0, ONE], |k| self.w.scene.chars[k].pos)
    }
}

impl Host for BattleHost<'_, '_> {
    fn rand(&mut self) -> i32 {
        piney_battle::rand::Rng::rand(self.w.rand)
    }
    fn genrand(&mut self) -> u32 {
        self.w.cc.next_u32()
    }
    fn player_pos(&self) -> V4 {
        self.kite_pos()
    }
    fn camera_rot(&self, out: V4) -> V4 {
        self.w.camera.get_rot(self.w.camera.cam_id, out)
    }
    fn bounds(&self) -> [u32; 4] {
        let b = self.w.bounds;
        [b.min[0], b.min[1], b.max[0], b.max[1]]
    }
    fn camera(&self) -> FxCamera {
        self.camera
    }
    fn char_pos(&self, c: CharRef) -> V4 {
        char_pos(self.w.scene, c)
    }
    fn char_dirc(&self, c: CharRef) -> V4 {
        let i = c as usize;
        if let Some(Some(f)) = self.w.foes.get(i) {
            return f.dirc;
        }
        self.w.cast.get(i).map_or([0; 4], |a| a.ch.dirc)
    }
    fn char_height(&self, c: CharRef) -> u32 {
        self.w.scene.chars.get(c as usize).map_or(0, |ch| ch.base().height)
    }
    fn char_width(&self, c: CharRef) -> u32 {
        self.w.scene.chars.get(c as usize).map_or(0, |ch| ch.base().width)
    }
    fn check_target(&self, c: CharRef) -> bool {
        self.w.scene.listed(c as usize)
    }
    fn char_type(&self, c: CharRef) -> i32 {
        self.w.scene.chars.get(c as usize).map_or(0, |ch| ch.ty())
    }
    fn party_slot(&self, c: CharRef) -> i32 {
        let Some(ch) = self.w.scene.chars.get(c as usize) else { return -1 };
        self.w.party.slot_of(i32::from(ch.id()))
    }
    fn object_size(&self, c: CharRef) -> i32 {
        match self.w.foes.get(c as usize) {
            Some(Some(f)) => enemy_ai::object_size(self.w.t, &f.ent),
            _ => 0,
        }
    }
    fn char_attribute(&self, c: CharRef, flag: i32) -> i32 {
        self.w.scene.chars.get(c as usize).map_or(-1, |ch| piney_battle::chara::check_char_attribute(ch, flag))
    }
    fn char_dead(&self, c: CharRef) -> i16 {
        self.w.scene.chars.get(c as usize).map_or(0, |ch| ch.cond[piney_battle::param::cond::DEAD])
    }
    /// An idol's `effsw` (+0x1e0), the Statue of God glow's switch; 0 once
    /// the idol is gone.
    fn char_int(&self, c: CharRef, off: u16) -> i32 {
        match off {
            piney_effect::gimmick::IDOL_EFFSW => match self.w.ctrl.entry_obj(c as usize).map(|o| &o.class) {
                Some(piney_battle::gimmick::Class::Idol(i)) => i.effsw,
                Some(piney_battle::gimmick::Class::Etc(e)) => e.effsw,
                _ => 0,
            },
            // A party character's armsEffectSW, its weapon's particles.
            piney_effect::ARMS_SW => self.w.scene.chars.get(c as usize).map_or(0, |ch| ch.spc_char.arms_effect_sw),
            _ => 1,
        }
    }
    /// A party character's weapon points (`weaponEffPos`, +0x160) as its
    /// last `ArmsEffect` left them; its pos and dirc.
    fn char_vec(&self, c: CharRef, off: u16) -> V4 {
        weapon_point(self.w.cast, c, off).unwrap_or_else(|| match off {
            0x40 => self.char_pos(c),
            0x60 => self.char_dirc(c),
            _ => [0, 0, 0, ONE],
        })
    }
    fn area(&self) -> (i32, i32, i32) {
        self.w.area
    }
    fn game_field(&self) -> i32 {
        self.w.field
    }
    fn land_hit_check(&mut self, pos: V4, mask: u32) -> u32 {
        self.w.hits.land(pos, mask)
    }
    fn ground_attribute(&mut self, pos: V4) -> u32 {
        self.w.hits.land(pos, 0x2000_0000);
        self.w.hits.attribute()
    }
    /// A spell's damage call, made on the battle at once: a system's in
    /// `ccThSkill`, an element's (a level 3 or 4 tornado's, fall's,
    /// upheaval's, summons') in `ccThEffect`.
    fn raise(&mut self, e: &piney_effect::Event) {
        use piney_effect::Event as E;
        let who = |c: Option<CharRef>| c.map(|c| c as usize);
        let call = match *e {
            E::SkillDamage { spell, attacker, target, sid } => {
                SpellDamage::Target { spell, attacker: who(attacker), target: who(target), sid }
            }
            E::SkillDamageAt { attacker, pos, ttype, sid } => {
                SpellDamage::At { attacker: who(attacker), pos, ttype, sid }
            }
            E::SkillDamage2 { spell, attacker, target, pos, ttype, sid } => {
                SpellDamage::Area { spell, attacker: who(attacker), target: who(target), pos, ttype, sid }
            }
            _ => return,
        };
        self.w.skill_damage(call);
    }
}

/// What the numbers read after the frame (no generator is drawn).
struct ViewHost<'a> {
    view: &'a FxView<'a>,
    camera: FxCamera,
}

impl Host for ViewHost<'_> {
    fn rand(&mut self) -> i32 {
        0
    }
    fn player_pos(&self) -> V4 {
        self.view.kite.map_or([0, 0, 0, ONE], |k| self.view.scene.chars[k].pos)
    }
    fn bounds(&self) -> [u32; 4] {
        let b = self.view.bounds;
        [b.min[0], b.min[1], b.max[0], b.max[1]]
    }
    fn camera(&self) -> FxCamera {
        self.camera
    }
    fn char_pos(&self, c: CharRef) -> V4 {
        char_pos(self.view.scene, c)
    }
    fn char_dirc(&self, c: CharRef) -> V4 {
        self.view.cast.get(c as usize).map_or([0; 4], |a| a.ch.dirc)
    }
    fn char_height(&self, c: CharRef) -> u32 {
        self.view.scene.chars.get(c as usize).map_or(0, |ch| ch.base().height)
    }
    fn char_width(&self, c: CharRef) -> u32 {
        self.view.scene.chars.get(c as usize).map_or(0, |ch| ch.base().width)
    }
    fn check_target(&self, c: CharRef) -> bool {
        self.view.scene.listed(c as usize)
    }
}
