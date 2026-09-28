//! Routes through a Root Town (gcmn `ccnavi.cpp` 0x005130b0-0x00514888,
//! tables in `navitbl.cpp`): the walking PCs' `ccNavi`.
//!
//! A town is a graph of numbered landmarks (`naviMapTown1`-`5`, gcmn
//! 0x006144a0..., `ccLandmark` 0x30 bytes), each at a dummy of the town's
//! file (`DMY_marker01`...; `ccSetNaviMap` 0x005130b0 copies their
//! positions in when the player task starts). A landmark lists up to eight
//! neighbours it links to directly, its junction (`near[0]`; negative on a
//! landmark that is not one itself) and the main lines (`line[]`) through
//! it. A main line (`naviMainLinesOfTownN[line]`) is
//!
//! ```text
//! u8 from, to, n;  n x (u8 line, u8 junction): the lines it meets and where
//! u8 count;  count x u8 landmark: its landmarks in order
//! ```
//!
//! `ccNavi::RouteSearchByMap(start, goal)` (0x00513720) routes from the
//! landmark nearest `start` to the one nearest `goal` (within 10000, and
//! within 301 in height): directly when they link, else along the start's
//! junction, the main lines between the two junctions (a common line, or
//! `SearchRouteMainLineLink`'s shortest chain of at most four lines by the
//! landmarks' distances) and the goal's junction; duplicates are squeezed
//! out. The route is `route[]` (254, landmarks..., 255), `step` its
//! length and `landmark` the index being walked to; `GetDestination`
//! (0x00514770) gives that landmark's position.
//!
//! All in the EE's arithmetic ([`crate::ee`]).

use piney_data::Result;
use piney_desktop::assets::SceneFile;

use crate::ee::{self, F, ONE, V4, add, dot, sqrtf, vsub};
use crate::hit::Hits;

/// A route's end marks: 254 before the first landmark, 255 after the last.
pub const START: u8 = 254;
pub const END: u8 = 255;
/// `ccNaviSearchNearLandmark`: nothing further than 10000 (xy), or 301 in z.
const NEAR_MAX: F = 0x461c_4000;
/// `SearchRouteMainLineLink`'s and `CheckMainLineLink`'s "no route" 50000.
const FAR: F = 0x4743_5000;
const MINUS_ONE: F = 0xbf80_0000;

/// One `ccLandmark`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Landmark {
    pub pos: V4,
    pub name: i32,
    pub link: [i8; 8],
    pub near: [i8; 4],
    pub line: [i8; 4],
}

/// A town's navigation map as `ccSetNaviMap` leaves it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NaviMap {
    /// `landMarkNum`.
    pub num: i32,
    /// `naviMapPtr[0..=landMarkNum]`: `ccNaviGetLandmarkPos` reads one past
    /// the last (the next town's first landmark).
    pub marks: Vec<Landmark>,
    /// `naviMainLineTbl`, by line number (0 is empty).
    pub lines: Vec<Vec<u8>>,
}

impl NaviMap {
    /// `ccSetNaviMap` for town `town` (0 Mac Anu) with the dummies of its
    /// scene file: `naviMarkTable`, `naviMapTownTable` and
    /// `naviMainLinesOfTownTable` (INF gcmn 0x00619408, 0x00619420,
    /// 0x00619440), the volume's (`tables::world`).
    pub fn read(volume: piney_data::volume::Volume, town: usize, file: &SceneFile) -> Result<NaviMap> {
        let t = piney_data::tables::world::of(volume);
        let num = i32::from(t.navi_marks().get(town).copied().unwrap_or(0));
        let mut marks: Vec<Landmark> = t
            .navi_landmarks()
            .get(town)
            .copied()
            .unwrap_or_default()
            .iter()
            .map(|m| Landmark { pos: m.pos.map(f32::to_bits), name: m.name, link: m.link, near: m.near, line: m.line })
            .collect();
        for (k, m) in marks.iter_mut().enumerate().take(num.max(0) as usize).skip(1) {
            let name = format!("DMY_marker{k:02}");
            if let Some(d) = file.ccs.find_object(&name).and_then(|o| file.scene.dummies.get(&o)) {
                m.pos = [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE];
            }
        }
        let lines = t
            .navi_lines()
            .get(town)
            .copied()
            .unwrap_or_default()
            .iter()
            .map(|l| l.map_or_else(Vec::new, <[u8]>::to_vec))
            .collect();
        Ok(NaviMap { num, marks, lines })
    }

    /// `naviMapPtr[i]`, a blank landmark outside the table.
    pub fn mark(&self, i: i32) -> Landmark {
        usize::try_from(i).ok().and_then(|i| self.marks.get(i)).copied().unwrap_or_default()
    }

    fn line(&self, l: i32) -> &[u8] {
        usize::try_from(l).ok().and_then(|l| self.lines.get(l)).map_or(&[], |v| v.as_slice())
    }

    /// `ccNaviGetLandmarkPos(pos, num)` (0x005132a0): landmark `num`'s
    /// position for 1 <= num <= `landMarkNum`.
    pub fn landmark_pos(&self, num: i32) -> Option<V4> {
        (num > 0 && num <= self.num).then(|| self.mark(num).pos)
    }

    /// `ccNaviSearchNearLandmark(pos)` (0x00513300): the nearest landmark
    /// in the plane among those within 301 in height, within 10000; -1.
    pub fn search_near(&self, pos: V4) -> i32 {
        let (mut best, mut idx) = (NEAR_MAX, -1);
        for k in 1..self.num {
            let mut d = vsub(self.mark(k).pos, pos);
            if f64::from(ee::f(d[2]).abs()) > 301.0 {
                continue;
            }
            d[2] = 0;
            d[3] = ONE;
            let dist = sqrtf(dot(d, d));
            if ee::lt(dist, best) {
                idx = k;
                best = dist;
            }
        }
        idx
    }

    /// `ccNavi::CheckDistMainLine(ml, sjct, gjct)` (0x00514410): the length
    /// of line `ml` between two of its landmarks.
    fn dist_main_line(&self, ml: i32, sj: i32, gj: i32) -> F {
        if sj == gj {
            return 0;
        }
        let line = self.line(ml);
        let at = |i: usize| i32::from(line.get(i).copied().unwrap_or(0));
        let base = 2 * at(2) as usize + 4;
        let count = at(base - 1) as usize;
        let Some((mut k, until)) = (0..count).find_map(|k| match at(base + k) {
            v if v == sj => Some((k, gj)),
            v if v == gj => Some((k, sj)),
            _ => None,
        }) else {
            return 0;
        };
        let mut total = 0;
        while at(base + k) != until && base + k + 1 < line.len() {
            let d = vsub(self.mark(at(base + k)).pos, self.mark(at(base + k + 1)).pos);
            total = add(total, sqrtf(dot(d, d)));
            k += 1;
        }
        total
    }

    /// `ccNavi::CheckCommonMainLine(jct1, jct2)` (0x00514580): the lines
    /// through both, one a byte, the first the highest.
    fn common_main_line(&self, j1: i32, j2: i32) -> u32 {
        let (a, b) = (self.mark(j1).line, self.mark(j2).line);
        let mut v = 0u32;
        for &x in &a {
            if x < 0 {
                break;
            }
            for &y in &b {
                if y < 0 {
                    break;
                }
                if x == y {
                    v = (v << 8) | (x as u8 as u32);
                }
            }
        }
        v
    }

    /// `ccNavi::SetRouteMainLine(rt, num, start, goal)` (0x00513cb0): line
    /// `num`'s landmarks from `start` to `goal`, either way along it.
    fn route_main_line(&self, rt: &mut Vec<u8>, num: i32, start: i32, goal: i32) {
        let line = self.line(num);
        let at = |i: usize| i32::from(line.get(i).copied().unwrap_or(0));
        let n = at(2) as usize;
        let count = at(2 * n + 3) as usize;
        let mut t = 2 * n + 4;
        let mut goal_first = false;
        for _ in 0..count {
            let v = at(t);
            if v == start {
                break;
            }
            if v == goal {
                goal_first = true;
            }
            t += 1;
        }
        let mut guard = 0;
        while at(t) != goal && guard < 256 {
            rt.push(at(t) as u8);
            if goal_first {
                t = t.wrapping_sub(1);
            } else {
                t += 1;
            }
            guard += 1;
        }
        rt.push(goal as u8);
    }

    /// `ccNavi::SetRouteM2M(rt, start, goal)` (0x00513b00): a landmark to a
    /// neighbour.
    fn route_m2m(&self, rt: &mut Vec<u8>, a: i32, b: i32) {
        if a == b {
            rt.push(a as u8);
            return;
        }
        if self.mark(a).link.iter().any(|&l| i32::from(l) == b) {
            rt.push(a as u8);
            rt.push(b as u8);
        }
    }

    /// `ccNavi::SetRouteL2L(rt, startJCT, goalJCT)` (0x00513b90): junction
    /// to junction, along a line they share or through the search.
    fn route_l2l(&self, rt: &mut Vec<u8>, sj: i32, gj: i32) {
        if sj == gj {
            rt.push(gj as u8);
            return;
        }
        for &a in &self.mark(sj).line {
            if a < 0 {
                break;
            }
            for &b in &self.mark(gj).line {
                if b < 0 {
                    break;
                }
                if a == b {
                    self.route_main_line(rt, i32::from(a), sj, gj);
                    return;
                }
            }
        }
        self.search_main_line_link(rt, sj, gj);
    }

    /// `ccNavi::SearchRouteMainLineLink(rt, startJCT, goalJCT)`
    /// (0x00513e00): the shortest chain of lines from a line through the
    /// start's junction to one through the goal's.
    fn search_main_line_link(&self, rt: &mut Vec<u8>, sj: i32, gj: i32) {
        let mut s = Search { depth: 0, flag: 0 };
        let mut best = FAR;
        let mut nearest = [0u8; 16];
        for &sl in &self.mark(sj).line {
            if sl < 0 {
                break;
            }
            s.flag = 0;
            for &gl in &self.mark(gj).line {
                if gl < 0 {
                    break;
                }
                s.flag |= 1u32 << (sl as u32 & 31);
                let mut temp = [0u8; 16];
                let d = s.check(self, &mut temp, i32::from(sl), i32::from(gl), sj, gj, true);
                if temp[0] != 0 && !ee::le(best, d) {
                    best = d;
                    nearest = temp;
                }
            }
        }
        let mut link = vec![0u8];
        for k in 0..nearest[0] as usize {
            link[0] = link[0].wrapping_add(1);
            link.push(nearest.get(1 + k).copied().unwrap_or(0));
        }
        link[0] = link[0].wrapping_add(1);
        link.push(sj as u8);
        self.route_main_line_link(rt, &link);
    }

    /// `ccNavi::SetRouteMainLineLink(rt, link)` (0x00514630): the chain of
    /// junctions `link[1..]` (goal first) walked from its end, each pair
    /// along the shortest line they share.
    fn route_main_line_link(&self, rt: &mut Vec<u8>, link: &[u8]) {
        let at = |i: usize| i32::from(link.get(i).copied().unwrap_or(0));
        let mut k = at(0) as usize;
        let mut chosen = 0;
        while k >= 2 {
            let mut best = FAR;
            let (a, b) = (at(k), at(k - 1));
            let mut common = self.common_main_line(a, b);
            loop {
                let l = (common & 0xff) as i32;
                common >>= 8;
                let d = self.dist_main_line(l, a, b);
                if !ee::lt(best, d) {
                    best = d;
                    chosen = l;
                }
                if common == 0 {
                    break;
                }
            }
            self.route_main_line(rt, chosen, a, b);
            k -= 1;
        }
    }
}

/// `CheckMainLineLink`'s globals: `searchDepth` and `linkCheckFlag`.
struct Search {
    depth: i32,
    flag: u32,
}

/// `path[0]++; path[path[0]] = v` on a 16-byte path.
fn push(path: &mut [u8; 16], v: i32) {
    path[0] = path[0].wrapping_add(1);
    let at = path[0] as usize;
    if let Some(p) = path.get_mut(at) {
        *p = v as u8;
    }
}

impl Search {
    /// `ccNavi::CheckMainLineLink(path, sml, gml, sjct, gjct, sw)`
    /// (0x005140d0): from junction `sjct` on line `sml` to `gjct` on `gml`,
    /// through at most four lines: the junctions (goal first) into `path`
    /// and the length; -1 past the depth.
    #[allow(clippy::too_many_arguments)]
    fn check(&mut self, map: &NaviMap, path: &mut [u8; 16], sml: i32, gml: i32, sj: i32, gj: i32, sw: bool) -> F {
        let mut best = FAR;
        self.depth += 1;
        if self.depth >= 5 {
            self.depth -= 1;
            return MINUS_ONE;
        }
        if sml == gml {
            if sj == gj {
                best = 0;
                push(path, gj);
            } else {
                best = map.dist_main_line(sml, sj, gj);
                push(path, gj);
                push(path, sj);
            }
            self.depth -= 1;
            return best;
        }
        if sw {
            let mut s1 = [0u8; 16];
            let line = map.line(sml).to_vec();
            let at = |i: usize| i32::from(line.get(i).copied().unwrap_or(0));
            for k in 0..at(2) as usize {
                let (l, jct) = (at(3 + 2 * k), at(4 + 2 * k));
                let bit = 1u32 << (l as u32 & 31);
                if self.flag & bit != 0 {
                    continue;
                }
                if l != gml {
                    self.flag |= bit;
                }
                let saved = self.flag;
                let mut s2 = [0u8; 16];
                let d = self.check(map, &mut s2, l, gml, jct, gj, true);
                self.flag = saved;
                if s2[0] != 0 && !ee::le(best, d) {
                    best = d;
                    s1 = s2;
                }
            }
            if s1[0] != 0 {
                for k in 0..s1[0] as usize {
                    push(path, i32::from(s1.get(1 + k).copied().unwrap_or(0)));
                }
                push(path, sj);
                let last = i32::from(s1.get(s1[0] as usize).copied().unwrap_or(0));
                best = add(best, map.dist_main_line(sml, sj, last));
            }
        }
        self.depth -= 1;
        best
    }
}

/// A `ccNavi` as a town uses it.
#[derive(Clone, Debug, PartialEq)]
pub struct Navi {
    /// +0x00 `goalPos`.
    pub goal_pos: V4,
    /// +0x18 `name` (the goal landmark's name), +0x1a `finishFlag`, +0x1c
    /// `step` (the route's length: the index of its 255), +0x1e `landmark`
    /// (the index being walked to).
    pub name: i16,
    pub finish: i16,
    pub step: i16,
    pub landmark: i16,
    /// +0x20, +0x24: from `start` to the first landmark.
    pub dist: F,
    pub dirc: F,
    /// +0x2e `route[48]`.
    pub route: [u8; 48],
}

impl Default for Navi {
    /// `ccNavi::ccNavi` (0x00513610) outside a dungeon.
    fn default() -> Self {
        Navi { goal_pos: ee::VF0, name: -1, finish: 1, step: 0, landmark: 0, dist: 0, dirc: 0, route: [0; 48] }
    }
}

impl Navi {
    /// `ccNavi::RouteSearchByMap(start, goal)`: the route from the
    /// landmark nearest `start` to the one nearest `goal`; `landmark` 1.
    /// Returns what the game returns (false when both are the same landmark
    /// and nothing stands between them, `ccHitCheckLM2` from 150 above).
    pub fn route_search(&mut self, map: &NaviMap, start: V4, goal: V4, hits: &mut Hits) -> bool {
        let mut ret = true;
        let s0 = map.search_near(start);
        let s1 = map.search_near(goal);
        // sp176 before it is set from a landmark: the line check's distance
        // in x when the two are the same landmark, else stack.
        let mut to_first: V4 = [0; 4];
        let mut rt: Vec<u8> = Vec::with_capacity(48);
        if s0 == s1 {
            let (mut a, mut b) = (start, goal);
            a[2] = add(a[2], 0x4316_0000);
            b[2] = add(b[2], 0x4316_0000);
            let d = hits.line(a, b, 1, 1).map_or(MINUS_ONE, |(_, d)| d);
            to_first[0] = d;
            rt.extend([START, s0 as u8, END]);
            if ee::eq(MINUS_ONE, d) {
                ret = false;
            }
        } else {
            let links = map.mark(s0).link;
            let direct = links.iter().take_while(|&&l| l != -1).any(|&l| i32::from(l) == s1);
            if direct {
                rt.extend([START, s0 as u8, s1 as u8, END]);
            } else {
                let s6 = i32::from(map.mark(s0).near[0]).abs();
                let s5 = i32::from(map.mark(s1).near[0]).abs();
                rt.extend([START, s0 as u8]);
                if s0 != s6 {
                    map.route_m2m(&mut rt, s0, s6);
                }
                if s6 != s5 {
                    map.route_l2l(&mut rt, s6, s5);
                }
                if s1 != s5 {
                    map.route_m2m(&mut rt, s5, s1);
                }
                rt.extend([s1 as u8, END]);
                optimize(&mut rt);
            }
        }
        self.route = [0; 48];
        for (d, s) in self.route.iter_mut().zip(&rt) {
            *d = *s;
        }
        self.name = map.mark(s1).name as i16;
        self.finish = 0;
        self.step = 0;
        loop {
            self.step += 1;
            if self.route.get(self.step as usize).is_none_or(|&r| r == END) {
                break;
            }
        }
        if s0 > 0 && s0 <= map.num {
            to_first = map.mark(s0).pos;
        }
        let d = vsub(to_first, start);
        self.dist = sqrtf(dot(d, d));
        self.dirc = ee::atan2f(d[0], ee::mul(MINUS_ONE, d[1]));
        self.landmark = 1;
        self.goal_pos = goal;
        ret
    }

    /// `ccNavi::GetDestination(p)` in a town: the landmark being walked to
    /// while there is one, else the goal (and 0).
    pub fn destination(&self, map: &NaviMap, p: &mut V4) -> i16 {
        if self.landmark < self.step {
            let r = self.route.get(self.landmark as usize).copied().unwrap_or(0);
            *p = map.mark(i32::from(r)).pos;
            self.landmark
        } else {
            *p = self.goal_pos;
            0
        }
    }
}

/// `ccNavi::SetRouteOptimize(rt)` (0x00513d80): the 254s at the front
/// skipped, runs of the same landmark squeezed to one, 255 after them.
fn optimize(rt: &mut Vec<u8>) {
    let mut src = 0;
    while rt.get(src) == Some(&START) {
        src += 1;
    }
    let mut out = vec![rt.first().copied().unwrap_or(START)];
    let mut dst = 0;
    while let Some(&v) = rt.get(src) {
        if v == END {
            break;
        }
        if out[dst] != v {
            out.truncate(dst + 1);
            out.push(v);
            dst += 1;
        }
        src += 1;
    }
    out.truncate(dst + 1);
    out.push(END);
    *rt = out;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optimize_squeezes_runs() {
        let mut rt = vec![254, 3, 3, 7, 7, 7, 9, 3, 255];
        optimize(&mut rt);
        assert_eq!(rt, [254, 3, 7, 9, 3, 255]);
    }
}
