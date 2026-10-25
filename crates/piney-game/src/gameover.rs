//! The game over after `ccThGameCtrl`'s signal (`docs/engine/field-walk.md`,
//! "The game over"): `ccThGameCtrl`'s second step (the sound's
//! `ccSndGameOver`, the task started), `ccThGameOver` (gcmn 0x00516a80)
//! with its `ccGameOverNoise` over the field ([`piney_world::gameover`]),
//! the field's tasks deleted once the noise is done, the "over" file's
//! `ANM_xdggame0` (GAME OVER) on its own layer, and `ccOpenGameOverMenu`'s
//! `ChangeRequest(1, 7)`: the soft reset to the title.

use std::rc::Rc;
use std::sync::Arc;

use glam::{Mat4, Vec4};
use piney_data::archive::Archive;
use piney_desktop::anm::{Anm, Ctx};
use piney_desktop::assets::SceneFile;
use piney_desktop::view::View;
use piney_draw::{Cmd, Frame, Rgba};
use piney_world::gameover::{GameOverNoise, ViewFrame};

use crate::field_host::ScFade;
use crate::mode::Event;

/// `ccThGameOver`'s first wait: 180 `Breath`s, then one more before the
/// noise's first `Main`.
const WAIT: u32 = 181;
/// Its wait after the files: 40 `Breath`s.
const HOLD: u32 = 40;
/// `ccThGameCtrl`'s `Breath(2)` between deleting the tasks and setting
/// `ccThGameOver` +0x14 to 2.
const TEAR_DOWN: u32 = 2;
/// The GAME OVER picture's file and clip, and its layer.
pub const OVER_FILE: &str = "over";
pub const OVER_ANM: &str = "ANM_xdggame0";
/// `ccLayer::Init(129, 0)`.
const OVER_LAYER: i16 = 129;
/// sysLayer: the noise's fader draws there.
const SYS_LAYER: i16 = 0;
/// `ccGame::ChangeRequest(1, 7)`: the mother task's soft reset.
pub const RESET: i32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// The frame of the signal (`ccThGameCtrl`'s first step, the host's).
    Signalled,
    /// The next frame: `ccThGameCtrl`'s second step.
    Start,
    /// `ccThGameOver` waiting: frames since it started.
    Wait(u32),
    /// `ccGameOverNoise::Main` each frame.
    Noise,
    /// The field's tasks gone (`ccDeleteAllThread`): frames of
    /// `ccThGameCtrl`'s `Breath(2)`, then of the hold.
    TornDown(u32),
    Hold(u32),
    /// The GAME OVER clip.
    Over,
    Done,
}

/// `ccThGameOver` and what `ccThGameCtrl` does around it.
pub struct GameOverTask {
    phase: Phase,
    noise: Option<GameOverNoise>,
    /// The noise's own `ccScFade` on sysLayer (its flashes).
    fade: ScFade,
    black: bool,
    over: Option<Anm>,
    archive: Option<Arc<Archive>>,
}

/// What a frame of the game over asks of the area.
pub struct Step {
    /// sysLayer's view as the noise left it, to squeeze the field's frame.
    pub view: Option<ViewFrame>,
    /// `ccSys.bgColor` 0.
    pub black: bool,
}

impl GameOverTask {
    pub fn new(archive: Option<Arc<Archive>>) -> GameOverTask {
        GameOverTask {
            phase: Phase::Signalled,
            noise: None,
            fade: ScFade::default(),
            black: false,
            over: None,
            archive,
        }
    }

    /// Whether the field's tasks are still there.
    pub fn field_runs(&self) -> bool {
        matches!(self.phase, Phase::Signalled | Phase::Start | Phase::Wait(_) | Phase::Noise)
    }

    /// One frame: `cc` is `ccRand`, `rand` newlib's; the noise's draws go
    /// into `ctx` (the field's frame, still being built), its sounds and
    /// the end into `events`.
    pub fn step(
        &mut self,
        cc: &mut dyn FnMut() -> i32,
        rand: &mut dyn FnMut() -> i32,
        ctx: &mut Ctx,
        events: &mut Vec<Event>,
    ) -> Step {
        match self.phase {
            // ccThGameCtrl's second step: CloseChat, ccChangeCmndTarget(0)
            // (the host's), ccSndGameOver, ccThGameOver started (its
            // ccGameOverNoise made, drawing the bands from rand).
            Phase::Signalled => self.phase = Phase::Start,
            Phase::Start => {
                events.push(Event::SoundGameOver);
                self.noise = Some(GameOverNoise::new(rand));
                self.phase = Phase::Wait(1);
            }
            Phase::Wait(n) => {
                self.phase = if n >= WAIT { Phase::Noise } else { Phase::Wait(n + 1) };
            }
            _ => {}
        }
        if self.phase == Phase::Noise
            && let Some(noise) = self.noise.as_mut()
        {
            let (active, out) = noise.main(cc, rand);
            ctx.layers.prepend(piney_desktop::noiz::NOIZ_LAYER, out.cmds);
            events.extend(out.se.into_iter().map(Event::Se));
            if out.flash {
                self.fade.flash(6, 0x80ff_ffff);
            }
            if out.send_fade {
                self.fade.send(ctx, SYS_LAYER);
            }
            self.black |= out.black;
            let view = noise.view;
            if !active {
                // Main's 0: +0x14 = 1; ccThGameCtrl deletes the tasks the
                // next frame.
                self.phase = Phase::TornDown(0);
            }
            return Step { view, black: self.black };
        }
        Step { view: None, black: self.black }
    }

    /// A frame after the field's tasks are gone: black, then after the hold
    /// the GAME OVER clip until it ends, then the reset.
    pub fn frame_alone(&mut self, events: &mut Vec<Event>) -> Frame {
        let mut ctx = Ctx::new(View::default());
        match self.phase {
            Phase::TornDown(n) => {
                self.phase = if n + 1 >= TEAR_DOWN { Phase::Hold(0) } else { Phase::TornDown(n + 1) };
            }
            Phase::Hold(n) => {
                if n + 1 >= HOLD {
                    // GetCCSAdrs("over"), the layer and the clip, fog off.
                    let file = self.archive.as_ref().and_then(|a| SceneFile::read(a, OVER_FILE).ok());
                    self.over = file.map(|f| {
                        let mut a = Anm::new();
                        a.set(&Rc::new(f), OVER_ANM);
                        a
                    });
                    self.phase = Phase::Over;
                } else {
                    self.phase = Phase::Hold(n + 1);
                }
            }
            Phase::Over => {
                let ended = match self.over.as_mut() {
                    Some(a) if a.is_set() => {
                        let ended = a.forward();
                        let mut layer = Ctx::new(View::default());
                        a.draw(&mut layer);
                        let f = layer.finish();
                        ctx.layers.prepend(OVER_LAYER, f.cmds);
                        ended
                    }
                    _ => true,
                };
                if ended {
                    self.phase = Phase::Done;
                    events.push(Event::ChangeMode { num: RESET, sf: 7 });
                }
            }
            _ => {}
        }
        let mut frame = ctx.finish();
        frame.clear = Rgba::BLACK;
        frame
    }
}

/// Whether layer `pri` (`older`: the older of its priority) draws
/// through sysLayer's view. These field layers have views of their own
/// (`ccLayer::Init(pri, 0)`, main 0x00108260): the maps (40, 45, 50), the
/// hack menu (127, 128), GAME OVER (129), the font (240), the menus and
/// event windows (242), the book's buttons (243) and `ccGame.layer` (254,
/// the older; a boss's newer 254 shares sysLayer's).
pub fn through_sys_view(pri: i16, older: bool) -> bool {
    !matches!((pri, older), (40 | 45 | 50 | 127..=129 | 240 | 242 | 243, _) | (254, true))
}

/// sysLayer's view squeezed as `ccView::SetFrame(x, y, w, h, cx, cy, ax,
/// ay)` leaves it: a point that the default frame (0, 0, 512, 384, 256,
/// 192, 1, 1) put at (px, py) now lands at (x + cx + (px - 256) ax, y + cy +
/// (py - 192) ay), and nothing is drawn outside the frame. Applied to a
/// command of a layer [`through_sys_view`], in frame-buffer pixels (the
/// view's 384 rows are the buffer's 448).
pub fn squeeze(c: &mut Cmd, v: &ViewFrame) {
    let s = f32::from(piney_draw::SCREEN_HEIGHT) / 384.0;
    let (bx, by) = (v.x + v.cx - 256.0 * v.ax, (v.y + v.cy) * s - 224.0 * v.ay);
    let m = Mat4::from_cols(
        Vec4::new(v.ax, 0.0, 0.0, 0.0),
        Vec4::new(0.0, v.ay, 0.0, 0.0),
        Vec4::Z,
        Vec4::new(bx, by, 0.0, 1.0),
    );
    let clip = |sc: &mut piney_draw::Scissor| {
        let (x0, x1) = (v.x.max(0.0) as u16, (v.x + v.w - 1.0).max(0.0) as u16);
        let (y0, y1) = ((v.y * s).round().max(0.0) as u16, ((v.y + v.h) * s).round().max(1.0) as u16 - 1);
        sc.x0 = sc.x0.max(x0);
        sc.x1 = sc.x1.min(x1);
        sc.y0 = sc.y0.max(y0);
        sc.y1 = sc.y1.min(y1);
    };
    match c {
        Cmd::Prim(p) => {
            for q in &mut p.verts {
                q.x = v.ax * q.x + bx;
                q.y = v.ay * q.y + by;
            }
            clip(&mut p.state.scissor);
        }
        Cmd::Model(md) => {
            md.to_screen = (m * Mat4::from_cols_array_2d(&md.to_screen)).to_cols_array_2d();
            for n in &mut md.nodes {
                *n = (m * Mat4::from_cols_array_2d(n)).to_cols_array_2d();
            }
            clip(&mut md.state.scissor);
        }
        // The buffer's cover moves with the view; its Z and counts are the
        // buffer's own.
        Cmd::Shadow(sh) => {
            let [x0, y0, x1, y1] = sh.rect;
            sh.rect = [v.ax * x0 + bx, v.ay * y0 + by, v.ax * x1 + bx, v.ay * y1 + by];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use piney_draw::{DrawState, Prim, PrimKind, Vertex};

    /// The picture squeezed to 48 rows about the middle: a sprite over the
    /// whole frame lands on the band (y 192 +- 24 view rows, 224 +- 28
    /// pixels), its scissor cut to it.
    #[test]
    fn a_squeeze_maps_the_frame_into_the_band() {
        let v = ViewFrame { x: 0.0, y: 168.0, w: 512.0, h: 48.0, cx: 256.0, cy: 24.0, ax: 1.0, ay: 48.0 / 384.0 };
        let mut f = Frame::new();
        let at = |x: f32, y: f32| Vertex { x, y, ..Default::default() };
        f.cmds.push(Cmd::Prim(Prim {
            kind: PrimKind::Sprite,
            gouraud: false,
            state: DrawState::sprite(piney_draw::Blend::MIX, None),
            verts: vec![at(0.0, 0.0), at(512.0, 448.0)],
        }));
        squeeze(&mut f.cmds[0], &v);
        let Cmd::Prim(p) = &f.cmds[0] else { panic!() };
        assert_eq!((p.verts[0].x, p.verts[0].y), (0.0, 196.0));
        assert_eq!((p.verts[1].x, p.verts[1].y), (512.0, 252.0));
        assert_eq!((p.state.scissor.y0, p.state.scissor.y1), (196, 251));
    }

    /// Only the layers sharing sysLayer's view squeeze: the field's,
    /// sysLayer, the noise's and a boss's 254; not the map, the menus, the
    /// font or `ccGame.layer`.
    #[test]
    fn the_squeeze_leaves_the_layers_with_views_of_their_own() {
        let v = ViewFrame { x: 0.0, y: 168.0, w: 512.0, h: 48.0, cx: 256.0, cy: 24.0, ax: 1.0, ay: 48.0 / 384.0 };
        let sprite = || {
            let at = |x: f32, y: f32| Vertex { x, y, ..Default::default() };
            Cmd::Prim(Prim {
                kind: PrimKind::Sprite,
                gouraud: false,
                state: DrawState::sprite(piney_draw::Blend::MIX, None),
                verts: vec![at(0.0, 0.0), at(512.0, 448.0)],
            })
        };
        let mut ctx = Ctx::new(View::default());
        let shared = [-100, 0, 10, 20, 60, 200, 241, 254];
        let own = [40, 45, 50, 127, 128, 129, 240, 242, 243];
        for p in shared.into_iter().chain(own) {
            ctx.layers.prepend(p, vec![sprite()]);
        }
        ctx.layers.older(0, vec![sprite()]);
        ctx.layers.older(254, vec![sprite()]);
        let mut seen = Vec::new();
        ctx.finish_with(|pri, older, c| {
            if through_sys_view(pri, older) {
                squeeze(c, &v);
            }
            let Cmd::Prim(p) = c else { panic!() };
            seen.push((pri, older, p.verts[0].y));
        });
        for (pri, older, y) in seen {
            let squeezed = shared.contains(&pri) && !(pri == 254 && older);
            assert_eq!(y, if squeezed { 196.0 } else { 0.0 }, "layer {pri} older {older}");
        }
    }
}
