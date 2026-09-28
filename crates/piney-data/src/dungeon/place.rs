//! Where the game puts a random dungeon's rooms and doors
//! (`docs/engine/dungeon.md`). `DUNGEON::SetRoom` (`INF gcmn.prg:0x005c1ca0`)
//! gives the room's `ANM_` animation `T(pos) * Rz(rotate)`, the `ROOM_INFO`
//! row's angle in radians turning +x towards +y as [`Mat4::from_rotation_z`]
//! does (`sceVu0RotMatrixZ`, main 0x00110a30). Each exit is marked by an
//! object 600 inside the edge, +y out ([`Exit`]): a gate wall, or a door dummy
//! where `DUNGEON::SetDoor` (0x005c7c30) plays the door, open when the room is
//! empty. The game draws only the room the player is in.

use std::collections::HashMap;

use glam::{Mat4, Vec3};

use super::{Doors, EAST, Floor, NORTH, Room, SOUTH, Tables, WEST};
use crate::ccs::Ccs;
use crate::scene::{Scene, Transform};
use crate::{Bytes, Error, Result, format_err};

/// The matrix `SetRoom` gives a room's animation: `T(pos) * Rz(rotate)`.
pub fn room_matrix(room: &Room) -> Mat4 {
    let rotate = room.model.as_ref().map_or(0.0, |m| m.rotate.radians());
    Mat4::from_translation(Vec3::new(room.pos[0], room.pos[1], 0.0)) * Mat4::from_rotation_z(rotate)
}

/// Which key an animation is posed from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frame {
    First,
    /// The last key of every track: where a play-once animation rests.
    Last,
}

/// One object an animation places.
#[derive(Clone, Copy, Debug)]
pub struct Piece {
    /// The controller's object: an ExtObj copy, or the object itself.
    pub controller: u32,
    /// The object whose models it draws.
    pub target: u32,
    /// Its own transform (the controller's `T * Rx * Ry * Rz * S`).
    pub own: Mat4,
    /// Its matrix in the animation's space, through its parents.
    pub world: Mat4,
    /// Its transparency track (bits 9-11 of the controller), 1 without one.
    pub alpha: f32,
}

/// Anime sub-chunk kind of an object controller.
const OBJECT_CONTROLLER: u16 = 0x0102;

/// Every object controller of Anime chunk `anime`, posed at its first or
/// last key. A controller is u32 object, u32 flags, then position,
/// rotation (degrees) and scale tracks picked by flag bits 0-2, 3-5, 6-8 and
/// a float (the object's transparency) by bits 9-11: kind 1 is one value, 2
/// a u32 key count and (u32 frame, value) keys. A rotation's last key is
/// where the game's composed turns end up, to the quantisation of its keys.
pub fn pieces(c: &Ccs, sc: &Scene, anime: &str, frame: Frame) -> Result<Vec<Piece>> {
    let a = c
        .find_object(anime)
        .and_then(|o| sc.animes.iter().find(|a| a.object == o))
        .ok_or_else(|| Error::NotFound(format!("{}: no Anime chunk {anime}", c.name)))?;
    let d = &c.data;
    let words = d.u32_at(a.offset + 16)? as usize;
    let mut p = a.offset + 20;
    let end = p + 4 * words;
    let mut tracks = Vec::new();
    let mut alpha = HashMap::new();
    while p < end {
        let kind = d.u32_at(p)? as u16;
        let n = d.u32_at(p + 4)? as usize;
        if kind == OBJECT_CONTROLLER {
            let obj = d.u32_at(p + 8)?;
            let flags = d.u32_at(p + 12)?;
            let mut q = p + 16;
            let mut vals = [Vec3::ZERO, Vec3::ZERO, Vec3::ONE];
            let v3 = |at: usize| -> Result<Vec3> { Ok(Vec3::new(d.f32_at(at)?, d.f32_at(at + 4)?, d.f32_at(at + 8)?)) };
            for (k, val) in vals.iter_mut().enumerate() {
                match flags >> (3 * k) & 7 {
                    0 => {}
                    1 => {
                        *val = v3(q)?;
                        q += 12;
                    }
                    2 => {
                        let keys = d.u32_at(q)? as usize;
                        if keys > 0 {
                            let key = if frame == Frame::Last { keys - 1 } else { 0 };
                            *val = v3(q + 4 + 16 * key + 4)?;
                        }
                        q += 4 + 16 * keys;
                    }
                    other => return format_err(format!("{anime}: controller kind {other}")),
                }
            }
            match flags >> 9 & 7 {
                0 => {}
                1 => {
                    alpha.insert(obj, d.f32_at(q)?);
                }
                2 => {
                    let keys = d.u32_at(q)? as usize;
                    if keys > 0 {
                        let key = if frame == Frame::Last { keys - 1 } else { 0 };
                        alpha.insert(obj, d.f32_at(q + 4 + 8 * key + 4)?);
                    }
                }
                other => return format_err(format!("{anime}: controller kind {other}")),
            }
            tracks.push((obj, Transform { pos: vals[0], rot: vals[1], scale: vals[2] }));
        }
        p += 8 + 4 * n;
    }
    if p != end {
        return format_err(format!("{anime}: sub-chunks overrun at 0x{p:x}"));
    }
    let own: HashMap<u32, Mat4> = tracks.iter().map(|&(o, t)| (o, t.matrix())).collect();
    Ok(sc
        .controller_worlds(&tracks)
        .into_iter()
        .map(|(controller, target, world)| Piece {
            controller,
            target,
            own: own[&controller],
            world,
            alpha: alpha.get(&controller).copied().unwrap_or(1.0),
        })
        .collect())
}

/// What marks an exit of a room model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExitKind {
    /// A door dummy: `SetDoor` puts the door animation here.
    Door,
    /// A gate wall, drawn with the room.
    Gate,
}

/// One exit of a room model.
#[derive(Clone, Copy, Debug)]
pub struct Exit {
    pub kind: ExitKind,
    /// The marker's object (the door dummy or the gate wall).
    pub target: u32,
    /// The marker's matrix in the room's space.
    pub world: Mat4,
    /// Its own matrix: what `SetDoor` places a door with.
    pub own: Mat4,
}

impl Exit {
    /// The side the exit faces out of, in the room's own space.
    pub fn side(&self) -> u8 {
        side_of(self.world.transform_vector3(Vec3::Y))
    }
}

/// The side a direction points out of (north is -y, east +x); 0 when it is
/// not along an axis.
pub fn side_of(v: Vec3) -> u8 {
    let v = v.normalize_or_zero();
    if v.y < -0.99 {
        NORTH
    } else if v.y > 0.99 {
        SOUTH
    } else if v.x < -0.99 {
        WEST
    } else if v.x > 0.99 {
        EAST
    } else {
        0
    }
}

/// The exits among a room animation's pieces, found by name as `SetDoor`
/// finds them.
pub fn exits(c: &Ccs, doors: &Doors, pieces: &[Piece]) -> Vec<Exit> {
    pieces
        .iter()
        .filter_map(|p| {
            let name = c.object_name(p.target)?;
            let kind = if name.starts_with(doors.dummy) {
                ExitKind::Door
            } else if name.starts_with(doors.gate) {
                ExitKind::Gate
            } else {
                return None;
            };
            Some(Exit { kind, target: p.target, world: p.world, own: p.own })
        })
        .collect()
}

/// One piece of a floor, placed in the world.
#[derive(Clone, Copy, Debug)]
pub struct Placed {
    /// The room it belongs to (a door belongs to the room whose dummy it
    /// stands on).
    pub room: usize,
    /// The object whose models it draws.
    pub target: u32,
    pub world: Mat4,
    pub alpha: f32,
    /// Part of a door animation rather than the room's own.
    pub door: bool,
}

/// A floor as the game builds it room by room, all at once: each room's
/// animation at its first frame under [`room_matrix`], and the type's door
/// animation posed at `doors` at each of its door dummies.
pub fn floor(c: &Ccs, sc: &Scene, tables: &Tables, dtype: u8, floor: &Floor, doors: Frame) -> Result<Vec<Placed>> {
    let door = pieces(c, sc, tables.doors.anime[dtype as usize], doors)?;
    let mut rooms: HashMap<&str, Vec<Piece>> = HashMap::new();
    let mut out = Vec::new();
    for room in &floor.rooms {
        let Some(m) = &room.model else { continue };
        if !rooms.contains_key(m.name) {
            rooms.insert(m.name, pieces(c, sc, m.name, Frame::First)?);
        }
        let own = &rooms[m.name];
        let root = room_matrix(room);
        for p in own {
            out.push(Placed { room: room.index, target: p.target, world: root * p.world, alpha: p.alpha, door: false });
        }
        for e in exits(c, &tables.doors, own).iter().filter(|e| e.kind == ExitKind::Door) {
            let at = root * e.own;
            for p in &door {
                out.push(Placed {
                    room: room.index,
                    target: p.target,
                    world: at * p.world,
                    alpha: p.alpha,
                    door: true,
                });
            }
        }
    }
    Ok(out)
}
