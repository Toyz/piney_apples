//! Enter Mac Anu as a new game does, run the effects the way the field's
//! tasks do, and write frames of an effect as PNGs.
//!
//! ```text
//! cargo run --release -p piney-effect --example effect_shot -- \
//!     [--iso work/infection/infection.iso] [--effect arrival] \
//!     [--shots 2,8,14,20,30,40] [--dir shots] [--soft] [--list]
//! ```
//!
//! Each frame is the world's (`World::step_into`: the field's logic tasks
//! and draws), then `ccThEffect` (`Effects::step`, fed from the world
//! through `host::Simple`) and `ccThParticle` (`Effects::step_particles`),
//! then the effects' draws into the same layers, and the numbers on the
//! menu layer as `ccMenuCtrl::Disp` runs them. `arrival` starts when the
//! world asks for it (`Request::Transfer`, Kite's act 13 at its count 30),
//! as the runtime would; the others start once the arrival is over, at
//! Kite or at a foe 150 in front of him (a position only: no model is
//! drawn for it). `--shots` are frames counted from the effect's start;
//! each is written to `DIR/EFFECT_NNN.png`, drawn by `piney-gs` on the GPU
//! when there is one, else (or with `--soft`) by the CPU GS.
//!
//! The effects draw from their own `rand()` here (seeded 1), not the
//! world's; in the game they share one. Everything but the arrival uses the
//! field's `effectTbl` over Mac Anu (a town resolves only the arrival's
//! ring), so what the fields show is shown here in the town.

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_desktop::anm::Ctx;
use piney_desktop::soft::{self, Canvas};
use piney_desktop::view::View;
use piney_draw::Frame;
use piney_effect::draw::Camera;
use piney_effect::host::{CharState, Simple};
use piney_effect::portal::{CircleInput, MagicCircle};
use piney_effect::{CharRef, Effects, Host, ONE, V4, ee, flyfont, particle};
use piney_input::{Pad, Raw};
use piney_world::{Rand, Request, SaveState, World};

/// Kite and the foe, as the effects name them.
const KITE: CharRef = 0;
const FOE: CharRef = 1;

/// What the example can show.
const SCENARIOS: &[(&str, &str)] = &[
    ("arrival", "effTransfer: the rings and sparks around Kite as he arrives"),
    ("warp", "effWarpTransfer: the arrival's sparks alone"),
    ("hit", "ccHitMarkDisp on the foe struck by Kite, and the damage number"),
    ("critical", "ccParticleCritical on the foe: sparks, the word, a big number"),
    ("dying", "ccParticleDying on the foe: the dying blow's burst and word"),
    ("protect", "effProtect: the foe's protect gauge breaking"),
    ("levelup", "effLevelUp on Kite"),
    ("drain", "effDrainCtrl: HP orbs from the foe to Kite, the word over the foe"),
    ("afterdrain", "effAfterDrain: the wave after a Data Drain, on the foe"),
    ("heal", "ccParticleHeal on Kite: the healing motes"),
    ("portal", "ccMagicCircle: a magic portal 400 in front of Kite, opening as he is near"),
    ("tornado", "skill 230, a level 2 fire tornado (TornadoSystem) cast by Kite on the foe"),
    ("tornado4", "skill 232, a level 4 fire tornado (the tornado element)"),
    ("fall", "skill 226, a level 2 fire fall (FallSystem): meteors"),
    ("convergence", "skill 214, a level 2 water convergence (ConvergenceSystem)"),
    ("upheaval", "skill 202, a level 2 soil upheaval (UpheavalSystem): pillars"),
    ("summons", "skill 238, a level 2 fire summons (SummonsSystem): the ring and the creature"),
    ("summons4", "skill 272, a level 4 thunder summons (the summoned bolts)"),
    ("skillstart", "effSkillStart: Kite's Saber Dance (skill 6) starting: sparks, rings, the circle at 7"),
    ("spellstart", "effSkillStart: Kite casting Repth (skill 150): a spell's sparks, rings and circle"),
    ("drainstart", "effSkillStartEffect(Kite, 0, 1): the Data Drain menu's side effect on Kite"),
    ("shockwave", "effPhysicalSkillHitShockWave: a fire skill's blow landing on the foe"),
    ("repth", "effHealSkill(Kite, 150): the healing ring and sparks"),
    ("fullheal", "effHealSkill(Kite, 152): Pha Repth's three bursts"),
    ("cure", "effCure on Kite: an antidote"),
    ("sanity", "effSanity on Kite: a restorative"),
    ("resurrect", "effResurrect on Kite: a revival"),
    ("openbox", "effOpenBox at the foe's feet"),
    ("removetrap", "effRemoveTrap at the foe's feet (a dead enemy's treasure box: 103, 121)"),
    ("abilityup", "effAbilityUp(Kite, 20): a stat up's sparks"),
    ("abilitydown", "effAbilityDown(Kite, 13): a stat down's"),
    ("shield", "effResistantShield on the foe (an enemy) as Kite's magic meets its immunity"),
];

/// The shots' host: `Simple`, with the foe an enemy (type 0x60, size 3)
/// whom Kite last struck, and Kite a PC (0x7).
struct Shot<'a> {
    s: Simple<'a>,
}

impl Host for Shot<'_> {
    fn rand(&mut self) -> i32 {
        self.s.rand()
    }
    fn genrand(&mut self) -> u32 {
        self.s.genrand()
    }
    fn player_pos(&self) -> V4 {
        self.s.player_pos()
    }
    fn bounds(&self) -> [u32; 4] {
        self.s.bounds()
    }
    fn camera(&self) -> Camera {
        self.s.camera()
    }
    fn char_pos(&self, c: CharRef) -> V4 {
        self.s.char_pos(c)
    }
    fn char_dirc(&self, c: CharRef) -> V4 {
        self.s.char_dirc(c)
    }
    fn char_height(&self, c: CharRef) -> u32 {
        self.s.char_height(c)
    }
    fn char_width(&self, c: CharRef) -> u32 {
        self.s.char_width(c)
    }
    fn char_type(&self, c: CharRef) -> i32 {
        if c == FOE { 0x60 } else { 0x7 }
    }
    fn object_size(&self, c: CharRef) -> i32 {
        if c == FOE { 3 } else { 0 }
    }
    fn affect_person(&self, c: CharRef) -> Option<CharRef> {
        (c == FOE).then_some(KITE)
    }
}

/// The attack spells' skill ids by scenario.
fn spell_id(name: &str) -> Option<i32> {
    Some(match name {
        "tornado" => 230,
        "tornado4" => 232,
        "fall" => 226,
        "convergence" => 214,
        "upheaval" => 202,
        "summons" => 238,
        "summons4" => 272,
        _ => return None,
    })
}

fn render_gpu(archive: Arc<Archive>, frame: &Frame) -> Result<(u32, u32, Vec<u8>), String> {
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive))?;
    gs.render(frame);
    let (w, h) = gs.target_size();
    Ok((w, h, gs.read_back()))
}

fn render_soft(archive: Arc<Archive>, frame: &Frame) -> (u32, u32, Vec<u8>) {
    let mut assets = soft::Assets::new(archive);
    let mut canvas = Canvas::new(frame.width as usize, frame.height as usize, frame.clear);
    canvas.draw(frame, &mut assets);
    (frame.width as u32, frame.height as u32, canvas.rgba())
}

fn new_game(iso: &mut Iso) -> Result<SaveState, Box<dyn std::error::Error>> {
    let tables = piney_demo::newgame::NewGameTables::of(iso.volume()?);
    let mut state = SaveState::fresh();
    piney_demo::newgame::new_game(&mut state.save, 0, &tables, piney_demo::newgame::SAVE_VA);
    Ok(state)
}

/// Starts `name`; false when nothing started.
fn start(name: &str, fx: &mut Effects, host: &mut Shot) -> bool {
    let feet = host.char_pos(FOE);
    match name {
        "arrival" => fx.transfer(host, KITE).is_some(),
        "warp" => fx.warp_transfer(host, KITE).is_some(),
        "hit" => {
            fx.hit_mark(host, FOE, KITE);
            fx.fly_font(Some(FOE), 2, 37);
            true
        }
        "critical" => {
            fx.hit_mark(host, FOE, KITE);
            fx.fly_font(Some(FOE), 2, 1234);
            fx.critical(host, FOE).is_some()
        }
        "dying" => fx.dying(host, FOE).is_some(),
        "protect" => fx.protect(host, FOE, 0, 1).is_some(),
        "levelup" => fx.level_up(host, KITE).is_some(),
        "drain" => fx.drain_ctrl(host, KITE, FOE, 0, 5, 5).is_some(),
        "afterdrain" => fx.after_drain(host, FOE, 1).is_some(),
        "heal" => {
            let (_, mut cx) = fx.split(host);
            particle::cc_particle_heal(&mut cx, KITE, 0);
            true
        }
        "skillstart" => fx.skill_start(host, KITE, 6, 0, 0).is_some(),
        "spellstart" => fx.skill_start(host, KITE, 150, 0, 0).is_some(),
        "drainstart" => fx.skill_start_effect(host, KITE, 0, 1).is_some(),
        "shockwave" => {
            let mut at = feet;
            at[2] = ee::add(at[2], ee::k(60.0));
            fx.shock_wave(host, at, 16).is_some()
        }
        "repth" => fx.heal_skill(host, KITE, 150).is_some(),
        "fullheal" => fx.heal_skill(host, KITE, 152).is_some(),
        "cure" => fx.cure(host, KITE).is_some(),
        "sanity" => fx.sanity(host, KITE).is_some(),
        "resurrect" => fx.resurrect(host, KITE).is_some(),
        "openbox" => {
            fx.open_box(host, feet);
            true
        }
        "removetrap" => fx.remove_trap(host, feet, 103, 121).is_some(),
        "abilityup" => fx.ability_up(host, KITE, 20).is_some(),
        "abilitydown" => fx.ability_down(host, KITE, 13).is_some(),
        "shield" => fx.resistant_shield(host, FOE, 1, -1).is_some(),
        _ => false,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut iso_path = "work/infection/infection.iso".to_string();
    let mut effect = "arrival".to_string();
    let mut shots: Vec<u32> = vec![2, 8, 14, 20, 30, 40];
    let mut dir = "shots".to_string();
    let mut soft_only = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--iso" => iso_path = args.next().ok_or("--iso PATH")?,
            "--effect" => effect = args.next().ok_or("--effect NAME")?,
            "--shots" => {
                shots = args
                    .next()
                    .ok_or("--shots N,N,...")?
                    .split(',')
                    .map(|s| s.trim().parse())
                    .collect::<Result<_, _>>()?
            }
            "--dir" => dir = args.next().ok_or("--dir DIR")?,
            "--soft" => soft_only = true,
            "--list" => {
                for (name, about) in SCENARIOS {
                    println!("{name:12} {about}");
                }
                return Ok(());
            }
            _ => return Err(format!("unknown argument {a}").into()),
        }
    }
    if !SCENARIOS.iter().any(|(n, _)| *n == effect) {
        return Err(format!("no effect {effect} (--list)").into());
    }
    std::fs::create_dir_all(&dir)?;
    let mut iso = Iso::open(&iso_path)?;
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN")?)?);
    let mut fx = Effects::new(&archive, iso.volume()?)?;
    let on_transfer = effect == "arrival";
    fx.ctrl.town = on_transfer;
    let font = flyfont::font_tex(&archive)?;
    let mut circle: Option<MagicCircle> = None;
    let mut spell: Option<i16> = None;
    let save = new_game(&mut iso)?;
    let mut world = World::enter(&mut iso, archive.clone(), save)?;
    let mut rand = Rand(1);
    let mut next = || rand.rand();
    let mut mt = piney_world::mt::Mt::seeded(1);
    let mut mt_next = || mt.genrand();
    let mut pad = Pad::default();
    let last = shots.iter().copied().max().unwrap_or(0);
    let mut started: Option<u32> = None;
    // The arrival ends 74 frames into the field's tasks, 12 after the fade.
    let free_at = 100;
    for i in 0..free_at + 400 {
        pad.read(&Raw::default());
        let mut ctx = Ctx::new(View::default());
        world.step_into(&pad, &mut ctx);
        let transfer = world.take_requests().contains(&Request::Transfer);
        let p = world.player();
        let camera = Camera::from_field(world.camera());
        let kite = CharState::player(KITE, p);
        // The foe: 150 in front of Kite (his heading 0 faces -y).
        let (s, c) = (ee::f(p.body.dirc[2]).sin(), ee::f(p.body.dirc[2]).cos());
        let at = |d: f32| -> V4 {
            [ee::k(ee::f(p.body.pos[0]) + d * s), ee::k(ee::f(p.body.pos[1]) - d * c), p.body.pos[2], ONE]
        };
        let foe = CharState { id: FOE, pos: at(150.0), dirc: [0; 4], height: ee::k(120.0), width: ee::k(45.0) };
        let (player_pos, circle_at) = (p.body.pos, at(400.0));
        let mut host = Simple::new(&mut next, player_pos, camera);
        host.genrand = Some(&mut mt_next);
        host.chars.push(kite);
        host.chars.push(foe);
        let mut host = Shot { s: host };
        let due = if on_transfer { transfer } else { i == free_at };
        if started.is_none() && due {
            if let Some(sid) = spell_id(&effect) {
                // _ccSkillRequest: the spell's ccSkill (key 1), Kite on the foe.
                fx.spell_request(&host, 1, sid, 0, Some(KITE), Some(FOE));
                spell = Some(0);
            } else if effect == "portal" {
                fx.assets.add_file(&archive, piney_effect::portal::FILE)?;
                let mut pos = circle_at;
                pos[2] = ee::add(pos[2], ee::k(250.0));
                circle = MagicCircle::new(&fx.assets, pos, [0; 4], player_pos, host.s.bounds);
                if circle.is_none() {
                    return Err("the portal did not start".into());
                }
            } else if !start(&effect, &mut fx, &mut host) {
                return Err("the effect did not start".into());
            }
            started = Some(0);
        }
        fx.step(&mut host);
        // ccThSkill (82): ccSkill::Main's system for the running spell, its
        // count the value before Main's increment.
        if let Some(count) = spell {
            let (c, t) = (host.char_pos(KITE), host.char_pos(FOE));
            if let Some(s) = fx.spells.get_mut(1) {
                s.sync(count, c, host.char_dirc(KITE), t, 0, Some(KITE), Some(FOE));
            }
            fx.spell_system(&mut host, 1);
            spell = match fx.spells.get(1) {
                Some(s) if s.status == 0 => Some(count + 1),
                _ => {
                    fx.spell_remove(1);
                    None
                }
            };
        }
        fx.step_particles(&mut host);
        fx.draw(&mut ctx.layers, &camera);
        if let Some(c) = circle.as_mut() {
            let d = ee::vsub(circle_at, player_pos);
            let input = CircleInput {
                freeze: false,
                disp_sw: true,
                pl_dist: ee::sqrtf(ee::add(ee::mul(d[0], d[0]), ee::mul(d[1], d[1]))),
                set_transparency: ONE,
                player_listed: true,
                ent_root: 0,
            };
            let f = c.main(&fx.assets, &mut host, &input);
            c.render(&f, &fx.assets, &mut ctx.layers, &camera);
            if f.delete {
                circle = None;
            }
        }
        // ccMenuCtrl::Disp: the numbers on the menu layer.
        fx.fly_fonts(&host, false, false);
        flyfont::send(&fx.font.take(), &font, &mut ctx.layers);
        let frame = ctx.finish();
        for _ in world.take_talk() {
            world.close_menu();
        }
        let Some(n) = started else { continue };
        if shots.contains(&n) {
            let (w, h, rgba) = if soft_only {
                render_soft(archive.clone(), &frame)
            } else {
                match render_gpu(archive.clone(), &frame) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("no GPU ({e}); drawing on the CPU");
                        render_soft(archive.clone(), &frame)
                    }
                }
            };
            let out = format!("{dir}/{effect}_{n:03}.png");
            std::fs::write(&out, piney_gs::png::encode(w, h, &rgba))?;
            let live = fx.ctrl.effects.iter().filter(|e| e.status != 0).count();
            println!(
                "{out}: frame {i}, {live} effects, {} particles, {} draws",
                fx.particles.live().count(),
                fx.draws().len() + fx.particle_draws().len()
            );
        }
        if n >= last {
            return Ok(());
        }
        started = Some(n + 1);
    }
    Err("the effect never started".into())
}
