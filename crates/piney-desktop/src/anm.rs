//! `ccAnm`: one Anime chunk played on its own copies of the objects it names,
//! drawn through the active layer's view (`docs/engine/animation.md`;
//! `ccAnm::Draw` 0x001524d0, `ccObj::Draw` 0x0013f220, `ccModel::Draw`
//! 0x0013eab0). Playback is `piney_data::anim`'s; this adds what the desktop
//! needs: the `F_Obj` records (`ccStream::DecodeF_Obj` 0x0014e950) and
//! `F_Camera` records, applied when a step crosses a whole frame, and the
//! draw with its per-mmat state.

use std::collections::HashMap;
use std::rc::Rc;

use glam::{Mat4, Vec3};
use piney_data::anim::Ticks;
use piney_draw::{
    AlphaFail, AlphaTest, Blend, Cmd, Compare, Depth, DrawState, Filter, Fog, Frame, MmatDraw, ModelDraw, Scissor,
    TexFunc, TexParams, Wrap, ZTest,
};

use crate::assets::{SceneFile, TEX_FLAG_CLAMP, TEX_FLAG_SORTED};
use crate::camera::Camera;
use crate::layers::{BACK_LAYER, Layers};
use crate::view::View;

/// `ccAnm.frameSpd` after the constructor: one frame a step.
pub const FRAME_SPD_DEFAULT: u32 = 256;

/// `ccObj::Draw` draws nothing at or below this transparency.
pub const MIN_TRANSPARENCY: f32 = 1.0 / 128.0;
/// `ccModel::Draw`: a mmat whose transparency reaches this is drawn
/// opaque (with t forced to 1).
pub const OPAQUE_TRANSPARENCY: f32 = 127.0 / 128.0;

/// One `F_Obj` record (0x0101, `CCSTRM_FSET_OBJ`, 52 bytes): an object's
/// pose from its frame on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FObj {
    pub pos: Vec3,
    /// Degrees.
    pub rot: Vec3,
    pub scale: Vec3,
    /// Clamped to 0..1.
    pub transparency: f32,
    /// `dispSW & 1`: drawn.
    pub disp: bool,
}

/// What `ccDrawEnv::SetFogBlend(amount, colour)` holds while the NEW mark
/// draws: a constant fog coefficient and colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FogBlend {
    /// `F` (the desktop passes 90.0, which the draw takes as F = 25; see
    /// `FOG_BLEND_F`).
    pub amount: f32,
    pub colour: [u8; 4],
}

/// The fog coefficient the NEW mark draws with (`SetFogBlend(90, ...)`,
/// F = 25; agent-traced, `ccSetFogBlendColor` 0x0013df20).
pub const FOG_BLEND_F: u8 = 25;

/// The draw context: the frame's layers and the state the game keeps in
/// `ccLayer::active` and `ccDrawEnv::active` while it draws.
pub struct Ctx {
    pub layers: Layers,
    /// The back layer's (126) view.
    pub view: View,
    pub fog_blend: Option<FogBlend>,
    pub uploads: Vec<piney_draw::Upload>,
}

impl Ctx {
    pub fn new(view: View) -> Self {
        Ctx { layers: Layers::default(), view, fog_blend: None, uploads: Vec::new() }
    }

    /// `ccKanji::Disp(str, -1)` on `k`, which sends at once: its texture as
    /// an upload, its runs at the front of `layer` through `view`.
    pub fn disp(
        &mut self,
        fonts: &crate::kanji::Fonts,
        k: &mut crate::kanji::Kanji,
        layer: i16,
        view: &crate::view::LayerView,
        s: &[u8],
        names: &crate::kanji::Names,
    ) {
        let quads = k.disp(fonts, s, -1, view, names);
        if quads.is_empty() {
            return;
        }
        let id = self.uploads.len() as u32;
        self.uploads.push(k.upload(id, fonts));
        let blend = Blend::TABLE[k.alpha_blend.min(8)];
        let prims = quads.iter().map(|q| Cmd::Prim(crate::kanji::quad_prim(q, view, id, blend))).collect();
        self.layers.prepend(layer, prims);
    }

    /// `ccKanji::Disp(str, count)`: the first `count` glyphs (-1 all).
    #[allow(clippy::too_many_arguments)]
    pub fn disp_count(
        &mut self,
        fonts: &crate::kanji::Fonts,
        k: &mut crate::kanji::Kanji,
        layer: i16,
        view: &crate::view::LayerView,
        s: &[u8],
        names: &crate::kanji::Names,
        count: i32,
    ) {
        let quads = k.disp(fonts, s, count, view, names);
        if quads.is_empty() {
            return;
        }
        let id = self.uploads.len() as u32;
        self.uploads.push(k.upload(id, fonts));
        let blend = Blend::TABLE[k.alpha_blend.min(8)];
        let prims = quads.iter().map(|q| Cmd::Prim(crate::kanji::quad_prim(q, view, id, blend))).collect();
        self.layers.prepend(layer, prims);
    }

    /// `ccView::SetView(active->view, anm->cam, 0)`.
    pub fn set_view(&mut self, camera_anm: &Anm) {
        if let Some(cam) = camera_anm.camera {
            self.view.set_camera(&cam);
        }
    }

    pub fn finish(self) -> Frame {
        let mut f = Frame::new();
        f.uploads = self.uploads;
        f.cmds = self.layers.flatten();
        f
    }
}

/// One object an animation draws.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Instance {
    /// The object whose model is drawn (ExtObj copies resolved).
    pub object: u32,
    pub model: u32,
    /// `lwMatrix`: model space to world.
    pub world: Mat4,
    /// `worldtp`: the object's own transparency (no object in the desktop's
    /// files inherits its parent's: every `Obj2.succession` is 0).
    pub alpha: f32,
}

/// Per object: its local matrix, transparency and display switch.
type Locals = HashMap<u32, (Mat4, f32, bool)>;

/// A `ccAnm`.
#[derive(Clone)]
pub struct Anm {
    file: Option<Rc<SceneFile>>,
    anim: usize,
    /// `frameNow * 256 + frameCnt`.
    time: Ticks,
    /// The time the controllers were last evaluated at.
    posed: Ticks,
    pub frame_spd: u32,
    /// Object -> its last F_Obj record.
    fobj: HashMap<u32, FObj>,
    /// `ccAnm.cam`, from the last F_Camera record applied.
    pub camera: Option<Camera>,
    /// The ccAnm's own coordinate: the parent of every object it draws.
    root: Mat4,
    /// `ccAnm.localtp`, which `ccAnm::Draw` hands `ccObj::Draw`.
    pub localtp: f32,
    /// Which member of its owner this is, for [`trace`].
    pub label: &'static str,
    /// `SetRenderState(CCRS_ZENABLE, on)` (main 0x00152b10) on every
    /// object: off, its models draw with no depth test.
    pub zenable: bool,
    /// An owner's writes to the animation's own objects (as
    /// `GetSubstAdrsF` hands them out), which win over the animation's:
    /// `dispSW` (+0xa2, drawn when 3) by object ...
    disp: HashMap<u32, bool>,
    /// ... and the local `matrix` (+0x40, with `matCalcSW` set).
    local: HashMap<u32, Mat4>,
}

/// A record of what the ccAnms did, for checking the desktop's logic
/// against the game's code run in eemu (`tools/test_desktop_rs.py`): while
/// a trace is on, `set`, `forward` and `draw` are recorded by label, and
/// `forward` returns the scripted result for its label instead of playing.
#[cfg(feature = "trace")]
pub mod trace {
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(Default)]
    pub struct Trace {
        pub calls: Vec<(&'static str, &'static str, String)>,
        pub forward: HashMap<&'static str, bool>,
    }

    thread_local! {
        pub static TRACE: RefCell<Option<Trace>> = const { RefCell::new(None) };
    }

    /// Record a call; returns the scripted forward result when tracing.
    pub fn record(op: &'static str, label: &'static str, arg: String) -> Option<bool> {
        TRACE.with(|t| {
            let mut t = t.borrow_mut();
            let t = t.as_mut()?;
            t.calls.push((op, label, arg));
            Some(t.forward.get(label).copied().unwrap_or(false))
        })
    }

    pub fn start(forward: HashMap<&'static str, bool>) {
        TRACE.with(|t| *t.borrow_mut() = Some(Trace { calls: Vec::new(), forward }));
    }

    pub fn stop() -> Vec<(&'static str, &'static str, String)> {
        TRACE.with(|t| t.borrow_mut().take().map(|t| t.calls).unwrap_or_default())
    }
}

impl Default for Anm {
    fn default() -> Self {
        Self::new()
    }
}

impl Anm {
    pub fn new() -> Self {
        Anm {
            file: None,
            anim: 0,
            time: 0,
            posed: 0,
            frame_spd: FRAME_SPD_DEFAULT,
            fobj: HashMap::new(),
            camera: None,
            root: Mat4::IDENTITY,
            localtp: 1.0,
            label: "",
            zenable: true,
            disp: HashMap::new(),
            local: HashMap::new(),
        }
    }

    /// An empty ccAnm named `label` for tracing.
    pub fn labelled(label: &'static str) -> Self {
        Anm { label, ..Anm::new() }
    }

    /// `ccAnm::SetAnm(ccsc, name, 0)` (0x00151c60): start `name` from frame
    /// 0; `frameSpd` is kept, the camera is cleared. An unknown name leaves
    /// the ccAnm empty.
    pub fn set(&mut self, file: &Rc<SceneFile>, name: &str) {
        #[cfg(feature = "trace")]
        trace::record("set", self.label, name.to_string());
        match file.anim(name) {
            Some(a) => {
                self.file = Some(file.clone());
                self.anim = a;
            }
            None => self.file = None,
        }
        self.time = 0;
        self.posed = 0;
        self.fobj.clear();
        self.camera = None;
        self.disp.clear();
        self.local.clear();
    }

    /// Whether an animation is set (`anmIndex != 0`).
    pub fn is_set(&self) -> bool {
        self.file.is_some()
    }

    pub fn frame_now(&self) -> u32 {
        self.time >> 8
    }

    pub fn time(&self) -> Ticks {
        self.time
    }

    /// The desktop's `if (anmIndex) r = _AnimateForward(frameSpd)`: one step
    /// (0x00152270). True when a play-once animation has ended.
    pub fn forward(&mut self) -> bool {
        #[cfg(feature = "trace")]
        if let Some(r) = trace::record("fwd", self.label, self.frame_spd.to_string()) {
            return r;
        }
        let Some(file) = self.file.clone() else { return false };
        let a = &file.anims[self.anim];
        let old_frame = self.time >> 8;
        let f = a.forward(self.time, self.frame_spd);
        if let Some(t) = f.pose_at {
            self.posed = t;
            let new_frame = t >> 8;
            if new_frame != old_frame {
                self.apply_frames(&file, old_frame + 1, new_frame);
            }
        }
        if a.looping && f.time == 0 && self.time != 0 {
            // DecodeFrameChunk hit the loop's end Top: every controller is
            // reset to the chunk's start.
            self.fobj.clear();
        }
        self.time = f.time;
        f.ended
    }

    /// The frame records of frames `from..=to` (`DecodeFrameChunk`).
    fn apply_frames(&mut self, file: &SceneFile, from: u32, to: u32) {
        for (frame, obj, rec) in crate::frames::fobj_records(file, self.anim) {
            if frame >= from && frame <= to {
                self.fobj.insert(obj, rec);
            }
        }
        for (frame, _, v) in crate::frames::camera_records(file, self.anim) {
            if frame >= from && frame <= to {
                self.camera = Some(Camera::from_record(&v));
            }
        }
    }

    /// `ccCoord::SetMatrix_PosRotXYZ(pos, rot)` on the ccAnm itself (the NEW
    /// mark): its objects hang under this.
    pub fn set_pos_rot(&mut self, pos: [f32; 3], rot: [f32; 3]) {
        self.root = Mat4::from_translation(Vec3::from(pos))
            * Mat4::from_rotation_z(rot[2])
            * Mat4::from_rotation_y(rot[1])
            * Mat4::from_rotation_x(rot[0]);
    }

    /// The objects the animation plays: its tracks', then those only its
    /// F_Obj records name.
    fn objects(&self) -> Vec<u32> {
        let Some(file) = self.file.as_ref() else { return Vec::new() };
        let mut out: Vec<u32> = Vec::new();
        for tr in &file.anims[self.anim].tracks {
            if !out.contains(&tr.object) {
                out.push(tr.object);
            }
        }
        for obj in crate::frames::fobj_objects(file, self.anim) {
            if !out.contains(&obj) {
                out.push(obj);
            }
        }
        out
    }

    /// `ccAnm::GetSubstAdrsF(name, 0)`: the animation's own object named
    /// `name`.
    pub fn object_named(&self, name: &str) -> Option<u32> {
        let file = self.file.as_ref()?;
        self.objects().into_iter().find(|&o| file.ccs.object_name(o) == Some(name))
    }

    /// `ccAnm::GetSubstAdrs("prefix*", ...)`: every object of the animation
    /// whose name starts with `prefix`.
    pub fn objects_prefixed(&self, prefix: &str) -> Vec<u32> {
        let Some(file) = self.file.as_ref() else { return Vec::new() };
        self.objects().into_iter().filter(|&o| file.ccs.object_name(o).is_some_and(|n| n.starts_with(prefix))).collect()
    }

    /// An owner's `obj->dispSW = on ? 3 : 0`, kept until the next `set`.
    pub fn set_disp(&mut self, obj: u32, on: bool) {
        self.disp.insert(obj, on);
    }

    /// An owner's `obj->matrix = m` with `matCalcSW = 1`, kept until the
    /// next `set`.
    pub fn set_local(&mut self, obj: u32, m: Mat4) {
        self.local.insert(obj, m);
    }

    /// `obj->matrix` as the animation last posed it (or the owner set it).
    pub fn local_of(&self, obj: u32) -> Mat4 {
        if let Some(m) = self.local.get(&obj) {
            return *m;
        }
        if let Some(rec) = self.fobj.get(&obj) {
            return fobj_matrix(rec);
        }
        let Some(file) = self.file.as_ref() else { return Mat4::IDENTITY };
        file.anims[self.anim]
            .controllers_at(self.posed)
            .into_iter()
            .find(|(o, _)| *o == obj)
            .map_or(Mat4::IDENTITY, |(_, p)| p.matrix())
    }

    /// Each drawn object: `dispSW == 3` and a model.
    pub fn instances(&self) -> Vec<Instance> {
        let Some(file) = self.file.as_ref() else { return Vec::new() };
        let a = &file.anims[self.anim];
        let sc = &file.scene;
        let mut locals: Locals = HashMap::new();
        let mut order: Vec<u32> = Vec::new();
        for (obj, pose) in a.controllers_at(self.posed) {
            if !locals.contains_key(&obj) {
                order.push(obj);
            }
            locals.insert(obj, (pose.matrix(), pose.alpha, true));
        }
        for obj in crate::frames::fobj_objects(file, self.anim) {
            // Named by an F_Obj record not yet reached: posed at rest and
            // shown (a new ccObj's dispSW is 3).
            if let std::collections::hash_map::Entry::Vacant(e) = locals.entry(obj) {
                e.insert((Mat4::IDENTITY, 1.0, true));
                order.push(obj);
            }
        }
        for (&obj, rec) in &self.fobj {
            locals.insert(obj, (fobj_matrix(rec), rec.transparency, rec.disp));
        }
        for (&obj, &m) in &self.local {
            if let Some(l) = locals.get_mut(&obj) {
                l.0 = m;
            }
        }
        for (&obj, &on) in &self.disp {
            if let Some(l) = locals.get_mut(&obj) {
                l.2 = on;
            }
        }
        fn world(sc: &piney_data::scene::Scene, l: &Locals, o: u32, d: u32) -> Mat4 {
            let m = l.get(&o).map_or(Mat4::IDENTITY, |x| x.0);
            let parent = sc.ext_parent.get(&o).or_else(|| sc.parent.get(&o)).copied().unwrap_or(0);
            // A parent the animation does not name is not in its index:
            // the object hangs from the ccAnm itself.
            if parent != 0 && parent != o && d < 64 && l.contains_key(&parent) {
                world(sc, l, parent, d + 1) * m
            } else {
                m
            }
        }
        let mut out = Vec::new();
        for obj in order {
            let (_, tp, disp) = locals[&obj];
            let target = sc.ext.get(&obj).copied().unwrap_or(obj);
            let Some(&model) = file.obj_model.get(&target) else { continue };
            if !disp {
                continue;
            }
            out.push(Instance { object: target, model, world: self.root * world(sc, &locals, obj, 0), alpha: tp });
        }
        out
    }

    /// `ccAnm::GetSubstAdrsF(name)`'s object: every object the animation
    /// poses, with its world matrix and transparency, model or not.
    pub fn instances_all(&self) -> Vec<(u32, Mat4, f32)> {
        let Some(file) = self.file.as_ref() else { return Vec::new() };
        let a = &file.anims[self.anim];
        let sc = &file.scene;
        let locals: Locals =
            a.controllers_at(self.posed).into_iter().map(|(o, p)| (o, (p.matrix(), p.alpha, true))).collect();
        let mut out = Vec::new();
        for (&obj, &(m, tp, _)) in &locals {
            let parent = sc.ext_parent.get(&obj).or_else(|| sc.parent.get(&obj)).copied().unwrap_or(0);
            let world = match locals.get(&parent) {
                Some(p) if parent != obj => p.0 * m,
                _ => m,
            };
            out.push((obj, self.root * world, tp));
        }
        out
    }

    /// The animated `ccMaterial::u/v` of the materials this animation has
    /// records for, as STROW values.
    fn uv_rows(&self, file: &SceneFile) -> HashMap<u32, [u8; 2]> {
        let a = &file.anims[self.anim];
        a.materials
            .iter()
            .map(|m| {
                let (cu, cv) = file.scene.materials.get(&m.material).map_or((0, 0), |mt| (mt.crop_u, mt.crop_v));
                let [u, v] = m.offsets(self.posed, cu, cv);
                (m.material, [(u >> 4) as u8, (v >> 4) as u8])
            })
            .collect()
    }

    /// `ccAnm::Draw` (0x001524d0) into the back layer.
    pub fn draw(&self, ctx: &mut Ctx) {
        #[cfg(feature = "trace")]
        if trace::record("draw", self.label, format!("{:?}", ctx.fog_blend.map(|f| f.colour))).is_some() {
            return;
        }
        let Some(file) = self.file.as_ref() else { return };
        let mut view = ctx.view.clone();
        if let Some(cam) = self.camera {
            view.set_camera(&cam);
        }
        let rows = self.uv_rows(file);
        for inst in self.instances() {
            draw_model(ctx, &view, file, &inst, self.localtp, &rows);
        }
    }

    /// `ccAnm::Draw` with `ccLayer::active` a layer of the caller's: its
    /// models on `layer`, through `view` (that layer's, as `SetView` last
    /// set it) unless the animation carries its own camera.
    pub fn draw_on(&self, ctx: &mut Ctx, layer: i16, view: &View) {
        let Some(file) = self.file.as_ref() else { return };
        let mut view = view.clone();
        if let Some(cam) = self.camera {
            view.set_camera(&cam);
        }
        let rows = self.uv_rows(file);
        for inst in self.instances() {
            draw_model_z(ctx, layer, &view, file, &inst, self.localtp, &rows, self.zenable);
        }
    }
}

/// An F_Obj record's local matrix: translate, then X, Y, Z turns, then
/// scale.
fn fobj_matrix(rec: &FObj) -> Mat4 {
    Mat4::from_translation(rec.pos)
        * Mat4::from_rotation_x(rec.rot.x.to_radians())
        * Mat4::from_rotation_y(rec.rot.y.to_radians())
        * Mat4::from_rotation_z(rec.rot.z.to_radians())
        * Mat4::from_scale(rec.scale)
}

/// `ccObj::Draw(tp)` and `ccModel::Draw`: the opaque mmats as one command
/// prepended to the back layer, the translucent ones as a node of its
/// sorted group. Each goes in reverse mmat order.
pub fn draw_model(
    ctx: &mut Ctx,
    view: &View,
    file: &SceneFile,
    inst: &Instance,
    localtp: f32,
    rows: &HashMap<u32, [u8; 2]>,
) {
    draw_model_on(ctx, BACK_LAYER, view, file, inst, localtp, rows);
}

/// [`draw_model`] on layer `layer`.
pub fn draw_model_on(
    ctx: &mut Ctx,
    layer: i16,
    view: &View,
    file: &SceneFile,
    inst: &Instance,
    localtp: f32,
    rows: &HashMap<u32, [u8; 2]>,
) {
    draw_model_z(ctx, layer, view, file, inst, localtp, rows, true);
}

/// [`draw_model_on`] with the depth test on or off (`TEST_1`'s ZTE, which
/// `ccObj::SetRenderState(CCRS_ZENABLE)` sets): off, every pixel passes.
#[allow(clippy::too_many_arguments)]
fn draw_model_z(
    ctx: &mut Ctx,
    layer: i16,
    view: &View,
    file: &SceneFile,
    inst: &Instance,
    localtp: f32,
    rows: &HashMap<u32, [u8; 2]>,
    zenable: bool,
) {
    let tp = localtp * inst.alpha;
    if tp < MIN_TRANSPARENCY {
        return;
    }
    let Some(info) = file.models.get(&inst.model) else { return };
    let (mut opaque, mut sorted) = (Vec::new(), Vec::new());
    for (i, mm) in info.mmats.iter().enumerate().rev() {
        let mut t = tp * mm.transparency;
        // A mmat below 1/128 is skipped (0x0013ecdc).
        if t < MIN_TRANSPARENCY {
            continue;
        }
        let translucent = t < OPAQUE_TRANSPARENCY || mm.tex_flag & TEX_FLAG_SORTED != 0 || info.blend_type != 0;
        if !translucent {
            t = 1.0;
        }
        let draw = MmatDraw {
            index: i as u32,
            alpha: t,
            alpha_ref: (f32::from(mm.aref) * t) as u8,
            wrap: if mm.tex_flag & TEX_FLAG_CLAMP != 0 { Wrap::Clamp } else { Wrap::Repeat },
            uv_row: mm.material.and_then(|m| rows.get(&m).copied()).unwrap_or([0, 0]),
        };
        if translucent { sorted.push(draw) } else { opaque.push(draw) }
    }
    let to_screen = view.to_screen(inst.world);
    let fog = ctx.fog_blend.map(|f| Fog { f: FOG_BLEND_F, colour: [f.colour[0], f.colour[1], f.colour[2]] });
    let make = |mmats: Vec<MmatDraw>| {
        Cmd::Model(Box::new(ModelDraw {
            file: file.stem.clone(),
            model: inst.model,
            to_screen: to_screen.to_cols_array_2d(),
            nodes: Vec::new(),
            mmats,
            state: {
                let mut st = model_state(info.blend_type);
                if !zenable {
                    st.depth.test = ZTest::Always;
                }
                st
            },
            tex: TexParams { func: TexFunc::Modulate, use_alpha: true, filter: Filter::Linear },
            clut_swaps: Vec::new(),
            tex_swaps: Vec::new(),
            fog,
            depth_fog: None,
            lights: None,
            morph: Vec::new(),
            reject_outside: true,
            div_z: piney_draw::DIV_Z,
            edits: None,
        }))
    };
    if !opaque.is_empty() {
        ctx.layers.prepend(layer, vec![make(opaque)]);
    }
    if !sorted.is_empty() {
        ctx.layers.sorted(layer, sort_key(&to_screen, info.centre), make(sorted));
    }
}

/// The key `ccModel::Draw` gives a model's node of the sorted group
/// (0x0013eb38..0x0013ebf0, passed to `ccDLSort::Add` at 0x0013f0c8): with a
/// bounding box, the screen Z of its centre; without one, the third of
/// `M[i][3] / M[3][3]` for the model's world-screen matrix M, which is the
/// w row (offsets 12, 28, 44 over 60), not the translation column.
pub fn sort_key(to_screen: &Mat4, centre: Option<Vec3>) -> f32 {
    match centre {
        Some(c) => {
            let q = *to_screen * c.extend(1.0);
            crate::eef::div(q.z, q.w)
        }
        None => crate::eef::div(to_screen.z_axis.w, to_screen.w_axis.w),
    }
}

/// The GS state `ccModel::Init` (0x0013a440) gives a model of
/// `blend_type` and `ccSetMaterialPacket` sends: ALPHA_1
/// `alphaBlendTbl[blend_type]` (0 `(Cs - Cd) As + Cd`, 1 additive, 2
/// subtractive, 3 `Cd As + Cs`); Z test GEQUAL. Type 0 alpha-tests GEQUAL
/// the mmat's reference, gating only the Z write (AFAIL FB_ONLY). The
/// others test NEVER with FB_ONLY (`SetRenderState(1, 0)`, 0x0013ae60):
/// their colour is always drawn, their Z never written.
pub fn model_state(blend_type: u8) -> DrawState {
    let method = if blend_type == 0 { Compare::GEqual } else { Compare::Never };
    DrawState {
        blend: Some(Blend::TABLE[usize::from(blend_type & 3)]),
        alpha_test: AlphaTest::On { method, reference: 0, fail: AlphaFail::FbOnly },
        depth: Depth { test: ZTest::GEqual, write: true },
        texture: None,
        scissor: Scissor::FULL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `alphaBlendTbl`'s first four entries, and the two tests
    /// `ccModel::Init` sets through `SetRenderState`.
    #[test]
    fn model_state_by_blend_type() {
        let regs: Vec<u64> = (0..4).map(|t| model_state(t).blend.unwrap().to_reg()).collect();
        assert_eq!(regs, [0x44, 0x48, 0x42, 0x09]);
        assert!(matches!(
            model_state(0).alpha_test,
            AlphaTest::On { method: Compare::GEqual, fail: AlphaFail::FbOnly, .. }
        ));
        for t in 1..4 {
            assert!(matches!(
                model_state(t).alpha_test,
                AlphaTest::On { method: Compare::Never, fail: AlphaFail::FbOnly, .. }
            ));
        }
    }
}
