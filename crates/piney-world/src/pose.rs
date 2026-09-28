//! A `ccAnm` playing one Anime chunk of a scene file: its time, one
//! `_AnimateForward` a frame (`docs/engine/animation.md`), the notes a step
//! passes, and what it poses - object matrices through the Obj parents
//! under the anm's own root, texture offsets, morph weights.

use std::collections::HashMap;

use glam::Mat4;
use piney_data::anim::Ticks;
use piney_desktop::assets::SceneFile;

/// One animation's playback.
#[derive(Clone, Debug, PartialEq)]
pub struct Play {
    /// Index into [`SceneFile::anims`].
    pub anim: usize,
    /// `frameNow * 256 + frameCnt`.
    pub time: Ticks,
    /// The time the controllers were last evaluated at.
    pub posed: Ticks,
    pub frame_spd: u32,
}

impl Play {
    /// `ccAnm::SetAnm` of the named animation: frame 0.
    pub fn new(file: &SceneFile, name: &str) -> Option<Self> {
        Some(Play { anim: file.anim(name)?, time: 0, posed: 0, frame_spd: 256 })
    }

    /// `_AnimateForward(frameSpd)`: true when a play-once animation ends.
    pub fn forward(&mut self, file: &SceneFile) -> bool {
        let f = file.anims[self.anim].forward(self.time, self.frame_spd);
        if let Some(t) = f.pose_at {
            self.posed = t;
        }
        self.time = f.time;
        f.ended
    }

    /// [`Play::forward`], and the (event, param) of each note the step passed, in
    /// the order `ccAnm::NoteProcess` (main 0x00152210) hands them on
    /// ([`piney_data::anim::Animation::passed_notes`]). Empty when the frame did
    /// not change. The game keeps these on the anm (`noteRoot`) until its next
    /// `_AnimateForward`, so a caller that hands notes on after a clip change
    /// without a step in between should keep the last ones.
    pub fn forward_notes(&mut self, file: &SceneFile) -> (bool, Vec<(u32, u32)>) {
        let (f, notes) = file.anims[self.anim].forward_notes(self.time, self.frame_spd);
        if let Some(t) = f.pose_at {
            self.posed = t;
        }
        self.time = f.time;
        (f.ended, notes)
    }

    /// The frame whose records were last applied (morph weights).
    pub fn frame(&self) -> u32 {
        self.posed >> 8
    }

    /// World matrices of the objects the animation poses and of every
    /// object in `extra` (a clump's nodes), each object's local matrix its
    /// pose (the identity when not animated) under its Obj parent, and
    /// `root` over those whose parent is outside the set.
    pub fn worlds(&self, file: &SceneFile, root: Mat4, extra: &[u32]) -> HashMap<u32, Mat4> {
        let a = &file.anims[self.anim];
        let locals = a.locals_at(self.posed);
        let sc = &file.scene;
        let mut set: Vec<u32> = locals.keys().copied().collect();
        set.extend(extra.iter().copied());
        let mut out = HashMap::new();
        fn w(
            sc: &piney_data::scene::Scene,
            locals: &HashMap<u32, Mat4>,
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
            let parent = sc.ext_parent.get(&obj).or_else(|| sc.parent.get(&obj)).copied().unwrap_or(0);
            let m = if parent != 0 && parent != obj && depth < 64 && set.contains(&parent) {
                w(sc, locals, set, root, cache, parent, depth + 1) * local
            } else {
                root * local
            };
            cache.insert(obj, m);
            m
        }
        for &o in &set {
            w(sc, &locals, &set, root, &mut out, o, 0);
        }
        out
    }

    /// The animated `ccMaterial::u/v` as STROW values, by material.
    pub fn uv_rows(&self, file: &SceneFile) -> HashMap<u32, [u8; 2]> {
        file.anims[self.anim]
            .materials
            .iter()
            .map(|m| {
                let (cu, cv) = file.scene.materials.get(&m.material).map_or((0, 0), |mt| (mt.crop_u, mt.crop_v));
                let [u, v] = m.offsets(self.posed, cu, cv);
                (m.material, [(u >> 4) as u8, (v >> 4) as u8])
            })
            .collect()
    }

    /// The morph targets and weights of each morpher at the current frame.
    pub fn morph(&self, file: &SceneFile) -> Vec<(u32, Vec<(u32, f32)>)> {
        file.anims[self.anim].morph_weights_at(self.frame()).into_iter().map(|(m, t)| (m, t.to_vec())).collect()
    }
}
