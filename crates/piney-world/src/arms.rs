//! A party character's weapon trails and points (gcmn spcchar.cpp):
//! `ccSpcChar::EquipWeapon` (0x0059d650) finds the weapon file's dummies and
//! makes the job's `ccLattice`s (job table 0x006f09e0); `ArmsEffect`
//! (0x0059ddd0) sets the points from the hands' matrices each frame (job table
//! 0x006f0a00) and lays the rows; `ClearArmsEffect` (0x0059e3b0, table
//! 0x006f0a20) and `_SetArmsEffectColor` (0x0059e4b0, table 0x006f0a40). A
//! dummy the file lacks reads the hand's origin here; the game reads address
//! 0x10. The per-job layout is in docs/engine/battle.md.

use std::collections::HashMap;

use glam::{Mat4, Vec4};

use crate::body::Body;
use crate::ee::{F, ONE, V4};
use crate::lattice::{Lattice, StripVertex};

/// The weapon's dummies, `DMY_xdummy_w01` to `_w04`.
pub const DUMMIES: [&str; 4] = ["DMY_xdummy_w01", "DMY_xdummy_w02", "DMY_xdummy_w03", "DMY_xdummy_w04"];
/// `ccPlayer::ccPlayer`'s hands: +0x130 and +0x12c.
pub const R_HAND: &str = "OBJ_t0 r hand";
pub const L_HAND: &str = "OBJ_t0 l hand";

/// A `ccSpcChar`'s weapon state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Arms {
    /// `spcParam` +0xd8 as `EquipWeapon` last saw it.
    pub job: i16,
    /// +0x140..+0x14c: the dummies' places in the weapon's space, None
    /// where `EquipWeapon` set none.
    pub dummies: [Option<Vec4>; 4],
    /// +0x150 and +0x154.
    pub a: Option<Lattice>,
    pub b: Option<Lattice>,
    /// +0x160..+0x190 `weaponEffPos`: where the weapon's particles go.
    pub points: [V4; 4],
    /// What the lattices' `Disp` sent this frame, with their sort keys.
    pub strips: Vec<(Vec<StripVertex>, F)>,
}

impl Arms {
    /// `EquipWeapon`'s dummies and lattices for `job`, from the file of
    /// the model hung on `body`'s hands (none: no dummies).
    pub fn equip(&mut self, body: &Body, job: i16) {
        self.job = job;
        let hands = [body.node(R_HAND), body.node(L_HAND)];
        let file = body.attached.iter().find(|a| hands.contains(&Some(a.node))).map(|a| a.file.clone());
        let dummy = |k: usize| -> Option<Vec4> {
            let f = file.as_ref()?;
            let d = f.scene.dummies.get(&f.ccs.find_object(DUMMIES[k])?)?;
            Some(Vec4::new(d.pos.x, d.pos.y, d.pos.z, 1.0))
        };
        let (order, a, b): (&[usize], Option<usize>, Option<usize>) = match job {
            0 => (&[0, 1, 2, 3], Some(2), Some(2)),
            1 | 2 | 5 => (&[0, 1], None, Some(2)),
            3 => (&[0, 1], Some(2), None),
            4 => (&[1, 0, 2], None, Some(3)),
            _ => return,
        };
        for (slot, &k) in order.iter().enumerate() {
            self.dummies[slot] = dummy(k);
        }
        if let Some(cols) = a {
            self.a.get_or_insert_with(|| Lattice::new(cols, 0));
        }
        if let Some(cols) = b {
            self.b.get_or_insert_with(|| Lattice::new(cols, 0));
        }
    }

    /// The lattices the job uses (`ClearArmsEffect`'s and
    /// `_SetArmsEffectColor`'s tables).
    fn used(&mut self) -> [Option<&mut Lattice>; 2] {
        match self.job {
            0 => [self.a.as_mut(), self.b.as_mut()],
            3 => [self.a.as_mut(), None],
            1 | 2 | 4 | 5 => [self.b.as_mut(), None],
            _ => [None, None],
        }
    }

    /// `ClearArmsEffect()`: the trails emptied at the next `ArmsEffect`.
    pub fn clear(&mut self) {
        if self.dummies.iter().all(Option::is_none) {
            return;
        }
        for l in self.used().into_iter().flatten() {
            l.clear = true;
        }
    }

    /// `_SetArmsEffectColor(t)`: the trails' colour type.
    pub fn set_color(&mut self, t: i32) {
        for l in self.used().into_iter().flatten() {
            l.ty = t;
        }
    }

    /// `ArmsEffect()` with the hands' world matrices `r` and `l` this
    /// frame, the weapon's aura (`ccEquipmentParam` +8, -1 none), whether
    /// `trajectorySW` is on and the character's skill (+0x7c).
    pub fn effect(&mut self, r: Mat4, l: Mat4, aura: i16, trajectory: bool, skill_id: i16) {
        self.strips.clear();
        if self.dummies.iter().all(Option::is_none) {
            return;
        }
        let at = |m: Mat4, d: Option<Vec4>| -> V4 {
            let p = m * d.unwrap_or(Vec4::W);
            [p.x.to_bits(), p.y.to_bits(), p.z.to_bits(), ONE]
        };
        let d = self.dummies;
        // Each lattice's rows: (which, the points by column).
        let rows: Vec<(bool, Vec<usize>)> = match self.job {
            0 => {
                self.points = [at(r, d[0]), at(l, d[1]), at(r, d[2]), at(l, d[3])];
                vec![(true, vec![0, 2]), (false, vec![1, 3])]
            }
            4 => {
                (self.points[0], self.points[1], self.points[2]) = (at(r, d[0]), at(r, d[1]), at(r, d[2]));
                vec![(true, vec![1, 0, 2])]
            }
            1 | 2 | 5 => {
                (self.points[0], self.points[1]) = (at(r, d[0]), at(r, d[1]));
                vec![(true, vec![1, 0])]
            }
            3 => {
                (self.points[0], self.points[1]) = (at(l, d[0]), at(l, d[1]));
                vec![(false, vec![1, 0])]
            }
            _ => return,
        };
        let lay = trajectory || aura != -1;
        let colour = aura != -1 && (!trajectory || skill_id == 1);
        let points = self.points;
        for (on_b, cols) in &rows {
            let Some(lat) = (if *on_b { self.b.as_mut() } else { self.a.as_mut() }) else { continue };
            lat.clear_cnt();
            if lay {
                lat.next_vertex();
                for (c, &p) in cols.iter().enumerate() {
                    lat.set_pos(points[p], c);
                }
            }
        }
        if colour {
            self.set_color(i32::from(aura));
        }
        for (on_b, _) in &rows {
            let Some(lat) = (if *on_b { self.b.as_mut() } else { self.a.as_mut() }) else { continue };
            let strip = lat.disp();
            if !strip.0.is_empty() {
                self.strips.push(strip);
            }
        }
    }

    /// The hands' world matrices of `body` posed as `worlds` holds it
    /// (`root` for a hand nothing poses): right, left.
    pub fn hands(body: &Body, worlds: &HashMap<u32, Mat4>, root: Mat4) -> (Mat4, Mat4) {
        let hand = |name: &str| body.node(name).and_then(|n| worlds.get(&n)).copied().unwrap_or(root);
        (hand(R_HAND), hand(L_HAND))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::BattleData;

    const ISO: &str = "../../work/infection/infection.iso";

    /// Kite's blades (job 0): `EquipWeapon` finds all four dummies and
    /// makes both trails. A swing's trails come once 16 rows have been
    /// laid, each a strip of its row pairs in the aura's colour or
    /// white; `ClearArmsEffect` empties them at the next `ArmsEffect`.
    /// Every weapon's aura is -1 or a colour of the table.
    #[test]
    fn kites_blades_leave_trails() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut iso = piney_data::iso::Iso::open(ISO).unwrap();
        let data = BattleData::read(&mut iso).unwrap();
        for (job, w) in data.t.weapons.iter().enumerate() {
            for e in w {
                assert!((-1..8).contains(&e.aura_sw), "job {job} {:?}: aura {}", e.ccsname, e.aura_sw);
            }
        }
        let kite = crate::chara::Kite::read(&archive).unwrap();
        let mut arms = Arms::default();
        arms.equip(&kite.body, 0);
        assert!(arms.dummies.iter().all(Option::is_some), "{:?}", arms.dummies);
        assert!(arms.a.is_some() && arms.b.is_some());
        let (r, l) = (Mat4::from_translation(glam::Vec3::X * 20.0), Mat4::from_translation(-glam::Vec3::X * 20.0));
        // No aura, no swing: nothing laid.
        arms.effect(r, l, -1, false, 0);
        assert_eq!(arms.b.as_ref().unwrap().made, 0);
        for f in 0..16 {
            arms.effect(r, l, -1, true, 1);
            assert_eq!(arms.strips.is_empty(), f < 15, "frame {f}");
        }
        assert_eq!(arms.strips.len(), 2);
        // Row pairs from the head to the tail (7 rows live), white.
        assert!(arms.strips.iter().all(|(s, _)| s.len() >= 4 && s.len() % 2 == 0 && s[0].rgba[..3] == [0xf0; 3]));
        // P0 the right hand's first dummy, P1 the left's second.
        let (p0, p1) = (r * arms.dummies[0].unwrap(), l * arms.dummies[1].unwrap());
        assert_eq!(arms.points[0], [p0.x.to_bits(), p0.y.to_bits(), p0.z.to_bits(), ONE]);
        assert_eq!(arms.points[1], [p1.x.to_bits(), p1.y.to_bits(), p1.z.to_bits(), ONE]);
        // An aura's colour on the normal attack.
        arms.effect(r, l, 2, true, 1);
        assert_eq!(arms.strips[0].0[0].rgba[..3], [0xf0, 0x80, 0]);
        arms.clear();
        arms.effect(r, l, -1, false, 0);
        assert!(arms.strips.is_empty());
        assert!(arms.b.as_ref().is_some_and(|b| b.head == b.tail && !b.clear));
    }
}
