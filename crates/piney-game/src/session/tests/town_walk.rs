//! Kite walked through a town as a player would: the way round the walls
//! planned each second (survey.rs's planner, [`super::survey::path_wide`]),
//! round the people standing about, and round a place he stopped at.

use piney_input::Raw;

/// A walk to within `near` of `to`.
pub(super) struct TownWalker {
    to: [f32; 2],
    near: f32,
    path: Vec<[f32; 2]>,
    mark: Option<[f32; 2]>,
    /// Places he stopped short of, and the frame: open again 600 on.
    avoid: Vec<([f32; 2], u32)>,
}

impl TownWalker {
    pub(super) fn new(to: [f32; 2], near: f32) -> Self {
        TownWalker { to, near, path: Vec::new(), mark: None, avoid: Vec::new() }
    }

    /// The stick for frame `f`, or None once he is within reach.
    pub(super) fn stick(&mut self, world: &piney_world::World, f: u32) -> Option<Raw> {
        let to = self.to;
        let p = world.player().body.pos.map(f32::from_bits);
        if (to[0] - p[0]).hypot(to[1] - p[1]) < self.near {
            return None;
        }
        if f.is_multiple_of(60) {
            if self.mark.is_some_and(|m| (m[0] - p[0]).hypot(m[1] - p[1]) < 60.0)
                && let Some(&q) = self.path.first()
            {
                self.avoid.push((q, f));
            }
            self.mark = Some([p[0], p[1]]);
            self.avoid.retain(|&(_, at)| f - at < 600);
            let mid = [(p[0] + to[0]) / 2.0, (p[1] + to[1]) / 2.0];
            let span = (to[0] - p[0]).abs().max((to[1] - p[1]).abs());
            let n = ((span + 4000.0) / 100.0) as i32 | 1;
            // The walking PCs and the merchants (but whom he walks to).
            let people = world.pcs().iter().filter(|c| c.disp_sw).map(|c| c.char.pos);
            let people = people.chain(world.merchants().iter().map(|m| m.ch.pos));
            let mut closed: Vec<[f32; 2]> = self.avoid.iter().map(|&(q, _)| q).collect();
            closed.extend(
                people
                    .map(|q| [f32::from_bits(q[0]), f32::from_bits(q[1])])
                    .filter(|q| (q[0] - to[0]).hypot(q[1] - to[1]) > 50.0),
            );
            let look = super::survey::Look { n, step: 100.0, wide: 75.0, heights: &[30.0, 95.0], avoid: &closed };
            let near = self.near - 10.0;
            let goal = |q: [f32; 2], _| (q[0] - to[0]).hypot(q[1] - to[1]) < near;
            self.path = super::survey::path_wide(&world.town().base.hits, [p[0], p[1], p[2]], mid, &look, goal);
        }
        while self.path.len() > 1 && (self.path[0][0] - p[0]).hypot(self.path[0][1] - p[1]) < 100.0 {
            self.path.remove(0);
        }
        let q = self.path.first().copied().unwrap_or(to);
        let cam_z = f32::from_bits(world.camera().rot()[2]);
        Some(super::stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1]))))
    }
}
