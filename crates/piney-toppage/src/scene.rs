//! The hand-off to the field game: what `ccGame::ChangeArea(0, lastTown)`
//! leaves in `ccGame` after Log in's `ChangeRequest(5, 8)`.
//!
//! `ChangeRequest(5, 8)` queues mode 5 (`ccSetupNewGame`, GCMN.PRG) and,
//! as every request but 6 does, `InitScene` (0x00167320) sets every scene
//! member to -1. `ChangeArea(a, n)` (0x001674a0) is `ChangeScene`
//! (0x00167380) by `a`: 0 a town, `(0, n, -1, -1, -1, -1)`; 1 a field,
//! `(1, -2, n, -1, -1, -1)`; 2 a dungeon, `(2, -2, -2, n, 0, 0)`; else
//! `(0, 0, -1, -1, -1, -1)`. `ChangeScene` keeps each member's old value as
//! its `Prev`, sets those given as -1 or more (a town also into
//! `saveData.lastTown`, and the server from the town by `@1489`), clears the
//! battle members and asks `ChangeRequest(6, 7)`: `ccSetupGameCtrl` after
//! the field game's overlay is loaded.

/// `ccGame`'s scene members (DWARF +0x14 .. +0x48).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scene {
    pub area: i32,
    pub area_prev: i32,
    pub server: i32,
    pub town: i32,
    pub field: i32,
    pub dungeon: i32,
    pub floor: i32,
    pub block: i32,
    pub server_prev: i32,
    pub town_prev: i32,
    pub field_prev: i32,
    pub dungeon_prev: i32,
    pub floor_prev: i32,
    pub block_prev: i32,
}

impl Scene {
    /// `ccGame::InitScene`: everything -1.
    pub fn cleared() -> Self {
        Scene {
            area: -1,
            area_prev: -1,
            server: -1,
            town: -1,
            field: -1,
            dungeon: -1,
            floor: -1,
            block: -1,
            server_prev: -1,
            town_prev: -1,
            field_prev: -1,
            dungeon_prev: -1,
            floor_prev: -1,
            block_prev: -1,
        }
    }

    /// `ChangeScene(a, t, fd, d, f, b)` on these members; `servers` is
    /// `@1489`. Returns the `saveData.lastTown` it writes, if any.
    #[allow(clippy::too_many_arguments)]
    pub fn change_scene(&mut self, a: i32, t: i32, fd: i32, d: i32, f: i32, b: i32, servers: &[i32; 8]) -> Option<i8> {
        let mut last_town = None;
        self.town_prev = self.town;
        if t >= -1 {
            self.town = t;
            last_town = Some(t as i8);
        }
        self.server_prev = self.server;
        if self.town >= 0 {
            // The table is a local copy of 8 entries; a town past it reads
            // the stack.
            if let Some(&s) = servers.get(self.town as usize) {
                self.server = s;
            }
        }
        self.field_prev = self.field;
        if fd >= -1 {
            self.field = fd;
        }
        self.dungeon_prev = self.dungeon;
        if d >= -1 {
            self.dungeon = d;
        }
        self.floor_prev = self.floor;
        if f >= -1 {
            self.floor = f;
        }
        self.block_prev = self.block;
        if b >= -1 {
            self.block = b;
        }
        self.area_prev = self.area;
        if a >= -1 {
            self.area = a;
        }
        last_town
    }

    /// `ChangeArea(a, n)`.
    pub fn change_area(&mut self, a: i32, n: i32, servers: &[i32; 8]) -> Option<i8> {
        match a {
            0 => self.change_scene(0, n, -1, -1, -1, -1, servers),
            1 => self.change_scene(1, -2, n, -1, -1, -1, servers),
            2 => self.change_scene(2, -2, -2, n, 0, 0, servers),
            _ => self.change_scene(0, 0, -1, -1, -1, -1, servers),
        }
    }

    /// The scene Log in leaves: `InitScene`, then `ChangeArea(0, town)`.
    pub fn login(town: i32, servers: &[i32; 8]) -> Self {
        let mut s = Scene::cleared();
        s.change_area(0, town, servers);
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVERS: [i32; 8] = [0, 1, 2, 3, 4, 0, 1, 2];

    #[test]
    fn login_to_a_town() {
        let s = Scene::login(3, &SERVERS);
        assert_eq!((s.area, s.town, s.server), (0, 3, 3));
        assert_eq!((s.field, s.dungeon, s.floor, s.block), (-1, -1, -1, -1));
        assert_eq!((s.area_prev, s.town_prev, s.server_prev), (-1, -1, -1));
        let s = Scene::login(6, &SERVERS);
        assert_eq!(s.server, 1);
    }

    #[test]
    fn a_dungeon() {
        let mut s = Scene::cleared();
        s.change_area(2, 7, &SERVERS);
        assert_eq!((s.area, s.town, s.dungeon, s.floor, s.block), (2, -1, 7, 0, 0));
    }
}
