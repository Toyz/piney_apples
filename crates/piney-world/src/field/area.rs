//! A field, area 1: `WORLD` (gcmn world.cpp) as `WORLD_MAN::GO(1)` (main
//! 0x0019f8e0) builds it and `ccThFieldDisp` (0x001a4430, priority 96) draws
//! it with `WORLD::Draw` (gcmn 0x005a97b0): the ground (piney_data::field),
//! the `FOBJECT2`s (entrance, key, sub, lake) and the `FOBJECT`s at their
//! chips, the 12 x 12 chips round the centre drawn each frame. An object's
//! hit models join the collision list as it is drawn, at the copy of its
//! place nearest the player; between them the height map is the ground
//! (docs/engine/field-walk.md). The weather, the ambient pictures and the
//! background are the field type's systems ([`crate::field::def::FieldDef`]).

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use piney_data::archive::Archive;
use piney_data::field::{self, Field, Kind, Tables};
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;
use piney_draw::VertexEdits;

use crate::area::AreaCore;
use crate::chara::pose::Play;
use crate::draw::{self, Draw, FLOOR_LAYER, OBJ_LAYER, OBJ2_LAYER};
use crate::ee::{self, F, ONE, V4};
use crate::field::ambient::{BOOT_STATICS, Weather};
use crate::field::def::FieldDef;
use crate::hit::{HitModel, Hits};
use crate::sprite::EffSprite;
use crate::systems::{
    BuildCx, Carried, CarriedMut, DrawCx, Drawn, FieldCx, FieldStatics, MapEvent, MapView, Request, Statics, Systems,
    paint,
};
use crate::town::{Light, TownLights};
use piney_data::dungeon::Rng;

/// The field's size, 48,000 units a side (`WORLD_MAN` bounds in `GO(1)`).
pub const SIZE: F = 0x473b_8000;
/// A chip, 1200 units.
const CHIP: F = 0x4496_0000;
/// `FOBJECT::Draw`: full to 6,700 from the player, gone at 7,200.
const OBJ_NEAR: F = 0x45d1_6000;
const OBJ_FAR: F = 0x45e1_0000;
const OBJ_FADE: F = 0x43fa_0000;
/// `FOBJECT2::Draw`: full to 8,600, gone (and its hit off) beyond 9,600.
const OBJ2_NEAR: F = 0x4606_6000;
const OBJ2_FAR: F = 0x4616_0000;
const OBJ2_FADE: F = 0x447a_0000;
/// The chips drawn round the centre's: -6..5 each way.
const WINDOW: i32 = 6;

/// `FOBJECT2.type` values `WORLD::Draw` and `DrawObject` sort on.
fn obj2_type(kind: Kind) -> u32 {
    kind.number()
}

/// One placed object: a `FOBJECT` (base and tree objects, 0x50 bytes) or a
/// `FOBJECT2` (the entrance, key, sub and lake objects, 0x80 bytes).
#[derive(Clone, Debug)]
pub struct FieldObject {
    pub kind: Kind,
    /// A `FOBJECT2`.
    pub two: bool,
    /// Its top-left chip.
    pub chip: [u32; 2],
    /// `wp`: its centre, and z (0 for levelled objects, else
    /// `WORLD::GetHeight` as it was placed).
    pub wp: V4,
    /// `pos`: where its last draw put it - `ccTransPosP2W(ccTransPosW2P(wp))`,
    /// the copy nearest the player.
    pub pos: V4,
    /// The distance across the ground from the player at the last draw.
    pub dist: F,
    /// The clump's transparency or the anm's `localtp` at the last draw.
    pub alpha: F,
    /// `FOBJECT2.disp`: drawn by the last pass.
    pub disp: bool,
    /// Its `ccAnm` (an Anime chunk plays), or None for a bare clump.
    pub play: Option<Play>,
    /// The clump's nodes (a bare clump's models are drawn at `pos`).
    pub nodes: Vec<u32>,
    /// Its models' `ccModelHit`s (`ccModel.hit`, +0x3c), its own, as
    /// indices into [`FieldArea::model_hits`].
    pub hits: Vec<usize>,
}

/// A field as `WORLD` holds it.
pub struct FieldArea {
    /// The field type's def.
    pub def: &'static FieldDef,
    /// The field's CCS file (`fieldccs[type]`, a hacked field's
    /// `fieldccs2[type]`), its collision (the height map and the objects'
    /// hits), the light the ground and objects were lit by (as Kite's draw
    /// takes it), `fieldrand` as `Generate` left it, which the drawing goes
    /// on drawing from, and the start.
    pub core: AreaCore,
    pub field: Rc<Field>,
    pub tables: &'static Tables,
    /// The background (`DrawBG`'s clumps and the lens flare's sun) and the
    /// effects' file (`field_eff`).
    pub bg: Rc<SceneFile>,
    pub eff: Option<Rc<SceneFile>>,
    pub scene: field::Scene,
    /// Every object, in `Generate`'s order.
    pub objects: Vec<FieldObject>,
    /// `fobj2[]`: indices of the `FOBJECT2`s, in order.
    fobj2: Vec<usize>,
    /// `fobj[x][y]`.
    fobj: HashMap<(u32, u32), usize>,
    /// `WORLD.ofs_x`, `ofs_y`: the centre the chips are drawn round.
    pub ofs: [F; 2],
    /// Every object's own `ccModelHit`s (one per model of it with a Hit
    /// chunk), each at its object's place as its draw last set it; and the
    /// list (`ccModelHitTop`..) as indices into them, in order.
    pub model_hits: Vec<HitModel>,
    pub list: Vec<usize>,
    /// The lit colours `WORLD::Init` wrote into the objects' models, by
    /// model; and the ground tiles' and covers' rewritten vertices.
    lit: HashMap<u32, Arc<VertexEdits>>,
    tile_edits: Vec<Arc<VertexEdits>>,
    cover_edits: Vec<Arc<VertexEdits>>,
    /// The ground tile model and the covers' models (MDL_ objects).
    ground_model: Option<u32>,
    cover_models: Vec<Option<u32>>,
    /// The objects the last pass drew, with their layers.
    drawn: Vec<(usize, i16)>,
    /// `WORLD`'s minimap (`crate::map::field`), made by the runtime.
    pub map: Option<Box<crate::map::field::FieldMap>>,
    /// The field type's systems, what the last frame drew, what their
    /// constructors started (for the host once the field is up), and the
    /// drawing's statics as the field took them (a system the field lacks
    /// leaves its own as they were).
    systems: Systems,
    frame: Vec<Drawn>,
    made: Vec<MapEvent>,
    statics: FieldStatics,
}

impl FieldArea {
    /// What outlives the field, into its systems: the executable's
    /// statics, and the drawing's (the executable's own before any field).
    pub fn take(&mut self, statics: &Statics, field: Option<&FieldStatics>) {
        self.statics = field.copied().unwrap_or(BOOT_STATICS);
        let carried = Carried { statics, move_tex: None, field: Some(&self.statics) };
        self.systems.take(&carried);
    }

    /// The systems' statics written back, and the drawing's for the next
    /// field.
    pub fn put(&self, statics: &mut Statics) -> FieldStatics {
        let mut field = self.statics;
        self.systems.put(&mut CarriedMut { statics, move_tex: None, field: Some(&mut field) });
        field
    }

    /// The systems, for tests and probes to read by kind.
    pub fn systems(&self) -> &Systems {
        &self.systems
    }

    pub fn systems_mut(&mut self) -> &mut Systems {
        &mut self.systems
    }

    /// What the last [`FieldArea::step`] put in the frame, in its order.
    pub fn frame(&self) -> &[Drawn] {
        &self.frame
    }

    /// What the systems' constructors started (TOBJ's hum), once.
    pub fn take_made(&mut self) -> Vec<MapEvent> {
        std::mem::take(&mut self.made)
    }
}

/// `ccTransPosW2P` / `ccPlayer::W2PPos` (gcmn 0x0059b5a0) in a field: `pos`
/// less the player's, wrapped into -24000..24000 (the bounds' half each
/// way); z kept; w 1.
pub fn w2p(pos: V4, player: V4, bounds: [F; 4]) -> V4 {
    let [minx, miny, maxx, maxy] = bounds;
    let mut d = ee::vsub(pos, player);
    d[3] = ONE;
    let hx = ee::div(ee::add(minx, maxx), 0x4000_0000);
    let hy = ee::div(ee::add(miny, maxy), 0x4000_0000);
    for (v, lo, hi, h) in [(0, minx, maxx, hx), (1, miny, maxy, hy)] {
        if ee::lt(d[v], ee::sub(lo, h)) {
            d[v] = ee::add(d[v], ee::sub(hi, lo));
        } else if !ee::le(d[v], ee::sub(hi, h)) {
            d[v] = ee::add(d[v], ee::sub(lo, hi));
        }
    }
    [d[0], d[1], pos[2], ONE]
}

/// `ccTransPosP2W` / `ccPlayer::P2WPos` (gcmn 0x0059b710): a point relative
/// to the player back into the world, on the player's side of the map.
pub fn p2w(d: V4, player: V4, bounds: [F; 4]) -> V4 {
    let [minx, miny, maxx, maxy] = bounds;
    let mut r = ee::vadd(d, player);
    r[3] = ONE;
    let hx = ee::div(ee::add(minx, maxx), 0x4000_0000);
    let hy = ee::div(ee::add(miny, maxy), 0x4000_0000);
    for (v, lo, hi, h) in [(0, minx, maxx, hx), (1, miny, maxy, hy)] {
        if ee::lt(player[v], h) {
            if !ee::le(d[v], ee::sub(hi, h)) {
                r[v] = ee::add(r[v], ee::sub(lo, hi));
            }
        } else if ee::lt(d[v], ee::sub(lo, h)) {
            r[v] = ee::add(r[v], ee::sub(hi, lo));
        }
    }
    [r[0], r[1], d[2], ONE]
}

/// A Clump chunk's node objects, by the clump's own (CMP_) object.
fn clump_nodes(scene: &piney_data::scene::Scene, clump: u32) -> Option<&[u32]> {
    scene.clumps.iter().find(|(c, _)| *c == clump).map(|(_, n)| n.as_slice())
}

/// The field's bounds as `GO(1)` sets them: 0..48000 both ways.
pub const BOUNDS: [F; 4] = [0, 0, SIZE, SIZE];

/// `WORLD::GetHeight` over a generated field, for `WORLD_MAN::GetHeight`.
#[derive(Debug)]
struct Heights(Rc<Field>);

impl crate::hit::Heights for Heights {
    fn height(&self, x: F, y: F) -> F {
        self.0.get_height(x, y)
    }
}

/// The ground distance of `d` (`FOBJECT::Draw`, OUT gcmn 0x005db030).
pub(crate) fn ground_dist(volume: Volume, d: V4) -> F {
    ee::ground_dist(volume, d[0], d[1])
}

/// The fade both object kinds use: 1 to `near`, then `(far - d) / len`.
fn fade(d: F, near: F, far: F, len: F) -> F {
    if ee::lt(d, near) { ONE } else { ee::div(ee::sub(far, d), len) }
}

impl FieldArea {
    /// [`FieldArea::with_world`] on server 0, not hacked.
    pub fn new(archive: &Arc<Archive>, params: field::Params) -> Result<FieldArea> {
        FieldArea::with_world(archive, Volume::Inf, params, 2, 0, None)
    }

    /// `WORLD::WORLD`, `Init` and `Generate` for `params` (a story area's
    /// `event` and `protect` set), reading the field's files and making the
    /// field type's systems. `hack` is `WORLD_MAN` +0xf0 (`SetHackFlag`),
    /// `server` `ccGame.server`; `centre` is `WORLD_MAN`'s centre as `Init`
    /// finds it (the last area's), the start when None: the flakes are
    /// wrapped about the player on the first frame either way.
    pub fn with_world(
        archive: &Arc<Archive>,
        volume: Volume,
        params: field::Params,
        hack: u32,
        server: i32,
        centre: Option<V4>,
    ) -> Result<FieldArea> {
        let ft = params.field_type;
        let def = FieldDef::of(ft)
            .filter(|d| d.field)
            .ok_or_else(|| Error::NotFound(format!("field type {ft} has no field")))?;
        let tables = field::tables_of(volume);
        let generated = field::generate(tables, &params).map_err(|e| Error::Format(e.to_string()))?;
        let field = Rc::new(generated);
        // The field file (+0x6148): FIELD::SetCCS's models, SNOW's, the
        // minimap's texture; a hacked field's is fieldccs2's.
        let weather = Weather { field_type: ft, b: params.weather, hack: hack as i32 };
        let file = Rc::new(SceneFile::read(archive, weather.file(volume))?);
        // isHacked other than 2: BGTBL2's background, light and fog.
        let hacked = hack != 2 && tables.backgrounds.has_hacked(ft);
        let bg_name = tables.backgrounds.file_name(ft, params.weather, hacked);
        let bg = Rc::new(SceneFile::read(archive, &bg_name)?);
        let eff = SceneFile::read(archive, tables.effect_ccs).ok().map(Rc::new);
        let scene = field.scene_of(tables, &file.ccs, &bg.ccs, hacked)?;
        let hit_models = crate::hit::HitModel::read(volume, &file.ccs)?;
        let mut model_hits = Vec::new();

        // WORLD::Init's CalcObjectVertexColor: the tables' clumps' rigid
        // models lit in place, a clump listed twice lit twice.
        let models = piney_data::model::models(&file.ccs)?;
        let mut lit = HashMap::new();
        for (clump, times) in tables.lit_clumps(params.field_type) {
            let Some(nodes) = file.ccs.find_object(clump).and_then(|o| clump_nodes(&file.scene, o)) else { continue };
            for m in &models {
                if !file.scene.model_owner.get(&m.object).is_some_and(|o| nodes.contains(o)) {
                    continue;
                }
                let mut edits = VertexEdits::default();
                for (k, mm) in m.mmats.iter().enumerate() {
                    if mm.kind != piney_data::model::Kind::Rigid {
                        continue;
                    }
                    let normals: Vec<[u8; 4]> =
                        mm.normals.iter().map(|n| [n[0] as u8, n[1] as u8, n[2] as u8, 0]).collect();
                    let mut colours = mm.colours.clone();
                    for _ in 0..times {
                        let c = field::object_colours(&normals, &colours, &scene.light);
                        colours[..c.len()].copy_from_slice(&c);
                    }
                    edits.rgb.extend(colours.iter().enumerate().map(|(v, c)| (k as u32, v as u32, [c[0], c[1], c[2]])));
                }
                lit.insert(m.object, Arc::new(edits));
            }
        }

        // The objects as Generate leaves them.
        let mut objects = Vec::new();
        let mut fobj2 = Vec::new();
        let mut fobj = HashMap::new();
        for o in &field.objects {
            let two = matches!(o.kind, Kind::Entrance | Kind::Key | Kind::Sub | Kind::Lake);
            let wp = [o.pos[0], o.pos[1], o.z, ONE];
            let (play, nodes) = match o.info.anm {
                Some(a) => {
                    // SetPosition2: SetAnm, then one _AnimateForward.
                    let mut p = Play::new(&file, a).ok_or_else(|| Error::NotFound(a.into()))?;
                    p.forward(&file);
                    (Some(p), Vec::new())
                }
                None => {
                    let nodes = file
                        .ccs
                        .find_object(o.info.clump)
                        .and_then(|c| clump_nodes(&file.scene, c))
                        .map(<[u32]>::to_vec)
                        .unwrap_or_default();
                    (None, nodes)
                }
            };
            // Its hits. A Hit chunk hangs on the model it names as its
            // parent. Each object's `ccObj`s make their own `ccModel`s
            // (`ccObj::Init`, main 0x0013b63c) and each of those its own
            // `ccModelHit` (`ccModel::Init`, 0x0013a574): a copy per object,
            // the polygons shared. `ccClump::HitEnable` / `ccAnm::HitEnable`
            // enable those of the clump's nodes or of the anm's objects.
            let objs: Vec<u32> = match &play {
                Some(p) => file.anims[p.anim].objects.iter().map(|&(_, t)| t).collect(),
                None => nodes.iter().map(|n| file.scene.ext.get(n).copied().unwrap_or(*n)).collect(),
            };
            let mut hits = Vec::new();
            for m in objs.iter().filter_map(|o| file.obj_model.get(o)) {
                for h in hit_models.iter().filter(|h| h.parent == *m) {
                    hits.push(model_hits.len());
                    model_hits.push(HitModel { id: model_hits.len() as u32, ..h.clone() });
                }
            }
            let k = objects.len();
            objects.push(FieldObject {
                kind: o.kind,
                two,
                chip: [o.x, o.y],
                wp,
                pos: [0, 0, 0, 0],
                dist: 0,
                alpha: ONE,
                disp: false,
                play,
                nodes,
                hits,
            });
            if two {
                fobj2.push(k);
            } else {
                fobj.insert((o.x, o.y), k);
            }
        }

        let tile_edits = scene
            .tiles
            .iter()
            .map(|t| {
                Arc::new(VertexEdits {
                    z: t.z.iter().map(|&(v, z)| (0, u32::from(v), z)).collect(),
                    rgb: t.colours.iter().map(|&(v, c)| (0, u32::from(v), c)).collect(),
                    ..VertexEdits::default()
                })
            })
            .collect();
        let cover_edits = scene
            .covers
            .iter()
            .map(|c| {
                Arc::new(VertexEdits {
                    z: c.z.iter().map(|&(v, z)| (0, u32::from(v), z)).collect(),
                    rgb: c.colours.iter().map(|&(v, c)| (0, u32::from(v), c)).collect(),
                    ..VertexEdits::default()
                })
            })
            .collect();
        let ground_model = file.ccs.find_object(scene.ground_model);
        let cover_models = scene.covers.iter().map(|c| file.ccs.find_object(c.mesh)).collect();

        let l = &scene.light;
        let amb = l.ambient_rgb().map(f32::from_bits);
        let col = l.colour_rgb().map(f32::from_bits);
        let dir = l.dirc();
        let lights = TownLights {
            ambient: Vec3::from(amb),
            lights: vec![Light {
                kind: 1,
                pos: Vec3::ZERO,
                dir: Vec3::new(ee::f(dir[0]), ee::f(dir[1]), ee::f(dir[2])),
                colour: Vec3::from(col),
                intensity: 1.0,
                far_start: 0.0,
                far_end: 0.0,
                radius: [0.0; 2],
                priority: -1,
            }],
            fog: Some(fog_params(&scene.background.fog)),
        };

        let heights: Rc<dyn crate::hit::Heights> = Rc::new(Heights(field.clone()));
        let hits = Hits {
            volume,
            area: 1,
            bounds: Some(BOUNDS),
            heights: Some(heights),
            def_se: def.def_se,
            event_area: false,
            ..Hits::default()
        };
        let start = [field.start_pos[0], field.start_pos[1], field.start_pos[2], ONE];
        let core = AreaCore { file, hits, lights, field_rand: None, start };
        let mut area = FieldArea {
            def,
            core,
            field,
            tables,
            bg,
            eff,
            scene,
            objects,
            fobj2,
            fobj,
            ofs: [0, 0],
            model_hits,
            list: Vec::new(),
            lit,
            tile_edits,
            cover_edits,
            ground_model,
            cover_models,
            drawn: Vec::new(),
            map: None,
            systems: Systems::default(),
            frame: Vec::new(),
            made: Vec::new(),
            statics: BOOT_STATICS,
        };
        // WORLD::Init's systems from the seed's fieldrand (before the
        // layout's draws), DrawBG's, then Generate's after SetStartPos from
        // the layout's.
        let bg_systems = if area.scene.background.hacked { def.hacked } else { def.plain };
        let (systems, made, rng) = {
            let fcx = FieldCx { def, weather, server, field: Some(&area), centre: centre.unwrap_or(start) };
            let mut cx = BuildCx {
                archive,
                tables: volume,
                volume,
                file: &area.bg,
                stem: "",
                crisis: false,
                rng: Rng::new(params.seed),
                map: None,
                cc: None,
                field: Some(fcx),
                made: Vec::new(),
                rooms: None,
            };
            let mut systems = Systems::build(def.init, &mut cx)?;
            systems.extend(bg_systems, &mut cx)?;
            cx.rng = area.field.rng;
            systems.extend(def.generate, &mut cx)?;
            (systems, cx.made, cx.rng)
        };
        area.systems = systems;
        area.made = made;
        area.core.field_rand = Some(rng);
        // SetDungeonEnter: FOBJECT2::HitEnable at the entrance's own place.
        if let Some(k) = area.objects.iter().position(|o| o.kind == Kind::Entrance) {
            let wp = area.objects[k].wp;
            area.set_hit_matrix(k, wp);
            area.hit_enable(k);
            area.rebuild_hits();
        }
        Ok(area)
    }

    /// `ccClump::SetHitMatrix` / `ccAnm::SetHitMatrix` for an object at
    /// `pos` unrotated: its models' hits at `T(pos)`
    /// (`SetMatrix_PosRotZYX(pos, 0)`'s world matrix; the inverse is not read
    /// for `type` 0).
    fn set_hit_matrix(&mut self, k: usize, pos: V4) {
        let rm = crate::town::pos_rot_zyx(pos, [0; 3]);
        for i in 0..self.objects[k].hits.len() {
            let h = self.objects[k].hits[i];
            self.model_hits[h].rm = rm;
        }
    }

    /// `ccClump::HitEnable` / `ccAnm::HitEnable`: its models' hits onto
    /// the list's tail unless they are on it (`ccModelHit::HitEnable`
    /// 0x00153760).
    fn hit_enable(&mut self, k: usize) {
        for &h in &self.objects[k].hits {
            if !self.list.contains(&h) {
                self.list.push(h);
            }
        }
    }

    /// `ccClump::HitDisable` / `ccAnm::HitDisable`: its models' hits off
    /// the list (`ccModelHit::HitDisable` 0x001537e0).
    fn hit_disable(&mut self, k: usize) {
        let hits = &self.objects[k].hits;
        self.list.retain(|h| !hits.contains(h));
    }

    /// Object `k`'s hits set at its own place and enabled, as
    /// `FOBJECT2::HitEnable` does for the dungeon entrance (and a test
    /// does for any object).
    pub fn place(&mut self, k: usize) {
        let wp = self.objects[k].wp;
        self.set_hit_matrix(k, wp);
        self.hit_enable(k);
        self.rebuild_hits();
    }

    /// The collision world's model list as the `ccModelHit` list stands.
    fn rebuild_hits(&mut self) {
        self.core.hits.models = self.list.iter().map(|&h| self.model_hits[h].clone()).collect();
    }

    /// `WORLD::SetCenter(x, y)` (gcmn 0x005aa2b0), from `ccPlayer::ccPlayer`:
    /// the centre the chips are drawn round.
    pub fn set_center(&mut self, x: F, y: F) {
        self.ofs = [x, y];
    }

    /// `WORLD::AddCenter(dx, dy)` (0x005aa3d0), each frame's move from
    /// `MapLoopAdjustPos`: the centre moved and wrapped into 0..48000.
    pub fn add_center(&mut self, dx: F, dy: F) {
        for (v, d) in self.ofs.iter_mut().zip([dx, dy]) {
            *v = ee::add(*v, d);
            if !ee::le(*v, SIZE) {
                *v = ee::add(*v, ee::sub(0, SIZE));
            } else if ee::lt(*v, 0) {
                *v = ee::add(*v, ee::sub(SIZE, 0));
            }
        }
    }

    /// The chip the centre is in (`fptosi(ofs / 1200)`).
    fn centre_chip(&self) -> (i32, i32) {
        (ee::to_int(ee::div(self.ofs[0], CHIP)), ee::to_int(ee::div(self.ofs[1], CHIP)))
    }

    /// The chips `DrawObject` and `DrawMesh` walk: -6..5 round the centre's,
    /// x outer, each wrapped into 0..39 once.
    fn window(&self) -> Vec<(u32, u32)> {
        let (cx, cy) = self.centre_chip();
        let wrap = |v: i32| -> u32 {
            (if v >= 40 {
                v - 40
            } else if v < 0 {
                v + 40
            } else {
                v
            }) as u32
        };
        let mut out = Vec::with_capacity(144);
        for x in cx - WINDOW..cx + WINDOW {
            for y in cy - WINDOW..cy + WINDOW {
                out.push((wrap(x), wrap(y)));
            }
        }
        out
    }

    /// `FOBJECT::Draw` (gcmn 0x005b0e90) up to the draw: its place nearest
    /// the player, the fade by its distance across the ground, the hit
    /// matrix and `HitEnable`, the anm stepped.
    fn fobject_step(&mut self, k: usize, player: V4) {
        let d = w2p(self.objects[k].wp, player, BOUNDS);
        let pos = p2w(d, player, BOUNDS);
        let dist = ground_dist(self.core.hits.volume, d);
        let file = self.core.file.clone();
        let o = &mut self.objects[k];
        o.pos = pos;
        o.dist = dist;
        if let Some(p) = &mut o.play {
            p.forward(&file);
        }
        o.alpha = fade(dist, OBJ_NEAR, OBJ_FAR, OBJ_FADE);
        o.disp = true;
        self.set_hit_matrix(k, pos);
        self.hit_enable(k);
    }

    /// `FOBJECT2::Draw` (gcmn 0x005b13f0) up to the draw: beyond 9,600 its
    /// hit is taken off and it is not drawn; within, placed, faded, its hit
    /// set and enabled, its anm stepped.
    fn fobject2_step(&mut self, k: usize, player: V4) {
        let d = w2p(self.objects[k].wp, player, BOUNDS);
        let pos = p2w(d, player, BOUNDS);
        let dist = ground_dist(self.core.hits.volume, d);
        let o = &mut self.objects[k];
        o.dist = dist;
        o.disp = true;
        if !ee::le(dist, OBJ2_FAR) {
            o.disp = false;
            // Only a bare clump's hit is taken off here.
            if o.play.is_none() {
                self.hit_disable(k);
            }
            return;
        }
        o.pos = [pos[0], pos[1], pos[2], ONE];
        let file = self.core.file.clone();
        if let Some(p) = &mut o.play {
            p.forward(&file);
        }
        o.alpha = fade(dist, OBJ2_NEAR, OBJ2_FAR, OBJ2_FADE);
        let pos = o.pos;
        self.hit_enable(k);
        self.set_hit_matrix(k, pos);
    }

    /// `WORLD::Draw`'s object passes, as they change state: `DrawObject`
    /// (its `FOBJECT2`s but types 0, 1 and 6, then the window's
    /// `FOBJECT`s, each taken off the hit list first), the entrance and key
    /// objects, and `DrawMesh`'s type-6 ones. The hit list is then the
    /// game's for the next frame's player. Returns the objects drawn, in
    /// order, with their layer.
    pub fn step_objects(&mut self, player: V4) -> Vec<(usize, i16)> {
        let mut drawn = Vec::new();
        for i in 0..self.fobj2.len() {
            let k = self.fobj2[i];
            if matches!(obj2_type(self.objects[k].kind), 0 | 1 | 6) {
                continue;
            }
            self.fobject2_step(k, player);
            if self.objects[k].disp {
                drawn.push((k, OBJ_LAYER));
            }
        }
        for (x, y) in self.window() {
            if let Some(&k) = self.fobj.get(&(x, y)) {
                self.hit_disable(k);
                self.fobject_step(k, player);
                drawn.push((k, OBJ_LAYER));
            }
        }
        for i in 0..self.fobj2.len() {
            let k = self.fobj2[i];
            if !matches!(obj2_type(self.objects[k].kind), 0 | 1) {
                continue;
            }
            self.fobject2_step(k, player);
            if self.objects[k].disp {
                drawn.push((k, OBJ2_LAYER));
            }
        }
        for i in 0..self.fobj2.len() {
            let k = self.fobj2[i];
            if obj2_type(self.objects[k].kind) == 6 {
                self.fobject2_step(k, player);
                if self.objects[k].disp {
                    drawn.push((k, FLOOR_LAYER));
                }
            }
        }
        self.rebuild_hits();
        drawn
    }

    /// The copy of a chip-centred point nearest the player, and its
    /// distance across the ground (`FIELD_MESH::draw`, `FCOVER::Draw`).
    fn nearest(p: [F; 3], player: V4) -> (V4, f32) {
        let q = [p[0], p[1], p[2], ONE];
        let d = w2p(q, player, BOUNDS);
        (p2w(d, player, BOUNDS), ee::f(d[0]).hypot(ee::f(d[1])))
    }

    /// `WORLD::Draw` for this frame (`ccThFieldDisp`, after the player): the
    /// objects stepped and drawn, the ground tiles and cover, the water and
    /// the background, into the field's layers; `eye` is the camera's, for
    /// the fog of each piece. With `step` false (the tasks asleep under a
    /// menu) everything is drawn as the last frame left it.
    pub fn draw(&mut self, layers: &mut Layers, to_screen: Mat4, player: V4, _eye: V4, step: bool) {
        layers.older(0, depth_shades());
        // VU1 fogs each vertex by its depth (`SetFog` as the background
        // row gives it), the same for every piece.
        let fog = depth_fog(&self.scene.background.fog);
        let fog_at = |_: Vec3| fog;
        if step {
            self.drawn = self.step_objects(player);
        }
        let drawn = self.drawn.clone();
        let file = self.core.file.clone();
        let rows = HashMap::new();
        for (k, layer) in drawn {
            let o = &self.objects[k];
            let root = draw::mat(&crate::town::pos_rot_zyx(o.pos, [0; 3]));
            let alpha = ee::f(o.alpha);
            let f = fog_at(root.w_axis.truncate());
            match &o.play {
                Some(p) => {
                    let own = p.uv_rows(&file);
                    let morph = p.morph(&file);
                    // ccAnm::Draw's objects. A bone or skin model takes its
                    // clump's nodes as the anm poses them (`SetAnm` builds
                    // its node table from the anm's own objects).
                    for inst in p.instances(&file, root) {
                        let Some((model, _)) = file.drawn_model(inst.object) else { continue };
                        let world = inst.world;
                        let node_mats = crate::systems::skin_nodes(&file, p, root, inst.object, model);
                        let m = crate::town::object_morph(&file, &morph, inst.entry);
                        let d = Draw {
                            file: &file,
                            model,
                            world,
                            alpha,
                            rows: &own,
                            lights: None,
                            nodes: &node_mats,
                            morph: m,
                            clut_swaps: Vec::new(),
                        };
                        model_fogged(layers, layer, to_screen, &d, self.lit.get(&model), f);
                    }
                }
                None => {
                    let mut models: Vec<u32> = o.nodes.iter().filter_map(|n| file.obj_model.get(n).copied()).collect();
                    models.sort_unstable();
                    models.dedup();
                    for model in models {
                        if file.models.get(&model).is_none_or(|i| i.mtype & 0x600 == 0x600) {
                            continue;
                        }
                        let d = Draw {
                            file: &file,
                            model,
                            world: root,
                            alpha,
                            rows: &rows,
                            lights: None,
                            nodes: &[],
                            morph: Vec::new(),
                            clut_swaps: Vec::new(),
                        };
                        model_fogged(layers, layer, to_screen, &d, self.lit.get(&model), f);
                    }
                }
            }
        }

        // DrawMesh: the window's ground tiles, then its covers, each at its
        // copy nearest the player, culled beyond 8,400 and faded from 6,700
        // to 7,200 (covers within 7,200).
        let tile_fade = |d: f32| {
            if d < field::FADE_FROM { 1.0 } else { ((field::COVER_RANGE - d) / 500.0).clamp(0.0, 1.0) }
        };
        let window = self.window();
        if let Some(model) = self.ground_model {
            for &(x, y) in &window {
                let i = (x * field::CHIPS as u32 + y) as usize;
                let t = &self.scene.tiles[i];
                if !t.visible {
                    continue;
                }
                let (pos, d) = Self::nearest(t.pos, player);
                if d > field::TILE_RANGE {
                    continue;
                }
                let world = Mat4::from_translation(Vec3::new(ee::f(pos[0]), ee::f(pos[1]), ee::f(pos[2])));
                let dr = Draw {
                    file: &file,
                    model,
                    world,
                    alpha: tile_fade(d),
                    rows: &rows,
                    lights: None,
                    nodes: &[],
                    morph: Vec::new(),
                    clut_swaps: Vec::new(),
                };
                let f = fog_at(world.w_axis.truncate());
                model_fogged(layers, FLOOR_LAYER, to_screen, &dr, Some(&self.tile_edits[i]), f);
            }
        }
        for (i, c) in self.scene.covers.iter().enumerate() {
            let Some(model) = self.cover_models[i] else { continue };
            let chip = &self.field.covers[i];
            if !window.contains(&(chip.x, chip.y)) {
                continue;
            }
            let (pos, d) = Self::nearest(c.pos, player);
            if d > field::COVER_RANGE {
                continue;
            }
            let world = Mat4::from_translation(Vec3::new(ee::f(pos[0]), ee::f(pos[1]), ee::f(pos[2])));
            let dr = Draw {
                file: &file,
                model,
                world,
                alpha: tile_fade(d),
                rows: &rows,
                lights: None,
                nodes: &[],
                morph: Vec::new(),
                clut_swaps: Vec::new(),
            };
            let f = fog_at(world.w_axis.truncate());
            model_fogged(layers, FLOOR_LAYER, to_screen, &dr, Some(&self.cover_edits[i]), f);
        }
    }

    /// `WORLD::Draw`'s and `DrawEffect`'s weather, ambient pictures and
    /// background for the frame (after the objects, the ground and the
    /// water), in the def's order: awake, its steps; asleep, the last
    /// frame's pieces again ([`FieldDef::asleep`]).
    pub fn step(&mut self, v: &MapView, awake: bool, cc: Option<&mut dyn piney_battle::rand::Rng>) -> Vec<MapEvent> {
        let mut events = Vec::new();
        if awake {
            self.frame.clear();
            self.systems.run_rand(self.def.order, &mut self.core, v, &mut self.frame, &mut events, cc);
        } else {
            let mut kept = Vec::new();
            self.systems.run(self.def.asleep, &mut self.core, v, &mut kept, &mut events);
        }
        events
    }

    /// The scrolled background materials' offsets as the last step left
    /// them.
    pub fn rows(&self) -> HashMap<u32, [u8; 2]> {
        self.systems.rows(&self.bg)
    }

    /// The last frame's pieces as generic draws, with `rows`.
    pub fn requests<'a>(&'a self, rows: &'a HashMap<u32, [u8; 2]>) -> Vec<Request<'a>> {
        let cx = DrawCx { core: &self.core, fog: draw::Fogging::None, rows };
        self.systems.requests(&self.frame, &cx)
    }

    /// The last frame's pieces into the field's layers (the models, the 2D
    /// smoke, the background), its sprites for the effects.
    pub fn paint(&self, layers: &mut Layers, to_screen: Mat4) -> Vec<EffSprite> {
        let rows = self.rows();
        let mut sprites = Vec::new();
        for r in self.requests(&rows) {
            paint(layers, to_screen, &r, &mut sprites);
        }
        sprites
    }

    /// The places of the nodes `names` (in order) of each chip's animated
    /// `FOBJECT` (`fobj[x][y]`, y the outer loop), each node's place in the
    /// animation (its own and its parents' matrices, `_SetLWMatrix`) plus
    /// the object's `wp` (+0x30), w 1 (`WORLD::SetSpecialObj`, gcmn
    /// 0x005aae90), the animations at their first frame.
    pub fn object_nodes(&self, names: &[&str]) -> Vec<V4> {
        let file = &self.core.file;
        let mut out = Vec::new();
        for y in 0..40 {
            for x in 0..40 {
                let Some(&k) = self.fobj.get(&(x, y)) else { continue };
                let o = &self.objects[k];
                let Some(play) = &o.play else { continue };
                let worlds = play.worlds(file, Mat4::IDENTITY, &[]);
                // GetSubstAdrs: the animation's object of that name.
                for &name in names {
                    let w = worlds.iter().find(|(o, _)| file.ccs.object_name(**o) == Some(name));
                    if let Some((_, w)) = w {
                        let t = w.w_axis;
                        let p = [t.x.to_bits(), t.y.to_bits(), t.z.to_bits(), 0];
                        let p = ee::vadd(p, o.wp);
                        out.push([p[0], p[1], p[2], ONE]);
                    }
                }
            }
        }
        out
    }

    /// What `WORLD::SetFood` and `WORLD::SetSpecialObj` (gcmn 0x005aa990,
    /// 0x005aae90, from `WORLD_MAN::EntryGimmick`) read of this field, the
    /// objects as `Generate` left them (animations stepped once, no draw yet).
    /// An object's nodes are its anm's controllers, or with no anm its clump's
    /// nodes at rest. `inPoint[k]` is `DMY_inpoint<k>` plus the entrance's `wp`;
    /// `water` the lake's index among the `FOBJECT2`s.
    pub fn field_gims(&self, event_area: i32) -> piney_battle::entry::FieldGims {
        use piney_battle::entry::{FieldGims, FieldObj};
        let file = &self.core.file;
        let name = |o: u32| file.ccs.object_name(o).unwrap_or("");
        let obj = |k: usize| -> FieldObj {
            let o = &self.objects[k];
            let mut f = FieldObj { wp: o.wp, anm: o.play.is_some(), ..FieldObj::default() };
            let places: Vec<(u32, V4)> = match &o.play {
                Some(play) => crate::dungeon::pose_in(file, play, &crate::hit::UNIT, None)
                    .iter()
                    .map(|i| (i.controller, i.world[3]))
                    .collect(),
                None => o.nodes.iter().map(|&n| (n, [0, 0, 0, ONE])).collect(),
            };
            for (n, p) in places {
                let nm = name(n);
                if nm.starts_with("OBJ_xgfood0a") {
                    f.food.push(p);
                }
                if nm.starts_with("OBJ_xgsymb") {
                    f.symb.push(p);
                }
            }
            f
        };
        let fobj2: Vec<FieldObj> = self.fobj2.iter().map(|&k| obj(k)).collect();
        let mut fobj = Vec::new();
        for y in 0..40 {
            for x in 0..40 {
                if let Some(&k) = self.fobj.get(&(x, y)) {
                    fobj.push(obj(k));
                }
            }
        }
        let water = self.fobj2.iter().position(|&k| self.objects[k].kind == Kind::Lake).unwrap_or(0) as i32;
        let mut in_point = [[0; 4]; 2];
        if let Some(e) = self.objects.iter().rev().find(|o| o.kind == Kind::Entrance) {
            for (k, p) in in_point.iter_mut().enumerate() {
                let d = file
                    .ccs
                    .find_object(&format!("DMY_inpoint{k}"))
                    .and_then(|o| file.scene.dummies.get(&o))
                    .map_or([0, 0, 0, 0], |d| [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE]);
                *p = ee::vadd(d, e.wp);
            }
        }
        FieldGims { fobj2, fobj, water, in_point, event_area, field_type: self.field.params.field_type as i32 }
    }

    /// The dungeon entrance's centre (`WORLD_MAN.dungeonPos[0]`).
    pub fn dungeon_pos(&self) -> Option<V4> {
        self.field.dungeon_pos.map(|p| [p[0], p[1], p[2], ONE])
    }

    /// `WORLD_MAN.fieldStartPos` (`SetStartPos`): where a party arriving
    /// from a town stands, z 0.
    pub fn start_pos(&self) -> V4 {
        let p = self.field.start_pos;
        [p[0], p[1], p[2], ONE]
    }
}

/// The field's layer for `WORLD_MAN::SetActiveLayer(k)` (the jump table at
/// main 0x00355790): 0 sysLayer, 1 objLayer, 2 obj2Layer, 3 effLayer, 4
/// floorLayer, 5 charLayer, 6 refLayer.
pub fn ambient_layer(k: u8) -> i16 {
    match k {
        1 => OBJ_LAYER,
        2 => OBJ2_LAYER,
        3 => draw::EFF_LAYER,
        4 => FLOOR_LAYER,
        5 => draw::CHAR_LAYER,
        6 => REF_LAYER,
        _ => 0,
    }
}

/// The `SetActiveLayer` number of a layer [`ambient_layer`] gives. sysLayer
/// and objLayer are one layer here; their sprites are sysLayer's (the snow
/// on 0), none goes on objLayer.
pub fn active_layer(layer: i16) -> u8 {
    (0..=6).find(|&k| ambient_layer(k) == layer).unwrap_or(0)
}

/// `refLayer` (`WORLD_MAN` +0x4d0, priority 30), where the heat haze and
/// the frame buffer's copy it samples go.
pub const REF_LAYER: i16 = 30;

/// `ccDrawEnv::SetFog(near, far, 0, percent, colour)` as VU1 fogs each
/// vertex by its depth.
pub fn set_fog(fog: &field::Fog) -> piney_draw::DepthFog {
    fog_params(fog).depth()
}

/// `WORLD::Init`'s `cc3d->SetFog(near, far, 0, percent, colour)`.
pub fn fog_params(fog: &field::Fog) -> crate::town::FogParams {
    let colour = fog.colour.map(|c| c.clamp(0.0, 255.0) as u8);
    crate::town::FogParams { near: fog.near, far: fog.far, near_rate: 0.0, far_rate: fog.percent, colour }
}

/// `WORLD_MAN`'s two `ccBufferSampling`s (+0x418, +0x41c), which every
/// `WORLD::Draw` sends first on sysLayer: `SetShade(0, 896, 0, 8, 7, 3000,
/// 0x48808080)` and `(0, 896, 0, 7, 6, 6000, ...)`. Each copies the picture
/// into a small texture and draws it back over every pixel farther than
/// 3,000 (then 6,000), blended at 0x48: the ground, objects and sky are
/// softened with distance, not the characters or effects (docs/engine/field.md).
fn depth_shades() -> Vec<piney_draw::Cmd> {
    use piney_desktop::noiz::Sampling;
    [(8, 7, 3000.0f32), (7, 6, 6000.0)]
        .into_iter()
        .map(|(tw, th, z)| piney_draw::Cmd::Prim(Sampling::shade_own(tw, th, z.to_bits(), 0x4880_8080).prim()))
        .collect()
}

/// [`set_fog`] as a model's fog.
pub fn depth_fog(fog: &field::Fog) -> draw::Fogging {
    draw::Fogging::Depth(set_fog(fog))
}

/// [`draw::model_edited`] with the model's fog.
fn model_fogged(
    layers: &mut Layers,
    layer: i16,
    to_screen: Mat4,
    d: &Draw,
    edits: Option<&Arc<VertexEdits>>,
    fog: draw::Fogging,
) {
    draw::model_edited(layers, layer, to_screen, d, edits, fog);
}

impl crate::camera::Camera {
    /// `cameraSet` (main 0x00161260) where the map wraps: the eye and the
    /// target through `ccTransPosFW2LW` (gcmn 0x0059b9c0) - each taken
    /// relative to the player, wrapped, and back onto the player's side -
    /// then the view matrix as [`crate::camera::Camera::set`] makes it.
    pub fn set_wrapped(&mut self, player_pos: V4, bounds: [F; 4]) {
        let fw2lw = |p: V4| p2w(w2p(p, player_pos, bounds), player_pos, bounds);
        let a = self.active();
        let p = fw2lw(a.pos);
        let v = fw2lw(a.view);
        self.world_view = crate::camera::pos_target(p, v);
        self.world_screen = piney_data::anim::vu_mul(&crate::camera::VIEW_SCREEN, &self.world_view);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::bird::Bird;
    use crate::systems::lake::Lake;
    use crate::systems::sky::Sky;
    use crate::systems::tobj::Tobj;

    /// Story area 14's field (fieldSeed 1420855, type 10, weather 0,
    /// ground 1, object 1), as `WORLD_MAN::GO(1)` makes it.
    pub(crate) fn area14(archive: &Arc<Archive>) -> FieldArea {
        let params = field::Params {
            seed: 1_420_855,
            field_type: 10,
            weather: 0,
            ground: 1,
            object: 1,
            event: 14,
            protect: false,
            skip_init: false,
        };
        FieldArea::new(archive, params).unwrap()
    }

    /// Issue #38: each field object owns its `ccModelHit`s (`ccObj::Init`
    /// makes a `ccModel` per Obj, `ccModel::Init` a hit per model), so two
    /// objects of one model both stand in the way, each at its own place.
    /// One hit per model had left only the last placed solid.
    #[test]
    fn objects_of_one_model_each_keep_their_hit() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut f = area14(&archive);
        let parent = |f: &FieldArea, k: usize| f.objects[k].hits.first().map(|&h| f.model_hits[h].parent);
        let n = f.objects.len();
        let (a, b) = (0..n)
            .flat_map(|a| (a + 1..n).map(move |b| (a, b)))
            .find(|&(a, b)| parent(&f, a).is_some() && parent(&f, a) == parent(&f, b))
            .expect("two objects of one model with a hit");
        f.place(a);
        f.place(b);
        let model = parent(&f, a);
        let at: Vec<V4> = f.core.hits.models.iter().filter(|h| Some(h.parent) == model).map(|h| h.rm[3]).collect();
        assert_eq!(at, [f.objects[a].wp, f.objects[b].wp]);
    }

    /// Area 14's systems as type 10's def makes them: `DrawBG`'s four
    /// clumps each with models, its lake (`SetLake` at height 0), and BIRD
    /// by day.
    #[test]
    fn area14_pieces() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let f = area14(&archive);
        let sky = f.systems().get::<Sky>().expect("DrawBG's clumps");
        assert!((0..4).all(|k| sky.made(k) && !sky.name(k).is_empty()), "the sky, two layers, the mountains");
        let at = f.systems().get::<Lake>().and_then(Lake::at).expect("area 14's lake");
        assert_eq!(at[2], 0, "the lake stands at height 0");
        assert!(f.systems().get::<Bird>().is_some_and(|b| b.hawk.is_some()), "BIRD by day");
    }

    /// Δ's airship hums: `TOBJ::Init` asks once for the loop
    /// (`tobjSeLoopStart`), and each frame it shows gives the loop its
    /// place and transparency (`tobjSeLoop`), fading in by 0.01 a frame to
    /// 0.8.
    #[test]
    fn the_airship_hums() {
        let Some(archive) = crate::town::tests::archive() else { return };
        // A random type 10 field of seed 1 on server 0 draws its TOBJ.
        let params = field::Params {
            seed: 1,
            field_type: 10,
            weather: 0,
            ground: 1,
            object: 1,
            event: 0,
            protect: false,
            skip_init: false,
        };
        let mut f = FieldArea::with_world(&archive, Volume::Inf, params, 2, 0, None).unwrap();
        assert_eq!(f.systems().get::<Tobj>().and_then(|t| t.tobj.as_ref()).map(|t| t.server), Some(0));
        assert_eq!(f.take_made(), [MapEvent::SeLoopStart]);
        f.take(&[0; piney_desktop::SCROLL_STATICS], None);
        let player = [24000f32.to_bits(), 24000f32.to_bits(), 0, 1f32.to_bits()];
        f.core.hits.center = [player[0], player[1]];
        let mut cc = piney_battle::rand::Genrand::default();
        let mut rates = Vec::new();
        for i in 0..120 {
            let v = MapView { player, eye: player, odd: i & 1 != 0, ..MapView::default() };
            let events = f.step(&v, true, Some(&mut cc));
            let hums: Vec<F> =
                events.iter().filter_map(|e| if let MapEvent::SeLoop(_, r) = e { Some(*r) } else { None }).collect();
            assert_eq!(hums.len(), 1, "frame {i}: {events:?}");
            rates.push(f32::from_bits(hums[0]));
        }
        assert!((rates[0] - 0.01).abs() < 1e-4, "{rates:?}");
        assert!((rates[40] - 0.41).abs() < 1e-3, "{rates:?}");
        assert_eq!(rates[119].to_bits(), 0x3f4c_cccd, "{rates:?}");
    }
}
