//! Level up: `effLevelUp(ch)` (main 0x001cdde0), which `ccPlayer::Main`
//! (gcmn 0x00598404) and `ccFellow::Main` (0x0041b718) call when
//! `ccChar::CheckLevelUp` (0x0056d430) finds 1000 exp waiting and raises
//! the level (piney-battle's `exp` and `chara::check_level_up`).
//!
//! ```text
//! effLevelUp(ch)     effect 2 (PARTICLE's EFF_x035, the words LEVEL UP, a
//!                    sprite of kind 2): target ch, posT its pos, lifeTime
//!                    80, velocity 10, temp[0] 0, temp[1] 20 + its height;
//!                    no depth test (SetRenderState(0, 0)), layer effSBL
//!                    (priority 60)
//! first switch (0x001c60cc), each frame: hidden until cnt 50, then the
//!   words' bounce (crate::hit, ids 150-154) at scale 3, following ch while it is
//!   on the lists: posT its pos, the word at posT raised by temp[1], 1000
//!   from the camera
//! second chain (0x001c8d98): ends at once when ch is off the lists or down
//!   (condition.dead not 0 or 5); by cnt:
//!     0, 15, 25      particleGeneratorTbl[222], [223], [224] on ch's pos,
//!                    offset (0, 0, temp[1], 0)
//!     40             [225] and [226] on ch's pos
//!     50             ccSeOn3D(84, posT)
//! ```

use crate::ee;
use crate::effect::{EffectCtrl, Next, Obj};
use crate::hit::{self, EFF_SBL_LAYER};
use crate::{CharRef, Cx, Event, VecRef};

/// The words' bounce speed at the start: 10.
const VELOCITY: u32 = 0x4120_0000;

pub const LEVEL_UP: i16 = 2;
/// `ccSeOn3D(84, posT)` at count 50.
pub const SE_LEVEL_UP: i32 = 84;
/// The generators by count: rows of `particleGeneratorTbl` (0x003402f0)
/// and whether the offset is set.
pub const GENERATORS: [(i16, usize, bool); 5] =
    [(0, 222, true), (15, 223, true), (25, 224, true), (40, 225, false), (40, 226, false)];

/// `effLevelUp(ch)` (main 0x001cdde0).
pub fn eff_level_up(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef) -> Option<usize> {
    let i = ctrl.new_effect(cx, LEVEL_UP)?;
    let pos = cx.host.char_pos(ch);
    let h = cx.host.char_height(ch);
    let e = &mut ctrl.effects[i];
    e.target = Some(ch);
    e.pos_t = pos;
    e.life_time = 80;
    e.velocity = VELOCITY;
    e.temp[0] = 0;
    e.temp[1] = ee::add(0x41a0_0000, h);
    // ccEff::SetRenderState(0, 0): no depth test.
    if let Obj::Eff(eff) = &mut e.obj {
        eff.test &= !(1 << 16);
    }
    e.layer = Some(EFF_SBL_LAYER);
    Some(i)
}

/// Effect 2's case of the first switch (main 0x001c60cc): the words'
/// appearance (hidden until 50, then scale 3) and bounce, at posT raised by
/// temp[1], posT following ch while it is on the lists.
pub fn pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    let cam = cx.host.camera().cam_pos;
    let dist = cx.hits.panel_cam_dist;
    let now = ctrl.effects[i].target.filter(|&t| cx.host.check_target(t)).map(|t| cx.host.char_pos(t));
    let e = &mut ctrl.effects[i];
    if hit::appear_after(e, 50, 0x4040_0000) {
        if let Some(p) = now {
            e.pos_t = p;
        }
        let mut p = e.pos_t;
        p[2] = ee::add(p[2], e.temp[1]);
        hit::bounce(e, p, cam, dist);
    }
    Next::Draw
}

/// Effect 2's case of the second chain (main 0x001c8d98).
pub fn post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let e = &mut ctrl.effects[i];
    let Some(t) = e.target.filter(|&t| cx.host.check_target(t) && matches!(cx.host.char_dead(t), 0 | 5)) else {
        e.end_flag = true;
        return;
    };
    let (cnt, h, pos_t) = (e.cnt, e.temp[1], e.pos_t);
    for &(at, row, offset) in &GENERATORS {
        if cnt == at {
            let mut g = cx.particles.generator(cx.assets, row);
            g.sync_pos_type = false;
            g.sync_pos = Some(VecRef::CharPos(t));
            if offset {
                g.offset = [0, 0, h, 0];
            }
            cx.particles.start(g);
        }
    }
    if cnt == 50 {
        cx.events.push(Event::Sound3d { se: SE_LEVEL_UP, pos: pos_t });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::Camera;
    use crate::ee::{ONE, k};
    use crate::host::{CharState, Simple};

    #[test]
    fn a_level_up_sparkles_and_sounds_once() {
        let Some(mut fx) = crate::testing::effects() else { return };
        fx.ctrl.town = false;
        let mut r = || 0;
        let eye = [0, k(-600.0), k(300.0), ONE];
        let cam = Camera { eye, cam_pos: eye, cam_view: [0, 0, k(100.0), ONE], ..Camera::default() };
        let mut host = Simple::new(&mut r, [0, 0, 0, ONE], cam);
        host.chars.push(CharState { id: 7, pos: [0, 0, 0, ONE], dirc: [0; 4], height: k(160.0), width: k(45.0) });
        let i = fx.level_up(&mut host, 7).unwrap();
        assert_eq!(fx.ctrl.effects[i].temp[1], k(180.0));
        let (mut rows, mut events, mut frames) = (Vec::new(), Vec::new(), 0);
        while fx.ctrl.effects[i].status != 0 && frames < 100 {
            fx.step(&mut host);
            rows.extend(fx.particles.started.drain(..).map(|g| g.row));
            events.extend(fx.take_events());
            frames += 1;
        }
        assert_eq!(rows, [222, 223, 224, 225, 226]);
        assert_eq!(events, [Event::Sound3d { se: SE_LEVEL_UP, pos: [0, 0, 0, ONE] }]);
        assert!(frames < 100);
    }
}
