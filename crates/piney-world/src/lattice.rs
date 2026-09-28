//! `ccLattice` (gcmn lattice.cpp): a ribbon of vertex rows, the newest at
//! the head, each fading over the frames after it was laid; the boss's
//! cross swing leaves one behind its sword (`ccBoss01::DrawCross`, gcmn
//! 0x0047c390, `new ccLattice(2, 7)`), the party's weapons theirs
//! (`ccSpcChar::ArmsEffect`, [`crate::arms`]: 2 columns, 3 for job 4).
//!
//! ```text
//! ccLattice(cols, type) (0x00579ad0) Init: 16 rows of `cols` vertices
//!   (2 or 3; a row 0x70 bytes: +0x20 the world places, +0x50 the screen
//!   places, +0x80 the made flags, +0x86 the life), the type +4 (the
//!   colour MakePacket reads, latticeAttributeColorTable[type] 0x006519b0),
//!   the alpha 48, no row made, head and tail 0
//! ClearCnt (0x00579c50): with +0x72c set (ClearArmsEffect), every row dead,
//!   the tail onto the head and +0x72c cleared
//! NextVertex (0x0057a3a0): head == tail marks the first row; the head one
//!   back (15 after 0), the tail with it when they meet; the new head
//!   row's life 6; rows made counted to 16 (then `full`)
//! SetPos(p, col) (0x0057a300): the head row's vertex col (the tail's too
//!   for the first row)
//! Disp (0x0057a470), on effLayer: for each pair of neighbouring columns,
//!   from the head row to the tail row, a strip of each row's two vertices
//!   (MakePacket 0x00579cb0: each vertex
//!   through ccTransPosFW2LW and the view's RotTransPers; RGBA the colour
//!   with alpha life * 48 / 5 below life 5, else 48; ADC (no triangle) for
//!   the strip's first pair, a vertex behind the eye or more than 640 x
//!   576 pixels off XYOFFSET; each vertex made once a Disp, its world z
//!   summed, so a middle column's once); SendPacket (0x0057a0e0): only
//!   once 16 rows have been made and with 2 pairs or more (head apart from
//!   tail always gives 2) - TEST 0x73001, ZBUF (ccSys +0xbc8), ALPHA 0x44,
//!   PRIM 0x4c (a gouraud, blended strip), ccDLSort::Add(the z sum over
//!   twice the pairs); the first-row flag and the vertices' made flags
//!   cleared - then each row's life down by 1: a row reaching 0 moves the
//!   tail one row back (all 16 dead: the tail onto the head)
//! ```

use crate::ee::{self, F, V4};

/// `latticeAttributeColorTable` (gcmn 0x006519b0): RGBA by type.
const COLOURS: [[u8; 4]; 8] = [
    [0xf0, 0xf0, 0xf0, 0x80],
    [0, 0xf0, 0xf0, 0x80],
    [0xf0, 0x80, 0, 0x80],
    [0, 0, 0xf0, 0x80],
    [0xf0, 0, 0, 0x80],
    [0, 0xf0, 0, 0x80],
    [0xf0, 0xf0, 0, 0x80],
    [0xf0, 0, 0xf0, 0x80],
];

const ROWS: usize = 16;
/// The most vertices a row holds (+0x20 to +0x50).
const COLS: usize = 3;
/// `+0x18`: the alpha of a row not yet fading.
const ALPHA: i32 = 48;
/// A new row's life.
const LIFE: i16 = 6;

/// A `ccLattice` (0x740 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct Lattice {
    /// +0x04 the colour's type, +0x12 the vertices a row.
    pub ty: i32,
    pub cols: usize,
    /// +0x20 each row's vertices, +0x86 its life.
    pub rows: [[V4; COLS]; ROWS],
    pub life: [i16; ROWS],
    /// +0x00 the first row pending (head met tail), +0x728 rows made,
    /// +0x72a all 16 made, +0x72c `ClearCnt` pending, +0x72e head, +0x730
    /// tail.
    pub first: bool,
    pub made: i16,
    pub full: bool,
    pub clear: bool,
    pub head: usize,
    pub tail: usize,
}

/// One vertex of the strip `Disp` sends: world place, RGBA, and whether
/// the GS draws a triangle with it (ADC clear) as far as `MakePacket`'s
/// own tests go (the view's tests are the caller's).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StripVertex {
    pub pos: V4,
    pub rgba: [u8; 4],
    pub restart: bool,
}

impl Lattice {
    /// `new ccLattice(cols, type)`.
    pub fn new(cols: usize, ty: i32) -> Lattice {
        Lattice {
            ty,
            cols: cols.clamp(2, COLS),
            rows: [[[0; 4]; COLS]; ROWS],
            life: [0; ROWS],
            first: false,
            made: 0,
            full: false,
            clear: false,
            head: 0,
            tail: 0,
        }
    }

    /// `ClearCnt()`: with the clear pending, every row dead and the tail
    /// on the head.
    pub fn clear_cnt(&mut self) {
        if !self.clear {
            return;
        }
        self.life = [0; ROWS];
        self.clear = false;
        self.tail = self.head;
    }

    /// `NextVertex()`.
    pub fn next_vertex(&mut self) {
        if self.head == self.tail {
            self.first = true;
        }
        self.head = (self.head + ROWS - 1) % ROWS;
        if self.head == self.tail {
            self.tail = (self.tail + ROWS - 1) % ROWS;
        }
        self.life[self.head] = LIFE;
        if !self.full {
            self.made += 1;
            if self.made >= ROWS as i16 {
                self.full = true;
            }
        }
    }

    /// `SetPos(p, col)`.
    pub fn set_pos(&mut self, p: V4, col: usize) {
        if col >= self.cols {
            return;
        }
        let Some(c) = self.rows.get_mut(self.head).and_then(|r| r.get_mut(col)) else { return };
        *c = p;
        if self.first
            && let Some(c) = self.rows.get_mut(self.tail).and_then(|r| r.get_mut(col))
        {
            *c = p;
        }
    }

    /// `Disp()`: the strip it sends this frame (empty when it sends none)
    /// and its sort key (bits), the rows then aged.
    pub fn disp(&mut self) -> (Vec<StripVertex>, F) {
        let mut out = Vec::new();
        // +0x0c: MakePacket adds each vertex's world z as it makes it, once
        // a Disp (+0x80, the made flags).
        let mut zsum: F = 0;
        let mut made = [[false; COLS]; ROWS];
        if self.head != self.tail {
            let colour = COLOURS[(self.ty & 7) as usize];
            for c in 0..self.cols - 1 {
                let mut row = self.head;
                let mut first = true;
                loop {
                    let a = if self.life[row] < 5 { i32::from(self.life[row]) * ALPHA / 5 } else { ALPHA };
                    let rgba = [colour[0], colour[1], colour[2], a as u8];
                    for k in [c, c + 1] {
                        let pos = self.rows[row][k];
                        if !made[row][k] {
                            made[row][k] = true;
                            zsum = ee::add(zsum, pos[2]);
                        }
                        out.push(StripVertex { pos, rgba, restart: first });
                    }
                    first = false;
                    if row == self.tail {
                        break;
                    }
                    row = (row + 1) % ROWS;
                }
            }
        }
        // SendPacket: only once the ribbon is full, with two pairs or more
        // (+0x02); its key the z sum over twice the pairs.
        let pairs = (out.len() / 2) as i32;
        let mut key = 0;
        if !self.full || pairs < 2 {
            out.clear();
        } else {
            key = ee::div(zsum, ee::mul(0x4000_0000, ee::from_int(pairs)));
        }
        self.first = false;
        let mut dead = 0;
        for r in 0..ROWS {
            if self.life[r] <= 0 {
                dead += 1;
                continue;
            }
            self.life[r] -= 1;
            if self.life[r] > 0 {
                continue;
            }
            self.life[r] = 0;
            dead += 1;
            self.tail = (self.tail + ROWS - 1) % ROWS;
        }
        if dead == ROWS && self.head != self.tail {
            self.tail = self.head;
        }
        (out, key)
    }
}
