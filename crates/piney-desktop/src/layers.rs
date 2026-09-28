//! `ccLayer` draw order (`docs/engine/desktop.md`, "Draw order").
//!
//! - Layers go to the GS in ascending priority, so a higher priority is on
//!   top (`ccLayer::Add` 0x00108930 keeps the list sorted, `AddAll`
//!   0x00108570 splices each layer after the root).
//! - Within a layer, `ccSprite::SendPacket` and opaque model chains are
//!   **prepended** (0x0015ac88, 0x0013f180): the last sent is drawn first.
//! - A layer's sorted group (translucent mmats, one node per model, keyed
//!   as `anm::sort_key` says) follows in ascending key, the later of equal
//!   keys first (`ccDLSort::Add` 0x001081d0, `StoreDLNode` 0x00108100).
//! - Of two layers of one priority the newer goes first: `Add` puts a new
//!   layer after the equal ones in the root list (kept in descending
//!   priority) and `AddAll` reverses it. sysLayer, made at start-up, draws
//!   after a field's objLayer (both 0): [`Layers::older`].

use std::collections::BTreeMap;

use piney_draw::Cmd;

/// `Desktop_control`'s back layer: every `ccAnm` draws here.
pub const BACK_LAYER: i16 = 126;
/// `Desktop_control`'s kanji layer: text, cursors and panels.
pub const KANJI_LAYER: i16 = 127;
/// `MailList_control`'s reply window layers.
pub const RES_LAYER: i16 = 128;
pub const RESK_LAYER: i16 = 129;
/// The font layer, where `ccScFade` draws (`fontSend` 0x0015cb00).
pub const FONT_LAYER: i16 = 240;

#[derive(Default)]
struct Layer {
    /// Groups in call order; each group keeps its own order.
    front: Vec<Vec<Cmd>>,
    sorted: Vec<(f32, Cmd)>,
    /// An older layer of the same priority: groups in call order.
    older: Vec<Vec<Cmd>>,
}

/// The display lists of one frame.
#[derive(Default)]
pub struct Layers {
    layers: BTreeMap<i16, Layer>,
    /// The shadow packets and the draw environment's shadow: models cast
    /// into them as they draw, and each packet's pass joins its layer at
    /// [`Layers::flatten`] (`SetShadowPacketAll`, at the frame's end).
    pub shadows: crate::shadow::Shadows,
}

impl Layers {
    /// A packet group put at the front of layer `pri`'s list.
    pub fn prepend(&mut self, pri: i16, group: Vec<Cmd>) {
        if !group.is_empty() {
            self.layers.entry(pri).or_default().front.push(group);
        }
    }

    /// A packet group put at the front of the older layer of priority
    /// `pri` (sysLayer beside objLayer), which draws after everything
    /// [`Layers::prepend`] and [`Layers::sorted`] give that priority.
    pub fn older(&mut self, pri: i16, group: Vec<Cmd>) {
        if !group.is_empty() {
            self.layers.entry(pri).or_default().older.push(group);
        }
    }

    /// A translucent model node for layer `pri`'s sorted group.
    pub fn sorted(&mut self, pri: i16, z: f32, cmd: Cmd) {
        self.layers.entry(pri).or_default().sorted.push((z, cmd));
    }

    /// Everything in GS order.
    pub fn flatten(mut self) -> Vec<Cmd> {
        for (pri, cmd) in self.shadows.flush().into_iter().rev() {
            self.layers.entry(pri).or_default().front.push(vec![cmd]);
        }
        let mut out = Vec::new();
        for (_, mut layer) in self.layers {
            for group in layer.front.into_iter().rev() {
                out.extend(group);
            }
            // `ccDLSort::Add` (0x001081d0) builds a binary tree, an equal
            // key going left, read in order: ascending, the later of equal
            // keys first.
            // The keys compare as floats: -0 and 0 are equal.
            let key = |z: f32| if z == 0.0 { 0.0 } else { z };
            layer.sorted.reverse();
            layer.sorted.sort_by(|a, b| key(a.0).total_cmp(&key(b.0)));
            out.extend(layer.sorted.into_iter().map(|(_, c)| c));
            for group in layer.older.into_iter().rev() {
                out.extend(group);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use piney_draw::{DrawState, Prim, PrimKind};

    fn tag(n: usize) -> Cmd {
        Cmd::Prim(Prim {
            kind: PrimKind::Sprite,
            gouraud: false,
            state: DrawState::sprite(piney_draw::Blend::MIX, None),
            verts: vec![Default::default(); n],
        })
    }

    fn tags(cmds: Vec<Cmd>) -> Vec<usize> {
        cmds.iter()
            .map(|c| match c {
                Cmd::Prim(p) => p.verts.len(),
                Cmd::Model(_) | Cmd::Shadow(_) => 0,
            })
            .collect()
    }

    #[test]
    fn order_within_a_layer() {
        let mut l = Layers::default();
        l.sorted(1, 2.0, tag(1));
        l.prepend(1, vec![tag(2), tag(3)]);
        l.sorted(1, 1.0, tag(4));
        l.sorted(1, 2.0, tag(5));
        l.prepend(1, vec![tag(6)]);
        l.sorted(0, 9.0, tag(7));
        l.older(1, vec![tag(8)]);
        l.older(1, vec![tag(9)]);
        assert_eq!(tags(l.flatten()), vec![7, 6, 2, 3, 4, 5, 1, 9, 8]);
    }
}
