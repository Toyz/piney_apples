//! Look at the scene files on a .hack disc.
//!
//!     piney-viewer [--iso PATH] [NAME]
//!     piney-viewer [--iso PATH] --shot OUT.png [--anime N] [--model N] [--size WxH] NAME
//!
//! Reads `DATA/DATA.BIN` straight from the disc image (default
//! `work/infection/infection.iso`) and shows one scene file at a time, every
//! model in it textured; bone and skin models are posed from frame 0 of the
//! file's first Anime chunk. NAME starts on the first file whose name
//! contains it; without one, on `town01` (Mac Anu).
//!
//! Mouse: left drag orbits, right or middle drag pans, wheel zooms.
//! Keys: Left/Right or PageUp/PageDown the previous/next file, A the next
//! animation pose, M the next model alone (then all again), B the exposure
//! (1x, 2x, 4x: unlit models are as dark as their baked colours), T only the
//! pieces the executable's table places (a town without its sky), R resets
//! the camera, Escape quits.
//! Gamepad (e.g. DualSense): left stick orbits, right stick pans, L2/R2 zoom,
//! L1/R1 or D-pad left/right the previous/next file, Triangle the next
//! animation pose, Square the next model alone, Cross resets the camera.
//!
//! Animations play as the game plays them (`piney_data::anim`), one frame
//! per game frame at 29.97 frames a second as in towns and fields: a town's
//! flags and ships, a character's chosen pose. Space or P (Options on the
//! pad) pauses and resumes; `--time SECONDS` poses a shot at that time.
//!
//! `--dungeon SEED[,TYPE[,SIZE[,SERVER]]]` shows a random dungeon instead:
//! generated from `dungeonSeed` SEED as the game does
//! (`piney_data::dungeon`), of type TYPE (0-9, default 0) and size SIZE (a
//! `dungeonData` row, 0-10, default 5) on server SERVER (0-4, default 0), as
//! a random area whose field gives that type. Each room is placed and given
//! its doors as `SetRoom` and `SetDoor` do (`dungeon::place`), against the
//! clear colour of its `dungeonFog*` row. Left/Right (L1/R1) change floor, A
//! (Triangle) the dungeon type, M (Square) the seed, F (Circle) turns the
//! row's fog on or off (off at first: from overhead everything is past the
//! fog's far distance); `--floor N` starts on floor N, `--fog` shoots with
//! fog.
//!
//! `--field SEED[,TYPE[,WEATHER]]` shows a random field: generated from
//! `fieldSeed` SEED (`piney_data::field`) of field type TYPE (0-10, not 4)
//! in weather WEATHER (0-9), drawn as `WORLD` draws it - each chip's ground
//! tile and cover rewritten from the height map and lit by the area's
//! background light, objects at the game's heights, the lake's water and
//! the background - the whole map at once. Left/Right (L1/R1) change the
//! weather, A (Triangle) the field type, M (Square) the seed, F (Circle) the
//! fog.
//!
//! `--shot` renders one frame of NAME (or of the dungeon) offscreen and writes it as a PNG, with
//! no window: from the starting camera, or `--cam YAW,PITCH,ZOOM` (degrees,
//! and a distance factor, 1 = the starting distance), looking at `--look
//! X,Y,Z` (world units) instead of the scene's middle.

mod mesh;
mod png;
mod render;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use gilrs::{Axis, Button, EventType, Gilrs};
use glam::{Mat4, Vec3};
use piney_data::archive::Archive;
use piney_data::ccs::{self, Ccs};
use piney_data::dungeon::{self, Dummies, Dungeon, INF, Params, TexClut};
use piney_data::field;
use piney_data::iso::Iso;
use piney_data::statics;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, OwnedDisplayHandle};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::mesh::Mesh;
use crate::render::{Gpu, GpuScene};

struct Camera {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    /// Of what is framed.
    radius: f32,
    /// Of everything, for the clip planes.
    extent: f32,
}

impl Camera {
    fn frame(mesh: &Mesh) -> Self {
        let radius = ((mesh.core_max - mesh.core_min).length() * 0.5).max(1e-3);
        let extent = ((mesh.max - mesh.min).length() * 0.5).max(radius);
        let target = (mesh.core_min + mesh.core_max) * 0.5;
        Camera { target, yaw: -0.9, pitch: 0.35, distance: radius * 2.4, radius, extent }
    }

    fn eye(&self) -> Vec3 {
        let dir = Vec3::new(self.pitch.cos() * self.yaw.cos(), self.pitch.cos() * self.yaw.sin(), self.pitch.sin());
        self.target + dir * self.distance
    }

    fn view_proj(&self, aspect: f32) -> Mat4 {
        // Z is up in the game's scenes.
        let view = glam::camera::rh::view::look_at_mat4(self.eye(), self.target, Vec3::Z);
        let near = (self.radius * 0.002).max(1e-3);
        let far = self.distance + self.extent * 4.0;
        // wgpu's clip space: depth 0..1, y up.
        glam::camera::rh::proj::directx::perspective(50f32.to_radians(), aspect, near, far) * view
    }

    fn orbit(&mut self, dyaw: f32, dpitch: f32) {
        self.yaw += dyaw;
        self.pitch = (self.pitch + dpitch).clamp(-1.55, 1.55);
    }

    fn pan(&mut self, dx: f32, dy: f32) {
        let forward = (self.target - self.eye()).normalize_or_zero();
        let right = forward.cross(Vec3::Z).normalize_or_zero();
        let up = right.cross(forward);
        let scale = self.distance * 0.0015;
        self.target += (-right * dx + up * dy) * scale;
    }

    fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(self.radius * 0.02, self.radius * 50.0);
    }
}

/// A scene file flattened, with its town's companion files merged in
/// (`statics::companions`: `wat1`, the water, for Mac Anu).
fn build_scene(
    archive: &Archive,
    data: Vec<u8>,
    anime: Option<usize>,
    only: Option<usize>,
    placed_only: bool,
) -> piney_data::Result<(String, Mesh)> {
    let c = Ccs::parse(data)?;
    let mut mesh = mesh::build(&c, anime, only, placed_only)?;
    if only.is_none() {
        for name in statics::companions(&c.name) {
            let Some(m) = archive.find(name) else { continue };
            let other = Ccs::parse(archive.inflate(m)?)?;
            if !other.walk().chunks.iter().any(|ch| !ch.in_frames && ch.kind == ccs::MODEL) {
                continue;
            }
            mesh.append(name, mesh::build(&other, None, None, false)?);
        }
    }
    Ok((c.name.clone(), mesh))
}

/// A generated dungeon: what it was generated from, and the floor shown.
struct DungeonView {
    params: Params,
    dungeon: Dungeon,
    ccs: Ccs,
    floor: usize,
    /// `WORLD_MAN.clutType` / `texType` for the area.
    tex_clut: TexClut,
}

impl DungeonView {
    fn generate(archive: &Archive, params: Params, floor: usize) -> Result<Self, String> {
        let tables = &INF;
        let tex_clut = tables.tex_clut(params.server, params.field_type);
        let tex = u32::from(tex_clut.tex_type);
        let dummies = Dummies::load(archive, tables, params.dtype, tex).map_err(|e| e.to_string())?;
        let generated = dungeon::generate(tables, &params, &dummies).map_err(|e| format!("{e:?}"))?;
        let name = tables.ccs_name(params.dtype, tex);
        let ccs = archive.inflate_named(name).and_then(Ccs::parse).map_err(|e| format!("{name}: {e}"))?;
        let floor = floor.min(generated.floors.len().saturating_sub(1));
        Ok(DungeonView { params, dungeon: generated, ccs, floor, tex_clut })
    }

    fn mesh(&self) -> piney_data::Result<Mesh> {
        let fl = &self.dungeon.floors[self.floor];
        // `WORLD_MAN::GetBG` (the lakes' row) is 0 here: no field sets it.
        let (table, row) = INF.fog_table(self.params.dtype, self.tex_clut, 0);
        let env = table.rows.get(row).map(mesh::Env::from_row);
        let label = format!(
            "dungeon seed {} type {} ({}) server {} floor {}/{} - {} rooms{} - {}[{row}]",
            self.params.seed,
            self.params.dtype,
            self.ccs.name,
            self.params.server,
            self.floor + 1,
            self.dungeon.floors.len(),
            fl.rooms.len(),
            if self.dungeon.statue.as_ref().is_some_and(|s| s.floor == self.floor) { ", Gott statue" } else { "" },
            table.name,
        );
        mesh::build_dungeon(&self.ccs, fl, self.params.dtype, env, label)
    }
}

/// A generated field: what it was generated from, and the result.
struct FieldView {
    params: field::Params,
    field: field::Field,
}

impl FieldView {
    fn generate(params: field::Params) -> Result<Self, String> {
        let generated = field::generate(&field::INF, &params).map_err(|e| e.to_string())?;
        Ok(FieldView { params, field: generated })
    }

    fn mesh(&self, archive: &Archive) -> Result<Mesh, String> {
        let t = &field::INF;
        let (ft, weather) = (self.params.field_type, self.params.weather);
        let name = t.ccs[ft as usize].ok_or(format!("field type {ft} has no field"))?;
        let read = |n: &str| archive.inflate_named(n).and_then(Ccs::parse).map_err(|e| format!("{n}: {e}"));
        let c = read(name)?;
        let bg = read(&t.backgrounds.ccs_name(ft, weather))?;
        let eff = read(t.effect_ccs).ok();
        let scene = self.field.scene(t, &c, &bg).map_err(|e| e.to_string())?;
        let label = format!(
            "field seed {} type {ft} ({name}) weather {weather} - {} objects, {} covers{} - background {}",
            self.params.seed,
            scene.objects.len(),
            scene.covers.len(),
            if scene.water.is_some() { ", a lake" } else { "" },
            bg.name,
        );
        mesh::build_field(&c, &bg, eff.as_ref(), &scene, ft, false, label).map_err(|e| e.to_string())
    }
}

/// The generator's inputs for a random area: the middle ground and object
/// settings, no story area.
fn field_params(seed: u32, field_type: u32, weather: u32) -> field::Params {
    field::Params { seed, field_type, weather, ground: 1, object: 2, event: 0, protect: false, skip_init: false }
}

/// The next field type that has a field (type 4's gate leads straight into
/// a dungeon).
fn next_field_type(ft: u32) -> u32 {
    let mut n = (ft + 1) % field::FIELD_TYPES as u32;
    while field::INF.ccs[n as usize].is_none() {
        n = (n + 1) % field::FIELD_TYPES as u32;
    }
    n
}

/// A field type whose random area gets dungeon type `dtype` (the E types,
/// 4-7 and 9, only come from story areas: their rule), preferring one whose
/// dungeon loads the plain textures (`texType` 0) on `server`.
fn field_type_for(dtype: u8, server: u32) -> u32 {
    let rows = if matches!(dtype, 4..=7 | 9) { &INF.types.story_e } else { &INF.types.random };
    let fits: Vec<u32> = (0..11).filter(|&ft| rows[ft as usize][0] == dtype).collect();
    fits.iter().copied().find(|&ft| INF.tex_clut(server, ft).tex_type == 0).or(fits.first().copied()).unwrap_or(0)
}

/// The generator's inputs for a seed, type, `dungeonData` row and server, as
/// a random area in volume 1 would give them: a field of a type that leads
/// to that dungeon type (4, for the lake types 8 and 9, leads straight in).
fn dungeon_params(seed: u32, dtype: u8, size: usize, server: u32) -> Params {
    let d = dungeon::INF.dungeon_data[size.min(10)];
    Params {
        seed,
        dtype,
        level_max: d.levels,
        room_max: d.rooms,
        server,
        volume: 1,
        word_a: 0,
        field_type: field_type_for(dtype, server),
        code: 0,
    }
}

struct Loaded {
    name: String,
    mesh: Mesh,
    gpu: Option<GpuScene>,
}

struct Viewer {
    archive: Archive,
    /// Members with at least one model chunk.
    files: Vec<usize>,
    current: usize,
    anime: Option<usize>,
    only: Option<usize>,
    placed_only: bool,
    /// Exposure, cycled by B.
    bright: f32,
    /// The scene's fog, toggled by F.
    fog: bool,
    /// Animations play, toggled by Space or P; `clock` is the time played,
    /// `shown` the game frame the vertices were last posed at.
    playing: bool,
    clock: f64,
    shown: Option<u64>,
    dungeon: Option<DungeonView>,
    field: Option<FieldView>,
    loaded: Option<Loaded>,
    camera: Camera,
    instance: Option<wgpu::Instance>,
    display: OwnedDisplayHandle,
    gpu: Option<Gpu>,
    gilrs: Option<Gilrs>,
    last: Instant,
    cursor: (f64, f64),
    dragging: Option<MouseButton>,
}

impl Viewer {
    fn load(&mut self, index: usize, anime: Option<usize>, only: Option<usize>) {
        self.current = index;
        let member = &self.archive.members()[self.files[index]];
        let result = self
            .archive
            .inflate(member)
            .and_then(|data| build_scene(&self.archive, data, anime, only, self.placed_only));
        match result {
            Ok((name, mesh)) => {
                let reframe = self.loaded.as_ref().is_none_or(|l| l.name != name || l.mesh.only != mesh.only);
                if reframe {
                    self.camera = Camera::frame(&mesh);
                }
                self.anime = mesh.anime;
                self.only = mesh.only;
                let gpu = self.gpu.as_ref().map(|g| g.renderer.upload(&mesh));
                self.loaded = Some(Loaded { name, mesh, gpu });
                self.shown = None;
            }
            Err(e) => {
                eprintln!("{}: {e}", member.name);
                self.loaded = None;
            }
        }
        self.update_title();
    }

    /// Show the current dungeon floor.
    fn load_dungeon(&mut self) {
        let Some(view) = &self.dungeon else { return };
        match view.mesh() {
            Ok(mesh) => {
                self.camera = Camera::frame(&mesh);
                let gpu = self.gpu.as_ref().map(|g| g.renderer.upload(&mesh));
                self.loaded = Some(Loaded { name: view.ccs.name.clone(), mesh, gpu });
                self.shown = None;
            }
            Err(e) => {
                eprintln!("dungeon: {e}");
                self.loaded = None;
            }
        }
        self.update_title();
    }

    /// Show the current field.
    fn load_field(&mut self) {
        let Some(view) = &self.field else { return };
        match view.mesh(&self.archive) {
            Ok(mesh) => {
                self.camera = Camera::frame(&mesh);
                let gpu = self.gpu.as_ref().map(|g| g.renderer.upload(&mesh));
                self.loaded = Some(Loaded { name: "field".into(), mesh, gpu });
                self.shown = None;
            }
            Err(e) => {
                eprintln!("field: {e}");
                self.loaded = None;
            }
        }
        self.update_title();
    }

    /// Generate the field again.
    fn regenerate_field(&mut self, params: field::Params) {
        match FieldView::generate(params) {
            Ok(v) => {
                self.field = Some(v);
                self.load_field();
            }
            Err(e) => eprintln!("field seed {} type {}: {e}", params.seed, params.field_type),
        }
    }

    /// Generate the dungeon again with a new seed or type, keeping its size
    /// and the floor.
    fn regenerate(&mut self, seed: u32, dtype: u8) {
        let Some(view) = &self.dungeon else { return };
        let size = dungeon::INF
            .dungeon_data
            .iter()
            .position(|d| d.levels == view.params.level_max && d.rooms == view.params.room_max)
            .unwrap_or(5);
        match DungeonView::generate(&self.archive, dungeon_params(seed, dtype, size, view.params.server), view.floor) {
            Ok(v) => {
                self.dungeon = Some(v);
                self.load_dungeon();
            }
            Err(e) => eprintln!("dungeon seed {seed} type {dtype}: {e}"),
        }
    }

    fn step(&mut self, delta: isize) {
        if let Some(view) = &self.field {
            let p = view.params;
            let weather = (p.weather as isize + delta).rem_euclid(field::WEATHERS as isize) as u32;
            self.regenerate_field(field_params(p.seed, p.field_type, weather));
            return;
        }
        if let Some(view) = &mut self.dungeon {
            let n = view.dungeon.floors.len() as isize;
            view.floor = (view.floor as isize + delta).rem_euclid(n) as usize;
            self.load_dungeon();
            return;
        }
        let n = self.files.len() as isize;
        let next = (self.current as isize + delta).rem_euclid(n) as usize;
        self.load(next, None, None);
    }

    fn next_anime(&mut self) {
        if let Some(view) = &self.field {
            let p = view.params;
            self.regenerate_field(field_params(p.seed, next_field_type(p.field_type), p.weather));
            return;
        }
        if let Some(view) = &self.dungeon {
            let (seed, dtype) = (view.params.seed, (view.params.dtype + 1) % 10);
            self.regenerate(seed, dtype);
            return;
        }
        let count = self.loaded.as_ref().map_or(0, |l| l.mesh.animes.len());
        if count > 0 {
            let next = self.anime.map_or(0, |a| (a + 1) % count);
            self.load(self.current, Some(next), self.only);
        }
    }

    /// All models, then each alone in turn, then all again.
    fn next_model(&mut self) {
        if let Some(view) = &self.field {
            let p = view.params;
            self.regenerate_field(field_params(dungeon::Rng::new(p.seed).advance(), p.field_type, p.weather));
            return;
        }
        if let Some(view) = &self.dungeon {
            // A new seed: the next value of the game's own RNG.
            let (seed, dtype) = (dungeon::Rng::new(view.params.seed).advance(), view.params.dtype);
            self.regenerate(seed, dtype);
            return;
        }
        let count = self.loaded.as_ref().map_or(0, |l| l.mesh.models.len());
        if count > 1 {
            let next = match self.only {
                None => Some(0),
                Some(i) if i + 1 < count => Some(i + 1),
                Some(_) => None,
            };
            self.load(self.current, self.anime, next);
        }
    }

    /// Pose what plays at the clock's game frame and send the vertices that
    /// moved to the GPU.
    fn animate(&mut self) {
        let frame = (self.clock * mesh::FIELD_FPS) as u64;
        let Some(l) = &mut self.loaded else { return };
        if self.shown == Some(frame) || !l.mesh.plays() {
            return;
        }
        self.shown = Some(frame);
        let ranges = l.mesh.pose(frame);
        if let (Some(gpu), Some(scene)) = (&self.gpu, &l.gpu) {
            for r in ranges {
                gpu.renderer.update_vertices(scene, r.start, &l.mesh.vertices[r]);
            }
        }
    }

    fn reset_camera(&mut self) {
        if let Some(l) = &self.loaded {
            self.camera = Camera::frame(&l.mesh);
        }
    }

    fn update_title(&self) {
        let Some(gpu) = &self.gpu else { return };
        let title = match &self.loaded {
            Some(l) => {
                let s = &l.mesh.stats;
                let mut t = format!(
                    "piney - {} ({}/{}) - {} models, {} vertices, {} triangles, {} textures",
                    l.name,
                    self.current + 1,
                    self.files.len(),
                    s.models,
                    s.vertices,
                    s.triangles,
                    l.mesh.textures.len()
                );
                if let Some(label) = &l.mesh.label {
                    t = format!("piney - {label} - {} vertices, {} triangles", s.vertices, s.triangles);
                }
                if let Some(table) = &l.mesh.table {
                    t += &format!(" - {} placed by {table}", s.placed);
                }
                if !l.mesh.companions.is_empty() {
                    t += &format!(" - with {}", l.mesh.companions.join(", "));
                }
                if let Some(m) = l.mesh.only {
                    t += &format!(" - model {} ({}/{})", l.mesh.models[m], m + 1, l.mesh.models.len());
                }
                if let Some(a) = l.mesh.anime {
                    t += &format!(" - pose {} ({}/{})", l.mesh.animes[a], a + 1, l.mesh.animes.len());
                }
                if s.unposed > 0 {
                    t += &format!(" - {} unposed", s.unposed);
                }
                if s.missing_textures > 0 {
                    t += &format!(" - {} textures in other files", s.missing_textures);
                }
                t
            }
            None => format!("piney - ({}/{}) failed to load", self.current + 1, self.files.len()),
        };
        gpu.window.set_title(&title);
    }

    fn poll_gamepad(&mut self, dt: f32) {
        let Some(gilrs) = &mut self.gilrs else { return };
        let mut actions = Vec::new();
        while let Some(ev) = gilrs.next_event() {
            if let EventType::ButtonPressed(b, _) = ev.event {
                actions.push(b);
            }
        }
        // Held sticks and triggers, from the first connected pad.
        let (mut lx, mut ly, mut rx, mut ry, mut l2, mut r2) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        if let Some((_, pad)) = gilrs.gamepads().next() {
            let dz = |v: f32| if v.abs() < 0.12 { 0.0 } else { v };
            lx = dz(pad.value(Axis::LeftStickX));
            ly = dz(pad.value(Axis::LeftStickY));
            rx = dz(pad.value(Axis::RightStickX));
            ry = dz(pad.value(Axis::RightStickY));
            l2 = pad.button_data(Button::LeftTrigger2).map_or(0.0, |d| d.value());
            r2 = pad.button_data(Button::RightTrigger2).map_or(0.0, |d| d.value());
        }
        self.camera.orbit(-lx * 2.2 * dt, ly * 1.6 * dt);
        self.camera.pan(rx * 600.0 * dt, ry * 600.0 * dt);
        self.camera.zoom((1.0 + 1.5 * dt).powf(l2 - r2));
        for b in actions {
            match b {
                Button::RightTrigger | Button::DPadRight => self.step(1),
                Button::LeftTrigger | Button::DPadLeft => self.step(-1),
                Button::North => self.next_anime(),
                Button::West => self.next_model(),
                Button::South => self.reset_camera(),
                Button::East => self.fog = !self.fog,
                Button::Start => self.playing = !self.playing,
                _ => {}
            }
        }
    }
}

impl ApplicationHandler for Viewer {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("piney")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0));
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        let instance = self.instance.get_or_insert_with(|| {
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(Box::new(self.display.clone())))
        });
        match Gpu::new(instance, window.clone()) {
            Ok(gpu) => self.gpu = Some(gpu),
            Err(e) => {
                eprintln!("no GPU: {e}");
                event_loop.exit();
                return;
            }
        }
        if let (Some(gpu), Some(l)) = (&self.gpu, &mut self.loaded) {
            l.gpu = Some(gpu.renderer.upload(&l.mesh));
        }
        self.shown = None;
        self.update_title();
        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(g) = &mut self.gpu {
                    g.resize(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.logical_key.as_ref() {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Named(NamedKey::ArrowRight | NamedKey::PageDown) => self.step(1),
                    Key::Named(NamedKey::ArrowLeft | NamedKey::PageUp) => self.step(-1),
                    Key::Character(c) if c.eq_ignore_ascii_case("a") => self.next_anime(),
                    Key::Character(c) if c.eq_ignore_ascii_case("m") => self.next_model(),
                    Key::Character(c) if c.eq_ignore_ascii_case("b") => {
                        self.bright = if self.bright >= 4.0 { 1.0 } else { self.bright * 2.0 };
                    }
                    Key::Character(c) if c.eq_ignore_ascii_case("t") => {
                        self.placed_only = !self.placed_only;
                        self.load(self.current, self.anime, self.only);
                    }
                    Key::Character(c) if c.eq_ignore_ascii_case("r") => self.reset_camera(),
                    Key::Character(c) if c.eq_ignore_ascii_case("f") => self.fog = !self.fog,
                    Key::Character(c) if c.eq_ignore_ascii_case("p") => self.playing = !self.playing,
                    Key::Named(NamedKey::Space) => self.playing = !self.playing,
                    _ => {}
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.dragging = (state == ElementState::Pressed).then_some(button);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (dx, dy) = ((position.x - self.cursor.0) as f32, (position.y - self.cursor.1) as f32);
                self.cursor = (position.x, position.y);
                match self.dragging {
                    Some(MouseButton::Left) => self.camera.orbit(-dx * 0.008, dy * 0.008),
                    Some(MouseButton::Right | MouseButton::Middle) => self.camera.pan(dx, dy),
                    _ => {}
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                self.camera.zoom(0.88f32.powf(lines));
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last).as_secs_f32().min(0.1);
                self.last = now;
                self.poll_gamepad(dt);
                if self.playing {
                    self.clock += f64::from(dt);
                }
                self.animate();
                if let Some(gpu) = &mut self.gpu {
                    let vp = self.camera.view_proj(gpu.aspect());
                    let light = (self.camera.eye() - self.camera.target).normalize_or_zero();
                    let light = (light + Vec3::Z * 0.6).normalize_or_zero();
                    gpu.draw(
                        self.loaded.as_ref().and_then(|l| l.gpu.as_ref()),
                        vp,
                        light.extend(self.bright),
                        self.fog,
                    );
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(g) = &self.gpu {
            g.window.request_redraw();
        }
    }
}

/// Render `name` offscreen to a PNG.
/// What `--shot` renders.
struct Shot {
    out: String,
    anime: Option<usize>,
    only: Option<usize>,
    /// Yaw and pitch in degrees, and a distance factor.
    cam: Option<(f32, f32, f32)>,
    /// Where the camera looks.
    look: Option<Vec3>,
    placed_only: bool,
    size: (u32, u32),
    /// Exposure: 1 draws what the GS would write.
    bright: f32,
    /// Draw the scene's fog, when it has one.
    fog: bool,
    /// Seconds of play to pose what plays at.
    time: Option<f64>,
}

fn shot(archive: &Archive, name: &str, opts: &Shot) -> Result<(), String> {
    let data = archive.inflate_named(name).map_err(|e| e.to_string())?;
    let (cname, mut mesh) =
        build_scene(archive, data, opts.anime, opts.only, opts.placed_only).map_err(|e| e.to_string())?;
    if let Some(t) = opts.time {
        mesh.pose((t * mesh::FIELD_FPS) as u64);
    }
    shot_mesh(&cname, &mesh, opts)
}

/// Render `mesh` offscreen from its starting camera (or `opts.cam`) to a PNG.
fn shot_mesh(cname: &str, mesh: &Mesh, opts: &Shot) -> Result<(), String> {
    let &Shot { ref out, cam, size: (w, h), .. } = opts;
    let renderer = render::headless()?;
    let scene = renderer.upload(mesh);
    let mut camera = Camera::frame(mesh);
    if let Some(at) = opts.look {
        camera.target = at;
    }
    if let Some((yaw, pitch, zoom)) = cam {
        camera.yaw = yaw.to_radians();
        camera.pitch = pitch.to_radians().clamp(-1.55, 1.55);
        camera.distance *= zoom;
    }
    let light = ((camera.eye() - camera.target).normalize_or_zero() + Vec3::Z * 0.6).normalize_or_zero();
    let vp = camera.view_proj(w as f32 / h as f32);
    let rgba = renderer.snapshot(Some(&scene), vp, light.extend(opts.bright), opts.fog, w, h);
    std::fs::write(out, png::encode(w, h, &rgba)).map_err(|e| e.to_string())?;
    let s = &mesh.stats;
    println!(
        "{}: {} models, {} mmats, {} vertices, {} triangles, {} textures{} -> {out}",
        cname,
        s.models,
        s.mmats,
        s.vertices,
        s.triangles,
        mesh.textures.len(),
        mesh.anime.map(|a| format!(", pose {}", mesh.animes[a])).unwrap_or_default()
            + &mesh.only.map(|m| format!(", model {}", mesh.models[m])).unwrap_or_default()
            + &mesh.table.as_ref().map(|t| format!(", {} placed by {t}", s.placed)).unwrap_or_default()
            + &if mesh.companions.is_empty() {
                String::new()
            } else {
                format!(", with {}", mesh.companions.join(", "))
            }
            + &mesh.label.as_ref().map(|l| format!(", {l}")).unwrap_or_default()
    );
    Ok(())
}

fn main() {
    let mut iso = PathBuf::from("work/infection/infection.iso");
    let mut start: Option<String> = None;
    let mut shot_out: Option<String> = None;
    let mut anime: Option<usize> = None;
    let mut only: Option<usize> = None;
    let mut cam: Option<(f32, f32, f32)> = None;
    let mut look: Option<Vec3> = None;
    let mut placed_only = false;
    let mut size = (1280u32, 800u32);
    let mut dungeon_args: Option<(u32, u8, usize, u32)> = None;
    let mut field_args: Option<field::Params> = None;
    let mut floor = 0usize;
    let mut bright = 1.0f32;
    let mut fog = false;
    let mut time: Option<f64> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--iso" => iso = args.next().map(PathBuf::from).unwrap_or(iso),
            "--shot" => shot_out = args.next(),
            "--anime" => anime = args.next().and_then(|n| n.parse().ok()),
            "--model" => only = args.next().and_then(|n| n.parse().ok()),
            "--placed-only" => placed_only = true,
            "--bright" => bright = args.next().and_then(|n| n.parse().ok()).unwrap_or(1.0),
            "--fog" => fog = true,
            "--time" => time = args.next().and_then(|n| n.parse().ok()),
            "--field" => {
                let v: Vec<u32> = args.next().unwrap_or_default().split(',').filter_map(|x| x.parse().ok()).collect();
                if let Some(&seed) = v.first() {
                    let mut ft = v.get(1).copied().unwrap_or(0) % field::FIELD_TYPES as u32;
                    if field::INF.ccs[ft as usize].is_none() {
                        ft = next_field_type(ft);
                    }
                    let weather = v.get(2).copied().unwrap_or(0).min(field::WEATHERS as u32 - 1);
                    field_args = Some(field_params(seed, ft, weather));
                }
            }
            "--dungeon" => {
                let v: Vec<u32> = args.next().unwrap_or_default().split(',').filter_map(|x| x.parse().ok()).collect();
                if let Some(&seed) = v.first() {
                    let dtype = v.get(1).copied().unwrap_or(0).min(9) as u8;
                    let server = v.get(3).copied().unwrap_or(0).min(4);
                    dungeon_args = Some((seed, dtype, v.get(2).copied().unwrap_or(5) as usize, server));
                }
            }
            "--floor" => floor = args.next().and_then(|n| n.parse::<usize>().ok()).unwrap_or(1).saturating_sub(1),
            "--cam" => {
                let v: Vec<f32> = args.next().unwrap_or_default().split(',').filter_map(|x| x.parse().ok()).collect();
                if let [yaw, pitch, zoom] = v[..] {
                    cam = Some((yaw, pitch, zoom));
                }
            }
            "--look" => {
                let v: Vec<f32> = args.next().unwrap_or_default().split(',').filter_map(|x| x.parse().ok()).collect();
                if let [x, y, z] = v[..] {
                    look = Some(Vec3::new(x, y, z));
                }
            }
            "--size" => {
                if let Some((w, h)) = args.next().as_deref().and_then(|s| s.split_once('x')) {
                    size = (w.parse().unwrap_or(size.0), h.parse().unwrap_or(size.1));
                }
            }
            "-h" | "--help" => {
                println!("piney-viewer [--iso PATH] [NAME]");
                println!("piney-viewer [--iso PATH] --dungeon SEED[,TYPE[,SIZE[,SERVER]]] [--floor N] [--fog]");
                println!("piney-viewer [--iso PATH] --field SEED[,TYPE[,WEATHER]] [--fog]");
                println!(
                    "piney-viewer [--iso PATH] --shot OUT.png [--anime N] [--model N] [--time SECONDS] [--cam YAW,PITCH,ZOOM] [--look X,Y,Z] [--size WxH] NAME"
                );
                return;
            }
            _ => start = Some(a.to_ascii_lowercase()),
        }
    }
    let data = match Iso::open(&iso).and_then(|mut i| i.read_path("DATA/DATA.BIN")) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {e}", iso.display());
            std::process::exit(1);
        }
    };
    let archive = Archive::new(data).expect("DATA.BIN");
    let dungeon_view = match dungeon_args {
        Some((seed, dtype, dsize, server)) => {
            match DungeonView::generate(&archive, dungeon_params(seed, dtype, dsize, server), floor) {
                Ok(v) => Some(v),
                Err(e) => {
                    eprintln!("dungeon seed {seed} type {dtype}: {e}");
                    std::process::exit(1);
                }
            }
        }
        None => None,
    };
    let field_view = match field_args {
        Some(p) => match FieldView::generate(p) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("field seed {} type {}: {e}", p.seed, p.field_type);
                std::process::exit(1);
            }
        },
        None => None,
    };
    if let (Some(out), Some(view)) = (&shot_out, &field_view) {
        let opts = Shot { out: out.clone(), anime, only, cam, look, placed_only, size, bright, fog, time };
        let mesh = view.mesh(&archive).map(|mut m| {
            if let Some(t) = time {
                m.pose((t * mesh::FIELD_FPS) as u64);
            }
            m
        });
        if let Err(e) = mesh.and_then(|m| shot_mesh("field", &m, &opts)) {
            eprintln!("field: {e}");
            std::process::exit(1);
        }
        return;
    }
    if let (Some(out), Some(view)) = (&shot_out, &dungeon_view) {
        let opts = Shot { out: out.clone(), anime, only, cam, look, placed_only, size, bright, fog, time };
        let mesh = view.mesh().map_err(|e| e.to_string());
        if let Err(e) = mesh.and_then(|m| shot_mesh(&view.ccs.name, &m, &opts)) {
            eprintln!("dungeon: {e}");
            std::process::exit(1);
        }
        return;
    }
    if let Some(out) = shot_out {
        let Some(name) = start else {
            eprintln!("--shot needs a NAME");
            std::process::exit(2);
        };
        let opts = Shot { out, anime, only, cam, look, placed_only, size, bright, fog, time };
        if let Err(e) = shot(&archive, &name, &opts) {
            eprintln!("{name}: {e}");
            std::process::exit(1);
        }
        return;
    }
    let t = Instant::now();
    let files: Vec<usize> = archive
        .members()
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            archive
                .inflate(m)
                .and_then(Ccs::parse)
                .is_ok_and(|c| c.walk().chunks.iter().any(|ch| !ch.in_frames && ch.kind == ccs::MODEL))
        })
        .map(|(i, _)| i)
        .collect();
    eprintln!("{} of {} files have models ({:.1} s)", files.len(), archive.members().len(), t.elapsed().as_secs_f32());
    if files.is_empty() {
        eprintln!("nothing to show");
        std::process::exit(1);
    }
    let want = start.unwrap_or_else(|| "town01".into());
    let first =
        files.iter().position(|&i| archive.members()[i].name.to_ascii_lowercase().contains(want.as_str())).unwrap_or(0);

    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let gilrs = Gilrs::new().map_err(|e| eprintln!("no gamepad support: {e}")).ok();
    let mut viewer = Viewer {
        archive,
        files,
        current: 0,
        anime: None,
        only: None,
        placed_only: false,
        bright,
        fog,
        playing: true,
        clock: 0.0,
        shown: None,
        dungeon: dungeon_view,
        field: field_view,
        loaded: None,
        camera: Camera { target: Vec3::ZERO, yaw: 0.0, pitch: 0.0, distance: 1.0, radius: 1.0, extent: 1.0 },
        instance: None,
        display: event_loop.owned_display_handle(),
        gpu: None,
        gilrs,
        last: Instant::now(),
        cursor: (0.0, 0.0),
        dragging: None,
    };
    if viewer.field.is_some() {
        viewer.load_field();
    } else if viewer.dungeon.is_some() {
        viewer.load_dungeon();
    } else {
        viewer.load(first, None, None);
    }
    if let Err(e) = event_loop.run_app(&mut viewer) {
        eprintln!("{e}");
    }
}
