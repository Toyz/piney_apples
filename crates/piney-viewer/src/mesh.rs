//! A scene file flattened into textured triangle batches, ready to upload.

use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

use glam::{Mat4, Vec3};
use piney_data::anim::{self, Animation};
use piney_data::ccs::Ccs;
use piney_data::dungeon::place::{self, Frame};
use piney_data::dungeon::{Floor, FogRow, INF};
use piney_data::field;
use piney_data::model::{self, Kind};
use piney_data::scene::{self, Scene};
use piney_data::statics::{self, ModelTable};
use piney_data::texture;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    /// Zero for unlit models: the shader lights only non-zero normals.
    pub normal: [f32; 3],
    /// S, T / 256; T = 0 is the first stored texture row, and textures are
    /// uploaded in stored row order, so v = T needs no flip.
    pub uv: [f32; 2],
    /// Vertex colour, 0x80 = 1.0, still in the art's gamma space; alpha is
    /// the material's transparency.
    pub colour: [f32; 4],
}

/// World matrices by object, shared by every model one pose places.
type Pose = Rc<HashMap<u32, Mat4>>;

/// How an animated job is posed again at a later time.
#[derive(Clone, Copy, Debug)]
enum Animator {
    /// One controller (object record) of animation `anim`, under `root`: its
    /// target's world is the controller's own chain.
    Controller { anim: usize, controller: u32, root: Mat4 },
    /// Every node animation `anim` poses, under `root`: bone and skin
    /// models, and what hangs under the nodes.
    Clump { anim: usize, root: Mat4 },
}

/// One model to draw: its index, the world map that poses it (None for model
/// space), an offset, and how to pose it again as it plays.
struct Job {
    model: usize,
    pose: Option<Pose>,
    offset: Vec3,
    animator: Option<Animator>,
}

impl Job {
    fn still(model: usize, pose: Option<Pose>, offset: Vec3) -> Job {
        Job { model, pose, offset, animator: None }
    }
}

/// Game frames per second in towns and fields: `frameRate` 2, every other
/// NTSC vertical blank.
pub const FIELD_FPS: f64 = 59.94 / 2.0;

/// The animated part of a mesh: what poses its vertices again at a game
/// frame.
pub struct Live {
    scene: Scene,
    models: Vec<model::Model>,
    anims: Vec<Animation>,
    jobs: Vec<LiveJob>,
    /// Model index -> the morpher (MPH_) that blends into it.
    morph_bases: HashMap<usize, u32>,
    /// MDL_ object -> model index, for morph targets.
    index_of: HashMap<u32, usize>,
}

struct LiveJob {
    model: usize,
    animator: Animator,
    /// (mmat index, first vertex, vertex count) of what `emit` wrote.
    spans: Vec<(usize, usize, usize)>,
}

impl Live {
    /// Pose every animated job at game frame `frame` - each animation
    /// stepped one frame (256 ticks) per game frame from 0, looping or
    /// holding its last frame as its chunk says - writing positions and
    /// normals into `vertices`. Returns the ranges written.
    pub fn pose(&self, frame: u64, vertices: &mut [Vertex]) -> Vec<Range<usize>> {
        let elapsed = frame * u64::from(anim::TICKS_PER_FRAME);
        let mut controllers: HashMap<usize, HashMap<u32, (u32, Mat4)>> = HashMap::new();
        let mut clumps: HashMap<usize, HashMap<u32, Mat4>> = HashMap::new();
        let mut out = Vec::new();
        for job in &self.jobs {
            let pose: HashMap<u32, Mat4> = match job.animator {
                Animator::Controller { anim, controller, root } => {
                    let worlds = controllers.entry(anim).or_insert_with(|| {
                        let a = &self.anims[anim];
                        let locals: Vec<(u32, Mat4)> =
                            a.controllers_at(a.looped(elapsed)).into_iter().map(|(o, p)| (o, p.matrix())).collect();
                        self.scene.controller_worlds_m(&locals).into_iter().map(|(c, t, w)| (c, (t, w))).collect()
                    });
                    let Some(&(target, w)) = worlds.get(&controller) else { continue };
                    HashMap::from([(target, root * w)])
                }
                Animator::Clump { anim, root } => {
                    let world = clumps.entry(anim).or_insert_with(|| {
                        let a = &self.anims[anim];
                        anim::world(&self.scene, &a.locals_at(a.looped(elapsed)))
                    });
                    world.iter().map(|(&o, &m)| (o, root * m)).collect()
                }
            };
            let morphed = self.morphed(job, elapsed);
            let m = morphed.as_ref().unwrap_or(&self.models[job.model]);
            let lit = m.mtype & 1 != 0;
            let placed = scene::place(&self.scene, m, Some(&pose));
            for &(mmat, first, count) in &job.spans {
                let Some(Some(p)) = placed.get(mmat) else { continue };
                for (i, v) in vertices[first..first + count].iter_mut().enumerate() {
                    if let Some(pos) = p.positions.get(i) {
                        v.pos = pos.to_array();
                    }
                    if lit && let Some(n) = p.normals.get(i) {
                        v.normal = n.to_array();
                    }
                }
                out.push(first..first + count);
            }
        }
        out
    }

    /// A morph base as its job's animation has blended it at `elapsed`: the
    /// last F_Morpher record's targets and weights at that frame, through
    /// `ccMorpher::Modify` (`anim::morph`), mmat by mmat. None when the model
    /// is not a morph base of that animation.
    fn morphed(&self, job: &LiveJob, elapsed: u64) -> Option<model::Model> {
        let morpher = *self.morph_bases.get(&job.model)?;
        let a = match job.animator {
            Animator::Controller { anim, .. } | Animator::Clump { anim, .. } => &self.anims[anim],
        };
        let frame = a.looped(elapsed) >> 8;
        let (_, pairs) = a.morph_weights_at(frame).into_iter().find(|(m, _)| *m == morpher)?;
        let base = &self.models[job.model];
        let mut out = base.clone();
        for (k, mm) in out.mmats.iter_mut().enumerate() {
            let targets: Vec<anim::MorphTarget> = pairs
                .iter()
                .filter_map(|&(t, weight)| {
                    let m = &self.models[*self.index_of.get(&t)?];
                    let positions = &m.mmats.get(k)?.positions;
                    Some(anim::MorphTarget { positions, scale: m.scale, weight })
                })
                .collect();
            if !targets.is_empty() {
                mm.positions = anim::morph(&base.mmats[k].positions, base.scale, &targets);
            }
        }
        Some(out)
    }

    /// Whether anything moves: some job is posed by an animation longer than
    /// one frame.
    pub fn moves(&self) -> bool {
        self.jobs.iter().any(|j| match j.animator {
            Animator::Controller { anim, .. } | Animator::Clump { anim, .. } => self.anims[anim].frames > 1,
        })
    }

    fn shift(&mut self, base: usize) {
        for job in &mut self.jobs {
            for span in &mut job.spans {
                span.1 += base;
            }
        }
    }
}

pub struct Batch {
    /// Index into [`Mesh::textures`], or None for untextured.
    pub texture: Option<usize>,
    /// The model's `flag & 3`, an index into `alphaBlendTbl`
    /// (`ccModel::Init` 0x0013a4f8): 0 normal alpha, 1 additive, 2
    /// subtractive, 3 `Cd * As + Cs`. The game draws 1-3 without depth
    /// writes.
    pub blend: u8,
    pub first: u32,
    pub count: u32,
}

pub struct TextureImage {
    pub width: u32,
    pub height: u32,
    /// RGBA8 levels, stored (bottom-up) row order: the file's own levels
    /// (level 0 and up to three mip levels), then 2x2 box-filtered down to
    /// 1x1.
    pub levels: Vec<Vec<u8>>,
}

fn mip_chain(width: u32, height: u32, mut levels: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    let last = levels.len() as u32 - 1;
    let (mut w, mut h) = (((width >> last).max(1)) as usize, ((height >> last).max(1)) as usize);
    while w > 1 || h > 1 {
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let src = levels.last().unwrap();
        let mut dst = vec![0u8; nw * nh * 4];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4 {
                    let mut sum = 0u32;
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let sx = (2 * x + dx).min(w - 1);
                        let sy = (2 * y + dy).min(h - 1);
                        sum += src[(sy * w + sx) * 4 + c] as u32;
                    }
                    dst[(y * nw + x) * 4 + c] = ((sum + 2) / 4) as u8;
                }
            }
        }
        levels.push(dst);
        (w, h) = (nw, nh);
    }
    levels
}

#[derive(Default)]
pub struct Stats {
    pub models: usize,
    pub mmats: usize,
    pub vertices: usize,
    pub triangles: usize,
    /// Bone and skin mmats left out for want of a pose.
    pub unposed: usize,
    /// Materials whose texture lives in another file.
    pub missing_textures: usize,
    /// Models placed by the executable's table, and LOD copies left out.
    pub placed: usize,
    pub lod_skipped: usize,
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub batches: Vec<Batch>,
    pub textures: Vec<TextureImage>,
    /// The full extent of the vertices.
    pub min: Vec3,
    pub max: Vec3,
    /// The 2nd to 98th percentile on each axis: what the camera frames, so
    /// a sky dome around a town does not decide the view.
    pub core_min: Vec3,
    pub core_max: Vec3,
    pub stats: Stats,
    /// Names of the file's Anime chunks, and which one posed this mesh.
    pub animes: Vec<String>,
    pub anime: Option<usize>,
    /// Every MDL_ name in the file, and the one shown alone, if any.
    pub models: Vec<String>,
    pub only: Option<usize>,
    /// The executable's table that placed this file's pieces, if one fits.
    pub table: Option<String>,
    /// Files merged in with it: a town's other files, e.g. `wat1`.
    pub companions: Vec<String>,
    /// What else to say in the title, e.g. a dungeon's seed and floor.
    pub label: Option<String>,
    /// The fog and light it is drawn in, when the game sets one.
    pub env: Option<Env>,
    /// What plays: one per file with animated jobs.
    pub live: Vec<Live>,
}

/// The fog and ambient light a dungeon room is drawn in: its `dungeonFog*`
/// row, as `SetRoom` hands it to `ccDrawEnv::SetFog` and `SetAmbient`.
#[derive(Clone, Copy, Debug)]
pub struct Env {
    /// The fog colour, which is also the frame's clear colour; 0-1 in the
    /// art's gamma.
    pub fog_colour: [f32; 3],
    /// No fog at `fog_near` and nearer, `fog_max` (0-1) of the fog colour
    /// at `fog_far` and beyond, linear between: the GS fog value VU1 writes
    /// from the vertex's depth.
    pub fog_near: f32,
    pub fog_far: f32,
    pub fog_max: f32,
    /// Ambient light, 0-1. Only the lit VU1 programs add it; unlit models -
    /// every piece of a dungeon room - draw their vertex colours as they are.
    pub ambient: [f32; 3],
}

impl Env {
    pub fn from_row(row: &FogRow) -> Env {
        Env {
            fog_colour: row.colour_bytes().map(|c| c as f32 / 255.0),
            fog_near: row.near,
            fog_far: row.far,
            fog_max: row.max / 100.0,
            ambient: row.ambient(),
        }
    }
}

impl Mesh {
    /// Pose everything that plays at game frame `frame` (see [`Live::pose`]);
    /// the vertex ranges written, merged where they touch.
    pub fn pose(&mut self, frame: u64) -> Vec<Range<usize>> {
        let Mesh { live, vertices, .. } = self;
        let mut ranges: Vec<Range<usize>> = live.iter().flat_map(|l| l.pose(frame, vertices)).collect();
        ranges.sort_by_key(|r| r.start);
        let mut out: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
        for r in ranges {
            match out.last_mut() {
                Some(last) if last.end >= r.start => last.end = last.end.max(r.end),
                _ => out.push(r),
            }
        }
        out
    }

    /// Whether anything in it moves when played.
    pub fn plays(&self) -> bool {
        self.live.iter().any(Live::moves)
    }

    /// Merge another file's mesh into this one (a town's companion file).
    pub fn append(&mut self, name: &str, other: Mesh) {
        let base = self.vertices.len() as u32;
        let first = self.indices.len() as u32;
        let textures = self.textures.len();
        self.vertices.extend(other.vertices);
        self.indices.extend(other.indices.iter().map(|i| i + base));
        self.batches.extend(other.batches.into_iter().map(|b| Batch {
            texture: b.texture.map(|t| t + textures),
            first: b.first + first,
            ..b
        }));
        self.batches.sort_by_key(|b| b.blend != 0);
        self.textures.extend(other.textures);
        self.live.extend(other.live.into_iter().map(|mut l| {
            l.shift(base as usize);
            l
        }));
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
        (self.core_min, self.core_max) = core_bounds(&self.vertices).unwrap_or((self.min, self.max));
        let (s, o) = (&mut self.stats, other.stats);
        s.models += o.models;
        s.mmats += o.mmats;
        s.vertices += o.vertices;
        s.triangles += o.triangles;
        s.unposed += o.unposed;
        s.missing_textures += o.missing_textures;
        s.placed += o.placed;
        s.lod_skipped += o.lod_skipped;
        self.companions.push(name.to_string());
    }
}

/// MDL_ object -> where the game draws it (`statics::ModelTable::place`:
/// a translation to its dummy). A row whose model also has an `...lod` row
/// is drawn in its full version; the LOD row is left out.
fn placements(c: &Ccs, sc: &Scene, table: &ModelTable) -> (HashMap<u32, Vec3>, Vec<u32>) {
    let mut out = HashMap::new();
    let mut lods = Vec::new();
    for p in table.place(c, sc) {
        let r = &table.rows[p.row];
        if let Some(base) = r.model.strip_suffix("lod")
            && table.rows.iter().any(|o| o.model == base)
        {
            lods.push(p.model);
            continue;
        }
        out.insert(p.model, p.pos);
    }
    (out, lods)
}

/// Flatten the models of `c`: all of them, or only model `only`. `anime`
/// picks the Anime chunk that poses the clump nodes and plays; when it is
/// None, a file with morph animations (cloth, faces) plays the first of
/// those, and one with bone or skin models otherwise its first Anime chunk.
/// `placed_only` keeps just the models the executable's table places (a
/// town without its sky, flags and ships).
pub fn build(c: &Ccs, anime: Option<usize>, only: Option<usize>, placed_only: bool) -> piney_data::Result<Mesh> {
    let models = model::models(c)?;
    let sc = Scene::read(c)?;
    let table = statics::for_scene(&c.name);
    let obj_table = statics::obj_for_scene(&c.name);
    let (placed_at, lods) = table.map(|t| placements(c, &sc, t)).unwrap_or_default();
    let (textures, cluts) = texture::read(c)?;
    let animes: Vec<String> = sc.animes.iter().map(|a| c.object_name(a.object).unwrap_or("?").to_string()).collect();

    let anims = Animation::all(c, &sc)?;
    let needs_pose = models.iter().any(|m| m.mmats.iter().any(|mm| matches!(mm.kind, Kind::Bone | Kind::Skin)));
    // A file with morph animations (cloth, faces) plays its first one.
    let morphs = anims.iter().position(|a| !a.morphs.is_empty());
    let chosen = match anime {
        Some(i) if i < sc.animes.len() => Some(i),
        _ if needs_pose && !sc.animes.is_empty() => Some(morphs.unwrap_or(0)),
        _ => morphs,
    };
    let world: Option<HashMap<u32, Mat4>> = match chosen {
        Some(i) => Some(sc.world(&sc.anime_frame0(c, &sc.animes[i])?)),
        None => None,
    };

    let mut mesh = Mesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        batches: Vec::new(),
        textures: Vec::new(),
        min: Vec3::splat(f32::MAX),
        max: Vec3::splat(f32::MIN),
        core_min: Vec3::ZERO,
        core_max: Vec3::ZERO,
        stats: Stats::default(),
        animes,
        anime: chosen,
        models: models.iter().map(|m| c.object_name(m.object).unwrap_or("?").to_string()).collect(),
        only: only.filter(|&i| i < models.len()),
        table: table.map(|t| t.name.to_string()),
        companions: Vec::new(),
        label: None,
        env: None,
        live: Vec::new(),
    };

    let mut jobs: Vec<Job> = Vec::new();
    let mut handled = vec![false; models.len()];
    let index_of: HashMap<u32, usize> = models.iter().enumerate().map(|(i, m)| (m.object, i)).collect();
    for (i, m) in models.iter().enumerate() {
        if let Some(&o) = placed_at.get(&m.object) {
            jobs.push(Job::still(i, None, o));
            handled[i] = true;
            mesh.stats.placed += 1;
        }
    }
    for row in obj_table.map_or(&[][..], |t| t.rows) {
        for (i, w, animator) in object_instances(c, &sc, row)? {
            if let Some(&k) = index_of.get(&i) {
                jobs.push(Job { model: k, pose: Some(w.clone()), offset: Vec3::ZERO, animator });
                handled[k] = true;
                mesh.stats.placed += 1;
            }
        }
    }
    let world = world.map(Rc::new);
    for (i, m) in models.iter().enumerate() {
        // Morph targets carry positions only (no colour, no ST): an
        // F_Morpher record blends them into a base model; they are never
        // drawn themselves.
        let morph_target = m.mtype & 0x600 == 0x600;
        if handled[i] || morph_target || (placed_only && (table.is_some() || obj_table.is_some())) {
            continue;
        }
        if lods.contains(&m.object) {
            mesh.stats.lod_skipped += 1;
            continue;
        }
        let animator = chosen.map(|anim| Animator::Clump { anim, root: Mat4::IDENTITY });
        jobs.push(Job { model: i, pose: world.clone(), offset: Vec3::ZERO, animator });
    }
    if let Some(o) = mesh.only {
        jobs.retain(|j| j.model == o);
        if jobs.is_empty() {
            jobs.push(Job::still(o, None, Vec3::ZERO));
        }
    }

    let spans = emit(&Source { sc: &sc, models: &models, textures: &textures, cluts: &cluts }, &jobs, &mut mesh);
    let live: Vec<LiveJob> = jobs
        .iter()
        .zip(spans)
        .filter_map(|(j, spans)| j.animator.map(|animator| LiveJob { model: j.model, animator, spans }))
        .collect();
    if !live.is_empty() {
        let index_of: HashMap<u32, usize> = models.iter().enumerate().map(|(i, m)| (m.object, i)).collect();
        let morph_bases = anim::morphers(c)?
            .into_iter()
            .filter_map(|(morpher, base)| Some((*index_of.get(&base)?, morpher)))
            .collect();
        mesh.live.push(Live { scene: sc, models, anims, jobs: live, morph_bases, index_of });
    }
    Ok(mesh)
}

/// What a mesh is built from: one scene file, read.
struct Source<'a> {
    sc: &'a Scene,
    models: &'a [model::Model],
    textures: &'a [texture::Texture],
    cluts: &'a HashMap<u32, texture::Clut>,
}

/// Flatten `jobs` into `mesh`, and finish its bounds. Returns, per job, the
/// (mmat index, first vertex, vertex count) of each mmat it wrote.
fn emit(src: &Source, jobs: &[Job], mesh: &mut Mesh) -> Vec<Vec<(usize, usize, usize)>> {
    let Source { sc, models, textures, cluts, .. } = *src;
    let mut tex_slot: HashMap<u32, Option<usize>> = HashMap::new();
    let mut spans = Vec::with_capacity(jobs.len());
    for job in jobs {
        let (m, offset) = (&models[job.model], job.offset);
        let mut written = Vec::new();
        mesh.stats.models += 1;
        let lit = m.mtype & 1 != 0;
        for (mmat, placed) in scene::place(sc, m, job.pose.as_deref()).into_iter().enumerate() {
            let Some(mut p) = placed else {
                mesh.stats.unposed += 1;
                continue;
            };
            p.positions.iter_mut().for_each(|v| *v += offset);
            let mm = p.mmat;
            if mm.kind == Kind::Shadow || mm.triangles.is_empty() {
                continue;
            }
            mesh.stats.mmats += 1;

            // The material's texture, uploaded once per file.
            let material = mm.material.and_then(|mat| sc.materials.get(&mat).copied());
            let texture = material.and_then(|mat| {
                *tex_slot.entry(mat.texture).or_insert_with(|| {
                    let t = textures.iter().find(|t| t.object == mat.texture)?;
                    let clut = cluts.get(&t.clut)?;
                    // The file's own levels first; a level that does not
                    // decode ends the list.
                    let own: Vec<Vec<u8>> = (0..t.levels.len()).map_while(|l| t.rgba(clut, l).ok()).collect();
                    if own.is_empty() {
                        return None;
                    }
                    let (width, height) = (t.width(0), t.height(0));
                    mesh.textures.push(TextureImage { width, height, levels: mip_chain(width, height, own) });
                    Some(mesh.textures.len() - 1)
                })
            });
            if texture.is_none() && material.is_some() {
                mesh.stats.missing_textures += 1;
            }
            // VU1 writes alpha as 128 * the material's transparency.
            let alpha = material.map_or(1.0, |m| m.transparency);

            let base = mesh.vertices.len() as u32;
            written.push((mmat, base as usize, p.positions.len()));
            for (i, pos) in p.positions.iter().enumerate() {
                let normal = if lit { p.normals.get(i).copied().unwrap_or(Vec3::ZERO) } else { Vec3::ZERO };
                let uv = mm.uvs.get(i).map(|&st| model::uv(st)).unwrap_or([0.0, 0.0]);
                let colour = match mm.colours.get(i) {
                    Some(c) => [c[0] as f32 / 128.0, c[1] as f32 / 128.0, c[2] as f32 / 128.0, alpha],
                    None => [1.0, 1.0, 1.0, alpha],
                };
                mesh.min = mesh.min.min(*pos);
                mesh.max = mesh.max.max(*pos);
                mesh.vertices.push(Vertex { pos: pos.to_array(), normal: normal.to_array(), uv, colour });
            }
            let first = mesh.indices.len() as u32;
            for t in &mm.triangles {
                mesh.indices.extend(t.iter().map(|&i| base + i));
            }
            mesh.stats.vertices += p.positions.len();
            mesh.stats.triangles += mm.triangles.len();
            mesh.batches.push(Batch {
                texture,
                blend: (m.flag & 3) as u8,
                first,
                count: mm.triangles.len() as u32 * 3,
            });
        }
        spans.push(written);
    }
    // Blended models last, over what they blend with.
    mesh.batches.sort_by_key(|b| b.blend != 0);
    if mesh.vertices.is_empty() {
        mesh.min = Vec3::splat(-1.0);
        mesh.max = Vec3::splat(1.0);
    }
    (mesh.core_min, mesh.core_max) = core_bounds(&mesh.vertices).unwrap_or((mesh.min, mesh.max));
    spans
}

/// One `STATIC_OBJ_INFO` row's models, each with the world matrices that
/// place it: the row's animation at frame 0 (or none), under the root
/// `STATICOBJECT` gives it (`statics::StaticObj::root`).
fn object_instances(
    c: &Ccs,
    sc: &Scene,
    row: &statics::StaticObj,
) -> piney_data::Result<Vec<(u32, Pose, Option<Animator>)>> {
    anime_instances(c, sc, row.anime, row.clump, row.root(c, sc))
}

/// The models an animation (at frame 0) and a clump carry, each with the
/// world matrices that place it under `root`. Every controller of the
/// animation is its own instance - ExtObj copies of one piece included -
/// drawing its target's model; a named clump's nodes are posed as the
/// animation leaves them. With each, how it plays: the animation's index in
/// the file and what it drives.
fn anime_instances(
    c: &Ccs,
    sc: &Scene,
    anime: Option<&str>,
    clump: Option<&str>,
    root: Mat4,
) -> piney_data::Result<Vec<(u32, Pose, Option<Animator>)>> {
    let index = anime.and_then(|n| c.find_object(n)).and_then(|o| sc.animes.iter().position(|a| a.object == o));
    let anim = index.map(|i| &sc.animes[i]);
    let controllers = match anim {
        Some(a) => sc.anime_controllers0(c, a)?,
        None => Vec::new(),
    };
    let mut out: Vec<(u32, Pose, Option<Animator>)> = Vec::new();
    for (controller, target, world) in sc.controller_worlds(&controllers) {
        let pose = Rc::new(HashMap::from([(target, root * world)]));
        let animator = index.map(|anim| Animator::Controller { anim, controller, root });
        for (&model, &owner) in &sc.model_owner {
            if owner == target {
                out.push((model, pose.clone(), animator));
            }
        }
    }
    // A clump's nodes (bone and skin models), posed by the animation.
    if let Some(nodes) = clump.and_then(|n| c.find_object(n)).and_then(|o| sc.clumps.iter().find(|(cl, _)| *cl == o)) {
        let locals = match anim {
            Some(a) => sc.anime_frame0(c, a)?,
            None => HashMap::new(),
        };
        let world: HashMap<u32, Mat4> = sc.world(&locals).into_iter().map(|(o, m)| (o, root * m)).collect();
        let world = Rc::new(world);
        let animator = index.map(|anim| Animator::Clump { anim, root });
        for (&model, &owner) in &sc.model_owner {
            if nodes.1.contains(&owner) && !out.iter().any(|(m, _, _)| *m == model) {
                out.push((model, world.clone(), animator));
            }
        }
    }
    out.sort_by_key(|(m, _, _)| *m);
    Ok(out)
}

/// One floor of a generated dungeon of type `dtype`, assembled from its
/// scene file (the type's `DungeonName`, e.g. `sd1`) as `SetRoom` and
/// `SetDoor` assemble each room (`piney_data::dungeon::place`): the room's
/// `ANM_` animation at frame 0 under `T(pos) * Rz(rotate)`, and the type's
/// door animation at each door dummy, open - its last frame, where `SetDoor`
/// leaves a room no entity is in. The game draws one room at a time; this
/// draws them all, so each doorway shows both rooms' halves.
pub fn build_dungeon(c: &Ccs, floor: &Floor, dtype: u8, env: Option<Env>, label: String) -> piney_data::Result<Mesh> {
    let models = model::models(c)?;
    let sc = Scene::read(c)?;
    let (textures, cluts) = texture::read(c)?;
    let mut owned: HashMap<u32, Vec<usize>> = HashMap::new();
    for (k, m) in models.iter().enumerate() {
        // Morph targets are never drawn themselves.
        if m.mtype & 0x600 != 0x600
            && let Some(&owner) = sc.model_owner.get(&m.object)
        {
            owned.entry(owner).or_default().push(k);
        }
    }
    let mut mesh = Mesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        batches: Vec::new(),
        textures: Vec::new(),
        min: Vec3::splat(f32::MAX),
        max: Vec3::splat(f32::MIN),
        core_min: Vec3::ZERO,
        core_max: Vec3::ZERO,
        stats: Stats::default(),
        animes: Vec::new(),
        anime: None,
        models: models.iter().map(|m| c.object_name(m.object).unwrap_or("?").to_string()).collect(),
        only: None,
        table: None,
        companions: Vec::new(),
        label: Some(label),
        env,
        live: Vec::new(),
    };
    let mut jobs = Vec::new();
    // A piece whose transparency track ends at 0 (a door leaf that fades
    // out as it opens) is not drawn.
    for p in place::floor(c, &sc, &INF, dtype, floor, Frame::Last)?.into_iter().filter(|p| p.alpha > 0.0) {
        let pose: Pose = Rc::new(HashMap::from([(p.target, p.world)]));
        for &k in owned.get(&p.target).map_or(&[][..], |v| v) {
            jobs.push(Job::still(k, Some(pose.clone()), Vec3::ZERO));
            mesh.stats.placed += 1;
        }
    }
    emit(&Source { sc: &sc, models: &models, textures: &textures, cluts: &cluts }, &jobs, &mut mesh);
    Ok(mesh)
}

/// A field as `WORLD` draws it (`piney_data::field::Scene`), from its CCS
/// file, its background file and the effect file holding the lake's water.
///
/// Every chip's ground tile is the type's template model with the listed
/// vertices' z and colour rewritten (`SetMESH2`), every cover likewise
/// (`SetSmallMESH`); the objects stand at the game's z, their models lit as
/// `WORLD::Init`'s `CalcObjectVertexColor` lights them - once per time the
/// tables list the clump - and animated ones play; the background is drawn
/// around the field's middle. The whole 80 x 80 map is drawn once, where the
/// game draws the 12 x 12 chips around the player on a torus.
pub fn build_field(
    c: &Ccs,
    bg: &Ccs,
    eff: Option<&Ccs>,
    fs: &field::Scene,
    field_type: u32,
    with_background: bool,
    label: String,
) -> piney_data::Result<Mesh> {
    let mut models = model::models(c)?;
    let sc = Scene::read(c)?;
    let (textures, cluts) = texture::read(c)?;
    let index_of: HashMap<u32, usize> = models.iter().enumerate().map(|(i, m)| (m.object, i)).collect();

    // `CalcObjectVertexColor` on the rigid models of the object clumps, in
    // place, once per listing.
    for (clump, times) in field::INF.lit_clumps(field_type) {
        let Some(nodes) = c.find_object(clump).and_then(|o| sc.clump_of(o)) else { continue };
        let owned: Vec<usize> = models
            .iter()
            .enumerate()
            .filter(|(_, m)| sc.model_owner.get(&m.object).is_some_and(|o| nodes.contains(o)))
            .map(|(i, _)| i)
            .collect();
        for k in owned {
            for mm in models[k].mmats.iter_mut().filter(|mm| mm.kind == Kind::Rigid) {
                let normals: Vec<[u8; 4]> =
                    mm.normals.iter().map(|n| [n[0] as u8, n[1] as u8, n[2] as u8, 0]).collect();
                for _ in 0..times {
                    let lit = field::object_colours(&normals, &mm.colours, &fs.light);
                    mm.colours[..lit.len()].copy_from_slice(&lit);
                }
            }
        }
    }

    // A template rewritten: (vertex of the first mmat, s16 z) and RGB.
    let rewrite = |base: &model::Model, z: &[(u8, i16)], colours: &[(u8, [u8; 3])]| {
        let mut m = base.clone();
        if let Some(mm) = m.mmats.first_mut() {
            for &(v, h) in z {
                if let Some(p) = mm.positions.get_mut(usize::from(v)) {
                    p[2] = h;
                }
            }
            for &(v, rgb) in colours {
                if let Some(c) = mm.colours.get_mut(usize::from(v)) {
                    c[..3].copy_from_slice(&rgb);
                }
            }
        }
        m
    };
    let at = |p: [u32; 3]| Vec3::new(f32::from_bits(p[0]), f32::from_bits(p[1]), f32::from_bits(p[2]));
    let mut jobs = Vec::new();
    let ground = c.find_object(fs.ground_model).and_then(|o| index_of.get(&o)).copied();
    if let Some(g) = ground {
        for t in fs.tiles.iter().filter(|t| t.visible) {
            models.push(rewrite(&models[g], &t.z, &t.colours));
            jobs.push(Job::still(models.len() - 1, None, at(t.pos)));
        }
    }
    for cv in &fs.covers {
        let Some(k) = c.find_object(cv.mesh).and_then(|o| index_of.get(&o)).copied() else { continue };
        models.push(rewrite(&models[k], &cv.z, &cv.colours));
        jobs.push(Job::still(models.len() - 1, None, at(cv.pos)));
    }
    let mut placed = 0;
    for o in &fs.objects {
        let root = Mat4::from_translation(Vec3::from(o.pos));
        let (anm, clump) = match o.anm {
            Some(a) => (Some(a), None),
            None => (None, Some(o.clump)),
        };
        for (model, pose, animator) in anime_instances(c, &sc, anm, clump, root)? {
            if let Some(&k) = index_of.get(&model)
                && models[k].mtype & 0x600 != 0x600
            {
                jobs.push(Job { model: k, pose: Some(pose), offset: Vec3::ZERO, animator });
                placed += 1;
            }
        }
    }

    let fog = &fs.background.fog;
    let ambient = fs.light.ambient_rgb().map(f32::from_bits);
    let mut mesh = Mesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        batches: Vec::new(),
        textures: Vec::new(),
        min: Vec3::splat(f32::MAX),
        max: Vec3::splat(f32::MIN),
        core_min: Vec3::ZERO,
        core_max: Vec3::ZERO,
        stats: Stats { placed, ..Stats::default() },
        animes: Vec::new(),
        anime: None,
        models: models.iter().map(|m| c.object_name(m.object).unwrap_or("?").to_string()).collect(),
        only: None,
        table: None,
        companions: Vec::new(),
        label: Some(label),
        env: Some(Env {
            fog_colour: fog.colour.map(|v| v / 255.0),
            fog_near: fog.near,
            fog_far: fog.far,
            fog_max: fog.percent / 100.0,
            ambient,
        }),
        live: Vec::new(),
    };
    let spans = emit(&Source { sc: &sc, models: &models, textures: &textures, cluts: &cluts }, &jobs, &mut mesh);
    let live: Vec<LiveJob> = jobs
        .iter()
        .zip(spans)
        .filter_map(|(j, spans)| j.animator.map(|animator| LiveJob { model: j.model, animator, spans }))
        .collect();
    if !live.is_empty() {
        let anims = Animation::all(c, &sc)?;
        let index_of: HashMap<u32, usize> = models.iter().enumerate().map(|(i, m)| (m.object, i)).collect();
        let morph_bases = anim::morphers(c)?
            .into_iter()
            .filter_map(|(morpher, base)| Some((*index_of.get(&base)?, morpher)))
            .collect();
        mesh.live.push(Live { scene: sc, models, anims, jobs: live, morph_bases, index_of });
    }

    // The background around the field's middle, each clump at rest - when
    // asked for: the game centres it on the player, and seen from outside it
    // hides the field.
    if !with_background {
        if let (Some(eff), Some(w)) = (eff, fs.water) {
            let root = Mat4::from_translation(Vec3::from(w.pos));
            let (core_min, core_max) = (mesh.core_min, mesh.core_max);
            mesh.append(&eff.name, placed_file(eff, |esc| anime_instances(eff, esc, Some(w.anime), None, root))?);
            (mesh.core_min, mesh.core_max) = (core_min, core_max);
        }
        return Ok(mesh);
    }
    let middle = Mat4::from_translation(Vec3::new(field::WORLD_SIZE / 2.0, field::WORLD_SIZE / 2.0, 0.0));
    let mut sky = placed_file(bg, |bsc| {
        let mut out = Vec::new();
        for clump in &fs.background.clumps {
            out.extend(anime_instances(bg, bsc, None, Some(clump), middle)?);
        }
        Ok(out)
    })?;
    // The sky is behind everything; the core framing should not include it.
    sky.core_min = sky.min;
    sky.core_max = sky.max;
    let (core_min, core_max) = (mesh.core_min, mesh.core_max);
    mesh.append(&bg.name, sky);
    if let (Some(eff), Some(w)) = (eff, fs.water) {
        let root = Mat4::from_translation(Vec3::from(w.pos));
        let water = placed_file(eff, |esc| anime_instances(eff, esc, Some(w.anime), None, root))?;
        mesh.append(&eff.name, water);
    }
    (mesh.core_min, mesh.core_max) = (core_min, core_max);
    Ok(mesh)
}

/// The models of `c` that `instances` places, as their own mesh (their
/// animations playing).
fn placed_file(
    c: &Ccs,
    instances: impl FnOnce(&Scene) -> piney_data::Result<Vec<(u32, Pose, Option<Animator>)>>,
) -> piney_data::Result<Mesh> {
    let models = model::models(c)?;
    let sc = Scene::read(c)?;
    let (textures, cluts) = texture::read(c)?;
    let index_of: HashMap<u32, usize> = models.iter().enumerate().map(|(i, m)| (m.object, i)).collect();
    let jobs: Vec<Job> = instances(&sc)?
        .into_iter()
        .filter_map(|(model, pose, animator)| {
            let k = *index_of.get(&model)?;
            (models[k].mtype & 0x600 != 0x600).then_some(Job {
                model: k,
                pose: Some(pose),
                offset: Vec3::ZERO,
                animator,
            })
        })
        .collect();
    let mut mesh = Mesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        batches: Vec::new(),
        textures: Vec::new(),
        min: Vec3::splat(f32::MAX),
        max: Vec3::splat(f32::MIN),
        core_min: Vec3::ZERO,
        core_max: Vec3::ZERO,
        stats: Stats::default(),
        animes: Vec::new(),
        anime: None,
        models: Vec::new(),
        only: None,
        table: None,
        companions: Vec::new(),
        label: None,
        env: None,
        live: Vec::new(),
    };
    let spans = emit(&Source { sc: &sc, models: &models, textures: &textures, cluts: &cluts }, &jobs, &mut mesh);
    let live: Vec<LiveJob> = jobs
        .iter()
        .zip(spans)
        .filter_map(|(j, spans)| j.animator.map(|animator| LiveJob { model: j.model, animator, spans }))
        .collect();
    if !live.is_empty() {
        let anims = Animation::all(c, &sc)?;
        let index_of: HashMap<u32, usize> = models.iter().enumerate().map(|(i, m)| (m.object, i)).collect();
        let morph_bases = anim::morphers(c)?
            .into_iter()
            .filter_map(|(morpher, base)| Some((*index_of.get(&base)?, morpher)))
            .collect();
        mesh.live.push(Live { scene: sc, models, anims, jobs: live, morph_bases, index_of });
    }
    Ok(mesh)
}

fn core_bounds(vertices: &[Vertex]) -> Option<(Vec3, Vec3)> {
    if vertices.len() < 16 {
        return None;
    }
    let mut lo = [0f32; 3];
    let mut hi = [0f32; 3];
    for axis in 0..3 {
        let mut v: Vec<f32> = vertices.iter().map(|x| x.pos[axis]).collect();
        v.sort_by(f32::total_cmp);
        lo[axis] = v[v.len() * 2 / 100];
        hi[axis] = v[(v.len() * 98 / 100).min(v.len() - 1)];
    }
    Some((Vec3::from(lo), Vec3::from(hi)))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use piney_data::archive::Archive;
    use piney_data::iso::Iso;

    use super::*;

    /// Every scene on Infection's disc flattens with indices in range.
    #[test]
    fn every_data_bin_scene_builds() {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return;
        }
        let arc = Archive::new(Iso::open(iso).unwrap().read_path("DATA/DATA.BIN").unwrap()).unwrap();
        let (mut built, mut tris) = (0, 0);
        for m in arc.members() {
            let c = Ccs::parse(arc.inflate(m).unwrap()).unwrap();
            let mesh = build(&c, None, None, false).unwrap();
            let n = mesh.vertices.len() as u32;
            assert!(mesh.indices.iter().all(|&i| i < n), "{}", m.name);
            assert!(mesh.batches.iter().all(|b| b.texture.is_none_or(|t| t < mesh.textures.len())));
            built += !mesh.indices.is_empty() as usize;
            tris += mesh.stats.triangles;
        }
        assert!(built > 500, "{built} scenes with geometry");
        assert!(tris > 1_000_000, "{tris} triangles");
    }

    /// A floor of every dungeon type builds, with its doors, lit by its
    /// type's fog row.
    /// Mac Anu's flags and ships play: posing moves vertices, keeps them
    /// finite, and posing frame 0 again puts them back.
    #[test]
    fn town_objects_play() {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return;
        }
        let arc = Archive::new(Iso::open(iso).unwrap().read_path("DATA/DATA.BIN").unwrap()).unwrap();
        let c = Ccs::parse(arc.inflate_named("town01").unwrap()).unwrap();
        let mut mesh = build(&c, None, None, false).unwrap();
        assert!(mesh.plays());
        mesh.pose(0);
        let at0: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| v.pos).collect();
        let ranges = mesh.pose(45);
        assert!(!ranges.is_empty());
        let moved = mesh.vertices.iter().zip(&at0).filter(|(v, p)| v.pos != **p).count();
        assert!(moved > 100, "{moved} vertices moved");
        assert!(mesh.vertices.iter().all(|v| v.pos.iter().all(|x| x.is_finite())));
        mesh.pose(0);
        assert!(mesh.vertices.iter().zip(&at0).all(|(v, p)| v.pos == *p));
    }

    /// `xp_flag`'s cloth is a morph: playing it moves the flag's vertices.
    #[test]
    fn morph_cloth_plays() {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return;
        }
        let arc = Archive::new(Iso::open(iso).unwrap().read_path("DATA/DATA.BIN").unwrap()).unwrap();
        let c = Ccs::parse(arc.inflate_named("xp_flag").unwrap()).unwrap();
        let mut mesh = build(&c, None, None, false).unwrap();
        assert!(mesh.anime.is_some() && mesh.plays());
        mesh.pose(1);
        let before: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| v.pos).collect();
        mesh.pose(15);
        let moved = mesh.vertices.iter().zip(&before).filter(|(v, p)| v.pos != **p).count();
        assert!(moved > 10, "{moved} vertices moved");
    }

    /// A field of every type builds: tiles, cover and objects, all vertices
    /// finite, and objects standing within the ground's height range.
    #[test]
    fn every_field_type_builds() {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return;
        }
        let arc = Archive::new(Iso::open(iso).unwrap().read_path("DATA/DATA.BIN").unwrap()).unwrap();
        let t = &field::INF;
        for ft in (0..field::FIELD_TYPES as u32).filter(|&ft| t.ccs[ft as usize].is_some()) {
            let params = field::Params {
                seed: 4321 + ft,
                field_type: ft,
                weather: ft % 3,
                ground: 1,
                object: 2,
                event: 0,
                protect: false,
                skip_init: false,
            };
            let f = field::generate(t, &params).unwrap();
            let c = Ccs::parse(arc.inflate_named(t.ccs[ft as usize].unwrap()).unwrap()).unwrap();
            let bg = Ccs::parse(arc.inflate_named(&t.backgrounds.ccs_name(ft, params.weather)).unwrap()).unwrap();
            let eff = Ccs::parse(arc.inflate_named(t.effect_ccs).unwrap()).unwrap();
            let scene = f.scene(t, &c, &bg).unwrap();
            let mesh = build_field(&c, &bg, Some(&eff), &scene, ft, true, String::new()).unwrap();
            assert!(mesh.stats.triangles > 10_000, "type {ft}: {} triangles", mesh.stats.triangles);
            assert!(mesh.vertices.iter().all(|v| v.pos.iter().all(|x| x.is_finite())), "type {ft}");
            let n = mesh.indices.len();
            assert!(mesh.indices.iter().all(|&i| (i as usize) < mesh.vertices.len()) && n > 0);
            assert!(scene.objects.iter().all(|o| o.pos[2] > -600.0 && o.pos[2] < 3000.0), "type {ft}");
        }
    }

    #[test]
    fn every_dungeon_type_builds() {
        use piney_data::dungeon::{self, Dummies, Params, TexClut};
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return;
        }
        let arc = Archive::new(Iso::open(iso).unwrap().read_path("DATA/DATA.BIN").unwrap()).unwrap();
        let mut doors = 0;
        for dtype in 0..10u8 {
            let c = Ccs::parse(arc.inflate_named(INF.ccs_name(dtype, 0)).unwrap()).unwrap();
            let dummies = Dummies::read(&c, &INF.gim_patterns).unwrap();
            let p = Params {
                seed: 154874216,
                dtype,
                level_max: 2,
                room_max: 9,
                server: 0,
                volume: 1,
                word_a: 0,
                field_type: if dungeon::is_lake(dtype) { 4 } else { 7 },
                code: 0,
            };
            let d = dungeon::generate(&INF, &p, &dummies).unwrap();
            let row = INF.fog_row(dtype, TexClut { clut_type: 2, tex_type: 0 }, 0).unwrap();
            let mesh = build_dungeon(&c, &d.floors[0], dtype, Some(Env::from_row(row)), String::new()).unwrap();
            let n = mesh.vertices.len() as u32;
            assert!(mesh.indices.iter().all(|&i| i < n) && mesh.stats.triangles > 1000, "type {dtype}");
            let sc = Scene::read(&c).unwrap();
            doors += place::floor(&c, &sc, &INF, dtype, &d.floors[0], Frame::Last)
                .unwrap()
                .iter()
                .filter(|p| p.door)
                .count();
        }
        assert!(doors > 50, "{doors} door pieces");
    }
}
