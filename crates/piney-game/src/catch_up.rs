//! Issue #77: the game frames a redraw catches up are drawn only when
//! something sees them. A redraw presents its last frame alone, so a frame
//! stepped before it is drawn only when the next one samples its picture
//! ([`Frame::reads_previous`]). Drawing is all of a town frame's cost but
//! its step (worklog 552), so a redraw that fell behind no longer pays it
//! twice or more to catch up. The frames and the game are unchanged.

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_draw::Frame;

/// A frame stepped and not yet drawn, with the overlay it was stepped
/// under.
#[derive(Default)]
pub struct CatchUp {
    held: Option<(Frame, Option<Arc<Archive>>)>,
}

impl CatchUp {
    /// The next frame stepped: the one held drawn first when this one reads
    /// its picture, then this one held in its place.
    pub fn stepped(
        &mut self,
        frame: Frame,
        overlay: Option<Arc<Archive>>,
        draw: impl FnOnce(&Frame, Option<Arc<Archive>>),
    ) {
        if frame.reads_previous() {
            self.draw(draw);
        }
        self.held = Some((frame, overlay));
    }

    /// The held frame drawn: the redraw's picture, or the last game frame
    /// before something else draws over it.
    pub fn draw(&mut self, draw: impl FnOnce(&Frame, Option<Arc<Archive>>)) {
        if let Some((frame, overlay)) = self.held.take() {
            draw(&frame, overlay);
        }
    }
}

#[cfg(test)]
mod tests {
    use piney_draw::{Cmd, DrawState, Prim, PrimKind, Rgba, TexRef, TexState};

    use super::*;

    /// A frame told apart by its clear colour's red, sampling the frame
    /// before it or not.
    fn frame(id: u8, reads_previous: bool) -> Frame {
        let mut f = Frame::new();
        f.clear = Rgba::new(id, 0, 0, 0);
        if reads_previous {
            let state = DrawState {
                texture: Some(TexState {
                    tex: TexRef::PreviousFrame,
                    func: piney_draw::TexFunc::Modulate,
                    use_alpha: false,
                    filter: piney_draw::Filter::Nearest,
                    wrap: piney_draw::Wrap::Clamp,
                }),
                ..DrawState::sprite(piney_draw::Blend::MIX, None)
            };
            f.cmds.push(Cmd::Prim(Prim { kind: PrimKind::Sprite, gouraud: false, state, verts: Vec::new() }));
        }
        f
    }

    /// The frames of one redraw stepped in turn, then its picture: the ids
    /// drawn, in order.
    fn drawn(frames: &[(u8, bool)]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut c = CatchUp::default();
        for &(id, reads) in frames {
            c.stepped(frame(id, reads), None, |f, _| out.push(f.clear.0[0]));
        }
        c.draw(|f, _| out.push(f.clear.0[0]));
        out
    }

    #[test]
    fn a_redraw_draws_the_frames_it_shows_or_that_are_read() {
        assert_eq!(drawn(&[(1, false)]), [1]);
        // Caught up: only the last, the one presented.
        assert_eq!(drawn(&[(1, false), (2, false), (3, false)]), [3]);
        // A frame sampling the one before it gets that one drawn first.
        assert_eq!(drawn(&[(1, false), (2, true), (3, false)]), [1, 3]);
        assert_eq!(drawn(&[(1, false), (2, true), (3, true)]), [1, 2, 3]);
        // Nothing stepped, nothing drawn: the last picture stays.
        assert_eq!(drawn(&[]), Vec::<u8>::new());
    }
}
