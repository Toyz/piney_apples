//! `ccLayer` draw order (`docs/engine/desktop.md`, "Draw order"). Layers go
//! to the GS in ascending priority (`ccLayer::Add` 0x00108930, `AddAll`
//! 0x00108570); within a layer sprites and opaque model chains are prepended
//! (0x0015ac88, 0x0013f180: the last sent is drawn first), then the sorted
//! group in ascending key, the later of equal keys first (`ccDLSort::Add`
//! 0x001081d0). Of two layers of one priority the newer goes first, so
//! sysLayer draws after a field's objLayer (both 0): [`Layers::older`].

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

    /// Not the game's: how many groups each layer has in front now, for
    /// [`Layers::map_since`].
    pub fn mark(&self) -> Vec<(i16, usize)> {
        self.layers.iter().map(|(&p, l)| (p, l.front.len())).collect()
    }

    /// Not the game's: every command put in front of a layer since `mark`
    /// through `f` (the HUD scale's shrink of a part drawn whole, as the
    /// minimap).
    pub fn map_since(&mut self, mark: &[(i16, usize)], mut f: impl FnMut(&mut Cmd)) {
        for (p, l) in self.layers.iter_mut() {
            let from = mark.iter().find(|m| m.0 == *p).map_or(0, |m| m.1);
            for c in l.front.iter_mut().skip(from).flatten() {
                f(c);
            }
        }
    }

    /// A translucent model node for layer `pri`'s sorted group.
    pub fn sorted(&mut self, pri: i16, z: f32, cmd: Cmd) {
        self.layers.entry(pri).or_default().sorted.push((z, cmd));
    }

    /// Everything in GS order.
    pub fn flatten(self) -> Vec<Cmd> {
        self.flatten_with(|_, _, _| {})
    }

    /// [`Layers::flatten`], each command first through `each(priority,
    /// older, cmd)`: what a layer's view does to its packets (the game
    /// over's squeeze of the layers sharing sysLayer's view).
    pub fn flatten_with(mut self, mut each: impl FnMut(i16, bool, &mut Cmd)) -> Vec<Cmd> {
        for (pri, cmd) in self.shadows.flush().into_iter().rev() {
            self.layers.entry(pri).or_default().front.push(vec![cmd]);
        }
        let mut out = Vec::new();
        for (pri, mut layer) in self.layers {
            let from = out.len();
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
            let newer = out.len();
            for group in layer.older.into_iter().rev() {
                out.extend(group);
            }
            for (i, c) in out[from..].iter_mut().enumerate() {
                each(pri, from + i >= newer, c);
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

    /// `flatten_with` sees each command with its layer and whether it is
    /// the older layer's.
    #[test]
    fn flatten_with_names_each_layer() {
        let mut l = Layers::default();
        l.prepend(254, vec![tag(1)]);
        l.older(254, vec![tag(2)]);
        l.sorted(0, 1.0, tag(3));
        l.older(0, vec![tag(4)]);
        let mut seen = Vec::new();
        let cmds = l.flatten_with(|pri, older, c| {
            if let Cmd::Prim(p) = c {
                seen.push((pri, older, p.verts.len()));
            }
        });
        assert_eq!(tags(cmds), vec![3, 4, 1, 2]);
        assert_eq!(seen, vec![(0, false, 3), (0, true, 4), (254, false, 1), (254, true, 2)]);
    }
}
