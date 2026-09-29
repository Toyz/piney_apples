//! The field's effects (piney-effect's `ccThEffect` and `ccThParticle`) over a
//! field's or dungeon's battle, installed into the world
//! ([`piney_world::field_world::FieldFx`]) and run at their places in the frame
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
use piney_world::evarea::StorySprite;
use piney_world::field_world::{FieldFx, FxView};
use piney_world::merchant::SysopEvent;

/// An ambient sprite's `ccEff` by file, name, fog bit, depth test and
/// palette change (`piney_world::field_ambient::Sprite`).
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
    /// A story map's `ccEff`s by name as its constructor made them (area
    /// 15's lens flare, an arena's fireflies; a flare's render state is its
    /// own), and the disc their file is read from.
    story: std::collections::HashMap<(&'static str, bool, bool), Option<Eff>>,
    /// A field's weather's `ccEff`s (and a dungeon's sparks and glows) by
    /// file, name, fog bit, depth test and palette change, and its smoke
    /// puffs waiting for the next effect task ([`ambient`]).
    ambient: std::collections::HashMap<AmbientKey, Option<Eff>>,
    ambient_smoke: Vec<piney_world::field_ambient::Op>,
    archive: Arc<Archive>,
    /// Each character's condition effect (`ccChar` +0x2c), by scene index.
    conditions: std::collections::HashMap<usize, piney_effect::particle::ConditionEffect>,
    /// The skills' own files (`ctu0ski4` ...), read once each, and each
    /// running skill's animation of its file, by the run's key.
    skill_files: std::collections::HashMap<String, Option<std::rc::Rc<piney_desktop::assets::SceneFile>>>,
    skill_anims:
        std::collections::HashMap<u32, (std::rc::Rc<piney_desktop::assets::SceneFile>, piney_world::pose::Play)>,
}

impl AreaFx {
    /// `ccThEffect`'s and `ccThParticle`'s set-up, the portal's file added.
    pub fn new(archive: &Arc<Archive>, volume: piney_data::volume::Volume) -> piney_data::Result<AreaFx> {
        let mut fx = Effects::new(archive, volume)?;
        // The boss effects' file (the WaveShock's animation).
        fx.assets.add_file(archive, "xeffect")?;
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
        })
    }

    /// A story map's `ccEff` for a sprite: `Init(chunk, 1)`; for a flare
    /// then `SetRenderState(CCRS_ZENABLE, 0)` (no depth test) and PRIM's
    /// fog bit cleared, as `EVENTAREA02`'s constructor leaves each.
    fn story_eff(&mut self, s: &StorySprite) -> Option<Eff> {
        let assets = &mut self.fx.assets;
        let archive = &self.archive;
        self.story
            .entry((s.eff, s.flare, s.fog))
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
    /// synced from the run, its system stepped; the damage calls and
    /// releases it raised go back to the battle, the rest to the events.
    fn spell(&mut self, w: &mut FxWorld, run: &SpellRun) -> SpellOut {
        let (creator, target) = (run.creator.map(cref), run.target.map(cref));
        let mut host = BattleHost::of(w);
        if self.fx.spells.get(run.key).is_none() {
            self.fx.spell_request(&host, run.key, run.sid, run.stype, creator, target);
        }
        if let Some(s) = self.fx.spells.get_mut(run.key) {
            s.sync(run.count, run.c_pos, run.c_dirc, run.t_pos, run.t_type, creator, target);
        }
        let ran = self.fx.spell_system(&mut host, run.key);
        let mut out = SpellOut { ran, ..SpellOut::default() };
        let who = |c: Option<CharRef>| c.map(|c| c as usize);
        for e in self.fx.take_events() {
            match e {
                piney_effect::Event::SkillDamage { attacker, target, sid, .. } => {
                    out.damage.push(SpellDamage::Target { attacker: who(attacker), target: who(target), sid });
                }
                piney_effect::Event::SkillDamageAt { attacker, pos, ttype, sid } => {
                    out.damage.push(SpellDamage::At { attacker: who(attacker), pos, ttype, sid });
                }
                piney_effect::Event::SkillDamage2 { attacker, target, pos, ttype, sid, .. } => {
                    out.damage.push(SpellDamage::Area {
                        attacker: who(attacker),
                        target: who(target),
                        pos,
                        ttype,
                        sid,
                    });
                }
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
            let play = piney_world::pose::Play::new(&f, &format!("ANM_{file}"))?;
            self.skill_anims.insert(key, (f, play));
        }
        let (f, play) = self.skill_anims.get_mut(&key)?;
        Some(play.forward_notes(f))
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
        let scene = w.scene;
        let spells = boss_spells(scene);
        let mut host = BattleHost::of(w);
        self.fx.boss_sync(&mut host, &spells);
        self.events.extend(self.fx.take_events());
    }
}

impl FieldFx for AreaFx {
    fn tasks(&mut self) -> &mut dyn FxTasks {
        self
    }

    fn census(&self) -> piney_world::field_world::FxCensus {
        piney_world::field_world::FxCensus {
            effects: self.fx.ctrl.effects.iter().filter(|e| e.status != 0).map(|e| e.id).collect(),
            particles: self.fx.particles.live().count(),
            weapon_generators: (self.fx.particles.gens.iter())
                .filter(|g| matches!(g.sync_sw, Some(piney_effect::IntRef::CharAt(_, piney_effect::ARMS_SW))))
                .count(),
            fly_fonts: self
                .fx
                .dam
                .nodes
                .iter()
                .flat_map(|n| n.uproll.lines.iter())
                .filter(|l| l.alpha > 0 && !l.text.is_empty())
                .map(|l| l.text.clone())
                .collect(),
            boss_draws: (self.fx.boss_draws().iter())
                .map(|d| match d {
                    DrawRec::Clump { obj, .. } | DrawRec::Anm { obj, .. } => self.fx.assets.name(*obj).to_string(),
                    DrawRec::Eff { eff, .. } => format!("eff {}", eff.chunk),
                })
                .collect(),
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
        let camera = FxCamera::from_field(view.camera);
        self.fx.draw(&mut ctx.layers, &camera);
        if let Some((mc, spark)) = &self.circle {
            let frame = circle_frame(view, mc, spark);
            mc.render(&frame, &self.fx.assets, &mut ctx.layers, &camera);
        }
        let host = ViewHost { view, camera };
        self.fx.fly_fonts(&host, false, false);
        flyfont::send(&self.fx.font.take(), &self.font, &mut ctx.layers);
    }

    /// A story map's sprites (`EVENTAREA02::DrawLensFlare`,
    /// `EVENTAREAB0`'s fireflies): each `ccEff::Draw(pos, pat)` on its
    /// layer.
    fn story_sprites(&mut self, sprites: &[StorySprite], camera: &piney_world::camera::Camera, ctx: &mut Ctx) {
        let cam = FxCamera::from_field(camera);
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
        ops: &[piney_world::field_ambient::Op],
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
fn boss_spells(scene: &Scene) -> Vec<(i32, &piney_battle::boss::fidchell::eff::Fx)> {
    let mut out = Vec::new();
    for ch in &scene.chars {
        let Some(b) = ch.foe_state().and_then(|f| f.boss.as_ref()).filter(|b| b.exit == 0) else { continue };
        for (k, e) in b.effects.slots.iter().enumerate() {
            if let Some(fx) = e.as_ref().and_then(|e| e.fidchell.as_deref()) {
                out.push((k as i32, fx));
            }
        }
    }
    out
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
        // A leaf of Magus's lands: ccEnemyEffDust(pos, 30, 4.0).
        Show::Boss(_, piney_battle::boss::Out::Magus(piney_battle::boss::magus::Pic::Dust { pos })) => {
            let (life, tex) = (piney_effect::dust::DUST_LIFE, piney_effect::dust::DUST_TEX);
            fx.enemy_dust(h, *pos, 30, 0x4080_0000, life, tex);
        }
        // The boss's ccBossEff*Create (those the effects have).
        Show::Boss(_, piney_battle::boss::Out::Effect { kind, pos, dirc, .. }) => {
            if let Some(m) = boss_make(*kind, *pos, *dirc) {
                fx.boss_create(h, m);
            }
        }
        // What Fidchell's rules ask beside: the back dash's and a tower's
        // dust, a meteor's landing, a tower's and a strike's rocks.
        Show::Boss(_, piney_battle::boss::Out::Fidchell(p)) => fx.fidchell_calls(h, p),
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
/// "Effects"): the rules name the effect, where and which way. Innis's
/// rings and missiles, Kyvia's meteors and Magus's needles have no
/// picture yet (boss-innis.md, boss-kyvia.md, boss-magus.md); Fidchell's
/// meteors, thunders and rock towers are drawn from its rules' state
/// ([`boss_spells`]).
fn boss_make(kind: piney_battle::boss::EffKind, pos: V4, dirc: V4) -> Option<piney_effect::boss::Make> {
    use piney_battle::boss::EffKind;
    use piney_effect::boss::Make;
    const TEN: u32 = 0x4120_0000;
    Some(match kind {
        EffKind::WaveShock => Make::WaveShock { pos, dirc, scale: ONE },
        EffKind::MagicSquare { n } => Make::MagicSquare { pos, n },
        EffKind::ForceGenerator { num, life, speed, r0, r1, clt } => {
            Make::ForceGenerator { p: pos, rot: dirc, speed, r0, r1, num, life, clt }
        }
        EffKind::AutoSamonRing { n } => Make::AutoSamonRing { pos, rot: dirc, param: [0, 0x3eaa_aaab, TEN, 0], n },
        EffKind::IceBreak => Make::IceBreak { pos, scale: 0x4000_0000 },
        EffKind::Dead => Make::Dead { pos },
        // Magus's leaf's ring (model 195, DeadEffect's parameter).
        EffKind::LeafRing => Make::AutoSamonRing { pos, rot: dirc, param: [0x3f00_0000, ONE, 0, 0x4220_0000], n: 195 },
        EffKind::SamonRing { .. }
        | EffKind::Missile { .. }
        | EffKind::Meteorite { .. }
        | EffKind::Needle { .. }
        | EffKind::MeteoSworm { .. }
        | EffKind::ThunderStorm { .. }
        | EffKind::RockTower { .. }
        // Gorre's finishing flash (boss-gorre.md): no picture yet.
        | EffKind::FinalPhotonFlash => {
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
