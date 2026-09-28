//! The enemies' feet: `ccEnemyDustCtrl` (gcmn dust.cpp 0x0043a8e0), the
//! controller a race's constructor makes over its `ccEnemyDustInfo` rows, run
//! from the race's `exclusive()` (flag 0) and on its notes 1 and 2 (flag 1)
//! ([`enemy_motion::Call::DustCtrl`]): each row raises dust rings in a window
//! of its animation's frames. It draws nothing random, so it runs where the
//! effects start the frame's shows (`Combat::fx_call`).
//! `tools/test_dust_ctrl_rs.py` runs the game's `ctrl` against [`ctrl`]
//! (docs/engine/effects.md).

use piney_battle::blocks::{self, InfoRef};
use piney_battle::enemy_motion::{Call, DustAt};
use piney_battle::entry;
use piney_data::tables::combat;

use super::Combat;
use super::stage::Show;
use crate::ee::V4;

impl Combat {
    /// The shows from `from` on, each `ccEnemyDustCtrl` constructor setting
    /// its nodes and each `ctrl` followed by the dust rings it raised.
    pub(super) fn dust_pass(&mut self, from: usize) {
        let any = self.shows[from..].iter().any(|s| {
            matches!(
                s,
                Show::Enemy(_, Call::DustCtrl { .. } | Call::FootDust { .. }) | Show::Entry(entry::Out::Dust { .. })
            )
        });
        if !any {
            return;
        }
        let tail: Vec<Show> = self.shows.drain(from..).collect();
        for s in tail {
            let rings = match &s {
                // The constructor: a node a row, its last frame 0.
                Show::Entry(entry::Out::Dust { who, n, .. }) => {
                    self.dust.insert(*who, vec![0; usize::try_from(*n).unwrap_or(0)]);
                    Vec::new()
                }
                Show::Enemy(who, Call::DustCtrl { flag, at }) => {
                    let rings = self.dust_ctrl(*who, *flag, at);
                    self.dust_rings += rings.len() as u64;
                    rings
                }
                Show::Enemy(who, Call::FootDust { foot, info, smoke }) => self.foot_dust(*who, *foot, *info, *smoke),
                _ => Vec::new(),
            };
            self.shows.push(s);
            self.shows.extend(rings);
        }
    }

    /// `ccEnemyE::note`'s ring at a turtle's foot: `footObjTbl[foot]`
    /// looked up in the clump, its place as the enemy was last drawn,
    /// `ccEnemyEffDustRing(place, info.s, info.r, info.n, info.life,
    /// eneSmoke)` by the `eet1DustInfo` row `info`; none without the node.
    fn foot_dust(&mut self, who: usize, foot: usize, info: InfoRef, smoke: i16) -> Vec<Show> {
        let volume = self.data.volume;
        let Some(name) = combat::of(volume).foot_objs().get(foot) else { return Vec::new() };
        let Some(row) = blocks::dust_rows(volume, info).first() else { return Vec::new() };
        let Some(m) = self.node_world(who, name) else { return Vec::new() };
        self.dust_rings += 1;
        vec![Show::Entry(entry::Out::DustRingAt {
            pos: m[3],
            s: row.scale.to_bits(),
            r: row.radius.to_bits(),
            n: i32::from(row.div),
            life: i32::from(row.life),
            tex: i32::from(smoke),
        })]
    }

    /// `ccEnemyDustCtrl::ctrl(obj, flag)` (gcmn 0x0043aa90): the rings.
    fn dust_ctrl(&mut self, who: usize, flag: i32, at: &DustAt) -> Vec<Show> {
        if !at.disp_sw {
            return Vec::new();
        }
        let rows = at.info.map(|info| blocks::dusts(self.data.volume, info, at.n)).unwrap_or_default();
        let Some(last) = self.dust.get_mut(&who) else { return Vec::new() };
        ctrl(&rows, last, flag, at)
            .into_iter()
            .map(|r| Show::DustRing {
                pos: at.pos,
                dirc: at.dirc,
                ofs: r.ofs,
                s: r.s,
                r: r.r,
                n: r.n,
                life: r.life,
                tex: i32::from(at.smoke),
            })
            .collect()
    }
}

/// A ring `ccEnemyDustCtrl::ctrl` raises: its row's `ofs`, `s`, `r`, `n`
/// and `life` (the texture is the enemy's `eneSmoke`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ring {
    pub row: usize,
    pub ofs: V4,
    pub s: u32,
    pub r: u32,
    pub n: i32,
    pub life: i32,
}

/// `ccEnemyDustCtrl::ctrl(obj, flag)` over the controller's rows and its
/// nodes' last frames (both in the rows' order), `dispSW` aside: the rings
/// in order.
pub fn ctrl(rows: &[[u8; 32]], last: &mut [i16], flag: i32, at: &DustAt) -> Vec<Ring> {
    let mut out = Vec::new();
    for (i, (r, last)) in rows.iter().zip(last.iter_mut()).enumerate() {
        let h = |o: usize| i16::from_le_bytes([r[o], r[o + 1]]);
        let w = |o: usize| u32::from_le_bytes([r[o], r[o + 1], r[o + 2], r[o + 3]]);
        let (anm, every) = (r[0], r[3]);
        let (first, end) = (h(4), h(6));
        let matches = if matches!(anm, 2 | 3) { matches!(at.anm_num, 2 | 3) } else { at.anm_num == i16::from(anm) };
        let f = at.frame_num;
        if !matches || f < first || end < f {
            continue;
        }
        if every == 0 {
            if flag == 0 {
                continue;
            }
        } else if first != end {
            let e = i32::from(every);
            let k = (i32::from(first) + (i32::from(f) - i32::from(first)) / e * e) as i16;
            if k == *last {
                continue;
            }
            *last = k;
        }
        out.push(Ring {
            row: i,
            ofs: [w(16), w(20), w(24), w(28)],
            s: w(8),
            r: w(12),
            n: i32::from(r[1]),
            life: i32::from(r[2]),
        });
    }
    out
}
