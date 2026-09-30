//! The party members' navigation (`ccnavi.cpp`, gcmn 0x005130b0-0x00515d2c):
//! a member's `ccNavi` (in its `ccAI` at +0xf0), the dungeon room's path
//! finding (`SetPathFindingMap` 0x00515aa0, `PathFindingInDungeon` 0x005148e0,
//! over `buf`, `buf2`, `bufFlag`) and a Root Town's landmarks (`ccSetNaviMap`;
//! the routes are [`NaviWorld::route_search_by_map`]'s). A way of 49-54
//! cells overruns `route[48]` byte for byte as in the game; from 55 the port
//! keeps its beacon table. The movers are [`crate::ai_move`]'s; the rules are
//! in docs/engine/battle.md ("Party navigation").

use piney_data::volume::Volume;

use crate::damage::fptosi;
use crate::geom::{self, F, V4, add, from_int, mul, sub, vsub};
use crate::world::World;

/// The map's side: `buf`, `buf2` and the dungeon's 2D map are `[256][256]`.
pub const SIDE: i32 = 256;
/// 300.0: a map cell, in world units (`WORLD_MAN::Get2DPos`).
pub const CELL: F = 0x4396_0000;
/// 150.0: half a cell.
pub const HALF_CELL: F = 0x4316_0000;
/// 10000.0: `ccNaviSearchNearLandmark`'s farthest landmark.
const NEAR_MAX: F = 0x461c_4000;

/// What the navigation asks of the world beyond [`World`].
pub trait NaviWorld: World {
    /// `WORLD_MAN::Get2DMapInfo(info)` (main 0x001a22f0) in a dungeon:
    /// `DUNGEON::GetRoom2DPos` (gcmn 0x005cf080) of the room the player is
    /// in (`ccGame` +0x30) on the current floor: `[x, y, size]`, the
    /// room's position (`DUNGEON` +0x2f6e0) / 300 through `fptosi`, less
    /// half the size; the size 80 for a story room (`ROOMDATA.type` 16 or
    /// more), else 10, 20, 40 by the room's minimap size code (0, 1, 2).
    fn map_2d_info(&mut self) -> [i32; 3];
    /// `WORLD_MAN::Get2DMapPtr()` (main 0x001a22b0): the current floor's
    /// 2D map, 65536 bytes `[x][y]` (see the module doc for the cells), as
    /// `DUNGEON::MakeMiniMap` (gcmn 0x005cd850) has filled it so far.
    fn map_2d(&mut self) -> &[u8];
    /// `ccNavi::RouteSearchByMap(start, goal)` (gcmn 0x00513720) in the
    /// Root Town: the route from the landmark nearest `start` to the one
    /// nearest `goal`. It sets `navi`'s `route` (254, the landmarks, 255),
    /// `name` (the goal landmark's), `step` (the index of the 255),
    /// `landmark` 1, `dist` and `dirc` (from `start` to the first
    /// landmark), `goal_pos` = `goal`, and `finish` (`finishFlag`) 0, and
    /// returns what the game returns (0 when both are the same landmark
    /// and nothing stands between them).
    fn route_search_by_map(&mut self, navi: &mut Navi, finish: &mut i16, start: V4, goal: V4) -> i32;
    /// `ccStream::GetChunkAdrsF(naviPointNameTable[name], 0)` (main
    /// 0x00144580, the name table at gcmn 0x00653c80) on the area's stream
    /// (`worldman` +0x430 +0x1a4): the position of the dummy (+0x10 of its
    /// chunk) a town landmark named `name` faces from.
    fn navi_point(&mut self, name: i16) -> V4;
    /// `ccStream::GetChunkAdrsF("DMY_markerNN", 0)` + 0x10: the position
    /// of landmark `k`'s dummy, as `ccSetNaviMap` reads it.
    fn marker(&mut self, k: i32) -> V4;
}

/// `ccNavi` (0x70 bytes, `ccAI` +0xf0) without `finishFlag` (+0x1a),
/// which is [`crate::party_ai::Ai::navi_finish`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Navi {
    /// +0x00 `goalPos`.
    pub goal_pos: V4,
    /// +0x10 `startX`, `startY`, `goalX`, `goalY`: the last dungeon way's
    /// ends, in map cells.
    pub start_x: i16,
    pub start_y: i16,
    pub goal_x: i16,
    pub goal_y: i16,
    /// +0x18 `name`: the goal landmark's name in a town, 0 for a dungeon
    /// way.
    pub name: i16,
    /// +0x1c `step`: the route's length; +0x1e `landmark`: the index being
    /// walked to.
    pub step: i16,
    pub landmark: i16,
    /// +0x20 `dist`, +0x24 `dirc`.
    pub dist: F,
    pub dirc: F,
    /// +0x28 `mapX`, `mapY`, `mapS`: the dungeon window the way is found
    /// in (`SetDungeonMapInfo`).
    pub map_x: i16,
    pub map_y: i16,
    pub map_s: i16,
    /// +0x2e `route[48]`.
    pub route: [u8; 48],
    /// +0x5e: the padding after `route`, which a long way's indices run
    /// into.
    pub pad: [u8; 2],
    /// +0x60 `beacon.num`.
    pub beacon_num: i32,
    /// `*beacon.pos` (+0x64): the beacons' map cells (x, y), the table
    /// `SetBeacon` allocates.
    pub beacon: Vec<[u8; 2]>,
}

impl Default for Navi {
    fn default() -> Self {
        Navi::new()
    }
}

impl Navi {
    /// `ccNavi::ccNavi()` (gcmn 0x00513610) outside a dungeon: `goalPos`
    /// (0, 0, 0, 1), `name` -1, `finishFlag` 1 (the AI's), the route and
    /// the counters 0, no beacons. `mapX`/`Y`/`S` and the way's ends are
    /// not set (0 here). In a dungeon (`game.area` 2) the constructor also
    /// runs [`Navi::set_dungeon_map_info`].
    pub fn new() -> Navi {
        Navi {
            goal_pos: geom::VF0,
            start_x: 0,
            start_y: 0,
            goal_x: 0,
            goal_y: 0,
            name: -1,
            step: 0,
            landmark: 0,
            dist: 0,
            dirc: 0,
            map_x: 0,
            map_y: 0,
            map_s: 0,
            route: [0; 48],
            pad: [0; 2],
            beacon_num: 0,
            beacon: Vec::new(),
        }
    }

    /// `ccNavi::SetDungeonMapInfo()` (gcmn 0x00514890): the window, from
    /// `WORLD_MAN::Get2DMapInfo`.
    pub fn set_dungeon_map_info(&mut self, w: &mut dyn NaviWorld) {
        let info = w.map_2d_info();
        self.map_x = info[0] as i16;
        self.map_y = info[1] as i16;
        self.map_s = info[2] as i16;
    }

    /// The byte at `route + i`, as the game reads `route[i]`: `route`, the
    /// padding, then `beacon.num`'s bytes (and before `route` the window's
    /// bytes). Further on (the beacon table's pointer) is not modelled: 0.
    pub fn route_byte(&self, i: i32) -> u8 {
        match i {
            0..=47 => self.route[i as usize],
            48 | 49 => self.pad[i as usize - 48],
            50..=53 => self.beacon_num.to_le_bytes()[i as usize - 50],
            -6..=-1 => {
                let v = [self.map_x, self.map_y, self.map_s][((i + 6) / 2) as usize];
                v.to_le_bytes()[((i + 6) % 2) as usize]
            }
            _ => 0,
        }
    }

    fn set_route_byte(&mut self, i: i32, v: u8) {
        match i {
            0..=47 => self.route[i as usize] = v,
            48 | 49 => self.pad[i as usize - 48] = v,
            50..=53 => {
                let mut b = self.beacon_num.to_le_bytes();
                b[i as usize - 50] = v;
                self.beacon_num = i32::from_le_bytes(b);
            }
            _ => {}
        }
    }

    /// Beacon `k`'s cell; (0, 0) past the table.
    fn beacon_at(&self, k: usize) -> [u8; 2] {
        self.beacon.get(k).copied().unwrap_or_default()
    }

    /// `ccNavi::CheckBeaconPos(p, n)` (gcmn 0x00515900): the centre of
    /// beacon `route[n]`'s cell (`WORLD_MAN::Get3DPos`: 300 x + 150, 300 y
    /// + 150, 0, 1).
    pub fn check_beacon_pos(&self, n: i32) -> V4 {
        let [x, y] = self.beacon_at(usize::from(self.route_byte(n)));
        pos_3d([from_int(i32::from(x)), from_int(i32::from(y))])
    }

    /// `ccNavi::CheckBeaconPos2D(p, n)` (gcmn 0x005159e0): beacon
    /// `route[n]`'s cell as floats (x, y, 0, 0).
    pub fn check_beacon_pos_2d(&self, n: i32) -> V4 {
        let [x, y] = self.beacon_at(usize::from(self.route_byte(n)));
        [from_int(i32::from(x)), from_int(i32::from(y)), 0, 0]
    }

    /// `ccNavi::GetDestination(p)` (gcmn 0x00514770): where to walk next.
    /// While `landmark < step`, the landmark's position (a town,
    /// `naviMapPtr[route[landmark]]`) or its beacon's centre (a dungeon),
    /// returning `landmark`; after it `goalPos`, returning 0. In a field
    /// (`area` 1) `p` is left as it is and 0 returned.
    pub fn get_destination(&self, area: i32, town: &TownMap, p: &mut V4) -> i16 {
        match area {
            0 => {
                if self.landmark < self.step {
                    *p = town.mark(i32::from(self.route_byte(i32::from(self.landmark)))).pos;
                    self.landmark
                } else {
                    *p = self.goal_pos;
                    0
                }
            }
            2 => {
                if self.landmark < self.step {
                    *p = self.check_beacon_pos(i32::from(self.landmark));
                    self.landmark
                } else {
                    *p = self.goal_pos;
                    0
                }
            }
            _ => 0,
        }
    }

    /// `ccNavi::PathFindingInDungeon(s, g)` (gcmn 0x005148e0): a way from
    /// world position `s` to `g` (their map cells through
    /// `WORLD_MAN::Get2DPos` and `fptosi`). See [`Navi::path_finding_cells`].
    pub fn path_finding(
        &mut self,
        finish: &mut i16,
        area: i32,
        map: &mut PathMap,
        w: &mut dyn NaviWorld,
        s: V4,
        g: V4,
    ) -> i32 {
        let a = pos_2d(s);
        let b = pos_2d(g);
        let (sx, sy) = (fptosi(a[0]), fptosi(a[1]));
        let (gx, gy) = (fptosi(b[0]), fptosi(b[1]));
        self.path_finding_cells(finish, area, map, w, sx, sy, gx, gy)
    }

    /// `ccNavi::PathFindingInDungeon(sx, sy, gx, gy)` (gcmn 0x005149a0):
    /// -1 before `SetPathFindingMap` has run or when the goal cell has no
    /// way into it; 0 outside a dungeon, for the same cell, for an end
    /// outside the window, or when no way is found; 1 with a way: the
    /// beacons set (`SetBeacon`), `name` and `landmark` 0, the ends kept,
    /// `finishFlag` 0.
    #[allow(clippy::too_many_arguments)]
    pub fn path_finding_cells(
        &mut self,
        finish: &mut i16,
        area: i32,
        map: &mut PathMap,
        w: &mut dyn NaviWorld,
        sx: i32,
        sy: i32,
        gx: i32,
        gy: i32,
    ) -> i32 {
        if map.ready == 0 {
            return -1;
        }
        if area != 2 {
            return 0;
        }
        if sx == gx && sy == gy {
            return 0;
        }
        let (x0, y0, s) = (i32::from(self.map_x), i32::from(self.map_y), i32::from(self.map_s));
        let (xe, ye) = (x0 + s - 1, y0 + s - 1);
        if sx < x0 || xe < sx || sy < y0 || ye < sy || gx < x0 || xe < gx || gy < y0 || ye < gy {
            return 0;
        }
        if map.link_at(gx, gy) & 0xf == 0 {
            return -1;
        }
        // Get2DMapPtr is only tested against null: the map is always there.
        let _ = w.map_2d();
        let mut x = i32::from(self.map_x);
        while x < i32::from(self.map_x) + i32::from(self.map_s) {
            let mut y = i32::from(self.map_y);
            while y < i32::from(self.map_y) + i32::from(self.map_s) {
                map.set_cost(x, y, -1);
                let l = map.link_at(x, y) & 0xf;
                map.set_link(x, y, l);
                y += 1;
            }
            x += 1;
        }
        map.set_cost(sx, sy, 0);
        map.set_cost(gx, gy, -2);
        if self.heuristic_type1(map) != 0 {
            // Never taken: HeuristicType1 returns 0 either way.
            return 2;
        }
        if !self.short_path(map, gx, gy) {
            return 0;
        }
        let Some(tbl) = self.make_beacon_tbl(map, gx, gy) else { return 0 };
        self.set_beacon(&tbl);
        self.name = 0;
        self.landmark = 0;
        self.start_x = sx as i16;
        self.start_y = sy as i16;
        self.goal_x = gx as i16;
        self.goal_y = gy as i16;
        *finish = 0;
        1
    }

    /// The four neighbour tests of `LinkInfo`, `MinimumLinkDirc` and
    /// `LinkNum`, each one-sided as the game makes them: y - 1 not above
    /// the window, x + 1 and y + 1 not past it, x - 1 not left of it.
    fn sides(&self, x: i32, y: i32) -> [(i32, i32, bool); 4] {
        let (x0, y0, s) = (i32::from(self.map_x), i32::from(self.map_y), i32::from(self.map_s));
        [(x, y - 1, y > y0), (x + 1, y, x + 1 < x0 + s), (x, y + 1, y + 1 < y0 + s), (x - 1, y, x > x0)]
    }

    /// `ccNavi::HeuristicType1(bp)` (gcmn 0x00514cc0): the wave. For each
    /// step `s` from 0 while `s < mapS * 14 / 10`, over the window: a cell
    /// at `s` gives `s + 1` to each neighbour (in the whole map) still at
    /// -1 whose `buf` is not 0; the goal (-2) with two neighbours reached
    /// ([`Navi::link_num`]) ends the wave. Returns 0 (the game's other way
    /// out returns 0 as well).
    pub fn heuristic_type1(&self, map: &mut PathMap) -> i32 {
        let mut s: i32 = 0;
        loop {
            let mut x = i32::from(self.map_x);
            while x < i32::from(self.map_x) + i32::from(self.map_s) {
                let mut y = i32::from(self.map_y);
                while y < i32::from(self.map_y) + i32::from(self.map_s) {
                    let v = i32::from(map.cost_at(x, y));
                    if v == -2 && self.link_num(map, x, y) >= 2 {
                        return 0;
                    }
                    if v == s {
                        let next = (s + 1) as i16;
                        for (nx, ny, ok) in
                            [(x, y - 1, y > 0), (x + 1, y, x + 1 < SIDE), (x, y + 1, y + 1 < SIDE), (x - 1, y, x > 0)]
                        {
                            if ok && map.cost_at(nx, ny) == -1 && map.link_at(nx, ny) != 0 {
                                map.set_cost(nx, ny, next);
                            }
                        }
                    }
                    y += 1;
                }
                x += 1;
            }
            s += 1;
            if s >= i32::from(self.map_s) * 14 / 10 {
                return 0;
            }
        }
    }

    /// `ccNavi::ShortPath(bp, x, y)` (gcmn 0x00514f60): from the goal down
    /// the wave to the start, each cell left marked `buf` 0x80. False on a
    /// cell with no way down, or (the game testing `bp` where it means
    /// `buf`) on a wave value with bit 7.
    pub fn short_path(&self, map: &mut PathMap, mut x: i32, mut y: i32) -> bool {
        loop {
            let d = self.minimum_link_dirc(map, x, y);
            if d == -1 {
                return false;
            }
            let l = map.link_at(x, y) | 0x80;
            map.set_link(x, y, l);
            match d {
                0 => y -= 1,
                1 => x += 1,
                2 => y += 1,
                3 => x -= 1,
                _ => {}
            }
            let v = map.cost_at(x, y);
            if v == 0 {
                let l = map.link_at(x, y) | 0x80;
                map.set_link(x, y, l);
                return true;
            }
            if v & 0x80 != 0 {
                return false;
            }
        }
    }

    /// `ccNavi::MakeBeaconTbl(tbl, bp, gx, gy)` (gcmn 0x005150c0): the
    /// cells of the way, the one after the start first and the goal last
    /// (`ccPathPos` x, y, prev, next as bytes, the ends 255), every cell of
    /// the window off the way set to -1 first, the goal and the corners
    /// (two perpendicular neighbours on the way) marked `buf` 0x20. None
    /// (the game's 0) when the walk has nowhere to go.
    pub fn make_beacon_tbl(&self, map: &mut PathMap, gx: i32, gy: i32) -> Option<Vec<[u8; 4]>> {
        let mut x = i32::from(self.map_x);
        while x < i32::from(self.map_x) + i32::from(self.map_s) {
            let mut y = i32::from(self.map_y);
            while y < i32::from(self.map_y) + i32::from(self.map_s) {
                if map.link_at(x, y) & 0x80 == 0 {
                    map.set_cost(x, y, -1);
                }
                y += 1;
            }
            x += 1;
        }
        let (mut x, mut y) = (gx, gy);
        let mut n: i32 = 0;
        while map.cost_at(x, y) != 0 {
            if map.cost_at(x, y) != -1 {
                let li = self.link_info(map, x, y);
                if map.cost_at(x, y) == -2 || matches!(li, 3 | 6 | 12 | 9) {
                    let l = map.link_at(x, y) | 0x20;
                    map.set_link(x, y, l);
                }
            }
            n += 1;
            (x, y) = step(self.minimum_link_dirc(map, x, y), x, y)?;
        }
        let num = n;
        let mut tbl = vec![[0u8; 4]; num.max(0) as usize];
        let (mut x, mut y) = (gx, gy);
        while map.cost_at(x, y) != 0 {
            if let Some(p) = usize::try_from(n - 1).ok().and_then(|k| tbl.get_mut(k)) {
                *p = [x as u8, y as u8, (n - 2) as u8, n as u8];
            }
            n -= 1;
            (x, y) = step(self.minimum_link_dirc(map, x, y), x, y)?;
        }
        if num == 0 {
            // ccMalloc(0), then pos[0].prev and pos[-1].next written past
            // it: the walk above found the goal on the start, which the
            // caller has already ruled out.
            return None;
        }
        tbl[0][2] = 255;
        tbl[num as usize - 1][3] = 255;
        Some(tbl)
    }

    /// `ccNavi::SetBeacon(tbl)` (gcmn 0x00515470): the way's cells into the
    /// beacon table (a new one; the old freed), following `next` from the
    /// first until 255; `beacon.num` and `step` the count, `route[i] = i`
    /// (running past `route` on a long way, see the module doc). Returns
    /// the count.
    pub fn set_beacon(&mut self, tbl: &[[u8; 4]]) -> i32 {
        let mut beacon = Vec::with_capacity(tbl.len());
        let mut i = 0usize;
        loop {
            let p = tbl.get(i).copied().unwrap_or_default();
            beacon.push([p[0], p[1]]);
            i = usize::from(p[3]);
            if i == 255 || beacon.len() > 0x1_0000 {
                break;
            }
        }
        let n = beacon.len() as i32;
        self.beacon = beacon;
        self.beacon_num = n;
        self.step = n as i16;
        for k in 0..n {
            self.set_route_byte(k, k as u8);
        }
        n
    }

    /// `ccNavi::LinkInfo(bp, x, y)` (gcmn 0x00515580): a bit for each
    /// neighbour in the window not at -1 (1 at y - 1, 2 at x + 1, 4 at
    /// y + 1, 8 at x - 1).
    pub fn link_info(&self, map: &PathMap, x: i32, y: i32) -> i32 {
        let mut v = 0;
        for (k, (nx, ny, ok)) in self.sides(x, y).into_iter().enumerate() {
            if ok && map.cost_at(nx, ny) != -1 {
                v |= 1 << k;
            }
        }
        v
    }

    /// `ccNavi::MinimumLinkDirc(bp, x, y)` (gcmn 0x005156b0): the
    /// neighbour in the window with the lowest wave value not below 0 (0
    /// y - 1, 1 x + 1, 2 y + 1, 3 x - 1; the first of equals); -1 without
    /// one.
    pub fn minimum_link_dirc(&self, map: &PathMap, x: i32, y: i32) -> i32 {
        let mut min: i16 = 32767;
        let mut d = -1;
        for (k, (nx, ny, ok)) in self.sides(x, y).into_iter().enumerate() {
            let v = map.cost_at(nx, ny);
            if ok && v >= 0 && v < min {
                min = v;
                d = k as i32;
            }
        }
        d
    }

    /// `ccNavi::LinkNum(bp, x, y)` (gcmn 0x005157f0): the neighbours in
    /// the window the wave has reached, `bp > 0` with `bp & 0xf` not 0.
    pub fn link_num(&self, map: &PathMap, x: i32, y: i32) -> i32 {
        let hit = |v: i16| v > 0 && v & 0xf != 0;
        self.sides(x, y).into_iter().filter(|&(nx, ny, ok)| ok && hit(map.cost_at(nx, ny))).count() as i32
    }
}

/// One move of the walks: 0 y - 1, 1 x + 1, 2 y + 1, 3 x - 1.
fn step(d: i32, x: i32, y: i32) -> Option<(i32, i32)> {
    match d {
        0 => Some((x, y - 1)),
        1 => Some((x + 1, y)),
        2 => Some((x, y + 1)),
        3 => Some((x - 1, y)),
        _ => None,
    }
}

/// The path finding's globals: `buf` (gcmn 0x006faa70, `u8[256][256]`,
/// the links and the 0x80 / 0x20 marks), `buf2` (0x0070aa70,
/// `short[256][256]`, the wave) and `bufFlag` (0x00378c30). One set serves
/// every member.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathMap {
    pub link: Vec<u8>,
    pub cost: Vec<i16>,
    pub ready: i32,
}

impl Default for PathMap {
    fn default() -> Self {
        PathMap { link: vec![0; 0x1_0000], cost: vec![0; 0x1_0000], ready: 0 }
    }
}

/// `[x][y]` of a 256 x 256 map as the game computes it (`x * 256 + y`),
/// None outside it.
fn cell(x: i32, y: i32) -> Option<usize> {
    let i = x.wrapping_mul(SIDE).wrapping_add(y);
    usize::try_from(i).ok().filter(|&i| i < 0x1_0000)
}

impl PathMap {
    /// `buf[x][y]` (0 outside the array, where the game reads other data).
    pub fn link_at(&self, x: i32, y: i32) -> u8 {
        cell(x, y).map_or(0, |i| self.link[i])
    }

    pub fn set_link(&mut self, x: i32, y: i32, v: u8) {
        if let Some(i) = cell(x, y) {
            self.link[i] = v;
        }
    }

    /// `buf2[x][y]`.
    pub fn cost_at(&self, x: i32, y: i32) -> i16 {
        cell(x, y).map_or(0, |i| self.cost[i])
    }

    pub fn set_cost(&mut self, x: i32, y: i32, v: i16) {
        if let Some(i) = cell(x, y) {
            self.cost[i] = v;
        }
    }

    /// `SetPathFindingMap()` (gcmn 0x00515aa0): in a dungeon, the current
    /// room's square of `buf` cleared, then each walkable cell of the 2D
    /// map (not 0 or 4) given its walkable neighbours' bits; `bufFlag` 1.
    pub fn set_path_finding_map(&mut self, area: i32, w: &mut dyn NaviWorld) {
        if area != 2 {
            return;
        }
        let [x0, y0, s] = w.map_2d_info();
        let p = w.map_2d();
        let at = |x: i32, y: i32| cell(x, y).and_then(|i| p.get(i)).copied().unwrap_or(0);
        let walk = |c: u8| c != 0 && c != 4;
        for x in x0..x0.wrapping_add(s) {
            for y in y0..y0.wrapping_add(s) {
                self.set_link(x, y, 0);
            }
        }
        for x in x0..x0.wrapping_add(s) {
            for y in y0..y0.wrapping_add(s) {
                if !walk(at(x, y)) {
                    continue;
                }
                let a = if y > 0 && walk(at(x, y - 1)) { 1 } else { 0 };
                let b = if x + 1 < SIDE && walk(at(x + 1, y)) { 2 } else { 0 };
                let c = if y + 1 < SIDE && walk(at(x, y + 1)) { 4 } else { 0 };
                let d = if x > 0 && walk(at(x - 1, y)) { 8 } else { 0 };
                self.set_link(x, y, a + b + c + d);
            }
        }
        self.ready = 1;
    }
}

/// `WORLD_MAN::Get2DPos(in, out)` (main 0x001a2210) in a dungeon
/// (`WORLD_MAN` +8 is 2; elsewhere the output is left alone): x / 300,
/// y / 300.
pub fn pos_2d(p: V4) -> [F; 2] {
    [geom::div(p[0], CELL), geom::div(p[1], CELL)]
}

/// `WORLD_MAN::Get3DPos(out, in)` (main 0x001a2250) in a dungeon: a cell's
/// centre, (150 + 300 x, 150 + 300 y, 0, 1).
pub fn pos_3d(c: [F; 2]) -> V4 {
    [add(HALF_CELL, mul(CELL, c[0])), add(HALF_CELL, mul(CELL, c[1])), 0, geom::ONE]
}

/// A town landmark (`ccLandmark`, 0x30 bytes): the fields the AI reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Landmark {
    /// +0x00 its position (the dummy's, after `ccSetNaviMap`).
    pub pos: V4,
    /// +0x10 `name`, the number `ccNaviSearchLandmark` looks for.
    pub name: i32,
}

/// A Root Town's landmarks: `landMarkNum` (0x00378c38) and the table
/// `naviMapPtr` (0x00378c3c) points at, entries 0 to `landMarkNum` (the
/// game reads one past the last: the next town's first).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TownMap {
    pub num: i32,
    pub marks: Vec<Landmark>,
}

impl TownMap {
    /// Town `town`'s table (0 Mac Anu), the volume's `naviMarkTable` and
    /// `naviMapTownTable` (`tables::world`), before `ccSetNaviMap`.
    pub fn of(volume: piney_data::volume::Volume, town: usize) -> TownMap {
        let t = piney_data::tables::world::of(volume);
        TownMap {
            num: i32::from(t.navi_marks().get(town).copied().unwrap_or(0)),
            marks: t
                .navi_landmarks()
                .get(town)
                .copied()
                .unwrap_or_default()
                .iter()
                .map(|m| Landmark { pos: m.pos.map(f32::to_bits), name: m.name })
                .collect(),
        }
    }

    /// `naviMapPtr[i]` (a blank landmark outside the table).
    pub fn mark(&self, i: i32) -> Landmark {
        usize::try_from(i).ok().and_then(|i| self.marks.get(i)).copied().unwrap_or_default()
    }

    /// `ccSetNaviMap()` (gcmn 0x005130b0): landmarks 1 to `landMarkNum - 1`
    /// moved to their `DMY_markerNN` dummies. (It also clears
    /// `linkCheckMatrix` and sets `naviMainLineTbl`, the route search's.)
    pub fn set_navi_map(&mut self, w: &mut dyn NaviWorld) {
        for k in 1..self.num {
            let p = w.marker(k);
            if let Some(m) = self.marks.get_mut(k as usize) {
                m.pos = p;
            }
        }
    }

    /// `ccNaviSearchLandmark(name)` (gcmn 0x00513240): the first landmark
    /// from 1 below `landMarkNum` with that name; -1.
    pub fn search_landmark(&self, name: i32) -> i32 {
        (1..self.num).find(|&k| self.mark(k).name == name).unwrap_or(-1)
    }

    /// `ccNaviGetLandmarkPos(pos, n)` (gcmn 0x005132a0): landmark `n`'s
    /// position for 1 <= n <= `landMarkNum` (one past the last entry
    /// included); None (the game's 0, `pos` untouched) otherwise.
    pub fn landmark_pos(&self, n: i32) -> Option<V4> {
        (n > 0 && n <= self.num).then(|| self.mark(n).pos)
    }

    /// The distance on the ground to landmark `k` (the volume's `sqrtf`)
    /// when it is within 301 in height (`fabs((double)dz) > 301.0` skips it).
    fn ground_dist(&self, volume: Volume, k: i32, pos: V4) -> Option<F> {
        let mut d = vsub(self.mark(k).pos, pos);
        if f64::from(f32::from_bits(d[2])).abs() > 301.0 {
            return None;
        }
        d[2] = 0;
        d[3] = geom::ONE;
        Some(geom::length_on(volume, d))
    }

    /// `ccNaviSearchNearLandmark(pos)` (gcmn 0x00513300): the nearest
    /// landmark (1 below `landMarkNum`) within 10000 on the ground and 301
    /// in height, the first of equals; -1.
    pub fn search_near_landmark(&self, volume: Volume, pos: V4) -> i32 {
        let (mut best, mut idx) = (NEAR_MAX, -1);
        for k in 1..self.num {
            if let Some(d) = self.ground_dist(volume, k, pos)
                && geom::lt(d, best)
            {
                idx = k;
                best = d;
            }
        }
        idx
    }

    /// `ccNaviSearchNearLandmarkN(pos, n)` (gcmn 0x00513420): the `n`th
    /// nearest landmark, as the game keeps them: `n` slots of (index as a
    /// signed byte, distance) from (-1, 10000), each landmark within 301
    /// in height put in before the first slot it is nearer than, the last
    /// dropped. Returns the last slot's index.
    pub fn search_near_landmark_n(&self, volume: Volume, pos: V4, n: i32) -> i32 {
        let n = n.max(0) as usize;
        let mut list = vec![(-1i8, NEAR_MAX); n];
        for k in 1..self.num {
            let Some(d) = self.ground_dist(volume, k, pos) else { continue };
            if let Some(i) = (0..n).find(|&i| geom::lt(d, list[i].1)) {
                for j in (i + 1..n).rev() {
                    list[j] = list[j - 1];
                }
                list[i] = (k as i8, d);
            }
        }
        // ccMalloc(8 n) and the last slot: with n 0 the game reads before
        // the block.
        list.last().map_or(-1, |e| i32::from(e.0))
    }
}

/// The world's cells: `sub(a, b)` truncated through `fptosi`, as the
/// movers compare positions.
pub fn cell_delta(a: F, b: F) -> i32 {
    fptosi(sub(a, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 20-cell window of open floor with a wall across it at y 15, open
    /// at x 19.
    fn room() -> (Navi, PathMap, Vec<u8>) {
        let mut map2d = vec![0u8; 0x1_0000];
        for x in 10..30 {
            for y in 10..30 {
                map2d[(x * 256 + y) as usize] = 1;
            }
        }
        for x in 10..19 {
            map2d[(x * 256 + 15) as usize] = 4;
        }
        let navi = Navi { map_x: 10, map_y: 10, map_s: 20, ..Navi::new() };
        (navi, PathMap::default(), map2d)
    }

    struct Map2d(Vec<u8>);

    impl World for Map2d {
        fn w2p(&mut self, pos: V4) -> V4 {
            pos
        }
        fn p2w(&mut self, pos: V4) -> V4 {
            pos
        }
        fn land(&mut self, _: V4, _: u32) -> F {
            0
        }
        fn hit_attribute(&mut self) -> u32 {
            0
        }
        fn line(&mut self, _: V4, _: V4, _: u32, _: i32) -> F {
            geom::MINUS_ONE
        }
        fn collide(&mut self, _: usize, _: &mut crate::world::CharHit) -> i32 {
            0
        }
        fn hit_char_type(&mut self) -> u32 {
            0
        }
        fn hit_switch(&mut self, _: usize, _: &mut crate::world::CharHit, _: bool) {}
        fn camera_deg(&mut self, _: V4, _: i16) -> bool {
            true
        }
        fn camera_transparency(&mut self, _: V4, _: F, _: F, _: F, _: F) -> F {
            geom::ONE
        }
        fn anim_set(&mut self, _: usize, _: crate::world::AnmSlot, _: &str) {}
        fn anim_frame(&mut self, _: usize, _: crate::world::AnmSlot) -> u16 {
            0
        }
        fn anim_forward(&mut self, _: usize, _: crate::world::AnmSlot, _: u16) -> i16 {
            0
        }
        fn anim_notes(&mut self, _: usize, _: crate::world::AnmSlot) -> Vec<crate::world::Note> {
            Vec::new()
        }
    }

    impl NaviWorld for Map2d {
        fn map_2d_info(&mut self) -> [i32; 3] {
            [10, 10, 20]
        }
        fn map_2d(&mut self) -> &[u8] {
            &self.0
        }
        fn route_search_by_map(&mut self, _: &mut Navi, _: &mut i16, _: V4, _: V4) -> i32 {
            0
        }
        fn navi_point(&mut self, _: i16) -> V4 {
            geom::VF0
        }
        fn marker(&mut self, _: i32) -> V4 {
            geom::VF0
        }
    }

    #[test]
    fn a_way_round_the_wall() {
        let (mut navi, mut map, map2d) = room();
        let mut w = Map2d(map2d);
        map.set_path_finding_map(2, &mut w);
        assert_eq!(map.link_at(10, 10), 2 | 4);
        assert_eq!(map.link_at(12, 15), 0);
        let mut finish = 1;
        let r = navi.path_finding_cells(&mut finish, 2, &mut map, &mut w, 10, 10, 10, 19);
        assert_eq!(r, 1);
        assert_eq!(finish, 0);
        // Round the wall through its gap: 27 cells after the start, the
        // wave's 28 steps (20 * 14 / 10) just enough.
        assert_eq!(navi.beacon.len(), 27);
        assert_eq!(*navi.beacon.last().unwrap(), [10, 19]);
        assert!(navi.beacon.contains(&[19, 15]));
        assert_eq!(navi.step, 27);
        assert!(navi.beacon.iter().all(|&[x, y]| map.link_at(i32::from(x), i32::from(y)) & 0x80 != 0));
        assert_eq!(map.link_at(10, 19) & 0x20, 0x20);
        // Too far for a 10-cell window's 14 steps.
        let mut small = Navi { map_x: 10, map_y: 10, map_s: 10, ..Navi::new() };
        assert_eq!(small.path_finding_cells(&mut finish, 2, &mut map, &mut w, 10, 10, 10, 19), 0);
    }

    #[test]
    fn a_long_way_runs_over_beacon_num() {
        let mut navi = Navi::new();
        let tbl: Vec<[u8; 4]> =
            (0..52u8).map(|k| [k, 0, k.wrapping_sub(1), if k == 51 { 255 } else { k + 1 }]).collect();
        assert_eq!(navi.set_beacon(&tbl), 52);
        assert_eq!(navi.step, 52);
        assert_eq!(navi.pad, [48, 49]);
        // beacon.num was 52, then route[50] and route[51] landed on it.
        assert_eq!(navi.beacon_num, i32::from_le_bytes([50, 51, 0, 0]));
        assert_eq!(navi.route_byte(51), 51);
    }

    /// Mac Anu's table from the disc (skipped without it; `PINEY_ISO`
    /// points elsewhere): 72 landmarks, the named ones where the AI's shop
    /// walks look for them.
    #[test]
    fn mac_anu_from_the_disc() {
        let p = std::env::var_os("PINEY_ISO").map(std::path::PathBuf::from).unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso")
        });
        if !p.exists() {
            return;
        }
        let mut iso = piney_data::iso::Iso::open(p).unwrap();
        let town = TownMap::of(iso.volume().unwrap(), 0);
        assert_eq!(town.num, 72);
        assert_eq!(town.marks.len(), 73);
        let found: Vec<i32> = (1..=6).map(|n| town.search_landmark(n)).collect();
        assert_eq!(found, [44, 48, 49, 46, 47, 50]);
        assert_eq!(town.search_landmark(7), -1);
    }

    #[test]
    fn nearest_landmarks() {
        let at = |x: f32, z: f32| [x.to_bits(), 0, z.to_bits(), geom::ONE];
        let town = TownMap {
            num: 4,
            marks: vec![
                Landmark::default(),
                Landmark { pos: at(100.0, 0.0), name: 7 },
                Landmark { pos: at(50.0, 400.0), name: 8 },
                Landmark { pos: at(30.0, 0.0), name: 9 },
                Landmark::default(),
            ],
        };
        let o = at(0.0, 0.0);
        assert_eq!(town.search_near_landmark(Volume::Inf, o), 3);
        assert_eq!(town.search_near_landmark_n(Volume::Inf, o, 2), 1);
        assert_eq!(town.search_near_landmark_n(Volume::Inf, o, 3), -1);
        assert_eq!(town.search_landmark(8), 2);
        assert_eq!(town.landmark_pos(4), Some(Landmark::default().pos));
        assert_eq!(town.landmark_pos(5), None);
    }
}
