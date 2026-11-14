//! Kite as the probes run him: the battle's `ccPlayer` (`Combat::add_kite`,
//! one in every area) beside the camera; a frame is `cameraMain` (40) then
//! the battle's tasks (`Combat::frame`: ccThSpc, ccThPlayer, ...) as the
//! world runs them. Included by the probes with `#[path]`.

#![allow(dead_code)]

use std::rc::Rc;

use piney_battle::kite::Out;
use piney_data::save::{SaveData, offset};
use piney_world::Rand;
use piney_world::area::{Scene, WorldMan};
use piney_world::camera::{CamPad, Camera, Scheme};
use piney_world::chara::body::Body;
use piney_world::chara::party::Spcs;
use piney_world::combat::spc::SpcRec;
use piney_world::combat::{BattleData, Combat, NoFx, Show, Tasks};
use piney_world::ee::V4;
use piney_world::hit::Hits;

/// Where and how Kite starts: `area` is `game.area` (0 a town, 1 a field,
/// 2 a dungeon); `arriving` his arrival (act 13) rather than standing.
pub struct Start {
    pub pos: V4,
    pub dirc_z: u32,
    pub scheme: i32,
    pub mode: i8,
    pub seed: u32,
    pub area: i32,
    pub arriving: bool,
}

pub struct KiteRun {
    pub combat: Combat,
    /// The registry and the party: Kite alone.
    pub spcs: Spcs,
    pub save: SaveData,
    pub camera: Camera,
    pub mode: i8,
    area: i32,
    /// This frame's first `ArmsEffect` / `ClearArmsEffect`, as the
    /// harnesses report it.
    pub arms: &'static str,
    /// `WORLD_MAN::Enter` this frame.
    pub enter: bool,
    /// [`Hits::attribute_qualified`] after the frame.
    pub qualified: bool,
}

impl KiteRun {
    /// Kite's record as the harnesses' machine has it (flags 7, height
    /// 160, width 45, velocity 27.5), built by `ccSPC::Reboot` at `s`.
    pub fn new(data: &Rc<BattleData>, body: &Rc<Body>, hits: &mut Hits, s: &Start) -> KiteRun {
        let mut save = SaveData::new();
        let at = offset::SPC_PARAM;
        save.set_i32(at + 8, 7);
        save.set_i32(at + 0x18, 0x4320_0000);
        save.set_i32(at + 0x1c, 0x4234_0000);
        save.set_i32(at + 0xd4, 0x41dc_0000);
        let mut combat = Combat::new(data.clone(), 1);
        let dirc = [0, 0, s.dirc_z, 0];
        combat.add_kite(&save, s.pos, s.dirc_z, s.arriving, 0, 1, body.clone(), Vec::new(), hits, s.area);
        combat.rand = Rand(u64::from(s.seed));
        let mut spcs = Spcs::new_game();
        spcs.set_party();
        combat.set_party(spcs.party());
        KiteRun {
            combat,
            spcs,
            save,
            camera: Camera::new(hits.volume, s.pos, dirc, s.mode, Scheme::new(s.scheme)),
            mode: s.mode,
            area: s.area,
            arms: "none",
            enter: false,
            qualified: true,
        }
    }

    /// His scene index.
    pub fn kite(&self) -> usize {
        self.combat.kite.expect("built at start")
    }

    pub fn pos(&self) -> V4 {
        self.combat.scene.chars[self.kite()].pos
    }

    pub fn dirc(&self) -> V4 {
        self.combat.kite_ref().map_or([0; 4], |k| k.spc.dirc)
    }

    /// `ccAI` +0 `manualSW`: his body passes through the walking PCs.
    pub fn set_manual(&mut self, on: bool) {
        let k = self.kite();
        if let Some(a) = self.combat.crew.ais.get_mut(&k) {
            a.manual_sw = on;
        }
    }

    /// One frame: `cameraMain`, then the battle's tasks.
    pub fn frame(&mut self, pad: CamPad, hits: &mut Hits) {
        let heading = self.dirc()[2];
        self.camera.main(&pad, heading, true, false, &mut self.mode);
        self.combat.shows.clear();
        self.combat.enter = false;
        let mut scene = Scene::init();
        scene.area = self.area;
        let mut x = Tasks {
            hits,
            camera: &mut self.camera,
            pad,
            save: &mut self.save,
            scene,
            wm: WorldMan::default(),
            menu_type: -1,
            menu_forbid: false,
            puppet_show: false,
            cmnd_target: false,
            count: 0,
            event_area: false,
            dungeon: [0; 4],
            map2d: Vec::new(),
            map2d_info: [0; 3],
            path_map: false,
            effect_sw: true,
            spcs: Some(&mut self.spcs),
            own: None,
        };
        self.combat.frame(&mut x, &mut NoFx);
        self.qualified = x.hits.attribute_qualified();
        self.enter = self.combat.enter;
        self.arms = self
            .combat
            .shows
            .iter()
            .find_map(|s| match s {
                Show::Kite(_, Out::ArmsEffect) => Some("effect"),
                Show::Kite(_, Out::ClearArmsEffect) => Some("clear"),
                _ => None,
            })
            .unwrap_or("none");
    }

    /// Kite's state under the names the harnesses read from `ccPlayer`.
    pub fn fields(&self) -> Vec<(&'static str, String)> {
        let c = &self.combat;
        let k = self.kite();
        let s = c.crew.spc.get(&k).copied().unwrap_or_default();
        let r = SpcRec::read(&c.scene, &c.crew, k, c.kite, true);
        let a = c.cast.get(k);
        let (time, frame_spd) = a.map_or((0, 0), |a| (a.ch.play.time, a.ch.play.frame_spd));
        let p = &c.player;
        vec![
            ("pos", list(&c.scene.chars[k].pos)),
            ("dirc", list(&s.dirc)),
            ("move", list(&s.move_pos)),
            ("now_speed", s.now_speed.to_string()),
            ("speed_rate", s.speed_rate.to_string()),
            ("move_flag", b(r.move_flag)),
            ("run_flag", b(r.run_flag)),
            ("stop_flag", b(r.stop_flag)),
            ("restraint", b(r.restraint)),
            ("act", r.act.to_string()),
            ("act_old", r.act_old.to_string()),
            ("act_cnt", r.act_cnt.to_string()),
            ("anm_flag", r.anm_flag.to_string()),
            ("react_cnt", s.react_cnt.to_string()),
            ("transfer_lag", r.transfer_lag.to_string()),
            ("walk_run_cnt", s.walk_run_cnt.to_string()),
            ("cloak", r.cloak.to_string()),
            ("transparency", s.transparency.to_string()),
            ("frame_spd", frame_spd.to_string()),
            ("time", time.to_string()),
            ("attribute", s.hit_attribute.to_string()),
            ("stop_cnt", s.stop_cnt.to_string()),
            ("drawn", b(a.is_some_and(|a| a.drawn))),
            ("arms", format!("{:?}", self.arms)),
            ("target_count", p.target_count.to_string()),
            ("angle", list(&p.angle)),
            ("stress", p.stress.to_string()),
            ("hitpos", list(&s.body_hit.pos)),
            ("hitsw", b(s.body_hit.sw)),
        ]
    }

    /// The field camera's state under the harnesses' names.
    pub fn camera_fields(&self) -> Vec<(&'static str, String)> {
        let (c, t) = (&self.camera, &self.camera.tcam);
        vec![
            ("cam_pos", list(&t.pos)),
            ("cam_view", list(&t.view)),
            ("cam_rot", list(&t.rot)),
            ("cam_rot2", list(&t.rot2)),
            ("cam_rot3", list(&t.rot3)),
            ("cam_dist", t.dist.to_string()),
            ("cam_deg", format!("[{}, {}]", t.deg[0], t.deg[1])),
            ("cam_type", t.kind.to_string()),
            ("cam_reset_flag", b(t.reset_flag)),
            ("cam_reset_dirc", t.reset_dirc.to_string()),
            ("resetting", b(c.resetting)),
            ("mem_dirc_z", c.mem_dirc_z.to_string()),
            ("mode", self.mode.to_string()),
            ("world_view", list(&c.world_view.iter().flatten().copied().collect::<Vec<_>>())),
            ("world_screen", list(&c.world_screen.iter().flatten().copied().collect::<Vec<_>>())),
        ]
    }

    /// [`KiteRun::fields`] and [`KiteRun::camera_fields`] as one JSON
    /// object, `extra` after them.
    pub fn json(&self, extra: &[(&str, String)]) -> String {
        let mut kv = self.fields();
        kv.extend(self.camera_fields());
        let mut body: Vec<String> = kv.iter().map(|(k, v)| format!("\"{k}\": {v}")).collect();
        body.extend(extra.iter().map(|(k, v)| format!("\"{k}\": {v}")));
        format!("{{{}}}", body.join(", "))
    }
}

pub fn list(v: &[u32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

pub fn b(v: bool) -> String {
    u8::from(v).to_string()
}

/// A pad as the requests give it: `direct push powL dircL powR dircR
/// pow[12]` from word `at`.
pub fn pad_at(w: &[&str], at: usize) -> CamPad {
    let n = |i: usize| u32::from_str_radix(w[at + i].trim_start_matches("0x"), 16).unwrap();
    CamPad {
        direct: n(0),
        push: n(1),
        pow_l: n(2) as u8,
        dirc_l: n(3),
        pow_r: n(4) as u8,
        dirc_r: n(5),
        pow: std::array::from_fn(|k| n(6 + k) as u8),
    }
}

/// Kite's body (`ctu1body`, the trail clump).
pub fn body(archive: &std::sync::Arc<piney_data::archive::Archive>) -> Rc<Body> {
    Rc::new(Body::read(archive, "ctu1body", piney_world::chara::body::TRALL).unwrap())
}

/// The battle's tables, once per probe.
pub fn data(iso: &mut piney_data::iso::Iso) -> Rc<BattleData> {
    Rc::new(BattleData::read(iso).unwrap())
}
