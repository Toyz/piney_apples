//! What `DUNGEON::SetObject` (gcmn 0x005c70c0) and `SetAnmObject`
//! (0x005c3ad0) stand in a room after its doors' turn: `object[10]`
//! (+0xd3588, a lake's statues and flowers, `ccClump`s), `anmobj[10]`
//! (types 3 and 7's walls) and `anmobj2[10]` (types 0-3's animated
//! objects), each joining the hit list; `Draw` steps the anms and draws,
//! for k under 10, `object[k]`, `anmobj[k]`, `anmobj2[k]` on objLayer,
//! fogged by depth, with the file's palette swaps.

use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;

use glam::Mat4;
use piney_data::Result;

use super::sparks::fieldrand;
use super::{BuildCx, DrawCx, Hook, Joined, Kind, MapSystem, Others, Piece, Request, RoomCx, RoomPart, StepCx};
use crate::draw;
use crate::dungeon::{Anm, Instance, Rooms, Slot, placed};
use crate::ee::V4;
use crate::hit::{HitModel, UNIT};

/// `object[10]`, `anmobj[10]`, `anmobj2[10]`: at most ten of each.
const PIECES: usize = 10;
/// `SetObject`'s lake pieces: each dummy's clump, then `OBJ_0ps4*`'s
/// picked by `fieldrand(4)`.
pub const LAKE_PIECES: [(&str, &str); 4] = [
    ("OBJ_0ps0*", "CMP_o_statue_l2_"),
    ("OBJ_0ps1*", "CMP_o_statue_l3_"),
    ("OBJ_0ps2*", "CMP_o_flower_l2_"),
    ("OBJ_0ps3*", "CMP_o_flower_l3_"),
];
const LAKE_PICKED: &str = "OBJ_0ps4*";
pub const LAKE_PICKS: [&str; 4] = ["CMP_o_statue_l0_", "CMP_o_statue_l1_", "CMP_o_flower_l0_", "CMP_o_flower_l1_"];

/// The type's pieces: its animated objects' dummy and anm, its walls' (two
/// searches), the lakes' statues.
#[derive(Clone, Copy, Debug)]
pub struct PiecesDef {
    pub anm: Option<(&'static str, &'static str)>,
    pub walls: Option<[(&'static str, &'static str); 2]>,
    pub statues: bool,
}

/// `object[k]`: a lake's statue or flower, `EntryObject`'s `ccClump` of
/// the dungeon file's `name`, its hits on (`HitEnable(1)`) and every
/// node's at its matrix `m`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clump {
    pub name: &'static str,
    pub nodes: Vec<u32>,
    pub m: [V4; 4],
}

/// Which list a piece is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum List {
    Object,
    Wall,
    Anm,
}

impl List {
    fn of(n: usize) -> (List, usize) {
        let l = match n / PIECES {
            0 => List::Object,
            1 => List::Wall,
            _ => List::Anm,
        };
        (l, n % PIECES)
    }

    fn at(self, k: usize) -> usize {
        self as usize * PIECES + k
    }

    /// The hit models' tag, as the game's lists kept them apart.
    fn tag(self, k: usize) -> u32 {
        0x10 * (1 + self as u32) + k as u32
    }
}

/// The room's pieces.
#[derive(Clone)]
pub struct Pieces {
    def: &'static PiecesDef,
    rooms: Rc<Rooms>,
    pub objects: Vec<Clump>,
    /// `anmobj[0..10]` (walls), `anmobj2[0..10]` (animated objects).
    pub walls: Vec<Anm>,
    pub anms: Vec<Anm>,
}

impl Pieces {
    pub fn new(def: &'static PiecesDef, cx: &mut BuildCx) -> Result<Pieces> {
        let rooms = cx.rooms.expect("a dungeon's pieces").clone();
        Ok(Pieces { def, rooms, objects: Vec::new(), walls: Vec::new(), anms: Vec::new() })
    }

    /// `EntryObject(dummy, f, i, name, num)`: for `num` under 10, the clump
    /// at the dummy's own matrix turned and moved with the room, its hits
    /// joining the list (the lakes' palette swaps by `GetBG` are the file's,
    /// [`Rooms::clut_swaps`]).
    fn entry_object(&mut self, d: &Instance, slot: &Slot, name: &'static str, num: usize, joined: &mut Vec<Joined>) {
        if num >= PIECES {
            return;
        }
        let nodes = self.rooms.clump_nodes(name);
        let k = self.objects.len();
        joined.extend((0..nodes.len()).map(|n| Joined::Piece(Kind::Pieces, List::Object.at(k), n)));
        self.objects.push(Clump { name, nodes, m: placed(&d.local, slot, true) });
    }

    /// `SetObject(f, i)`. The lakes: a clump at each `OBJ_0ps0*` ..
    /// `OBJ_0ps3*` dummy of the room on the dungeon's level, then
    /// `OBJ_0ps4*`'s each picked by `fieldrand(4)`. Walls: an anm at each
    /// `OBJ_0paw0*`, then `OBJ_0paw1*` dummy (at most ten). The game stores
    /// each new anm's matrix by the dummy's index in its own search, so a
    /// paw1 wall moves the paw0 wall of its index and keeps its own unit
    /// matrix, unless the room has no paw0 walls (docs/engine/dungeon.md).
    fn set_object(&mut self, room: &Anm, slot: &Slot, cx: &mut RoomCx) {
        if self.def.statues && cx.level == cx.floor {
            let mut num = 0;
            for (pattern, name) in LAKE_PIECES {
                for d in self.rooms.subst(room, pattern) {
                    self.entry_object(&d, slot, name, num, cx.joined);
                    num += 1;
                }
            }
            for d in self.rooms.subst(room, LAKE_PICKED) {
                let k = fieldrand(cx.rng, 4) as usize;
                self.entry_object(&d, slot, LAKE_PICKS[k], num, cx.joined);
                num += 1;
            }
        }
        let Some(walls) = self.def.walls else { return };
        let mut s1 = 0usize;
        for (pattern, name) in walls {
            for (s0, d) in self.rooms.subst(room, pattern).into_iter().enumerate() {
                let Some(anm) = self.rooms.make_anm_in(false, name, &UNIT, 1) else { return };
                self.walls.push(anm);
                let m = placed(&d.local, slot, true);
                if let Some(a) = self.walls.get(s0) {
                    let objects = self.rooms.pose(&a.play, &m);
                    let a = &mut self.walls[s0];
                    a.root = m;
                    a.objects = objects;
                }
                let a = &mut self.walls[s1];
                a.hit_world = a.objects.iter().map(|o| o.world).collect();
                cx.joined.extend((0..a.objects.len()).map(|o| Joined::Piece(Kind::Pieces, List::Wall.at(s1), o)));
                s1 += 1;
                if s1 == PIECES {
                    return;
                }
            }
        }
    }

    /// `SetAnmObject(f, i)`: the type's anm at each of its dummies (at most
    /// ten), one step on, turned and moved with the room, its hits on.
    fn set_anm_object(&mut self, room: &Anm, slot: &Slot, cx: &mut RoomCx) {
        let Some((pattern, name)) = self.def.anm else { return };
        for d in self.rooms.subst(room, pattern).into_iter().take(PIECES) {
            let m = placed(&d.local, slot, true);
            let Some(anm) = self.rooms.make_anm_in(false, name, &m, 1) else { return };
            let k = self.anms.len();
            cx.joined.extend((0..anm.objects.len()).map(|o| Joined::Piece(Kind::Pieces, List::Anm.at(k), o)));
            self.anms.push(anm);
        }
    }

    /// An anm's objects' models at their world matrices.
    fn anm_pieces(&self, a: &Anm) -> Vec<(u32, Mat4)> {
        let file = &*self.rooms.file;
        let mut out = Vec::new();
        for o in &a.objects {
            let Some(models) = self.rooms.obj_models.get(&o.target) else { continue };
            let world = draw::mat(&o.world);
            for &model in models {
                if file.models.get(&model).is_none_or(|i| i.mtype & 0x600 == 0x600) {
                    continue;
                }
                out.push((model, world));
            }
        }
        out
    }
}

impl MapSystem for Pieces {
    /// `Draw`: the anms one step on, posed again; then (and `Again`) for k
    /// under 10 `object[k]`, `anmobj[k]`, `anmobj2[k]`.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        match hook {
            Hook::Draw => {
                let file = self.rooms.file.clone();
                for a in self.walls.iter_mut().chain(&mut self.anms) {
                    a.play.forward(&file);
                }
                let posed: Vec<Vec<Instance>> =
                    self.walls.iter().chain(&self.anms).map(|a| self.rooms.pose(&a.play, &a.root)).collect();
                for (a, objects) in self.walls.iter_mut().chain(&mut self.anms).zip(posed) {
                    a.objects = objects;
                }
            }
            Hook::Again => {}
            _ => return,
        }
        for k in 0..PIECES {
            for (list, n) in
                [(List::Object, self.objects.len()), (List::Wall, self.walls.len()), (List::Anm, self.anms.len())]
            {
                if k < n {
                    cx.put(Kind::Pieces, Piece::Item(list.at(k)));
                }
            }
        }
    }

    fn draw<'a>(&'a self, piece: Piece, cx: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Item(n) = piece else { return None };
        let (pieces, rows) = match List::of(n) {
            (List::Object, k) => {
                let c = self.objects.get(k)?;
                (self.rooms.clump_pieces(&c.nodes, &c.m), HashMap::new())
            }
            (List::Wall, k) => {
                let a = self.walls.get(k)?;
                (self.anm_pieces(a), a.play.uv_rows(&self.rooms.file))
            }
            (List::Anm, k) => {
                let a = self.anms.get(k)?;
                (self.anm_pieces(a), a.play.uv_rows(&self.rooms.file))
            }
        };
        Some(Request::Pieces {
            file: &self.rooms.file,
            pieces,
            layer: draw::OBJ_LAYER,
            rows: Cow::Owned(rows),
            fog: cx.fog,
            clut_swaps: &self.rooms.clut_swaps,
            edit: None,
        })
    }

    /// `SetObject`'s and `SetAnmObject`'s; gone with the room.
    fn room(&mut self, part: RoomPart, cx: &mut RoomCx, _: &mut Others) {
        match part {
            RoomPart::Clear => {
                self.objects.clear();
                self.walls.clear();
                self.anms.clear();
            }
            RoomPart::Object | RoomPart::AnmObject => {
                let (Some(room), Some(slot)) = (cx.room, cx.slot) else { return };
                if part == RoomPart::Object {
                    self.set_object(room, slot, cx);
                } else {
                    self.set_anm_object(room, slot, cx);
                }
            }
            _ => {}
        }
    }

    /// Node or object `o` of piece `n`'s hit models, `type` 1.
    fn hits(&self, n: usize, o: usize) -> Vec<HitModel> {
        match List::of(n) {
            (List::Object, k) => {
                let Some(c) = self.objects.get(k) else { return Vec::new() };
                self.rooms.clump_hits(&c.nodes, &c.m, List::Object.tag(k), o)
            }
            (List::Wall, k) => {
                self.walls.get(k).map_or_else(Vec::new, |a| self.rooms.obj_hits(a, o, List::Wall.tag(k)))
            }
            (List::Anm, k) => self.anms.get(k).map_or_else(Vec::new, |a| self.rooms.obj_hits(a, o, List::Anm.tag(k))),
        }
    }

    /// The names of piece `n`'s entry `o`'s hit models (for the checks).
    fn hit_names(&self, n: usize, o: usize) -> Vec<String> {
        let count = self.hits(n, o).len();
        let name = match List::of(n) {
            (List::Object, k) => self.objects.get(k).map(|c| c.name.to_string()),
            (List::Wall, k) => self.walls.get(k).and_then(|a| a.objects.get(o)).map(|i| self.rooms.name(i.controller)),
            (List::Anm, k) => self.anms.get(k).and_then(|a| a.objects.get(o)).map(|i| self.rooms.name(i.controller)),
        };
        name.map_or_else(Vec::new, |n| vec![n; count])
    }
}
