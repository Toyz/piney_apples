//! Issue #62: a town's Grunty ranch plays the breeder's tune. `bgmBreed`
//! reads the town's `DMY_merchant6` once after each `ccSndSQLoad` that
//! clears `ccSnd`'s `free[4]` (a town other than Mac Anu); the port read it
//! once a session, so every ranch after the first listened at the first
//! town's breeder.

use std::path::PathBuf;

use piney_input::{Buttons, Raw};

use super::*;

fn still() -> Raw {
    Raw { buttons: Buttons::NONE, analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() }
}

/// A session whose every frame's events go to a headless engine as `main`
/// routes them, from its start.
struct Ear {
    s: Session,
    pad: Pad,
    audio: piney_audio::Audio,
}

impl Ear {
    fn step(&mut self, raw: &Raw) {
        self.pad.read(raw);
        let rate = Mode::frame_rate(&self.s);
        self.s.step(&self.pad);
        crate::handle(self.s.take_events(), Some(&self.audio));
        self.audio.frame();
        self.audio.render(&mut vec![0i16; 1600 * rate as usize]);
    }

    /// The sequencers playing.
    fn sequences(&self) -> [bool; 3] {
        self.audio.with_engine(|e| e.seq.each_ref().map(|q| q.playing()))
    }

    fn world(&self) -> &piney_world::World {
        match &self.s.stage {
            Stage::World(w) => w.world(),
            _ => panic!("not in a town: {}", Mode::title(&self.s)),
        }
    }

    /// Frames with nothing held until Kite stands in town `town`.
    fn arrive(&mut self, town: i32) {
        for _ in 0..3000 {
            if let Stage::World(w) = &self.s.stage
                && w.world().town().base.no == town
                && matches!(w.world().phase(), piney_world::Phase::Play(n) if n >= 30)
            {
                return;
            }
            self.step(&still());
        }
        panic!("not in town {town}: {}", Mode::title(&self.s));
    }

    /// Kite walks round the walls to within 990 of the town's
    /// `DMY_merchant6` (the tune comes in within 1000), then 60 frames more
    /// for its fades. The way is planned again each second, round the people
    /// standing about; where he has stopped short of the next turn, as a
    /// player would, he goes round it.
    fn walk_to_ranch(&mut self) {
        let town = &self.world().town().base;
        let d = town.file.ccs.find_object("DMY_merchant6").and_then(|o| town.file.scene.dummies.get(&o));
        let to = d.map(|d| [d.pos.x, d.pos.y]).expect("no DMY_merchant6");
        let mut path: Vec<[f32; 2]> = Vec::new();
        let (mut mark, mut avoid) = (None::<[f32; 2]>, Vec::<([f32; 2], u32)>::new());
        for f in 0..4000u32 {
            let world = self.world();
            let p = world.player().body.pos.map(f32::from_bits);
            if (to[0] - p[0]).hypot(to[1] - p[1]) < 990.0 {
                for _ in 0..60 {
                    self.step(&still());
                }
                return;
            }
            if f % 60 == 0 {
                if mark.is_some_and(|m| (m[0] - p[0]).hypot(m[1] - p[1]) < 60.0)
                    && let Some(&q) = path.first()
                {
                    avoid.push((q, f));
                }
                mark = Some([p[0], p[1]]);
                // A walker moves on: a place he stopped at is open again later.
                avoid.retain(|&(_, at)| f - at < 600);
                let mid = [(p[0] + to[0]) / 2.0, (p[1] + to[1]) / 2.0];
                let span = (to[0] - p[0]).abs().max((to[1] - p[1]).abs());
                let n = ((span + 4000.0) / 100.0) as i32 | 1;
                // The walking PCs and the merchants stand in the way too.
                let people = world.pcs().iter().filter(|c| c.disp_sw).map(|c| c.char.pos);
                let people = people.chain(world.merchants().iter().map(|m| m.ch.pos));
                let mut closed: Vec<[f32; 2]> = avoid.iter().map(|&(q, _)| q).collect();
                closed.extend(people.map(|q| [f32::from_bits(q[0]), f32::from_bits(q[1])]));
                let look = super::survey::Look { n, step: 100.0, wide: 75.0, heights: &[30.0, 95.0], avoid: &closed };
                let near = |q: [f32; 2], _| (q[0] - to[0]).hypot(q[1] - to[1]) < 980.0;
                path = super::survey::path_wide(&world.town().base.hits, [p[0], p[1], p[2]], mid, &look, near);
            }
            while path.len() > 1 && (path[0][0] - p[0]).hypot(path[0][1] - p[1]) < 100.0 {
                path.remove(0);
            }
            let q = path.first().copied().unwrap_or(to);
            let cam_z = f32::from_bits(world.camera().rot()[2]);
            let raw = super::stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1])));
            self.step(&raw);
        }
        panic!("Kite did not reach the ranch: {:?}", self.world().player().body.pos.map(f32::from_bits));
    }
}

/// Dun Loireag's ranch, then each other town's by the console's `town N`
/// (Carmina Gadelica, Fort Ouph, Lia Fail; Dun Loireag again last), on the
/// three later discs. Outbreak and Quarantine play in the crisis their
/// new games start in, so their towns load the crisis rows (6 on). At each
/// ranch the breeder's tune (sequence 1) plays and the town's (0) is off.
/// Before the fix every ranch after the first kept the town's music.
#[test]
fn every_ranch_plays_the_breeders_tune() {
    for (disc, crisis) in [("mutation", false), ("outbreak", true), ("quarantine", true)] {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{disc}/{disc}.iso"));
        if !iso.exists() {
            eprintln!("{disc}.iso not present; skipped");
            continue;
        }
        let mut d = Iso::open(&iso).unwrap();
        let archive = Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let mut state = crate::world::new_game_state(&mut d).unwrap();
        state.save.set_u8(offset::LAST_TOWN, 1);
        state.save.set_u8(offset::CRISIS, u8::from(crisis));
        let scene = piney_world::area::Scene::log_in(&mut state.save);
        let s = Session::in_world(iso.clone(), archive, None, state, None, scene, None).unwrap();
        let mut e = Ear { s, pad: Pad::default(), audio: piney_audio::Audio::headless(&iso).unwrap() };
        for (k, town) in [1, 2, 3, 4, 1].into_iter().enumerate() {
            if k > 0 {
                e.s.console(&format!("town {town}"));
            }
            e.arrive(town);
            e.walk_to_ranch();
            let heard = e.sequences();
            eprintln!("{disc} town {town}: sequences {heard:?}");
            assert!(heard[1] && !heard[0], "{disc} town {town}: the breeder's tune is not on: {heard:?}");
        }
    }
}
