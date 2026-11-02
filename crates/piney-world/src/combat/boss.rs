//! The bosses in the field: what `ccBossEntryStart(code)` (gcmn 0x0045b2a0)
//! starts for an event's `entry 7 code` - `ccThBossEffect` and
//! `bossFunc[code]` at priority 66 - over the battle's scene
//! ([`piney_battle::boss`]), by the code's def ([`BossDef`]); a code the
//! port lacks starts nothing. Every boss is read the same way: its def,
//! its frame's requests ([`Out`]) and what it shows ([`boss::Output`]).
//! Once `CheckExit()` the task sets its parameter's +0x14, which
//! `absent 7` reads.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use piney_battle::affect::AffectCtx;
use piney_battle::boss::defs::{BodyPick, BossDef, CamDef, PartBody, Spawn};
use piney_battle::boss::{self, Boss, BossEnv, CamView, Out};
use piney_battle::chara::{AffectFunc, Env};
use piney_battle::enemy_ai::World;
use piney_battle::event::{Event, Events, Who};
use piney_battle::param::cond;
use piney_battle::world::CharHit;

use super::Combat;
use super::cast::{Actor, Look};
use super::stage::Show;
use crate::camera::bosscam::BossCam;
use crate::camera::cinema::{Cinema, Name as CinemaName, Sent as CinemaSent};
use crate::camera::{CamPad, Camera, id};
use crate::chara::body::Body;
use crate::ee::{self, F, ONE, V4};
use crate::sky::lattice::{Lattice, StripVertex};

/// Gorre's `m_exit` (the manager's +0x2c, which its constructor leaves):
/// the word of stream 64's decoded mesh where the manager lands in the
/// hole the cutscene leaves: 0x1882c in, for the save loaded after 900
/// frames at the title (docs/engine/heap.md "Gorre's m_exit"). Where it
/// lands moves with the play since power-on; the port has no game heap
/// and keeps this history's word. Not 0: no blur.
pub const HEAP_EXIT: u32 = 0x0100_003f;

/// Quarantine's `ccBoss08` (code 7, event 313): its first form writes
/// nothing to the blur, so `m_exit` is the word under the manager. On the
/// story's way (town 4, then field 8) it lands in the room town 4 left:
/// 0 (a clump object's +0x2c) with two members after 900 frames at the title,
/// a blit buffer's pixels in other histories (docs/engine/heap.md
/// "ccBoss08's m_exit"). The port keeps the 900-frame word: the
/// manager's own blur until `ccBoss08_b`'s constructor writes its own.
pub const BOSS08_HEAP_EXIT: u32 = 0;

/// The effects' file.
pub const EFF_FILE: &str = "xeffect";
/// The wave's animation in [`EFF_FILE`].
pub const ANM_WAVE: &str = "ANM_xx11wave";
/// A clip's frames and whether it loops, by name, in the boss's files.
pub type Clips = HashMap<String, (u32, bool)>;

/// What the field keeps of the boss's tasks.
pub struct BossRun {
    /// `bossFunc`'s code (`eventMng.bossEntry`).
    pub code: i32,
    /// The boss's scene index.
    pub me: usize,
    /// `DMY_center01`, which a later form is made at too.
    pub center: V4,
    /// The task's parameter +0x14: the boss has exited.
    pub exit: bool,
    pub clips: Rc<Clips>,
    /// `bodyHit` (`ccCharHit`): radius `base->width`, height
    /// `base->height`, its kind `base->type`.
    pub hit: CharHit,
    pub look: Rc<BossLook>,
    /// `m_aiMngr`'s after-images being drawn.
    pub afterimages: Vec<AfterImage>,
    /// The waves' `DrawParts` this frame: each animation's pose time, at
    /// the boss's place and heading (Gorre's brothers each their own).
    pub waves: Vec<(u32, V4, V4, f32)>,
    /// The boss's omni light while it is in the group ([`Out::Light`]).
    pub light: Option<piney_battle::prim::OmniLight>,
    /// Its `PreDrawAnm`'s `SetFogBlend` this frame ([`Out::Fog`]), which its
    /// body's draw keeps when `ccBoss::Draw` sets no blend of its own.
    pub fog: Option<crate::chara::foe::FogBlend>,
    /// What the boss draws this frame besides its bodies, its waves and
    /// its after-images ([`Picture`]).
    pub pictures: Vec<Picture>,
    /// `bossCam` and `bossCamSW` (+0x14c, +0x150): the camera, and whether
    /// `Main` still runs it.
    pub cam: Option<BossCam>,
    pub cam_sw: bool,
    /// `ccBufferReverce::MakePacket` this frame (Skeith's magic's last
    /// part): the frame inverted on the boss's own layer.
    pub reverse: bool,
    /// The effect manager's blur, and the reflex its `Main` drew this frame
    /// (on the manager's layer, the manager's task outlasting the boss's).
    pub blur: boss::blur::Blur,
    pub reflex: Option<boss::blur::Reflex>,
    /// The class's sword trail (Skeith's `ccLattice(2, 7)`, +0x2935c) and what its
    /// `Disp` sent this frame, with its sort key.
    pub lattice: Lattice,
    pub trail: (Vec<StripVertex>, u32),
    /// `ccBossEffManager`'s cinema (+0x00) and what its `Draw` sent this
    /// frame.
    pub cinema: Cinema,
    pub cinema_sent: CinemaSent,
    /// A clip of the boss's file on a layer of its own ([`Out::AnmText`],
    /// Fidchell's prediction): its clip, the layer and its `ccAnm`; and
    /// whether it was drawn this frame.
    pub text: Option<(String, i16, piney_desktop::anm::Anm)>,
    pub text_shown: bool,
    /// The images it keeps (their characters), drawn while theirs `drawSW`
    /// holds.
    pub images: Vec<usize>,
    /// Its parts (their characters), drawn while theirs `drawSW` holds.
    pub parts: Vec<usize>,
    /// `WORLD_MAN.eventmap` as an `EVENTAREAB8` this frame (Kyvia's disc),
    /// and whether the boss asked it on (`EVENTAREAB8::Move`).
    pub disc: boss::DiscView,
    /// `EVENTAREAB8`'s `body[n]` hits the fight turned off or on this
    /// frame (field 12's), for the area.
    pub body_hits: Vec<(usize, bool)>,
    /// Field 8's arena clump switched this frame (`eventArea` +0x2bc,
    /// Corbenik's lasers), in order.
    pub arena_clump: Vec<bool>,
    pub disc_next: bool,
    /// The body's objects its after-images hide this frame.
    pub dropped: Vec<&'static str>,
    /// Each part's `ccAnm` transparency (+0x88) as its last `ccBoss::Draw`
    /// copied it (the constructor's 1 before).
    pub anm_alpha: HashMap<usize, F>,
}

/// One `ccBossAfterImageEx` (`Init` MUT gcmn 0x004710b0, `Draw`
/// 0x00471320): a pose when it was entered, its blend's rate moved by its
/// speed and its transparency by its fade each draw, then drawn on
/// effLayer at the new values, its scale grown first by `grow`'s step to
/// its end. The boss's (`m_eai`), a blinking leaf's of Magus's (its own
/// model) or one of a `ccAnm` of the boss's file (Kyvia's arm).
#[derive(Clone, Debug, PartialEq)]
pub struct AfterImage {
    pub clip: String,
    pub time: u32,
    pub pos: V4,
    pub dirc: V4,
    pub transparency: f32,
    pub fade: f32,
    pub image: Image,
    /// `SetFogBlend(colorAlpha, color)`; rate 0 for none.
    pub blend: crate::chara::foe::FogBlend,
    pub blend_spd: ee::F,
    pub scale: [f32; 3],
    pub grow: Option<([f32; 3], [f32; 3])>,
}

/// What an after-image poses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Image {
    /// The boss's body.
    Boss,
    /// One of the look's images (a part's body: Magus's leaf).
    Look(usize),
    /// The objects a clip of the boss's file animates (`ccAnm::Draw`).
    Anm,
}

/// A picture the boss's code or effects draw beside the bodies, as the
/// field draws it: an object of its file or xeffect (`eff`) at its world
/// matrix (lit, in its fog, on charLayer), a clip posed at `time` under
/// `root` on `layer`, a part's clip drawn once more, or a clump of
/// [`BossLook::shards`] (objLayer); each at its transparency.
#[derive(Clone, Debug, PartialEq)]
pub enum Picture {
    Obj {
        eff: bool,
        name: &'static str,
        world: glam::Mat4,
        alpha: f32,
        fog: Option<piney_draw::Fog>,
        blend: Option<u8>,
    },
    Anm {
        eff: bool,
        layer: i16,
        clip: String,
        time: u32,
        root: glam::Mat4,
        alpha: f32,
        blend: Option<u8>,
    },
    /// A part's clip its `PreDrawAnm` draws ([`boss::BodyDraw::again`])
    /// before its `ccBoss::Draw`, at the matrix it set and the anm's last
    /// transparency, on the layer the body's `Draw` left active (sysLayer:
    /// priority 0, after objLayer).
    Again {
        who: usize,
        alpha: f32,
    },
    Clump {
        clump: usize,
        world: glam::Mat4,
    },
}

/// `m_eai.transSpd`.
pub const AFTERIMAGE_FADE: f32 = 0.05;

impl AfterImage {
    /// `EntryAfterImage(&m_eai)`: the boss's pose, white.
    pub fn boss(clip: String, time: u32, pos: V4, dirc: V4) -> AfterImage {
        AfterImage {
            clip,
            time,
            pos,
            dirc,
            transparency: 1.0,
            fade: AFTERIMAGE_FADE,
            image: Image::Boss,
            blend: crate::chara::foe::FogBlend { rate: 0x42c8_0000, colour: 0x00ff_ffff },
            blend_spd: 0,
            scale: [1.0; 3],
            grow: None,
        }
    }

    /// `EntryAfterImageAnm(anm, pos, dirc, p)` of a clip of the boss's
    /// file at `time`, as `Init` takes `p` (colour -1: no blend; a scale
    /// only with `scaleTime` over 0).
    pub fn anm(clip: String, time: u32, pos: V4, dirc: V4, p: &boss::AfterImageParams) -> AfterImage {
        let (ss, es) = (p.ss.map(f32::from_bits), p.es.map(f32::from_bits));
        let scale_time = f32::from_bits(p.scale_time);
        let (scale, grow) = if scale_time > 0.0 {
            let step = std::array::from_fn(|k| (es[k] - ss[k]) * (1.0 / scale_time));
            ([ss[0], ss[1], ss[2]], Some((step, [es[0], es[1], es[2]])))
        } else {
            ([1.0; 3], None)
        };
        let rate = if p.color == u32::MAX { 0 } else { p.color_alpha };
        AfterImage {
            transparency: f32::from_bits(p.transparency),
            fade: -f32::from_bits(p.trans_spd),
            image: Image::Anm,
            blend: crate::chara::foe::FogBlend { rate, colour: p.color },
            blend_spd: p.color_spd,
            scale,
            grow,
            ..AfterImage::boss(clip, time, pos, dirc)
        }
    }

    /// `EntryAfterImage` on a part's own `ccAnm` in `p` (a leaf's blink),
    /// drawn with look image `k`.
    pub fn part(k: usize, clip: String, time: u32, pos: V4, dirc: V4, p: &boss::AfterImageParams) -> AfterImage {
        AfterImage { image: Image::Look(k), ..AfterImage::anm(clip, time, pos, dirc, p) }
    }

    /// `EntryAfterImage(&t)`: the boss's pose in `t`'s params (Macha's
    /// white ones: no blend, transparency 1 by -1/30).
    pub fn boss_with(clip: String, time: u32, pos: V4, dirc: V4, p: &boss::AfterImageParams) -> AfterImage {
        AfterImage { image: Image::Boss, ..AfterImage::anm(clip, time, pos, dirc, p) }
    }

    /// The blend, the fade and the growth of one `Draw`; false once faded
    /// out.
    fn step(&mut self) -> bool {
        if !ee::eq(self.blend_spd, 0) {
            let r = f32::from_bits(ee::add(self.blend.rate, self.blend_spd)).clamp(0.0, 100.0);
            self.blend.rate = r.to_bits();
        }
        self.transparency -= self.fade;
        if let Some((step, to)) = self.grow {
            for k in 0..3 {
                self.scale[k] += step[k];
                if self.scale[k].abs() > to[k].abs() {
                    self.scale[k] = to[k];
                }
            }
        }
        self.transparency > 0.0
    }
}

/// What the boss's look needs from the disc: the body, every clip of
/// its files, the effects' file (the wave), the cinema's skill names and
/// Innis's images.
pub struct BossLook {
    pub code: i32,
    /// The row it looks like (Skeith's for a code the port lacks).
    pub def: &'static BossDef,
    pub body: Rc<Body>,
    pub clips: Rc<Clips>,
    pub eff: Rc<piney_desktop::assets::SceneFile>,
    /// `_g_cinemaSkillName[n]` of this boss's file (`OnCinemaMode(n)`'s
    /// name), by `n`: on Infection Skeith's 2 (x11's `TEX_ske_skl`, row
    /// 0); on Mutation Innis's 5-8 and 70 (`TEX_ini_skl`, rows 0-4).
    pub names: HashMap<i32, CinemaName>,
    /// `OnCinemaMode` with a skill's name (OUT gcmn 0x004733b0): the row
    /// OUT 0x00472550 gives for the fight's `game.field`, by skill id
    /// (field 4: Fidchell's seven, x41's `TEX_fid_skl`).
    pub skill_names: HashMap<i32, CinemaName>,
    /// The clumps of its file [`BossDef::images`] names: Innis's images by
    /// `MonsterID`, Kyvia's core and gomora, Magus's leaf, the Epitaphs'
    /// bodies, Gorre's brothers, Corbenik's pieces.
    pub images: Vec<Rc<Body>>,
    /// Innis's mirrors' shards (`BreakMirror::SetStream(x21, ID)`:
    /// `CMP_ex21ef{ID}1`-`5`), the file and each clump's models, by
    /// `ID * 5 + k`.
    pub shards: Option<(Rc<piney_desktop::assets::SceneFile>, Vec<Vec<u32>>)>,
    /// `DMY_dummy1` of the boss's file (Corbenik's third form's eyes ride
    /// it), 0 without one.
    pub dummy: V4,
}

impl BossLook {
    /// Boss `code`'s look (Skeith's for any code the port lacks), fought
    /// in field `field` (`game.field`).
    pub fn load(
        archive: &std::sync::Arc<piney_data::archive::Archive>,
        code: i32,
        volume: piney_data::volume::Volume,
        field: i32,
    ) -> piney_data::Result<BossLook> {
        let def = BossDef::of(code).unwrap_or(&piney_battle::boss::defs::SKEITH);
        let (file, clump) = (def.file, def.clump);
        let body = Rc::new(Body::read(archive, file, clump)?);
        let dummy = body
            .file
            .scene
            .dummies
            .iter()
            .find(|(o, _)| body.file.ccs.object_name(**o) == Some("DMY_dummy1"))
            .map_or([0; 4], |(_, d)| [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE]);
        let mut clips = Clips::new();
        for stem in [file, EFF_FILE] {
            let ccs = piney_data::ccs::Ccs::parse(archive.inflate_named(stem)?)?;
            let scene = piney_data::scene::Scene::read(&ccs)?;
            for a in piney_data::anim::Animation::all(&ccs, &scene)? {
                if let Some(name) = ccs.object_name(a.object) {
                    clips.insert(name.to_string(), (a.frames, a.looping));
                }
            }
        }
        let eff = Rc::new(piney_desktop::assets::SceneFile::read(archive, EFF_FILE)?);
        // SetupSkillName (MUT gcmn 0x00480540, OUT 0x0047bed0): in fields
        // 9-12 the file is "x01" to "x04" whatever the row says (Kyvia's
        // rows name none).
        let row_file = |f: Option<&str>| match field {
            9..=12 => Some(format!("x0{}", field - 8)),
            _ => f.map(str::to_string),
        };
        let mut names = HashMap::new();
        let mut textures: HashMap<&str, Option<piney_desktop::message::WindowTexture>> = HashMap::new();
        for (n, row) in piney_data::tables::combat::of(volume).cinema_skill_names.iter().enumerate() {
            let (Some(f), Some(tex)) = (row_file(row.file), row.tex) else { continue };
            if f != file {
                continue;
            }
            let w = textures
                .entry(tex)
                .or_insert_with(|| piney_desktop::message::WindowTexture::read_named(archive, file, tex, None));
            if let Some(w) = w {
                names.insert(n as i32, CinemaName { tex: w.tex.clone(), tex_h: w.tex_h, row: row.row });
            }
        }
        let mut skill_names = HashMap::new();
        let mut skill_tex: HashMap<(String, &str), Option<piney_desktop::message::WindowTexture>> = HashMap::new();
        for row in piney_data::tables::combat::of(volume).cinema_skill_rows.iter().filter(|r| r.field == field) {
            let (Some(f), Some(tex)) = (row_file(row.file), row.tex) else { continue };
            let w = skill_tex
                .entry((f.clone(), tex))
                .or_insert_with(|| piney_desktop::message::WindowTexture::read_named(archive, &f, tex, None));
            if let Some(w) = w {
                skill_names.insert(row.sid, CinemaName { tex: w.tex.clone(), tex_h: w.tex_h, row: row.row });
            }
        }
        let of_file = |c: &&str| Body::of(body.file.clone(), c).ok().map(Rc::new);
        let images = def.images.iter().filter_map(of_file).collect();
        let shards = def.shards.then(|| {
            let clumps = (0..3)
                .flat_map(|id| (1..=5).map(move |k| format!("CMP_ex21ef{id}{k}")))
                .map(|c| crate::town::clump_models(&body.file, &c))
                .collect();
            (body.file.clone(), clumps)
        });
        Ok(BossLook { code, def, body, clips: Rc::new(clips), eff, names, skill_names, images, shards, dummy })
    }
}

impl Combat {
    /// `ccBossEntryStart(code)`: the boss its def makes at the arena's
    /// centre `center` (`DMY_center01`) or Kyvia's disc (`disc`), and its
    /// actor. Nothing for a code the port lacks, without the tables or
    /// without Kite. `third` is Corbenik's: `saveData` +0x652f
    /// (`eventStatus[55]`) 1, which `ccThBoss08` reads for its third form.
    pub fn start_boss(
        &mut self,
        look: &Rc<BossLook>,
        center: V4,
        disc: boss::DiscView,
        env: &Env,
        camera: &mut Camera,
        third: bool,
    ) {
        let code = look.code;
        let Some(def) = BossDef::of(code).filter(|_| self.boss.is_none()) else { return };
        let d = self.data.clone();
        let Some(kite) = self.kite else { return };
        let Some(row) = d.t.bosses.get(code as usize).cloned() else { return };
        let mut ch = piney_battle::Char::foe(row);
        // ccChar::ccChar: no condition (conditionNum -1).
        ch.condition_num = -1;
        ch.affect.func = AffectFunc::Boss;
        let me = self.scene.add(ch, 3);
        let kite_pos = self.scene.chars[kite].pos;
        let kite_dirc = self.kite_dirc();
        let clips = look.clips.clone();
        let clip = |name: &str| clips.get(name).copied();
        let party = self.party;
        let check = |_: usize| 0;
        let benv = BossEnv { t: &d.t, clips: &clip, env, game_over: false, cc: None };
        let actx =
            AffectCtx { party: &party, menu: true, skill_check: &check, boss: Some(&benv), volume: self.data.volume };
        let mut none = |_| None;
        let mut land = |p: V4| p[2];
        let mut cx = boss::Cx {
            t: &d.t,
            scene: &mut self.scene,
            party: &party,
            world: World { player: Some(kite), ..World::default() },
            env,
            actx: &actx,
            rand: &mut self.rand,
            cc: &mut self.cc,
            clips: &clip,
            collide: &mut none,
            field: &mut boss::NoField,
            game_over: false,
            boss_cam: true,
            cam: cam_view(camera, None, false),
            land: &mut land,
            disc,
            gen_alive: &|_| true,
            me,
            out: Vec::new(),
            ev: Events::new(),
        };
        let spawn = Spawn { kite_pos, kite_dirc, center, third, dummy: look.dummy };
        let mut b = (def.make)(&mut cx, &spawn);
        b.show(&mut cx);
        let out = std::mem::take(&mut cx.out);
        drop(cx);
        let ctor_out = out.clone();
        for o in out {
            self.shows.push(Show::Boss(me, o));
        }
        let pos = self.scene.chars[me].pos;
        let (h, w) = (self.scene.chars[me].base().height, self.scene.chars[me].base().width);
        let clip0 = b.base.anm.clip.clone().unwrap_or_default();
        let body0 = match look.def.body {
            BodyPick::ByClip => {
                clip_image(&look.images, &clip0).map_or_else(|| look.body.clone(), |k| look.images[k].clone())
            }
            BodyPick::Body | BodyPick::Epitaph(_) => look.body.clone(),
        };
        if let Some(a) = Actor::new(body0, &clip0, Look::Boss, pos, b.base.dirc, h, w) {
            self.cast.actors.insert(me, a);
        }
        let images = b.output.figures.iter().map(|f| f.me).collect();
        let parts = b.output.parts.clone();
        let (cam_def, radius) = (b.def.cam, b.def.radius.unwrap_or(w));
        if let Some(f) = self.scene.chars[me].foe_state_mut() {
            f.boss = Some(Box::new(b));
        }
        let ty = self.scene.chars[me].ty() as u32;
        let hit = CharHit { pos, radius, height: h, kind: ty, ..CharHit::default() };
        // InitBossCamera(z, y), in the constructor, and what the
        // constructor set on it then.
        let cam = new_cam(cam_def, camera, kite_pos, &ctor_out);
        self.boss = Some(BossRun {
            code,
            me,
            center,
            exit: false,
            clips: look.clips.clone(),
            hit,
            look: look.clone(),
            afterimages: Vec::new(),
            waves: Vec::new(),
            light: None,
            fog: None,
            pictures: Vec::new(),
            cam_sw: cam.is_some(),
            cam,
            reverse: false,
            blur: {
                let mut blur = boss::blur::Blur::new(self.heap_exit);
                for o in &ctor_out {
                    if let Out::Blur(s) = o {
                        blur.set(*s);
                    }
                }
                blur
            },
            reflex: None,
            lattice: Lattice::new(2, 7),
            trail: (Vec::new(), 0),
            cinema: Cinema::new(),
            cinema_sent: CinemaSent::default(),
            text: None,
            text_shown: false,
            images,
            parts,
            disc,
            disc_next: false,
            body_hits: Vec::new(),
            arena_clump: Vec::new(),
            dropped: Vec::new(),
            anm_alpha: HashMap::new(),
        });
    }

    /// The end of `ccBossEffManager::Draw` (INF gcmn 0x00461750): the
    /// blur's `Main`, after the writes the boss's and its parts' affects
    /// asked since their frame. The manager's task outlives the boss's.
    pub(super) fn blur_pass(&mut self) {
        let Some(run) = self.boss.as_mut() else { return };
        for c in std::iter::once(run.me).chain(run.parts.iter().copied()) {
            if let Some(b) = self.scene.chars[c].foe_state_mut().and_then(|f| f.boss.as_mut()) {
                for s in b.base.take_blur() {
                    run.blur.set(s);
                }
            }
        }
        run.reflex = run.blur.main(&mut self.blur_statics);
    }

    /// `ccThBossEffect` and the boss task's frame: the queued affects,
    /// the boss's `Main`, its actors set from what it shows; what it asks
    /// goes to the shows ([`Show::Boss`]). The events its affects cause
    /// come back.
    pub(super) fn boss_frame(
        &mut self,
        env: &Env,
        world: World,
        check: &dyn Fn(usize) -> i32,
        hits: &mut crate::hit::Hits,
        camera: &mut Camera,
        pad: &CamPad,
    ) -> Events {
        let mut ev = Events::new();
        let Some(run) = self.boss.as_mut() else { return ev };
        if run.exit {
            // The manager's task outlives the boss's: the blur goes on.
            self.blur_pass();
            return ev;
        }
        // ccThBossEffect's pass, before the boss's task: the cinema's Draw.
        run.cinema_sent = run.cinema.step();
        let me = run.me;
        let hit = run.hit;
        let clips = run.clips.clone();
        // checkCameraShakeRange of the boss's place, as its Think asks it.
        let shake = shake_range(camera, &world, self.scene.chars[me].pos);
        let cv = cam_view(camera, run.cam.as_ref(), shake);
        let d = self.data.clone();
        let clip = |name: &str| clips.get(name).copied();
        let party = self.party;
        let benv = BossEnv { t: &d.t, clips: &clip, env, game_over: false, cc: None };
        let actx =
            AffectCtx { party: &party, menu: true, skill_check: check, boss: Some(&benv), volume: self.data.volume };
        // The frame's hits on the boss, with the fight's ccRand.
        let cc = std::cell::RefCell::new(std::mem::take(&mut self.cc));
        {
            let benv = BossEnv { cc: Some(&cc), ..benv };
            let actx = AffectCtx { boss: Some(&benv), ..actx };
            boss::apply_queued(&mut self.scene, &actx, me, &mut self.rand, &mut ev);
        }
        self.cc = cc.into_inner();
        // The parts: their affects of the frame so far, and what they
        // asked (their hits' marks and numbers).
        let parts = run.parts.clone();
        // The blur as the manager's pass finds it (the fourth fight's
        // effects and its great meteor read its colour).
        let mut disc = run.disc;
        disc.blur_abgr = run.blur.abgr;
        disc.blur_scale = run.blur.scale;
        disc.blur_exit = run.blur.exit;
        let mut part_out = Vec::new();
        for &p in &parts {
            boss::apply_queued(&mut self.scene, &actx, p, &mut self.rand, &mut ev);
            if let Some(pb) = self.scene.chars[p].foe_state_mut().and_then(|f| f.boss.as_mut()) {
                part_out.extend(pb.base.take_pending().into_iter().map(|o| (p, o)));
            }
        }
        let Some(mut b) = self.scene.chars[me].foe_state_mut().and_then(|f| f.boss.take()) else { return ev };
        let mut out = b.base.take_pending();
        // ccBoss::Move: SetHitSW(bodyHitSW), then the body at the new place
        // pushed out of the others (CollisionDetection).
        let kite = self.kite;
        let mut body = super::spc::to_body(me, kite, &hit, false);
        hits.set_hit_sw(&mut body, b.base.body_hit_sw != 0);
        let hits = RefCell::new(hits);
        let mut collide = |pos: V4| {
            let mut hits = hits.borrow_mut();
            let mut body = super::spc::to_body(me, kite, &CharHit { pos, sw: true, ..hit }, false);
            hits.sync(&body);
            let off = (hits.collision_detection(&mut body) != 0).then_some(body.offset)?;
            // After a push the hit stands where the boss was put (the
            // push's height dropped), for the parts that move after it.
            body.pos = ee::vadd(pos, [off[0], off[1], 0, off[3]]);
            hits.sync(&body);
            Some(off)
        };
        let mut field = FieldView { hits: &hits, kite, actors: &self.cast.actors, spc: &self.crew.spc };
        // BreakMirror's ccLandHitCheck(pos, 0x20000000).
        let mut land = |pos: V4| hits.borrow_mut().land(pos, 0x2000_0000);
        // ccCheckParticleGenerator on a generator the rules keep.
        let alive = self.gens_alive.clone();
        let gen_alive = |k: u32| alive.as_ref().is_none_or(|s| s.contains(&k));
        let mut cx = boss::Cx {
            t: &d.t,
            scene: &mut self.scene,
            party: &party,
            world,
            env,
            actx: &actx,
            rand: &mut self.rand,
            cc: &mut self.cc,
            clips: &clip,
            collide: &mut collide,
            field: &mut field,
            game_over: false,
            boss_cam: true,
            cam: cv,
            land: &mut land,
            disc,
            gen_alive: &gen_alive,
            me,
            out: Vec::new(),
            ev: Events::new(),
        };
        // ccThBossEffect's pass: the effects the rules keep, then the
        // blur's Main (with what they and the affects wrote), then the
        // boss's task, whose writes the next frame's Main sees.
        b.effects_pass(&mut cx);
        let fx_out = std::mem::take(&mut cx.out);
        if let Some(r) = self.boss.as_mut() {
            let parts = r.parts.clone();
            let mut pending = Vec::new();
            for &p in &parts {
                if let Some(pb) = cx.scene.chars[p].foe_state_mut().and_then(|f| f.boss.as_mut()) {
                    pending.extend(pb.base.take_blur());
                }
            }
            let fx_sets = fx_out.iter().filter_map(|o| if let Out::Blur(s) = o { Some(*s) } else { None });
            for s in b.base.take_blur().into_iter().chain(pending).chain(fx_sets) {
                r.blur.set(s);
            }
            r.reflex = r.blur.main(&mut self.blur_statics);
            cx.disc.blur_abgr = r.blur.abgr;
            cx.disc.blur_scale = r.blur.scale;
            cx.disc.blur_exit = r.blur.exit;
            // A mode the effects set (the ball's end) is the camera's
            // before the boss reads its ResetFlg.
            if let Some(c) = r.cam.as_mut() {
                for o in &fx_out {
                    match o {
                        Out::CamMode { mode } => c.set_mode(*mode, camera),
                        Out::CamModeRange { .. } => cam_out(c, o, camera),
                        _ => {}
                    }
                }
                cx.cam.reset = c.reset;
            }
        }
        let early = |o: &Out| matches!(o, Out::Blur(_) | Out::CamMode { .. } | Out::CamModeRange { .. });
        out.extend(fx_out.into_iter().filter(|o| !early(o)));
        b.main(&mut cx);
        b.hand_over(&mut cx);
        b.show(&mut cx);
        out.extend(std::mem::take(&mut cx.out));
        ev.extend(std::mem::take(&mut cx.ev));
        drop(cx);
        let hits = hits.into_inner();
        // A boss made anew on its character (Corbenik's next form, the same
        // frame): its hit, parts, after-images and camera as its def makes
        // them, and the parts gone with the old one off the field.
        let (mut hit, mut parts) = (hit, parts);
        if out.contains(&Out::Remade)
            && let (Some(r), Some(k)) = (self.boss.as_mut(), kite)
        {
            let base = self.scene.chars[me].base();
            let (h, w, ty) = (base.height, base.width, base.ty as u32);
            let radius = b.def.radius.unwrap_or(w);
            r.hit = CharHit { pos: self.scene.chars[me].pos, radius, height: h, kind: ty, ..CharHit::default() };
            r.parts = b.output.parts.clone();
            r.afterimages.clear();
            r.cam = new_cam(b.def.cam, camera, self.scene.chars[k].pos, &[]);
            r.cam_sw = true;
            (hit, parts) = (r.hit, r.parts.clone());
        }
        for o in &out {
            if let Out::PartRemoved(k) = *o {
                self.cast.actors.remove(&k);
                let id = super::spc::body_id(k, self.kite);
                hits.chars.retain(|c| c.id != id);
            }
        }
        let exited = b.base.exit != 0;
        // The body's copy in the list where the boss stands now.
        let at = CharHit { pos: self.scene.chars[me].pos, sw: b.base.body_hit_sw != 0 && !exited, ..hit };
        let mut body = super::spc::to_body(me, kite, &at, false);
        hits.set_hit_sw(&mut body, at.sw);
        hits.sync(&body);
        // The boss's sword trail after its Main: a new row at the dummies
        // it names (as the body was last drawn) when it adds one, then the
        // trail's Disp.
        let shown = &b.output;
        let row = match shown.trail {
            boss::Trail::Row { node, points } => (self.cast.actors.get(&me))
                .and_then(|a| dummy_points(&a.ch.body, &a.ch.play, a.ch.pos, a.ch.dirc, node, points)),
            boss::Trail::None | boss::Trail::Keep => None,
        };
        if let Some(r) = self.boss.as_mut()
            && shown.trail != boss::Trail::None
        {
            if let Some([p1, p2]) = row {
                r.lattice.next_vertex();
                r.lattice.set_pos(p1, 0);
                r.lattice.set_pos(p2, 1);
            }
            r.trail = r.lattice.disp();
        }
        let dirc = b.base.dirc;
        let wave_time = shown.wave.unwrap_or(0);
        let clip = b.base.anm.clip.clone().unwrap_or_default();
        let time = b.base.anm.posed;
        let kite_pos = kite.map(|k| self.scene.chars[k].pos);
        if let Some(r) = self.boss.as_mut() {
            // Think and Action's QuakeCam, SetMode, SetFreeCamPosView and
            // BeginDeadEffect, then after Move the camera (bossCam &&
            // bossCamSW: CamMain).
            let text_was = std::mem::take(&mut r.text_shown);
            for o in &out {
                match o {
                    Out::Blur(s) => r.blur.set(*s),
                    Out::BlurSinRot(v) => self.blur_statics.sin_rot = *v,
                    // OnCinemaMode's other checks (the party wiped out, a
                    // forced game over) end the fight first here.
                    Out::Cinema(Some(n)) => {
                        let name = r.look.names.get(n).cloned();
                        r.cinema.on(name);
                    }
                    Out::Cinema(None) => r.cinema.off(),
                    // CinemaOn with a skill (OUT 0x0047c0a0): the name OUT
                    // 0x00472550 gives for the field, when it has one.
                    Out::CinemaSkill(sid) => {
                        let name = r.look.skill_names.get(sid).cloned();
                        r.cinema.on(name);
                    }
                    // A clip on its own layer: SetAnm anew when the clip
                    // changed or was not up the frame before, then
                    // _AnimateForward and Draw each frame.
                    Out::AnmText { clip, layer } => {
                        if !text_was || r.text.as_ref().is_none_or(|(c, _, _)| c != clip) {
                            let mut a = piney_desktop::anm::Anm::new();
                            a.set(&r.look.body.file, clip);
                            a.zenable = false;
                            r.text = Some((clip.clone(), *layer, a));
                        }
                        if let Some((_, _, a)) = r.text.as_mut() {
                            a.forward();
                        }
                        r.text_shown = true;
                    }
                    Out::Quake(v) => {
                        if let Some(c) = r.cam.as_mut() {
                            c.quake(*v);
                        }
                    }
                    Out::CamMode { mode } => {
                        if let Some(c) = r.cam.as_mut() {
                            c.set_mode(*mode, camera);
                        }
                    }
                    Out::FreeCam { pos, view } => {
                        if let Some(c) = r.cam.as_mut() {
                            c.free_cam_pos_view(*pos, *view);
                        }
                    }
                    // Mutation's +0xe0, the pitch CamMain turns the view by.
                    Out::CamPitch { add, v } => {
                        if let Some(c) = r.cam.as_mut() {
                            c.xrot = if *add { ee::add(c.xrot, *v) } else { *v };
                        }
                    }
                    Out::CamModeRange { .. }
                    | Out::CamInitLock
                    | Out::CamRotXLimit(_)
                    | Out::CamMaxRange(_)
                    | Out::CamLift(_)
                    | Out::CamSway(_) => {
                        if let Some(c) = r.cam.as_mut() {
                            cam_out(c, o, camera);
                        }
                    }
                    Out::DiscNextStage => r.disc_next = true,
                    // Kyvia's fourth fight: CamPos written, field 12's walls.
                    // Corbenik's forms write CamView and the lock too.
                    Out::CamEye(pos) => {
                        if let Some(c) = r.cam.as_mut() {
                            c.pos = *pos;
                        }
                    }
                    Out::CamLook(view) => {
                        if let Some(c) = r.cam.as_mut() {
                            c.view = *view;
                        }
                    }
                    Out::CamLock(on) => {
                        if let Some(c) = r.cam.as_mut() {
                            c.lock = *on;
                        }
                    }
                    // activeCamPtr->type (Corbenik's death: 3).
                    Out::CameraType(t) => camera.active_mut().kind = *t,
                    Out::FieldBodyHit { n, on } => r.body_hits.push((*n, *on)),
                    Out::StageClump(on) => r.arena_clump.push(*on),
                    // Magus's cameras: changeCamera, cameraSetPos/View.
                    Out::CameraChange(n) => camera.change_camera(*n as i16),
                    Out::CameraPos { cam, pos } => camera.cam_mut(*cam as i16).pos = *pos,
                    Out::CameraView { cam, view } => camera.cam_mut(*cam as i16).view = *view,
                    // Gorre's fall (OffBossCamera): the boss camera handed
                    // to camera 1, off.
                    Out::BossCamOff => {
                        if r.cam_sw {
                            if r.cam.is_some() {
                                camera.boss_cam_off();
                            } else {
                                camera.change_camera(id::FIELD);
                            }
                        }
                        r.cam_sw = false;
                    }
                    Out::DeadCamera { eye, view, .. } => {
                        if r.cam_sw {
                            if r.cam.is_some() {
                                camera.boss_cam_off();
                            } else {
                                camera.change_camera(id::FIELD);
                            }
                        }
                        r.cam_sw = false;
                        camera.change_camera(id::EVENT);
                        let e = camera.cam_mut(id::EVENT);
                        e.pos = *eye;
                        e.view = *view;
                    }
                    _ => {}
                }
            }
            if r.cam_sw
                && let (Some(c), Some(k)) = (r.cam.as_mut(), kite_pos)
            {
                c.cam_main(camera, pad, k, at.pos);
            }
            r.hit = at;
            // EntryAfterImage in the frame's Think, then DrawAfterImages:
            // each image steps its fade as it draws, the new one too.
            r.waves.clear();
            r.fog = None;
            r.reverse = out.contains(&Out::Reverse);
            // The shards it drew (`DrawPartsClump`), then its pictures.
            r.pictures = (shown.clumps.iter())
                .map(|&(clump, pos, rot)| Picture::Clump { clump, world: crate::chara::body::root(pos, rot) })
                .collect();
            // Where a picture on a node of the body rides: the body as
            // it was last drawn (`GetSubstAdrsF`).
            let actor = self.cast.actors.get(&me);
            let node = |name: &str| actor.and_then(|a| node_world(&a.ch.body, &a.ch.play, a.ch.pos, a.ch.dirc, name));
            for o in &out {
                match o {
                    Out::AfterImage => r.afterimages.push(AfterImage::boss(clip.clone(), time, at.pos, dirc)),
                    // A part's after-image: its anm at `frameNow`
                    // (`_AnimateFrame`; at frame 0 `Init` steps it once).
                    &Out::AfterImagePart { who, frame, ref params } => {
                        let Some(k) = parts.iter().position(|&p| p == who) else { continue };
                        let Some(pb) = self.scene.chars[who].foe_state().and_then(|f| f.boss.as_ref()) else {
                            continue;
                        };
                        let mut a = pb.base.anm.clone();
                        (a.time, a.posed) = (frame as u32 * 256, frame as u32 * 256);
                        if frame == 0 {
                            a.forward();
                        }
                        let clip = a.clip.unwrap_or_default();
                        let Some(image) = part_image(&r.look, k, &clip) else { continue };
                        let p = self.scene.chars[who].pos;
                        r.afterimages.push(AfterImage::part(image, clip, a.posed, p, pb.base.dirc, params));
                    }
                    Out::DrawWave => r.waves.push((wave_time, at.pos, dirc, 1.0)),
                    Out::AfterImageWith(p) => {
                        r.afterimages.push(AfterImage::boss_with(clip.clone(), time, at.pos, dirc, p));
                    }
                    Out::Light(p) => r.light = *p,
                    Out::Wave { time, pos, dirc, scale, .. } => r.waves.push((*time, *pos, *dirc, ee::f(*scale))),
                    &Out::Fog { who, blend } if who == me => {
                        r.fog = Some(crate::chara::foe::FogBlend { rate: blend.rate, colour: blend.colour });
                    }
                    Out::Picture(p) => {
                        let alpha = |a: Option<F>| ee::f(a.unwrap_or(b.base.set_transparency));
                        let root = |at: &boss::Root| match *at {
                            boss::Root::At { pos, dirc } => Some(crate::chara::body::root(pos, dirc)),
                            boss::Root::Matrix(m) => Some(mat4(&m)),
                            boss::Root::Node(name) => node(name),
                        };
                        let shown = match p {
                            &boss::Picture::Obj { file, name, ref at, alpha: a, fog, blend } => {
                                root(at).map(|world| Picture::Obj {
                                    eff: file == boss::PicFile::Eff,
                                    name,
                                    world,
                                    alpha: alpha(a),
                                    fog: fog.and_then(|f| {
                                        crate::chara::foe::FogBlend { rate: f.rate, colour: f.colour }.fog()
                                    }),
                                    blend,
                                })
                            }
                            boss::Picture::Anm { file, layer, clip: c, time, at, alpha: a, blend } => {
                                root(at).map(|root| Picture::Anm {
                                    eff: *file == boss::PicFile::Eff,
                                    layer: match layer {
                                        boss::AnmLayer::Obj => crate::draw::OBJ_LAYER,
                                        boss::AnmLayer::Eff => crate::draw::EFF_LAYER,
                                        boss::AnmLayer::Char => crate::draw::CHAR_LAYER,
                                    },
                                    clip: c.clone().unwrap_or_else(|| clip.clone()),
                                    time: *time,
                                    root,
                                    alpha: alpha(*a),
                                    blend: *blend,
                                })
                            }
                        };
                        r.pictures.extend(shown);
                    }
                    // ccBossAfterImageEx::Init: its own anm at the frame,
                    // `_AnimateFrame(frame)`, or at frame 0 one step on.
                    Out::AfterImageAnm { clip, frame, pos, dirc, params, .. } => {
                        let mut a = piney_battle::boss::Anm::new();
                        a.set(clip, &|c: &str| clips.get(c).copied());
                        (a.time, a.posed) = (*frame as u32 * 256, *frame as u32 * 256);
                        if *frame == 0 {
                            a.forward();
                        }
                        r.afterimages.push(AfterImage::anm(clip.clone(), a.posed, *pos, *dirc, params));
                    }
                    _ => {}
                }
            }
            r.afterimages.retain_mut(AfterImage::step);
        }
        // The actor: ccAnm::Draw of the boss's clip at its frame, placed by
        // SetMatrix_PosRotZYX(pos, dirc) where its draw puts it, at the
        // transparency ccBoss::Draw left (the camera's fade), while drawn;
        // the objects its draw hides hidden.
        let body_draw = b.output.body;
        self.clip_body(&b, me);
        if let Some(a) = self.cast.actors.get_mut(&me) {
            pose(a, &b.base.anm, body_draw.pos, body_draw.dirc);
            a.drawn = body_draw.shown && !exited && b.base.drawn;
            a.alpha = b.base.transparency;
            a.trans_dist = false;
            a.ch.play.hidden = objects_named(&a.ch.body, &a.ch.play, &b.output.hidden);
        }
        if let Some(r) = self.boss.as_mut() {
            r.dropped = if b.output.hidden_in_images { b.output.hidden.clone() } else { Vec::new() };
        }
        self.image_actors(&b, exited);
        if let Some(f) = self.scene.chars[me].foe_state_mut() {
            f.boss = Some(b);
        }
        // The parts' own fog (an eye's flash) as their PreDrawAnm set it.
        let blends: HashMap<usize, crate::chara::foe::FogBlend> = out
            .iter()
            .filter_map(|o| match *o {
                Out::Fog { who, blend } if who != me => {
                    Some((who, crate::chara::foe::FogBlend { rate: blend.rate, colour: blend.colour }))
                }
                _ => None,
            })
            .collect();
        self.part_actors(&parts, exited, &blends);
        for (p, o) in part_out {
            match &o {
                Out::HitMark { by } => {
                    let by = by.map_or(Who::Nobody, Who::Char);
                    self.shows.push(Show::Rule(Event::HitMark { on: Who::Char(p), by }));
                }
                Out::FlyFont { kind, n } => {
                    self.shows.push(Show::Rule(Event::FlyFont { on: Who::Char(p), kind: *kind, value: *n }));
                }
                _ => {}
            }
            self.shows.push(Show::Boss(p, o));
        }
        for o in out {
            match &o {
                // ccDeleteCmnd: the rules took the character off the
                // lists themselves (a part's comes with the boss's outs).
                // checkCameraShakeRange(pos), then cameraShake: its `rand()`.
                Out::CameraShake { pos, s } => {
                    if shake_range(camera, &world, *pos) {
                        let rand = &mut self.rand;
                        camera.shake.shake(s[0], s[1], s[2], s[3], &mut || piney_battle::Rng::rand(rand));
                    }
                }
                // ccItemSkillRequest: the skill run from the boss on its
                // target, as an item's through a character.
                Out::Skill(tp, k) => self.item_skill(me, *tp, k),
                // A part's skill (ccItemSkillRequest, ccSkillRequestParam).
                Out::SkillFrom { user, target, skill } => self.item_skill(*user, *target, skill),
                // ccHitMarkDisp and ccEntryFlyFontNew over the boss: the
                // effects' as for any character's.
                Out::HitMark { by } => {
                    let by = by.map_or(Who::Nobody, Who::Char);
                    self.shows.push(Show::Rule(Event::HitMark { on: Who::Char(me), by }));
                }
                Out::FlyFont { kind, n } => {
                    self.shows.push(Show::Rule(Event::FlyFont { on: Who::Char(me), kind: *kind, value: *n }));
                }
                _ => {}
            }
            self.shows.push(Show::Boss(me, o));
        }
        if exited && let Some(r) = self.boss.as_mut() {
            r.exit = true;
            self.cast.actors.remove(&me);
            for k in std::mem::take(&mut r.images).into_iter().chain(std::mem::take(&mut r.parts)) {
                self.cast.actors.remove(&k);
                // A part's body (a gomora's) goes with its boss.
                let id = super::spc::body_id(k, self.kite);
                hits.chars.retain(|c| c.id != id);
            }
        }
        ev
    }

    /// The body for the boss's clip ([`BossDef::body`]): an Epitaph's
    /// under its clips, Corbenik's the clump of x81 its form's clip poses.
    fn clip_body(&mut self, b: &Boss, me: usize) {
        let Some(look) = self.boss.as_ref().map(|r| r.look.clone()) else { return };
        let clip = b.base.anm.clip.clone().unwrap_or_default();
        let body = match look.def.body {
            BodyPick::Body => return,
            BodyPick::ByClip => match clip_image(&look.images, &clip) {
                Some(k) => look.images[k].clone(),
                None => return,
            },
            BodyPick::Epitaph(start) => match look.images.first() {
                Some(x) if clip.starts_with(start) => x.clone(),
                _ => look.body.clone(),
            },
        };
        let pos = self.scene.chars[me].pos;
        let (h, w) = (self.scene.chars[me].base().height, self.scene.chars[me].base().width);
        if let Some(a) = self.cast.actors.get_mut(&me)
            && !Rc::ptr_eq(&a.ch.body, &body)
            && let Some(n) = Actor::new(body, &clip, Look::Boss, pos, b.base.dirc, h, w)
        {
            *a = n;
        }
    }

    /// The images the boss keeps ([`boss::Output::figures`]: Innis's
    /// `ccBoss02Slave::Draw` through `ccBoss::Draw`): each the look's image
    /// at its clip, place and turn, at the transparency `Draw` left, while
    /// its `drawSW` holds and it drew.
    fn image_actors(&mut self, b: &Boss, exited: bool) {
        let Some(look) = self.boss.as_ref().map(|r| r.look.clone()) else { return };
        for f in &b.output.figures {
            let shown = f.shown && !exited;
            let Some(body) = look.images.get(f.image).cloned() else { continue };
            let pos = self.scene.chars[f.me].pos;
            let entry = self.cast.actors.entry(f.me);
            let a = match entry {
                std::collections::btree_map::Entry::Occupied(o) => o.into_mut(),
                std::collections::btree_map::Entry::Vacant(v) => {
                    if !shown {
                        continue;
                    }
                    let clip = f.anm.clip.clone().unwrap_or_default();
                    let Some(a) = Actor::new(body.clone(), &clip, Look::Boss, pos, f.dirc, 0, 0) else { continue };
                    v.insert(a)
                }
            };
            if !Rc::ptr_eq(&a.ch.body, &body) {
                let clip = f.anm.clip.clone().unwrap_or_default();
                if let Some(n) = Actor::new(body.clone(), &clip, Look::Boss, pos, f.dirc, 0, 0) {
                    *a = n;
                }
            }
            pose(a, &f.anm, pos, f.dirc);
            a.drawn = shown;
            a.alpha = f.alpha;
            a.trans_dist = false;
        }
    }

    /// The parts (`ccBoss::Draw` of each): Kyvia's core's model and the
    /// gomoras', Magus's leaves, Gorre's brothers, Corbenik's parts, at
    /// their clips, places and turns ([`BossDef::part`]), at their `Alpha`,
    /// while their `drawSW` holds; at the matrix their `PreDrawAnm` set,
    /// in the fog it set (`blends`).
    fn part_actors(&mut self, parts: &[usize], exited: bool, blends: &HashMap<usize, crate::chara::foe::FogBlend>) {
        let Some(look) = self.boss.as_ref().map(|r| r.look.clone()) else { return };
        for (k, &p) in parts.iter().enumerate() {
            let Some(pb) = self.scene.chars[p].foe_state().and_then(|f| f.boss.as_ref()) else { continue };
            let drawn = pb.output.body;
            let shown = drawn.shown && pb.base.exit == 0 && !exited && pb.base.drawn;
            let clip = pb.base.anm.clip.as_deref().unwrap_or_default();
            let Some(body) = part_image(&look, k, clip).map(|i| look.images[i].clone()) else { continue };
            let (anm, dirc, alpha) = (pb.base.anm.clone(), pb.base.dirc, pb.base.transparency);
            let pos = self.scene.chars[p].pos;
            let matrix = drawn.matrix;
            let again = drawn.again && !exited;
            if let Some(r) = self.boss.as_mut() {
                let prev = r.anm_alpha.insert(p, alpha).unwrap_or(ONE);
                if again {
                    r.pictures.push(Picture::Again { who: p, alpha: ee::f(prev) });
                }
            }
            if let Some(a) = self.cast.actors.get_mut(&p)
                && !Rc::ptr_eq(&a.ch.body, &body)
                && let Some(n) = Actor::new(body.clone(), clip, Look::Boss, pos, dirc, 0, 0)
            {
                *a = n;
            }
            let a = match self.cast.actors.entry(p) {
                std::collections::btree_map::Entry::Occupied(o) => o.into_mut(),
                std::collections::btree_map::Entry::Vacant(v) => {
                    if !shown {
                        continue;
                    }
                    let clip = anm.clip.clone().unwrap_or_default();
                    let Some(a) = Actor::new(body, &clip, Look::Boss, pos, dirc, 0, 0) else { continue };
                    v.insert(a)
                }
            };
            pose(a, &anm, pos, dirc);
            a.drawn = shown;
            a.alpha = alpha;
            a.trans_dist = false;
            a.matrix = matrix;
            if let Some(blend) = blends.get(&p) {
                a.ch.blend = blend.fog();
            }
        }
    }

    /// The boss's scene index while it is in the field.
    pub fn boss_char(&self) -> Option<usize> {
        self.boss.as_ref().filter(|r| !r.exit).map(|r| r.me)
    }

    /// An event's `present 7` / `absent 7`: the boss task exists and has
    /// not exited.
    pub fn boss_present(&self) -> bool {
        self.boss.as_ref().is_some_and(|r| !r.exit)
    }

    /// The boss's HP for the HUD: (HP, max HP), None without a boss.
    pub fn boss_hp(&self) -> Option<(i16, i16)> {
        let me = self.boss_char()?;
        let c = &self.scene.chars[me];
        (c.cond[cond::DEAD] == 0).then_some((c.hp, c.max_hp))
    }
}

/// A boss's parts' bodies in the field's list (Kyvia's gomoras).
/// What a boss reaches of the field in its frame ([`boss::Field`]): the
/// collision list as its parts reach it, the land, its bodies as last
/// drawn (the cast's actors) and the characters' headings (the spcs').
struct FieldView<'h, 'w, 'c> {
    hits: &'h RefCell<&'w mut crate::hit::Hits>,
    kite: Option<usize>,
    actors: &'c std::collections::BTreeMap<usize, Actor>,
    spc: &'c std::collections::BTreeMap<usize, piney_battle::party_ai::Spc>,
}

impl boss::Field for FieldView<'_, '_, '_> {
    fn switch(&mut self, who: usize, hit: &mut CharHit, on: bool) {
        let mut body = super::spc::to_body(who, self.kite, hit, false);
        self.hits.borrow_mut().set_hit_sw(&mut body, on);
        hit.sw = body.sw;
    }

    fn collide(&mut self, who: usize, hit: &mut CharHit) -> bool {
        let mut body = super::spc::to_body(who, self.kite, hit, false);
        let touched = self.hits.borrow_mut().collision_detection(&mut body) != 0;
        hit.offset = body.offset;
        touched
    }

    fn sync(&mut self, who: usize, hit: &CharHit) {
        self.hits.borrow_mut().sync(&super::spc::to_body(who, self.kite, hit, false));
    }

    fn line(&mut self, sp: V4, ep: V4, mask: u32) -> Option<V4> {
        self.hits.borrow_mut().line(sp, ep, mask, 0).map(|(cp, _)| cp)
    }

    fn node(&mut self, who: usize, name: &str) -> Option<V4> {
        let a = self.actors.get(&who)?;
        let m = node_world(&a.ch.body, &a.ch.play, a.ch.pos, a.ch.dirc, name)?;
        let p = m * glam::Vec4::new(0.0, 0.0, 0.0, 1.0);
        Some([p.x.to_bits(), p.y.to_bits(), p.z.to_bits(), p.w.to_bits()])
    }

    fn clip_entries(
        &mut self,
        who: usize,
        clip: &str,
        time: [u32; 2],
        names: &[&str],
        out: &mut [Option<(V4, piney_battle::geom::M4)>],
    ) -> bool {
        let Some(a) = self.actors.get(&who) else { return false };
        let body = &a.ch.body;
        let Some(mut play) = body.play(clip) else {
            out.fill(None);
            return true;
        };
        (play.time, play.posed) = (time[0], time[1]);
        let file = &body.file;
        let root = crate::chara::body::root(a.ch.pos, a.ch.dirc);
        let poses = file.anims[play.anim].entry_poses_at(play.posed);
        let found: Vec<_> = play.instances(file, root).into_iter().zip(poses).collect();
        // An ExtObj copy is its own entry; the last of a name is taken.
        for (o, name) in out.iter_mut().zip(names) {
            *o = (found.iter().rev())
                .find(|(i, _)| file.ccs.object_name(i.entry) == Some(name))
                .map(|(i, p)| (rows(i.world)[3], rows(p.map_or(glam::Mat4::IDENTITY, |p| p.matrix()))));
        }
        true
    }

    fn dirc(&mut self, who: usize) -> Option<V4> {
        self.spc.get(&who).map(|s| s.dirc)
    }
}

/// Of `bodies`, the clump whose nodes `clip` poses: `ccAnm::SetAnm` takes
/// the clip's own objects, and its first track's is one of that clump's.
fn clip_image(bodies: &[Rc<Body>], clip: &str) -> Option<usize> {
    let poses = |b: &Body| {
        let first = b.anim(clip).and_then(|k| b.file.anims[k].tracks.first());
        first.is_some_and(|t| b.nodes.contains(&t.target))
    };
    bodies.iter().position(|b| poses(b))
}

/// Which of the look's images draws part `k` (of the boss's parts) at
/// `clip` ([`BossDef::part`]).
fn part_image(look: &BossLook, k: usize, clip: &str) -> Option<usize> {
    let k = match look.def.part {
        PartBody::ByClip => clip_image(&look.images, clip)?,
        PartBody::First => 0,
        PartBody::InOrder => usize::from(k > 0),
    };
    (k < look.images.len()).then_some(k)
}

/// The boss camera its def makes (`InitBossCamera` at the transfer, behind
/// `player`), and what its constructor set on it then (`ctor`).
fn new_cam(def: CamDef, camera: &mut Camera, player: V4, ctor: &[Out]) -> Option<BossCam> {
    let CamDef::Transfer(transfer) = def else { return None };
    let mut cam = BossCam::new(camera, transfer, player);
    for o in ctor {
        cam_out(&mut cam, o, camera);
    }
    Some(cam)
}

/// An actor posed as a `ccAnm` left it: its clip (set anew when it
/// changed), the time and the pose `PreDrawAnm`'s step made, the place.
fn pose(a: &mut Actor, anm: &piney_battle::boss::Anm, pos: V4, dirc: V4) {
    if let Some(c) = &anm.clip
        && a.ch.play.anim != a.ch.body.anim(c).unwrap_or(usize::MAX)
    {
        a.set(super::AnmSlot::Main, c);
    }
    a.ch.play.time = anm.time;
    a.ch.play.posed = anm.posed;
    a.ch.pos = pos;
    a.ch.dirc = dirc;
}

/// What a boss's rules read of the cameras this frame: the active
/// camera's turn (`cameraGetRot(camID)`), camera 2's eye and view, and
/// the boss camera's `CheckMoveCamera`.
pub(super) fn cam_view(camera: &Camera, bcam: Option<&BossCam>, shake: bool) -> CamView {
    let c = camera.cam(id::BATTLE);
    CamView {
        rot: camera.rot(),
        pos: c.pos,
        view: c.view,
        moving: bcam.is_some_and(BossCam::check_move_camera),
        reset: bcam.map_or(0, |b| b.reset),
        shake,
        eye: bcam.map_or(c.pos, |b| b.pos),
        active: i32::from(camera.cam_id),
        active_pos: camera.cam(camera.cam_id).pos,
        active_view: camera.cam(camera.cam_id).view,
        boss_view: bcam.map_or(c.view, |b| b.view),
        // ccBoss::Draw's fade: the active camera, a field's distances.
        fade: Some(boss::CamFade {
            eye: camera.active().pos,
            deg1: camera.active().deg[1],
            // checkCameraType(): activeCamPtr->type.
            eye_view: camera.active().kind == crate::camera::kind::EYE,
            far: crate::chara::foe::AREA_FAR,
            fade: crate::chara::foe::AREA_FADE,
        }),
    }
}

/// Two dummies of the boss's file (`points`) under `node`'s world matrix
/// (`GetObjAdrsF` of the clump), the body posed by `play` at `pos`, `dirc`
/// (`sceVu0ApplyMatrix`): a sword trail's row.
fn dummy_points(
    body: &Body,
    play: &crate::chara::pose::Play,
    pos: V4,
    dirc: V4,
    node: &str,
    points: [&str; 2],
) -> Option<[V4; 2]> {
    let m = node_world(body, play, pos, dirc, node)?;
    let point = |name: &str| -> Option<V4> {
        let d = body.file.scene.dummies.get(&body.file.ccs.find_object(name)?)?;
        let p = m * glam::Vec4::new(d.pos.x, d.pos.y, d.pos.z, 1.0);
        Some([p.x.to_bits(), p.y.to_bits(), p.z.to_bits(), p.w.to_bits()])
    };
    Some([point(points[0])?, point(points[1])?])
}

/// Node `name`'s world matrix, the body posed by `play` at `pos`, `dirc`.
fn node_world(body: &Body, play: &crate::chara::pose::Play, pos: V4, dirc: V4, name: &str) -> Option<glam::Mat4> {
    let n = body.node(name)?;
    body.worlds(play, crate::chara::body::root(pos, dirc)).get(&n).copied()
}

/// `checkCameraShakeRange(pos)` (main 0x00162f10): `pos` in the active
/// camera's view (`ccCheckCameraDeg(pos, 12288)`) and nearer its eye than
/// 2000, all through `W2PPos`.
fn shake_range(camera: &Camera, world: &World, pos: V4) -> bool {
    let cam = camera.active();
    let (p, c, v) = (world.frame.w2p(pos), world.frame.w2p(cam.pos), world.frame.w2p(cam.view));
    let a = ee::rad2deg(ee::atan2f(ee::sub(p[1], c[1]), ee::sub(p[0], c[0])));
    let b = ee::rad2deg(ee::atan2f(ee::sub(v[1], c[1]), ee::sub(v[0], c[0])));
    let d = (12288 + (i32::from(a) - i32::from(b))) as i16;
    if !(d > 0 && i32::from(d) < 2 * 12288) {
        return false;
    }
    let dv = ee::vsub(pos, cam.pos);
    ee::lt(ee::sqrtf_on(camera.volume, ee::dot(dv, dv)), 0x44fa_0000)
}

/// A matrix in the EE's rows (x, y, z axes, the place).
fn rows(m: glam::Mat4) -> piney_battle::geom::M4 {
    let row = |v: glam::Vec4| [v.x.to_bits(), v.y.to_bits(), v.z.to_bits(), v.w.to_bits()];
    [row(m.x_axis), row(m.y_axis), row(m.z_axis), row(m.w_axis)]
}

/// An EE matrix (its rows) as glam's.
fn mat4(m: &piney_battle::geom::M4) -> glam::Mat4 {
    glam::Mat4::from_cols_array_2d(&m.map(|r| r.map(f32::from_bits)))
}

/// The entries of `play` whose objects `names` are: what a draw hides.
pub fn objects_named(body: &Body, play: &crate::chara::pose::Play, names: &[&str]) -> Vec<u32> {
    if names.is_empty() {
        return Vec::new();
    }
    let file = &body.file;
    let entries = file.anims[play.anim].objects.iter().map(|&(e, _)| e);
    entries.filter(|&e| file.ccs.object_name(e).is_some_and(|n| names.contains(&n))).collect()
}

/// What a boss asks of its camera beside the frame's moves: a mode with a
/// range (Kyvia's 3 and 4), `InitLock`, `SetRotXLimit` and the pitch.
fn cam_out(c: &mut BossCam, o: &Out, camera: &mut Camera) {
    match o {
        Out::CamModeRange { mode, range } => c.set_mode_range(*mode, *range, camera),
        Out::CamInitLock => c.init_lock = true,
        Out::CamRotXLimit(v) => c.set_rot_x_limit(*v),
        Out::CamPitch { add, v } => c.xrot = if *add { ee::add(c.xrot, *v) } else { *v },
        Out::CamMaxRange(v) => c.max_range = *v,
        Out::CamLift(v) => c.lift = *v,
        Out::CamSway(on) => c.zrot_control = *on,
        _ => {}
    }
}

/// The boss's position with its z raised by `dz`, for a sound or a look.
pub fn above(pos: V4, dz: f32) -> V4 {
    [pos[0], pos[1], ee::add(pos[2], ee::k(dz)), ONE]
}

#[cfg(test)]
mod tests {
    use piney_battle::boss::defs;

    use super::*;

    /// `ccBoss08`'s first form on [`BOSS08_HEAP_EXIT`]: the manager's own
    /// blur from its first `Main`, never exiting: 0x40808080, turn 0, the
    /// scale eased from 1.01 to 1.0 by a thirtieth a frame and held.
    #[test]
    fn boss08s_first_form_shows_the_managers_own_blur() {
        let mut blur = boss::blur::Blur::new(BOSS08_HEAP_EXIT);
        let mut statics = boss::blur::Statics::default();
        let shown: Vec<_> = (0..600).map(|_| blur.main(&mut statics).expect("a reflex every frame")).collect();
        assert!(shown.iter().all(|r| r.abgr == 0x4080_8080 && r.turn == 0));
        let scales: Vec<f32> = shown.iter().map(|r| f32::from_bits(r.scale)).collect();
        assert!(scales[0] > 1.0 && scales[0] < 1.01, "{}", scales[0]);
        assert!(scales.windows(2).all(|w| w[1] <= w[0]), "the scale only falls");
        assert!(scales[40..].iter().all(|&x| x == 1.0), "held at 1.0");
    }

    fn disc(name: &str) -> Option<std::sync::Arc<piney_data::archive::Archive>> {
        let p = std::path::PathBuf::from(format!("../../work/{name}/{name}.iso"));
        if !p.exists() {
            eprintln!("skipped: no {}", p.display());
            return None;
        }
        let mut iso = piney_data::iso::Iso::open(p).ok()?;
        Some(std::sync::Arc::new(piney_data::archive::Archive::new(iso.read_path("DATA/DATA.BIN").ok()?).ok()?))
    }

    /// Fidchell in field 4: its seven skills' cinema names (OUT gcmn
    /// 0x00472550's field 4 rows: `@2048` and `@2081`), rows 8-14 of x41's
    /// `TEX_fid_skl`; none in field 3, which has no rows.
    fn outbreak() -> Option<std::sync::Arc<piney_data::archive::Archive>> {
        disc("outbreak")
    }

    /// Kyvia's attacks name themselves (#72): `_g_cinemaSkillName`'s rows
    /// 32-34 (Mutation) and 36-38, 46 (Outbreak) name no file, and
    /// `SetupSkillName` reads `TEX_kbi_skl` from the field's x01 or x02.
    #[test]
    fn kyvias_attacks_have_their_names_on_the_disc() {
        for (name, v, def, field, want) in [
            ("mutation", piney_data::volume::Volume::Mut, &defs::KYVIA, 9, &[(32, 0), (33, 1), (34, 2)][..]),
            (
                "outbreak",
                piney_data::volume::Volume::Out,
                &defs::KYVIA_SECOND,
                10,
                &[(36, 1), (37, 0), (38, 2), (46, 8)][..],
            ),
        ] {
            let Some(a) = disc(name) else { continue };
            let look = BossLook::load(&a, def.code.unwrap(), v, field).unwrap();
            for &(n, row) in want {
                assert_eq!(look.names.get(&n).map(|c| c.row), Some(row), "{name}: cinema {n}");
            }
        }
    }

    #[test]
    fn fidchells_skills_have_their_names_in_field_4() {
        let Some(a) = outbreak() else { return };
        let fidchell = defs::FIDCHELL.code.unwrap();
        let look = BossLook::load(&a, fidchell, piney_data::volume::Volume::Out, 4).unwrap();
        let mut sids: Vec<i32> = look.skill_names.keys().copied().collect();
        sids.sort();
        assert_eq!(sids, [157, 158, 160, 203, 219, 227, 259]);
        let rows: Vec<i32> =
            look.skill_names.values().map(|n| n.row).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
        assert_eq!(rows, (8..=14).collect::<Vec<_>>());
        let elsewhere = BossLook::load(&a, fidchell, piney_data::volume::Volume::Out, 3).unwrap();
        assert!(elsewhere.skill_names.is_empty());
    }
}
