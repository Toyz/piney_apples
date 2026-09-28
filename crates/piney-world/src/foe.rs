//! The enemies and the magic portals drawn on the field: what an enemy's
//! row loads (`ccEntryCtrl::initEntryCCS`, `ccEnemy::initEnemyCCS`, the race
//! constructors), `ccChar::Draw` (gcmn 0x0056b1c0) at the matrix
//! `ccEnemy::dispEnemy` (0x004348b0) builds, a middle boss's second model,
//! and the portal's model (`ccMagicCircle`, gcmn 0x00455900) at the
//! transparency `ccMagicCircle::main` (0x00455b60) draws it at.
//!
//! ```text
//! ccAddRequestFileListEntry (0x0042f5a0)  every registered row's fileList
//!                          loaded; a middle boss (base.type 0x40) first
//!                          takes its base form's anm table, clut and
//!                          fileList (base.gold is that row), and loads
//!                          the file named with its fourth letter 'X' too
//! ccEntryCtrl::initEntryCCS (0x0042feb0)  entry.ccsc = GetCCSAdrs(the
//!                          fileList name cut at '.', lower case): EGN1 ->
//!                          egn1; a middle boss's ccsc2 the 'X' file
//! ccEnemy::initEnemyCCS (0x00432fc0)  clump CMP_trall of ccsc (ccClump::
//!                          Init), ccEntryChangeCLUT (0x0042e670: with an
//!                          entry.clut, Duplicate(0x2000) and ChangeClut(
//!                          that CLT_, MAT_clut)), a ccAnm on it playing
//!                          anmTbl[6]; a middle boss's second clump
//!                          CMP_trall of ccsc2, its ccAnm playing the name
//!                          with its eighth letter 'x', its shadow off
//! ccEnemyG::ccEnemyG (0x00446070)  rows 154-157: ChangeClut(
//!                          CLT_egmfrod1c1, MAT_clut2) as well
//! dispEnemy               the matrix into the anm (+0x40), the second
//!                          model on layer 5 at ccGetCameraTransparency(pos,
//!                          width, height, 7000, 600) x setTransparency,
//!                          then ccChar::Draw
//! ccChar::Draw            layer 5 (priority 10); the fog blend by
//!                          condition.dead: 0-1 the affect flash or the
//!                          condition tint, 4 (60, 0xc0c060), 5 none, else
//!                          (60, black); the camera's transparency (a town
//!                          4000 / 400, else 7000 / 600) times
//!                          setTransparency; not drawn under 0.05 unless the
//!                          camera's nearness faded it; the distant light
//!                          asleep on shaded ground; ccAnm::Draw
//! ccMagicCircle           CMP_xmagcir0 of gimmickTbl[15]'s XMAGCIR.CCS
//!                          (fog off, ChangeClut(CLT_x031c2, MAT_clut)),
//!                          playing ANM_xmagcir1, then ANM_xmagcir2; drawn
//!                          on layer 3 (priority 20) at
//!                          SetMatrix_PosRotZYX(pos, dirc)
//! ```
//!
//! `ccSetFogBlendColor` (main 0x0013df20) turns the draw environment's blend
//! into each model's fog: for a model whose fog is on (`ccObj::Init` leaves
//! every node's on, 0x0013b7b4) the coefficient is `2.55 (100 - rate)` and
//! the colour the blend's, the fog forced on; with no blend (rate 0) the
//! model draws as it would.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use glam::Mat4;
use piney_battle::chara::{AffectState, Char as Fighter};
use piney_battle::enemy_ai;
use piney_battle::enemy_motion::{CLIP_SLOTS, Kind};
use piney_battle::geom::M4;
use piney_battle::param::cond;
use piney_battle::races::boss_anm_name;
use piney_battle::tables::Tables;
use piney_data::archive::Archive;
use piney_data::tables::types::GimmickTable;
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;
use piney_draw::Fog;

use crate::body::Body;
use crate::chara::light_matrix;
use crate::draw::{self, CHAR_LAYER, Draw, EFF_LAYER};
use crate::ee::{self, F, ONE, V4};
use crate::pose::Play;
use crate::town::{self, TownLights};

/// The skeleton every enemy file carries (`initEnemyCCS`).
pub const CLUMP: &str = "CMP_trall";
/// The material `ccEntryChangeCLUT` recolours with the entry's `clut`.
pub const MAT_CLUT: &str = "MAT_clut";
/// `ccEnemyG::ccEnemyG`'s second swap, rows 154-157 (Martina's rod).
pub const G_MAT: &str = "MAT_clut2";
pub const G_CLUT: &str = "CLT_egmfrod1c1";
/// `enemyTbl` rows whose `ccEnemyG` constructor makes [`G_MAT`]'s swap.
pub const G_CLUT_ROWS: std::ops::RangeInclusive<i32> = 154..=157;

/// `gimmickTbl`'s row of the magic portal.
pub const CIRCLE_GIMMICK: i32 = 15;
/// The portal's clump, its two animations and its particles' effect
/// (`ccMagicCircle::ccMagicCircle`, `main`).
pub const CIRCLE_CLUMP: &str = "CMP_xmagcir0";
pub const CIRCLE_ANIMS: [&str; 2] = ["ANM_xmagcir1", "ANM_xmagcir2"];
pub const CIRCLE_EFF: &str = "EFF_xmagpat1";

/// `ccChar::Draw`'s camera distances (`ccGetCameraTransparency`'s far end
/// and fade): a town (`game.area` 0) 4000 and 400, a field or dungeon 7000
/// and 600; the magic portal and a middle boss's second model use the
/// field's.
pub const TOWN_FAR: F = 0x457a_0000;
pub const TOWN_FADE: F = 0x43c8_0000;
pub const AREA_FAR: F = 0x45da_c000;
pub const AREA_FADE: F = 0x4416_0000;
/// Nothing is drawn under this transparency (0.05) unless the camera's
/// nearness faded it.
pub const MIN_DRAWN: F = 0x3d4c_cccd;
/// `hitAttribute` bit: shaded ground (`WORLD_MAN::SleepDistantLight`
/// around the draw).
pub const SHADED: u32 = 0x4_0000;

/// `ccChar::Draw`'s jump table (gcmn 0x006e10d0): the tint of each
/// `conditionNum` 0-6 (2 none), as `SetFogBlend` takes a colour (red in
/// the low byte).
pub const CONDITION_COLOURS: [u32; 7] =
    [0x0070_00e0, 0x0000_e0e0, 0, 0x00b0_70e0, 0x0000_00e0, 0x0000_e0e0, 0x0010_0010];
/// The blends of the dead states (`condition.dead` 4, and 2, 3 and the
/// rest but 5): rate 60.
pub const DEAD_BLEND: F = 0x4270_0000;
pub const DEAD4_COLOUR: u32 = 0x00c0_c060;

// files ---------------------------------------------------------------------------------

/// The scene files of the area's enemies and portals, each read once
/// (`ccStream::GetCCSAdrs` of a file the area loaded).
pub struct Files {
    archive: Arc<Archive>,
    files: HashMap<String, Rc<SceneFile>>,
}

impl Files {
    pub fn new(archive: Arc<Archive>) -> Files {
        Files { archive, files: HashMap::new() }
    }

    /// The `DATA.BIN` member `stem` (`egn1`).
    pub fn get(&mut self, stem: &str) -> Result<Rc<SceneFile>> {
        if let Some(f) = self.files.get(stem) {
            return Ok(f.clone());
        }
        let f = Rc::new(SceneFile::read(&self.archive, stem)?);
        self.files.insert(stem.to_string(), f.clone());
        Ok(f)
    }
}

/// `initEntryCCS`'s file name: the fileList name cut at its first '.' and
/// lower-cased (`strlwr`).
pub fn stem_of(name: &[u8]) -> String {
    let cut = name.iter().position(|&c| c == b'.').map_or(name, |k| &name[..k]);
    String::from_utf8_lossy(cut).to_ascii_lowercase()
}

/// A middle boss's second file: its fileList name with the fourth letter
/// made 'X' (`ccAddRequestFileListEntry`, `initEntryCCS`), then as
/// [`stem_of`].
pub fn second_stem_of(name: &[u8]) -> String {
    let mut n = name.to_vec();
    if n.len() > 3 {
        n[3] = b'X';
    }
    stem_of(&n)
}

// a posed clump -------------------------------------------------------------------------

/// `ccClump::ChangeClut(clut, material)` (main 0x0013d610) as a swap: the
/// material `material`'s texture drawn through `clut` in place of its own
/// palette (`from`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClutSwap {
    pub material: String,
    pub clut: String,
    pub from: u32,
    pub to: u32,
}

/// A clump of a scene file as `ccClump::Init` makes it and a `ccAnm` draws
/// it: its nodes, its morphers, and the palette swaps made on it.
#[derive(Clone)]
pub struct Model {
    pub body: Body,
    /// Morpher object -> the model it deforms.
    pub morphers: HashMap<u32, u32>,
    pub swaps: Vec<ClutSwap>,
    /// The objects' shadow switch (`ccAnm::SetShadowSw`; on unless set
    /// off).
    pub shadow: bool,
}

impl Model {
    /// `ccClump::Init` of `clump` in `file`.
    pub fn new(file: Rc<SceneFile>, clump: &str) -> Result<Model> {
        let morphers = piney_data::anim::morphers(&file.ccs).unwrap_or_default();
        Ok(Model { body: Body::of(file, clump)?, morphers, swaps: Vec::new(), shadow: true })
    }

    pub fn file(&self) -> &Rc<SceneFile> {
        &self.body.file
    }

    /// `ccAnm::SetAnm` of the named animation of the file: frame 0.
    pub fn play(&self, anim: &str) -> Option<Play> {
        self.body.play(anim)
    }

    /// `ccClump::Duplicate(0x2000)` then `ChangeClut(clut, material)`:
    /// every node model's material `material` drawn through `clut`. The GS
    /// draw swaps palettes by the texture's own; in the files checked
    /// (`only_material_uses_swapped_palettes`) no other material draws the
    /// swapped material's texture, so that is a swap by material.
    pub fn change_clut(&mut self, material: &str, clut: &str) -> Result<()> {
        let f = self.body.file.clone();
        let m = f.ccs.find_object(material).ok_or_else(|| Error::NotFound(material.into()))?;
        let to = f.ccs.find_object(clut).ok_or_else(|| Error::NotFound(clut.into()))?;
        let tex = f.scene.materials.get(&m).ok_or_else(|| Error::NotFound(material.into()))?.texture;
        let (textures, _) = piney_data::texture::read(&f.ccs)?;
        let from = textures.iter().find(|t| t.object == tex).ok_or_else(|| Error::NotFound(material.into()))?.clut;
        self.swaps.push(ClutSwap { material: material.into(), clut: clut.into(), from, to });
        Ok(())
    }

    /// The swaps as the draw takes them.
    pub fn clut_swaps(&self) -> Vec<(u32, u32)> {
        self.swaps.iter().map(|s| (s.from, s.to)).collect()
    }

    /// Each object `play`'s animation drives, with its parent object in
    /// that animation: `ccAnm::SetAnm` puts an ExtObj copy under its own
    /// ExtObj parent (what that copy drives; 0, the anm itself), so a node
    /// can have another parent in each animation than its Obj chunk gives
    /// it (`ebl1`'s `OBJ_dummy05` is under the tail in the clump and under
    /// the head in every animation); an object driven directly keeps its
    /// Obj parent. The first track for a target wins, as for the pose.
    pub fn parents(&self, play: &Play) -> HashMap<u32, u32> {
        let file = &self.body.file;
        let sc = &file.scene;
        let mut out = HashMap::new();
        for tr in &file.anims[play.anim].tracks {
            let p = if sc.ext.contains_key(&tr.object) {
                let copy = sc.ext_parent.get(&tr.object).copied().unwrap_or(0);
                if copy == 0 { 0 } else { sc.ext.get(&copy).copied().unwrap_or(copy) }
            } else {
                sc.parent.get(&tr.target).copied().unwrap_or(0)
            };
            out.entry(tr.target).or_insert(p);
        }
        out
    }

    /// The world matrix of every clump node and every object the animation
    /// drives under the anm's `root` (`ccCoord::_SetLWMatrix`, main
    /// 0x00138380): its pose (the identity when not driven) under its
    /// parent's ([`Model::parents`]; a node not driven, its Obj parent).
    pub fn worlds(&self, play: &Play, root: Mat4) -> HashMap<u32, Mat4> {
        let file = &self.body.file;
        let locals = file.anims[play.anim].locals_at(play.posed);
        let parents = self.parents(play);
        let sc = &file.scene;
        let mut set: Vec<u32> = locals.keys().copied().collect();
        set.extend(self.body.nodes.iter().copied());
        let mut out = HashMap::new();
        #[allow(clippy::too_many_arguments)]
        fn w(
            sc: &piney_data::scene::Scene,
            locals: &HashMap<u32, Mat4>,
            parents: &HashMap<u32, u32>,
            set: &[u32],
            root: Mat4,
            cache: &mut HashMap<u32, Mat4>,
            obj: u32,
            depth: u32,
        ) -> Mat4 {
            if let Some(m) = cache.get(&obj) {
                return *m;
            }
            let local = locals.get(&obj).copied().unwrap_or(Mat4::IDENTITY);
            let parent = parents.get(&obj).or_else(|| sc.parent.get(&obj)).copied().unwrap_or(0);
            let m = if parent != 0 && parent != obj && depth < 64 && set.contains(&parent) {
                w(sc, locals, parents, set, root, cache, parent, depth + 1) * local
            } else {
                root * local
            };
            cache.insert(obj, m);
            m
        }
        for &o in &set {
            w(sc, &locals, &parents, &set, root, &mut out, o, 0);
        }
        out
    }

    /// Each clump node's transparency from the animation (`ccCoord::
    /// _GetTransparency`, main 0x00138490): its own `localtp` times its
    /// parents'.
    pub fn node_alphas(&self, play: &Play) -> HashMap<u32, f32> {
        let file = &self.body.file;
        let a = &file.anims[play.anim];
        let mut local = HashMap::new();
        for (tr, pose) in a.tracks.iter().zip(a.poses_at(play.posed)) {
            local.entry(tr.target).or_insert(pose.alpha);
        }
        for (target, pose) in a.obj_poses_at(play.posed) {
            local.entry(target).or_insert(pose.alpha);
        }
        let parents = self.parents(play);
        let sc = &file.scene;
        let mut set: Vec<u32> = local.keys().copied().collect();
        set.extend(self.body.nodes.iter().copied());
        self.body
            .nodes
            .iter()
            .map(|&n| {
                let (mut obj, mut t, mut depth) = (n, 1.0f32, 0);
                loop {
                    t *= local.get(&obj).copied().unwrap_or(1.0);
                    let parent = parents.get(&obj).or_else(|| sc.parent.get(&obj)).copied().unwrap_or(0);
                    if parent == 0 || parent == obj || depth >= 64 || !set.contains(&parent) {
                        break;
                    }
                    obj = parent;
                    depth += 1;
                }
                (n, t)
            })
            .collect()
    }

    /// `ccAnm::Draw` (main 0x001524d0) of the clump posed by `play` under
    /// `root` on `layer`: every node with a model, in the clump's order, at
    /// its world matrix and `alpha` times its transparency; lit models lit
    /// by `lights` at their position (the main light asleep when shaded);
    /// skinned and boned models over every node's matrix; the animation's
    /// texture offsets and morph weights, the palette swaps, and `fog`.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        layers: &mut Layers,
        layer: i16,
        to_screen: Mat4,
        play: &Play,
        root: Mat4,
        alpha: f32,
        lights: Option<(&TownLights, bool)>,
        fog: Option<Fog>,
    ) {
        // No blend: the draw environment's fog, by each vertex's depth.
        let fog = match (fog, lights.and_then(|(l, _)| l.fog)) {
            (Some(f), _) => draw::Fogging::Const(f),
            (None, Some(d)) => draw::Fogging::Depth(d),
            (None, None) => draw::Fogging::None,
        };
        let lit = lights.map(|(l, shaded)| move |world: Mat4| light_matrix(l, world, shaded));
        let lit = lit.as_ref().map(|f| f as &dyn Fn(Mat4) -> piney_draw::Lights);
        self.draw_with(layers, layer, to_screen, play, root, alpha, lit, fog);
    }

    /// [`Model::draw`] with another draw environment's lights: `lights`
    /// gives a lit model's light matrix at its world matrix (a stream's,
    /// for `ccThDrainEnemy`).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_with(
        &self,
        layers: &mut Layers,
        layer: i16,
        to_screen: Mat4,
        play: &Play,
        root: Mat4,
        alpha: f32,
        lights: Option<&dyn Fn(Mat4) -> piney_draw::Lights>,
        fog: draw::Fogging,
    ) {
        let file = &self.body.file;
        let nodes = &self.body.nodes;
        let worlds = self.worlds(play, root);
        let alphas = self.node_alphas(play);
        let node_mats: Vec<Mat4> = nodes.iter().map(|o| worlds.get(o).copied().unwrap_or(root)).collect();
        let rows = play.uv_rows(file);
        let morph = play.morph(file);
        let swaps = self.clut_swaps();
        let mut drawn: Vec<(usize, u32, u32)> = file
            .scene
            .model_owner
            .iter()
            .filter_map(|(&m, &o)| Some((nodes.iter().position(|&n| n == o)?, o, m)))
            .filter(|&(_, _, m)| file.models.get(&m).is_some_and(|i| i.mtype & 8 == 0 && !i.mmats.is_empty()))
            .collect();
        drawn.sort_unstable();
        for (_, obj, model) in drawn {
            let Some(info) = file.models.get(&model) else { continue };
            let world = worlds.get(&obj).copied().unwrap_or(root);
            let lit = info.mtype & 1 != 0;
            let m: Vec<(u32, f32)> = morph
                .iter()
                .filter(|(mph, _)| self.morphers.get(mph) == Some(&model))
                .flat_map(|(_, t)| t.iter().copied())
                .collect();
            let d = Draw {
                file,
                model,
                world,
                alpha: alpha * alphas.get(&obj).copied().unwrap_or(1.0),
                rows: &rows,
                lights: lights.filter(|_| lit).map(|l| l(world)),
                nodes: if info.mtype & 6 != 0 { &node_mats } else { &[] },
                morph: m,
                clut_swaps: swaps.clone(),
            };
            draw::model_edited(layers, layer, to_screen, d, None, fog);
        }
        if self.shadow {
            draw::cast_shadows(layers, file, nodes, &worlds, root);
        }
    }
}

// an enemy's look -----------------------------------------------------------------------

/// What an `enemyTbl` row draws with, as the area's loading and the race's
/// constructor leave it.
#[derive(Clone)]
pub struct EnemyLook {
    pub ene_id: i32,
    /// The row the file, animations and palette come from: the row itself,
    /// or a middle boss's base form (`ccCheckMiddleBoss`).
    pub src: i32,
    /// `CMP_trall` of the row's file, with its palette swaps.
    pub model: Model,
    /// `ccEntry.anm` (`anmTbl`): the animation names by slot.
    pub anm_tbl: u32,
    pub clips: Vec<String>,
    /// A middle boss's second model (`anm2`, `ccEnemy` +0x1bc).
    pub second: Option<Model>,
    /// The base's +0x18 height and +0x1c width (`ccGetCameraTransparency`).
    pub height: F,
    pub width: F,
}

impl EnemyLook {
    /// Row `ene_id` of `enemyTbl` from the volume's battle tables.
    pub fn load(files: &mut Files, t: &Tables, ene_id: i32) -> Result<EnemyLook> {
        let row = usize::try_from(ene_id)
            .ok()
            .and_then(|i| t.enemies.get(i))
            .ok_or(Error::NotFound(format!("enemyTbl row {ene_id}")))?;
        let base = enemy_ai::middle_boss(t, ene_id);
        let src = if base >= 0 { base } else { ene_id };
        let e = t.enemies.get(src as usize).ok_or(Error::NotFound(format!("enemyTbl row {src}")))?;
        let name = e.file.as_bytes().to_vec();
        let clut = e.clut.as_bytes().to_vec();
        // Whose animation names: the row + 1, 0 for none.
        let anm_tbl = if e.anm.is_some() { src as u32 + 1 } else { 0 };
        let clips = e.anm.unwrap_or_default().iter().take(CLIP_SLOTS as usize).map(|s| s.to_string()).collect();
        let mut model = Model::new(files.get(&stem_of(&name))?, CLUMP)?;
        if !clut.is_empty() {
            model.change_clut(MAT_CLUT, &String::from_utf8_lossy(&clut))?;
        }
        if Kind::of_row(t, ene_id) == Kind::G && G_CLUT_ROWS.contains(&ene_id) {
            model.change_clut(G_MAT, G_CLUT)?;
        }
        // initEnemyCCS: the second model's shadow off.
        let second = if base >= 0 {
            Some(Model { shadow: false, ..Model::new(files.get(&second_stem_of(&name))?, CLUMP)? })
        } else {
            None
        };
        Ok(EnemyLook {
            ene_id,
            src,
            model,
            anm_tbl,
            clips,
            second,
            height: row.param.base.height,
            width: row.param.base.width,
        })
    }

    /// The file drawn (`egn1`).
    pub fn stem(&self) -> &str {
        &self.model.body.file.stem
    }

    /// Slot `anm_num` of the animation table (empty past it).
    pub fn clip(&self, anm_num: i16) -> &str {
        usize::try_from(anm_num).ok().and_then(|k| self.clips.get(k)).map_or("", |s| s.as_str())
    }

    /// `ccAnm::SetAnm` of the named animation (`ANM_egn1nut0`).
    pub fn play(&self, name: &str) -> Option<Play> {
        self.model.play(name)
    }

    /// The second model's player for the main one's clip `name`
    /// (`ccGetNameBossAnm`: its eighth letter 'x').
    pub fn play_second(&self, name: &str) -> Option<Play> {
        self.second.as_ref()?.play(&boss_anm_name(name))
    }

    /// `ccChar::Draw`'s `ccAnm::Draw` as `how` decided it: at `dispEnemy`'s
    /// matrix (`Call::Draw`), on the character layer, lit by the area's
    /// lights, with the fog blend.
    pub fn draw(
        &self,
        layers: &mut Layers,
        to_screen: Mat4,
        play: &Play,
        matrix: &M4,
        how: &CharDraw,
        lights: &TownLights,
    ) {
        if !how.drawn {
            return;
        }
        let root = draw::mat(matrix);
        let fog = how.blend.fog();
        self.model.draw(
            layers,
            CHAR_LAYER,
            to_screen,
            play,
            root,
            ee::f(how.transparency),
            Some((lights, how.shaded)),
            fog,
        );
    }

    /// A middle boss's second model (`Call::DrawSecond`): `ccAnm::Draw` on
    /// layer 5 at the matrix and the alpha `dispEnemy` gives, without
    /// `ccChar::Draw`'s blend or the shade.
    pub fn draw_second(
        &self,
        layers: &mut Layers,
        to_screen: Mat4,
        play: &Play,
        matrix: &M4,
        alpha: F,
        lights: &TownLights,
    ) {
        if let Some(m) = &self.second {
            m.draw(layers, CHAR_LAYER, to_screen, play, draw::mat(matrix), ee::f(alpha), Some((lights, false)), None);
        }
    }
}

// ccThDrainEnemy ------------------------------------------------------------------------

/// `ccThDrainEnemy` (gcmn 0x00432000): Data Drain's movie draws the drained
/// enemy into its scene. The task builds the enemy's clump from its row's
/// file (`CMP_trall`, its palette), a `ccAnm` on its animation slot 6 (and
/// a middle boss's second model with the same clip, 'x' its eighth
/// letter), and each frame, while the stream plays and its layer is on,
/// animates them forward and draws them at (0, -900, 130) turned half
/// about z, on the stream's layer through its draw environment
/// (`ccLayer::active` = stream +0x154, `ccDrawEnv::active` = +0x158).
///
/// `state` (+0x18) is `DataDrainMenu`'s: 0 nothing drawn; 1 from the
/// stream's frame 30; 3 from 229, and at the end. It also builds the race's
/// base form (the row's +0x14 for type 0x40 rows, else the first row of its
/// `ccEntryRaceTbl` group, 203 for 203-206) for a state 2 no caller sets;
/// the port does not.
pub struct DrainEnemy {
    look: EnemyLook,
    play: Option<Play>,
    second: Option<Play>,
    pub state: i32,
}

/// `DataDrainMenu`'s frames (0x00532f68): the enemy on at 30, off at 229.
pub const DRAIN_ENEMY_ON: u32 = 30;
pub const DRAIN_ENEMY_OFF: u32 = 229;

impl DrainEnemy {
    /// The task's start: its animation slot 6 at frame 0.
    pub fn new(look: EnemyLook) -> DrainEnemy {
        let clip = look.clip(6).to_string();
        let play = look.play(&clip);
        let second = look.play_second(&clip);
        DrainEnemy { look, play, second, state: 0 }
    }

    /// The menu task's switch on the stream's frame (`ccGetStreamFrame`,
    /// its scene 0).
    pub fn at_frame(&mut self, frame: u32) {
        match frame {
            DRAIN_ENEMY_ON => self.state = 1,
            DRAIN_ENEMY_OFF => self.state = 3,
            _ => {}
        }
    }

    /// One frame of the task in state 1: `_AnimateForward`, then
    /// `SetMatrix_PosRotZYX((0, -900, 130), (0, 0, pi))` and `ccAnm::Draw`,
    /// the second model first.
    pub fn draw(
        &mut self,
        layers: &mut Layers,
        layer: i16,
        to_screen: Mat4,
        lights: &dyn Fn(Mat4) -> piney_draw::Lights,
    ) {
        if self.state != 1 {
            return;
        }
        let root =
            Mat4::from_translation(glam::Vec3::new(0.0, -900.0, 130.0)) * Mat4::from_rotation_z(std::f32::consts::PI);
        if let (Some(m), Some(p)) = (&self.look.second, &mut self.second) {
            p.forward(&m.body.file);
            m.draw_with(layers, layer, to_screen, p, root, 1.0, Some(lights), draw::Fogging::None);
        }
        if let Some(p) = &mut self.play {
            p.forward(&self.look.model.body.file);
            self.look.model.draw_with(layers, layer, to_screen, p, root, 1.0, Some(lights), draw::Fogging::None);
        }
    }
}

// ccChar::Draw ---------------------------------------------------------------------------

/// Where the camera is, for `ccGetCameraTransparency` (main 0x001da8b0):
/// the player's position (`plw->pos`, the frame `ccTransPosW2P` measures
/// from), the active camera's eye and pitch (`activeCamPtr->pos`,
/// `->deg[1]`), the eye view (`checkCameraType() == 1`), the map's bounds
/// (`WORLD_MAN`'s: a field or dungeon 0..48000) and whether this is a town
/// (`game.area` 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Camera {
    pub player: V4,
    pub cam: V4,
    pub deg1: i16,
    pub eye: bool,
    pub bounds: [F; 4],
    pub town: bool,
}

impl Camera {
    /// `ccTransPosW2P` (gcmn 0x0059b940): `pos` in the player's frame.
    pub fn w2p(&self, pos: V4) -> V4 {
        crate::field_area::w2p(pos, self.player, self.bounds)
    }

    /// `ccGetCameraTransparency(pos, width, height, far, fade, &hide)`.
    #[allow(clippy::too_many_arguments)]
    pub fn transparency(&self, pos: V4, width: F, height: F, far: F, fade: F, hide: &mut bool) -> F {
        camera_transparency(self.w2p(pos), self.w2p(self.cam), self.deg1, width, height, far, fade, hide)
    }

    /// The five-argument `ccGetCameraTransparency(pos, width, height, far,
    /// fade)` (main 0x001da880): `hide` starts clear. What
    /// `ccMagicCircle::main` (0, 0, 7000, 600) and `dispEnemy`'s second
    /// model (width, height, 7000, 600) ask.
    pub fn transparency5(&self, pos: V4, width: F, height: F, far: F, fade: F) -> F {
        self.transparency(pos, width, height, far, fade, &mut false)
    }

    /// `ccChar::Draw`'s far end and fade by `game.area`.
    pub fn fade(&self) -> (F, F) {
        if self.town { (TOWN_FAR, TOWN_FADE) } else { (AREA_FAR, AREA_FADE) }
    }
}

/// `ccGetCameraTransparency` with both points already in the player's
/// frame: toward 0 as the camera comes within 1.25 r of the point (r the
/// pitch's blend of `width` and `height`, at least 180; `hide` set, unless
/// it was), 1 out to `far`, then down to 0 over `fade`.
#[allow(clippy::too_many_arguments)]
pub fn camera_transparency(
    pos_p: V4,
    cam_p: V4,
    deg1: i16,
    width: F,
    height: F,
    far: F,
    fade: F,
    hide: &mut bool,
) -> F {
    let d = ee::vsub(cam_p, pos_p);
    let dist = ee::sqrtf(ee::dot(d, d));
    let t = ee::div(ee::from_int(8192 - i32::from(deg1)), 0x4600_0000);
    let u = ee::sub(ONE, t);
    let mut r = ee::add(ee::mul(width, t), ee::mul(height, u));
    if ee::lt(r, 0x4334_0000) {
        r = 0x4334_0000;
    }
    let r2 = ee::mul(0x3fa0_0000, r);
    if !*hide && ee::lt(dist, r2) {
        *hide = true;
        let near = ee::mul(0x3f4c_cccd, r);
        let f = ee::sub(dist, near);
        return if ee::le(f, 0) { 0 } else { ee::div(f, ee::sub(r2, near)) };
    }
    *hide = false;
    if ee::le(far, 0) || ee::le(dist, far) {
        return ONE;
    }
    let over = ee::sub(dist, far);
    if ee::le(over, fade) { ee::div(ee::sub(fade, over), fade) } else { 0 }
}

/// `ccDrawEnv::SetFogBlend(rate, colour)` (main 0x001057f0): a rate of 0
/// is no blend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FogBlend {
    pub rate: F,
    /// Red in the low byte.
    pub colour: u32,
}

impl FogBlend {
    /// `ccSetFogBlendColor` (main 0x0013df20) for a model with its fog on:
    /// the coefficient `2.55 (100 - rate)` (the GS takes its whole part)
    /// and the blend's colour; None without a blend
    /// (`ccSetMatrixPacket` 0x0013e5d0).
    pub fn fog(self) -> Option<Fog> {
        if ee::eq(self.rate, 0) {
            return None;
        }
        let f = ee::mul(0x4023_3333, ee::sub(0x42c8_0000, self.rate));
        let c = self.colour;
        Some(Fog { f: ee::to_int(f).clamp(0, 255) as u8, colour: [c as u8, (c >> 8) as u8, (c >> 16) as u8] })
    }
}

/// What `ccChar::Draw` decides before `ccAnm::Draw`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CharDraw {
    /// The blend it set (rate 0: none).
    pub blend: FogBlend,
    /// +0x88, and the anm's: `setTransparency` times the camera's.
    pub transparency: F,
    /// `ccGetCameraTransparency`'s `hide` as it left it.
    pub hide: bool,
    /// `ccDrawEnv::shadowAlpha` and `shadowLength` for the shadow model
    /// (`draw::shadow_env`).
    pub shadow_alpha: u8,
    pub shadow_length: F,
    /// `ccAnm::Draw` ran.
    pub drawn: bool,
    /// On shaded ground: `SleepDistantLight` around the draw.
    pub shaded: bool,
}

/// `ccChar::Draw`'s blend from `condition.dead` and the affect members:
/// alive (0, 1) the affect's flash while its rate lasts (its rate moving by
/// its count, the flash over at 0), else the condition's tint pulsing
/// between 0 and 60 (+12 up, -2 down) when condition effects are on
/// (`ccSpcConditionEffectSW`, main 0x00378ce4); dead 4 (60, 0xc0c060), 5
/// none (the flash cleared), and 2, 3 and the rest (60, black). The
/// affect's `affectColorFix` (+0xa6) is 0 for every enemy (only
/// `ccRestoreSpcCondition` sets it) and is not modelled.
pub fn char_blend(dead: i16, condition_num: i32, effect_sw: bool, a: &mut AffectState) -> FogBlend {
    match dead {
        0 | 1 => {
            if a.color_rate != 0 {
                a.color_rate = a.color_rate.wrapping_add(a.color_cnt);
                if a.color_rate <= 0 {
                    a.color_rate = 0;
                    a.color = 0;
                }
                return FogBlend { rate: ee::from_int(i32::from(a.color_rate)), colour: a.color };
            }
            a.cond_color = 0;
            if condition_num != -1 && effect_sw && (condition_num as u32) < 7 {
                a.cond_color = CONDITION_COLOURS[condition_num as usize];
            }
            if a.cond_color == 0 {
                return FogBlend::default();
            }
            a.cond_color_rate = a.cond_color_rate.wrapping_add(a.cond_color_cnt);
            if a.cond_color_rate >= 60 {
                a.cond_color_rate = 60;
                a.cond_color_cnt = -2;
            } else if a.cond_color_rate <= 0 {
                a.cond_color_rate = 0;
                a.cond_color_cnt = 12;
            }
            FogBlend { rate: ee::from_int(i32::from(a.cond_color_rate)), colour: a.cond_color }
        }
        4 => FogBlend { rate: DEAD_BLEND, colour: DEAD4_COLOUR },
        5 => {
            a.color_rate = 0;
            a.color = 0;
            FogBlend::default()
        }
        _ => FogBlend { rate: DEAD_BLEND, colour: 0 },
    }
}

/// `ccChar::Draw` (gcmn 0x0056b1c0) up to `ccAnm::Draw`, for a character
/// at `pos` of the base's `width` and `height`: the blend
/// ([`char_blend`]), then the camera's transparency (`hide` starting set
/// when `trans_dist` (+0x90) is off or in the eye view) times
/// `set_transparency`, the shadow's alpha, and whether it draws.
#[allow(clippy::too_many_arguments)]
pub fn char_draw_parts(
    dead: i16,
    condition_num: i32,
    effect_sw: bool,
    a: &mut AffectState,
    pos: V4,
    width: F,
    height: F,
    set_transparency: F,
    trans_dist: bool,
    hit_attribute: u32,
    camera: &Camera,
) -> CharDraw {
    let blend = char_blend(dead, condition_num, effect_sw, a);
    let mut hide = !trans_dist || camera.eye;
    let (far, fade) = camera.fade();
    let t = camera.transparency(pos, width, height, far, fade, &mut hide);
    let transparency = ee::mul(set_transparency, t);
    let s = if hide { set_transparency } else { transparency };
    let shadow_alpha = ((ee::to_int(ee::mul(0x4380_0000, s)).wrapping_add(1)) >> 1) as u8;
    let drawn = !(ee::lt(transparency, MIN_DRAWN) && !hide);
    CharDraw {
        blend,
        transparency,
        hide,
        shadow_alpha,
        shadow_length: if drawn { ee::add(height, ee::mul(0x4000_0000, height)) } else { 0 },
        drawn,
        shaded: drawn && hit_attribute & SHADED != 0,
    }
}

/// [`char_draw_parts`] on a battle character (an enemy's `Char`, whose
/// affect members it moves on as the game does each frame it draws).
pub fn char_draw(
    ch: &mut Fighter,
    set_transparency: F,
    trans_dist: bool,
    effect_sw: bool,
    hit_attribute: u32,
    camera: &Camera,
) -> CharDraw {
    let (dead, num, pos) = (ch.cond[cond::DEAD], ch.condition_num, ch.pos);
    let (width, height) = (ch.base().width, ch.base().height);
    char_draw_parts(
        dead,
        num,
        effect_sw,
        &mut ch.affect,
        pos,
        width,
        height,
        set_transparency,
        trans_dist,
        hit_attribute,
        camera,
    )
}

// the magic portal ----------------------------------------------------------------------

/// The magic portal's model and effect, from `gimmickTbl[15]`.
#[derive(Clone)]
pub struct Circle {
    /// `CMP_xmagcir0` with the entry's palette (`CLT_x031c2` for
    /// `MAT_clut`).
    pub model: Model,
    /// `patNum` of `EFF_xmagpat1` (`ccEff::Init`, main 0x0013ba20, from
    /// `Decode_Eff` 0x0014cca0): what `entry::Cx::part_pats` wants.
    pub pat_num: u16,
}

impl Circle {
    /// `ccMagicCircle::ccMagicCircle`'s clump of the gimmick row's file,
    /// with the row's `ccEntryChangeCLUT`.
    pub fn load(files: &mut Files, volume: Volume) -> Result<Circle> {
        let (file, clut) = gimmick_file(files, volume, CIRCLE_GIMMICK)?;
        let pat_num = eff_pat_num(&file, CIRCLE_EFF)?;
        let mut model = Model::new(file, CIRCLE_CLUMP)?;
        if !clut.is_empty() {
            model.change_clut(MAT_CLUT, clut)?;
        }
        Ok(Circle { model, pat_num })
    }

    pub fn play(&self, anim: &str) -> Option<Play> {
        self.model.play(anim)
    }

    /// `ccCoord::SetMatrix_PosRotZYX(pos, dirc)` as stored.
    pub fn root(pos: V4, dirc: V4) -> [V4; 4] {
        town::pos_rot_zyx(pos, [dirc[0], dirc[1], dirc[2]])
    }

    /// `Out::CircleDraw`: `ccAnm::Draw` on layer 3 (priority 20) at the
    /// circle's position and heading, at `transparency` (the camera's
    /// times its own, as `ccMagicCircle::main` gives it); its fog is off.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        layers: &mut Layers,
        to_screen: Mat4,
        play: &Play,
        pos: V4,
        dirc: V4,
        transparency: F,
        lights: &TownLights,
    ) {
        let root = draw::mat(&Self::root(pos, dirc));
        self.model.draw(layers, EFF_LAYER, to_screen, play, root, ee::f(transparency), Some((lights, false)), None);
    }
}

/// A box's, breakable's, idol's or Grunty food's look (`ccGimBox::
/// ccGimBox`, `ccGimIdol::ccGimIdol`, `ccGimFood::ccGimFood`): the clump of its gimmick row's file with the
/// row's `ccEntryChangeCLUT` (the trapped box, wooden box and barrel's
/// other palette), and the row's height and width.
pub struct GimLook {
    pub row: i32,
    pub model: Model,
    pub height: F,
    pub width: F,
}

impl GimLook {
    pub fn load(files: &mut Files, volume: Volume, row: i32) -> Result<GimLook> {
        let clump = if (38..=44).contains(&row) {
            piney_battle::gimmick::IDOL_MODEL
        } else if piney_battle::gimmick::FOOD_ROWS.contains(&row) {
            piney_battle::gimmick::FOOD_MODEL
        } else if row == 17 {
            piney_battle::gimmick::SYMBOL_MODEL
        } else {
            piney_battle::gimmick::box_model(row).ok_or_else(|| Error::NotFound(format!("gimmick row {row}")))?.0
        };
        GimLook::load_clump(files, volume, row, clump)
    }

    /// The row's file with clump `clump` (the spring's two).
    pub fn load_clump(files: &mut Files, volume: Volume, row: i32, clump: &str) -> Result<GimLook> {
        let (file, clut) = gimmick_file(files, volume, row)?;
        let mut model = Model::new(file, clump)?;
        if !clut.is_empty() {
            model.change_clut(MAT_CLUT, clut)?;
        }
        let base = &gimmick_row(volume, row)?.param.base;
        Ok(GimLook { row, model, height: base.height.to_bits(), width: base.width.to_bits() })
    }
}

/// `gimmickTbl[row]` of the volume.
fn gimmick_row(volume: Volume, row: i32) -> Result<&'static GimmickTable> {
    let rows = piney_data::tables::battle::of(volume).gimmicks();
    usize::try_from(row).ok().and_then(|r| rows.get(r)).ok_or_else(|| Error::NotFound(format!("gimmick row {row}")))
}

/// A gimmick row's scene file (its `ccEntry`'s file list) and its
/// `ccEntryChangeCLUT` palette ("" for none).
fn gimmick_file(files: &mut Files, volume: Volume, row: i32) -> Result<(Rc<SceneFile>, &'static str)> {
    let entry = &gimmick_row(volume, row)?.entry;
    let name = entry.file_list.name.ok_or_else(|| Error::NotFound(format!("gimmick row {row}'s file")))?;
    Ok((files.get(&stem_of(name.as_bytes()))?, entry.clut))
}

/// An Eff chunk's (0x0e00) `patNum`: after the object and texture words,
/// the flag, `zoffs` and a halfword, at +14 of the payload.
pub fn eff_pat_num(file: &SceneFile, name: &str) -> Result<u16> {
    let obj = file.ccs.find_object(name).ok_or_else(|| Error::NotFound(name.into()))?;
    let d = &file.ccs.data;
    for ch in file.ccs.walk().chunks {
        if ch.in_frames || ch.kind != 0x0e00 {
            continue;
        }
        let q = ch.payload();
        let word = |k: usize| d.get(q + k..q + k + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        if word(0) == Some(obj) {
            return d
                .get(q + 14..q + 16)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .ok_or(Error::Format("short Eff".into()));
        }
    }
    Err(Error::NotFound(name.into()))
}

// ccThStrParty --------------------------------------------------------------------------

/// `spcAnmTbl` (gcmn 0x00651890): each `charTbl` row's stream pose.
pub const SPC_ANM: [&str; 18] = [
    "ANM_ctu1nut2",
    "ANM_cbu3nut2",
    "ANM_cbu4nut2",
    "ANM_cbu2nut2",
    "ANM_chb2nut2",
    "ANM_cla1nut2",
    "ANM_cbu1nut2",
    "ANM_ctu3nut2",
    "ANM_cha1nut2",
    "ANM_cwm3nut2",
    "ANM_cwm1nut2",
    "ANM_ctu2nut2",
    "ANM_cbu5nut2",
    "ANM_cla2nut2",
    "ANM_cha2nut2",
    "ANM_chb1nut2",
    "ANM_cwm4nut2",
    "ANM_cwm2nut2",
];

/// `spcDmyTbl` (0x006518d8): where the members stand in the stream.
pub const SPC_DMY: [&str; 3] = ["OBJ_dmy_spc0", "OBJ_dmy_spc1", "OBJ_dmy_spc2"];

/// `ccThStrParty` (gcmn 0x0056ab00), `StreamMenu`'s task: the party
/// members `streamFlag`'s low bits name (slots 0-2), each its own
/// `CMP_trall` posed by its `spcAnmTbl` clip (one `_AnimateForward` at the
/// start, shadow off), drawn each frame of the stream at the stream's
/// `OBJ_dmy_spcN` (N counting the members drawn): `_AnimateForward`,
/// the root the dummy's `lwMatrix`, `ccAnm::Draw` on the stream's layer
/// and draw environment.
pub struct StrParty {
    members: Vec<(Model, Play)>,
}

impl StrParty {
    /// The task's start: `members` each (file, `charTbl` row).
    pub fn new(members: &[(Rc<SceneFile>, i32)]) -> StrParty {
        let mut out = Vec::new();
        for (file, id) in members {
            let Ok(model) = Model::new(file.clone(), "CMP_trall") else { continue };
            let Some(clip) = usize::try_from(*id).ok().and_then(|i| SPC_ANM.get(i)) else { continue };
            let Some(mut play) = model.body.play(clip) else { continue };
            play.forward(&model.body.file);
            out.push((model, play));
        }
        StrParty { members: out }
    }

    /// One frame: each member at its dummy (`dummy(name)`, the stream's
    /// `lwMatrix`), stepped and drawn; a member whose dummy the stream lacks
    /// is skipped.
    pub fn draw(
        &mut self,
        layers: &mut Layers,
        layer: i16,
        to_screen: Mat4,
        lights: &dyn Fn(Mat4) -> piney_draw::Lights,
        dummy: &dyn Fn(&str) -> Option<Mat4>,
    ) {
        for (k, (model, play)) in self.members.iter_mut().enumerate() {
            let Some(root) = SPC_DMY.get(k).and_then(|n| dummy(n)) else { continue };
            play.forward(&model.body.file);
            model.draw_with(layers, layer, to_screen, play, root, 1.0, Some(lights), draw::Fogging::None);
        }
    }
}

#[cfg(test)]
mod tests {
    use piney_data::iso::Iso;

    use super::*;

    const ISO: &str = "../../work/infection/infection.iso";

    struct Disc {
        files: Files,
        t: Tables,
    }

    fn disc() -> Option<Disc> {
        if !std::path::Path::new(ISO).exists() {
            eprintln!("skipped: no {ISO}");
            return None;
        }
        let mut iso = Iso::open(ISO).unwrap();
        let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let t = Tables::read(&mut iso).unwrap();
        Some(Disc { files: Files::new(archive), t })
    }

    #[test]
    fn stems() {
        assert_eq!(stem_of(b"EGN1"), "egn1");
        assert_eq!(stem_of(b"XMAGCIR.CCS"), "xmagcir");
        assert_eq!(second_stem_of(b"EBQX"), "ebqx");
        assert_eq!(second_stem_of(b"EGN1"), "egnx");
    }

    /// The first field's and dungeon's rows: their files, animations and
    /// palettes (`tools/test_foe_rs.py`'s `looks` checks them against the
    /// constructors run in the game).
    #[test]
    fn first_field_looks() {
        let Some(mut d) = disc() else { return };
        type Case = (i32, &'static str, &'static str, &'static [(&'static str, &'static str)]);
        let cases: [Case; 7] = [
            (67, "ebl1", "ANM_ebl1nut0", &[]),
            (130, "egn1", "ANM_egn1nut0", &[]),
            (151, "egmf", "ANM_egmfnut0", &[]),
            (185, "ekd1", "ANM_ekd1nut0", &[]),
            (189, "eks1", "ANM_eks1nut0", &[(MAT_CLUT, "CLT_eks1bodyc2")]),
            (219, "epg1", "ANM_epg1nut0", &[]),
            (268, "evm1", "ANM_evm1nut0", &[]),
        ];
        for (id, stem, wait, swaps) in cases {
            let l = EnemyLook::load(&mut d.files, &d.t, id).unwrap();
            assert_eq!((l.stem(), l.clip(6)), (stem, wait), "row {id}");
            let got: Vec<(&str, &str)> = l.model.swaps.iter().map(|s| (s.material.as_str(), s.clut.as_str())).collect();
            assert_eq!(got, swaps, "row {id}");
            assert!(l.second.is_none());
            assert!(l.play(wait).is_some());
            assert!(!l.model.body.nodes.is_empty());
        }
        let m = EnemyLook::load(&mut d.files, &d.t, 155).unwrap();
        let got: Vec<(&str, &str)> = m.model.swaps.iter().map(|s| (s.material.as_str(), s.clut.as_str())).collect();
        assert_eq!(got, [(MAT_CLUT, "CLT_egmfbodyc3"), (G_MAT, G_CLUT)]);
    }

    /// A swap by palette is a swap by material: no other material of the
    /// file draws the swapped material's texture.
    #[test]
    fn only_material_uses_swapped_palettes() {
        let Some(mut d) = disc() else { return };
        let mut looks: Vec<Model> = [67, 130, 151, 154, 185, 187, 189, 219, 268]
            .iter()
            .map(|&id| EnemyLook::load(&mut d.files, &d.t, id).unwrap().model)
            .collect();
        looks.push(Circle::load(&mut d.files, Volume::Inf).unwrap().model);
        for m in looks {
            let f = m.file();
            for s in &m.swaps {
                let mat = f.ccs.find_object(&s.material).unwrap();
                let tex = f.scene.materials[&mat].texture;
                let others: Vec<u32> = f
                    .scene
                    .materials
                    .iter()
                    .filter(|(o, mt)| **o != mat && mt.texture == tex)
                    .map(|(o, _)| *o)
                    .collect();
                assert!(others.is_empty(), "{}: {} shares its texture with {others:?}", f.stem, s.material);
            }
        }
    }

    /// The Chicken Hand's `OBJ_dummy05` hangs from the tail in the clump
    /// but from the head in its animations: six frames into
    /// `ANM_ebl1dmg1` at a `dispEnemy` matrix, as the game's animation code
    /// and `ccCoord::_SetLWMatrix` leave the head and the dummy
    /// (`tools/test_foe_rs.py`'s `pose`, which checks many more).
    #[test]
    fn dummies_follow_the_animation() {
        let Some(mut d) = disc() else { return };
        let l = EnemyLook::load(&mut d.files, &d.t, 67).unwrap();
        let mut p = l.play("ANM_ebl1dmg1").unwrap();
        (p.time, p.posed) = (1536, 1536);
        let m: M4 = [
            [0x3f60_a940, 0x3ef5_7744, 0, 0],
            [0xbef5_7744, 0x3f60_a940, 0, 0],
            [0, 0, ONE, 0],
            [0x447a_0000, 0x44fa_0000, 0x41f0_0000, ONE],
        ];
        let worlds = l.model.worlds(&p, draw::mat(&m));
        #[rustfmt::skip]
        let cases: [(&str, [f32; 16]); 2] = [
            ("OBJ_t0 head", [-0.5254571, -0.1185403, 0.842519, 0.0, 0.7796808, -0.463468, 0.4210581, 0.0,
                0.3405692, 0.8781464, 0.3359567, 0.0, 956.4118, 2079.04, 303.0281, 1.0]),
            ("OBJ_dummy05", [-0.5254571, -0.1185403, 0.842519, 0.0, 0.7796808, -0.463468, 0.4210581, 0.0,
                0.3405692, 0.8781464, 0.3359567, 0.0, 942.27466, 2075.8506, 325.6956, 1.0]),
        ];
        for (name, want) in cases {
            let got = worlds[&l.model.body.node(name).unwrap()].to_cols_array();
            for (k, (g, w)) in got.iter().zip(want).enumerate() {
                let tolerance = if k >= 12 { 1e-2 } else { 1e-4 };
                assert!((g - w).abs() <= tolerance, "{name} element {k}: {g} against the game's {w}");
            }
        }
    }

    #[test]
    fn portal() {
        let Some(mut d) = disc() else { return };
        let c = Circle::load(&mut d.files, Volume::Inf).unwrap();
        assert_eq!(c.model.file().stem, "xmagcir");
        assert_eq!(c.pat_num, 111);
        assert_eq!(c.model.swaps[0].clut, "CLT_x031c2");
        assert!(CIRCLE_ANIMS.iter().all(|a| c.play(a).is_some()));
    }

    /// The flash runs down by its count and ends at 0; a tint pulses up by
    /// 12 to 60 and back down by 2; the dead states' blends.
    #[test]
    fn blends() {
        let mut a = AffectState { color_rate: 30, color_cnt: -10, color: 0x00ff_ffff, ..AffectState::default() };
        let b = char_blend(0, -1, true, &mut a);
        assert_eq!((b.rate, b.colour), (ee::k(20.0), 0x00ff_ffff));
        char_blend(0, -1, true, &mut a);
        let b = char_blend(1, -1, true, &mut a);
        assert_eq!((b.rate, a.color_rate, a.color), (0, 0, 0));
        assert_eq!(b.fog(), None);
        let mut rates = Vec::new();
        for _ in 0..8 {
            rates.push(i32::from(char_blend(0, 0, true, &mut a).rate != 0) * i32::from(a.cond_color_rate));
        }
        assert_eq!(rates, [0, 12, 24, 36, 48, 60, 58, 56]);
        assert_eq!(char_blend(0, 0, false, &mut a), FogBlend::default());
        let dying = char_blend(2, -1, true, &mut a);
        assert_eq!(dying.fog(), Some(Fog { f: 101, colour: [0, 0, 0] }));
        assert_eq!(char_blend(4, -1, true, &mut a).fog(), Some(Fog { f: 101, colour: [0x60, 0xc0, 0xc0] }));
    }
}
