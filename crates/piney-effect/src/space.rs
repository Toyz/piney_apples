//! The field's map wrap as the effects meet it: `ccPlayer::W2PPos`,
//! `ccPlayer::P2WPos` and `ccTransPosFW2LW` (gcmn 0x0059b9c0), which takes a
//! point to where it is drawn - relative to the player, wrapped into the
//! map, and back.

use crate::ee::{self, F, ONE, V4};

/// `WORLD_MAN` +0x420..+0x42c in a town: x and y from -24000 to 24000.
pub const TOWN_BOUNDS: [F; 4] = [0xc6bb_8000, 0xc6bb_8000, 0x46bb_8000, 0x46bb_8000];

/// `ccPlayer::W2PPos(pos)` (gcmn 0x0059b5a0): x and y less the player's,
/// wrapped into the map's span about its centre; z kept; w 1.
pub fn w2p(pos: V4, player: V4, b: [F; 4]) -> V4 {
    let wrap = |v: F, lo: F, hi: F| -> F {
        let half = ee::div(ee::add(lo, hi), 0x4000_0000);
        if ee::lt(v, ee::sub(lo, half)) {
            ee::add(v, ee::sub(hi, lo))
        } else if !ee::le(v, ee::sub(hi, half)) {
            ee::add(v, ee::sub(lo, hi))
        } else {
            v
        }
    };
    [wrap(ee::sub(pos[0], player[0]), b[0], b[2]), wrap(ee::sub(pos[1], player[1]), b[1], b[3]), pos[2], ONE]
}

/// `ccPlayer::P2WPos(pos)` (gcmn 0x0059b710): `pos` plus the player's
/// position (all four lanes), w 1, and x and y wrapped back by the map's
/// span when the player is on one side of its centre and the point past
/// the other; z the input's.
pub fn p2w(pos: V4, player: V4, b: [F; 4]) -> V4 {
    let mut out = ee::vadd(pos, player);
    out[3] = ONE;
    let back = |o: &mut F, p: F, pl: F, lo: F, hi: F| {
        let c = ee::div(ee::add(lo, hi), 0x4000_0000);
        if ee::lt(pl, c) {
            if !ee::le(p, ee::sub(hi, c)) {
                *o = ee::add(*o, ee::sub(lo, hi));
            }
        } else if ee::lt(p, ee::sub(lo, c)) {
            *o = ee::add(*o, ee::sub(hi, lo));
        }
    };
    back(&mut out[0], pos[0], player[0], b[0], b[2]);
    back(&mut out[1], pos[1], player[1], b[1], b[3]);
    out[2] = pos[2];
    out
}

/// `ccTransPosFW2LW(out, pos)`: `P2WPos(W2PPos(pos))` about the player.
pub fn fw2lw(pos: V4, player: V4, b: [F; 4]) -> V4 {
    p2w(w2p(pos, player, b), player, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_town_point_comes_back_where_it_was() {
        let k = |x: f32| x.to_bits();
        let player = [k(0.0), k(5600.0), k(600.0), ONE];
        let p = [k(12.5), k(6300.0), k(700.0), ONE];
        assert_eq!(fw2lw(p, player, TOWN_BOUNDS), p);
        // Across the edge: 23000 seen from -23000 is -3000 away.
        let far = [k(23000.0), 0, 0, ONE];
        let pl = [k(-23000.0), 0, 0, ONE];
        assert_eq!(ee::f(w2p(far, pl, TOWN_BOUNDS)[0]), -2000.0);
        assert_eq!(ee::f(fw2lw(far, pl, TOWN_BOUNDS)[0]), -25000.0);
    }
}
