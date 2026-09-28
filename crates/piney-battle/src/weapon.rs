//! The enemies' weapon trails and flashes: `ccEnemyWeaponCtrl`,
//! `ccEnemyWeapon` and `ccEnemyWeaponRad` (gcmn 0x0043c600-0x0043ece8). A
//! race's constructor makes a controller over its type's `ccEnemyWpInfo`
//! rows (`entry::Out::Weapon`); the race's `exclusive()` runs it each frame
//! ([`WeaponCtrl::ctrl`]) and its `note()` hands it every note
//! ([`WeaponCtrl::note`]).
//!
//! ```text
//! ccEnemyWpInfo (0x60 bytes): +0 chunk name (one row's is set), +4
//!   node name, +8 flags, +0xc the flash's scale, +0x10 the spline's
//!   tension, +0x14 cells put between two (u16), +0x16 a cell's life (u16),
//!   +0x18 edges (u16, 1-4), +0x20 the edges' points under the node
//! flags: 1 rgb colours (attrRgbTable), 2 hsv colours (attrHsvTable), 4
//!   the strip packet (cczGsPrimPoly), 8 the line packet (cczGsPrimLine),
//!   0x10 the flash, 0x20 bezier, 0x40 spline, 0x80 << atkNum the attacks
//!   it shows on (actNum 6)
//! ccEnemyWeaponCtrl(n, info, obj) 0x0043e760  a ccEnemyWeapon a row
//! ccEnemyWeapon(info, obj)   0x0043c970  cells: 1 for a life below 2, else
//!                                        life (between + 1); edges, flags;
//!                                        setEdgeRate; init(0); the node:
//!                                        GetSubstAdrsF of the anm for a name
//!                                        whose part before '_' is EXT (bit
//!                                        0, looked up again each frame), else
//!                                        GetObjAdrsF of the clump; a
//!                                        ccEnemyWeaponRad (wpRadInfo: a
//!                                        plate of 8 rays facing the camera)
//! setEdgeRate                0x0043ce60  each edge's share along the axis
//!                                        the first and last edges differ
//!                                        most on (in doubles)
//! ctrl(obj)                  0x0043e900  each weapon: ctrlCell; with dispSW:
//!                                        if checkWeapon(actNum, atkNum) (on
//!                                        actCnt 0 init(the skill's element
//!                                        first) and setCell), dispCell;
//!                                        ctrlRadiate
//! note(obj, note)            0x0043eae0  atkNum 0, 1, 4, 5 on 0x8005: weapon
//!                                        param (1 up to the count) flashes
//!                                        for 20 frames; atkNum 2, 3 on
//!                                        0x8003: every weapon for 80, kept
//!                                        on the node (bit 2)
//! setCell                    0x0043db50  a new cell (the first free, else
//!                                        the oldest) at the front: the edges
//!                                        under the node's matrix, the full
//!                                        life; bezier or spline cells put
//!                                        between the second and third once
//!                                        there are 4; a flash asked for
//!                                        starts at the middle of the last two
//!                                        edges
//! ctrlCell                   0x0043de90  each cell fades by 0.4 / life, its
//!                                        colours' alphas times that; one
//!                                        out of life leaves the list
//! dispCell                   0x0043e110  layer 6: for each pair of edges,
//!                                        each cell from the newest, the two
//!                                        points into Kite's frame (once a
//!                                        frame) and into the packets; the
//!                                        line packet's alphas 1.1 times
//! ccEnemyWeaponRad::ctrl     0x0043c600  act 0: colours from the element,
//!                                        width 2 pi / 8; act 1: turning 0.24
//!                                        about x, alpha sin, a light 1.5 sin,
//!                                        length 40 sin, width 20 sin, zoom
//!                                        10 sin, the angle on by pi / life;
//!                                        after life + 2 frames act 2 puts it
//!                                        out
//! ```
//!
//! Nothing here draws a random number: the flash's plate has no dpLength or
//! dpBank. `tools/test_enemy_weapon_rs.py` runs the game's controller
//! against [`WeaponCtrl`].

use crate::damage::fptosi;
use crate::geom::{
    self, F, M4, ONE, PI, TWO_PI, V4, add, apply_matrix, div, fptoui, from_int, le, lt, mul, mul_matrix, rot_matrix,
    sinf, sub, trans_matrix, unit_matrix, vadd, vscale, vsub,
};
use crate::prim::{RadInfo, RadOut, RadWorld, Radiate, fractional_hsv};
use crate::world::Note;

/// `sizeof(ccEnemyWpInfo)`.
pub const INFO_SIZE: usize = 0x60;

/// `attrHsvTable` (gcmn 0x005d5900): each element's colour (v, s, h).
pub const ATTR_HSV: [u32; 8] = [0x80ffff, 0x80ffff, 0x1cffff, 0x98ffff, 0x08ffff, 0x55ffff, 0x2affff, 0xd5ffff];
/// `attrRgbTable` (gcmn 0x005d58e0).
pub const ATTR_RGB: [u32; 8] =
    [0x80ff_ffff, 0x80ff_ff00, 0x8000_80ff, 0x80ff_0000, 0x8000_00ff, 0x8000_ff00, 0x8000_ffff, 0x80ff_00ff];
/// `wpRadInfo` (gcmn 0x005d59b0): a plate (4) facing the camera (1) of 8
/// rays, all else 0.
pub const WP_RAD_INFO: RadInfo = RadInfo { ty: 5, pnum: 8, center: 0, length: 0, width: 0, fzoom: 0, col0: 0, col1: 0 };

/// The info's flags.
pub mod flag {
    pub const RGB: u32 = 1;
    pub const HSV: u32 = 2;
    pub const POLY: u32 = 4;
    pub const LINE: u32 = 8;
    pub const RADIATE: u32 = 0x10;
    pub const BEZIER: u32 = 0x20;
    pub const SPLINE: u32 = 0x40;
}

const K_0_4: F = 0x3ecc_cccd;
const K_0_5: F = 0x3f00_0000;
const K_0_7: F = 0x3f33_3333;
const K_1_1: F = 0x3f8c_cccd;
const K_1_5: F = 0x3fc0_0000;
const K_2: F = 0x4000_0000;
const K_3: F = 0x4040_0000;
const K_M2: F = 0xc000_0000;
const K_10: F = 0x4120_0000;
const K_20: F = 0x41a0_0000;
const K_40: F = 0x4220_0000;
/// The flash's turn a frame.
const ROT_STEP: F = 0x3e75_c28f;

/// `ccEnemyWpInfo`, one row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WpInfo {
    /// +0x00 a chunk of the enemy's scene file whose turn the node's
    /// matrix takes on (one row's, `DMY_xdummy_w05`), +0x04 the node the
    /// edges hang on.
    pub chunk: String,
    pub node: String,
    /// +0x08.
    pub flags: u32,
    /// +0x0c the flash's `scale`, +0x10 the spline's tension.
    pub scale: F,
    pub tension: F,
    /// +0x14 cells put between two, +0x16 a cell's life, +0x18 edges.
    pub between: u16,
    pub life: u16,
    pub edges: u16,
    /// +0x20 the edges' points in the node's frame.
    pub pts: [V4; 4],
}

impl WpInfo {
    /// A row from its 0x60 bytes and the names its +0x00 and +0x04 point
    /// at.
    pub fn from_bytes(b: &[u8], chunk: String, node: String) -> WpInfo {
        let w = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        let h = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
        WpInfo {
            chunk,
            node,
            flags: w(8),
            scale: w(0xc),
            tension: w(0x10),
            between: h(0x14),
            life: h(0x16),
            edges: h(0x18),
            pts: std::array::from_fn(|i| std::array::from_fn(|k| w(0x20 + 16 * i + 4 * k))),
        }
    }

    /// A row of the volume's tables; none without its node.
    pub fn of(r: &piney_data::tables::types::EnemyWpInfo) -> Option<WpInfo> {
        Some(WpInfo {
            chunk: r.dmy_name.unwrap_or_default().to_owned(),
            node: r.obj_name?.to_owned(),
            flags: r.kind as u32,
            scale: r.scale.to_bits(),
            tension: r.inp_alpha.to_bits(),
            between: r.inp_num,
            life: r.life,
            edges: r.edge_num,
            pts: r.pos.map(|p| p.map(f32::to_bits)),
        })
    }
}

/// `ccCheckNameExtObject(name)` (gcmn 0x0042e5d0): the part before the
/// first '_' is `EXT`.
pub fn is_ext_name(name: &str) -> bool {
    name.split('_').next() == Some("EXT")
}

/// One point of a cell (`ccPrimVert`, 0x30 bytes): +0 bit 0 its screen
/// place made this frame, +4 its colour (RGBA, alpha in the top byte),
/// +0x10 its place (in Kite's frame once drawn). The screen place (+0x20)
/// is the renderer's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vert {
    pub drawn: bool,
    pub color: u32,
    pub pos: V4,
}

/// `ccEnemyCell` (0xd0 bytes): +0 bit 0 put between two, +2 life, +4
/// alpha, +8 the newer cell, +0xc the older, +0x10 the edges' points.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub between: bool,
    pub life: i16,
    pub alpha: F,
    pub newer: Option<usize>,
    pub older: Option<usize>,
    pub vert: [Vert; 4],
}

/// Two points `ccPrimPacket::makePacket` is given: edges e and e + 1 of a
/// cell, in Kite's frame, with their colours; `cont` false for the newest
/// cell (its pair starts the strip: both points ADC).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pair {
    pub pos: [V4; 2],
    pub color: [u32; 2],
    pub cont: bool,
}

/// What a weapon asks of the rest of the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WeaponOut {
    /// `dispCell`: on layer 6, the line packet (flag 8, `cczGsPrimLine`)
    /// then the strip packet (flag 4, `cczGsPrimPoly`) sent, each with its
    /// pairs in order (edge pair by edge pair, the cells newest first);
    /// None for a packet the weapon has not. `sendPacket` sends nothing
    /// under 2 pairs. `weapon` is the weapon's place in the controller.
    Trail { weapon: usize, polys: Option<Vec<Pair>>, lines: Option<Vec<Pair>> },
    /// The flash ([`rad_ctrl`], `create`, `disp`).
    Rad(RadOut),
}

/// What the controller reads of its enemy: `dispSW` (+0xe0 bit 4),
/// `actNum` (+0x25a), `actCnt` (+0x25c), `atkNum` (+0x25e) and
/// `ccSkillCheckTypeAttribute(skillParam->type)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WeaponAt {
    pub disp_sw: bool,
    pub act_num: i16,
    pub act_cnt: i16,
    pub atk_num: i16,
    pub attr: i32,
}

/// What a weapon asks of the world besides the flash's.
pub trait WeaponWorld: RadWorld {
    /// The world matrix of the weapon's node as the enemy was last drawn
    /// (the `ccCoord`'s +0): the clump's node `name` (`GetObjAdrsF`), or
    /// with `ext` the animation's object (`ccAnm::GetSubstAdrsF`); None
    /// when there is none (the unit matrix then).
    fn node_matrix(&mut self, name: &str, ext: bool) -> Option<M4>;
    /// The place (+0x10) and turn (+0x20) of the enemy's scene file's
    /// chunk `name` (`ccStream::GetChunkAdrsF(name, 1)`), None when it has
    /// none.
    fn chunk(&mut self, name: &str) -> Option<(V4, V4)>;
}

/// `ccEnemyWeapon` (0x74 bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Weapon {
    /// +0x00 bit 0 the node is the animation's (`EXT_`), bit 1 a flash
    /// asked for, bit 2 the flash follows the node (a skill's).
    pub ext: bool,
    pub on: bool,
    pub skill: bool,
    /// +0x04 the info's flags, +0x08 the element.
    pub flags: u32,
    pub attr: i32,
    /// +0x0e edges, +0x10 each edge's share.
    pub edges: i16,
    pub rate: [F; 4],
    /// +0x20 the cells (+0x0c their number), +0x24 the newest, +0x28 the
    /// oldest, +0x2c how many live.
    pub cells: Vec<Cell>,
    pub newest: Option<usize>,
    pub oldest: Option<usize>,
    pub count: i32,
    /// +0x30.
    pub info: WpInfo,
    /// +0x40 the flash's life.
    pub rad_life: i32,
    /// +0x44.
    pub rad: Radiate,
    /// +0x48, +0x5c the packets' double-buffer indices.
    pub packet: [i32; 2],
}

impl Weapon {
    /// `new ccEnemyWeapon(info, obj)` (gcmn 0x0043c970).
    pub fn new(info: WpInfo) -> Weapon {
        let n = if info.life < 2 { 1 } else { i32::from(info.life) * (i32::from(info.between as i16) + 1) };
        let mut w = Weapon {
            ext: is_ext_name(&info.node),
            on: false,
            skill: false,
            flags: info.flags,
            attr: 0,
            edges: info.edges as i16,
            rate: [0; 4],
            cells: vec![Cell::default(); usize::try_from(n as i16).unwrap_or(0)],
            newest: None,
            oldest: None,
            count: 0,
            info,
            rad_life: 0,
            rad: Radiate::new(WP_RAD_INFO, None),
            packet: [0; 2],
        };
        w.set_edge_rate();
        w.init(0);
        w
    }

    fn edges(&self) -> usize {
        usize::try_from(self.edges).unwrap_or(0).min(4)
    }

    /// `setEdgeRate()` (gcmn 0x0043ce60): each edge's distance from the
    /// first along the axis the last is farthest on, over that distance
    /// (`fptodp`, `fabs`, `dpdiv`, `dptofp`); by index (1 / edges i) should
    /// no axis be the greatest.
    fn set_edge_rate(&mut self) {
        self.rate = [ONE; 4];
        let n = self.edges();
        let pts = self.info.pts;
        let first = pts[0];
        let last = pts[(self.edges as usize).wrapping_sub(1) & 3];
        let d: [F; 3] = std::array::from_fn(|i| sub(last[i], first[i]) & 0x7fff_ffff);
        let (dx, dy, dz) = (d[0], d[1], d[2]);
        let axis = if !lt(dx, dy) && !lt(dx, dz) {
            Some(0)
        } else if !lt(dy, dx) && !lt(dy, dz) {
            Some(1)
        } else if !lt(dz, dx) && !lt(dz, dy) {
            Some(2)
        } else {
            None
        };
        match axis {
            Some(a) => {
                for (r, p) in self.rate.iter_mut().zip(&pts).take(n) {
                    *r = dp_ratio(sub(p[a], first[a]) & 0x7fff_ffff, d[a]);
                }
            }
            None => {
                let step = div(ONE, from_int(i32::from(self.edges)));
                for i in 0..n {
                    self.rate[i] = mul(step, from_int(i as i32));
                }
            }
        }
    }

    /// `init(attr)` (gcmn 0x0043cda0): the element, no flash asked for,
    /// every cell free, the list empty.
    pub fn init(&mut self, attr: i32) {
        self.attr = attr;
        self.on = false;
        self.skill = false;
        self.rad_life = 0;
        for k in 0..self.cells.len() {
            self.init_cell(k);
        }
        self.count = 0;
        self.oldest = None;
        self.newest = None;
    }

    /// `initCell(cell)` (gcmn 0x0043d130): life 0, alpha 1, colours afresh,
    /// unlinked.
    fn init_cell(&mut self, k: usize) {
        let c = &mut self.cells[k];
        c.life = 0;
        c.alpha = ONE;
        c.between = false;
        self.init_cell_color(k);
        let c = &mut self.cells[k];
        c.older = None;
        c.newer = None;
    }

    /// `initCellColorHSV(cell)` (gcmn 0x0043d190): each edge not yet drawn,
    /// its colour the element's (flag 1), or the element's hue with its
    /// saturation and alpha by the edge's share (flag 2:
    /// `ccFractionalHsv(hsv, rate, 0.7 rate)`).
    fn init_cell_color(&mut self, k: usize) {
        let a = usize::try_from(self.attr).unwrap_or(0) & 7;
        let hsv = ATTR_HSV[a] | 0x8000_0000;
        for i in 0..self.edges() {
            let v = &mut self.cells[k].vert[i];
            v.drawn = false;
            if self.flags & flag::RGB != 0 {
                v.color = ATTR_RGB[a];
            } else if self.flags & flag::HSV != 0 {
                v.color = fractional_hsv(hsv, self.rate[i], mul(K_0_7, self.rate[i]));
            }
        }
    }

    /// `getNewCell()` (gcmn 0x0043d2c0): the first free cell (one more
    /// live), else the oldest taken off the list; made afresh.
    fn get_new_cell(&mut self) -> usize {
        let k = match self.cells.iter().position(|c| c.life == 0) {
            Some(k) => {
                self.count += 1;
                k
            }
            None => {
                let Some(o) = self.oldest else { return 0 };
                let n = self.cells[o].newer;
                if let Some(n) = n {
                    self.cells[n].older = None;
                }
                self.oldest = n;
                o
            }
        };
        self.init_cell(k);
        k
    }

    /// `checkWeapon(act, atk)` (gcmn 0x0043e680): the attacking act (6) and
    /// an attack 0-5 the weapon shows on.
    pub fn check_weapon(&self, act: i16, atk: i16) -> bool {
        act == 6 && (0..6).contains(&atk) && self.flags & (0x80 << atk) != 0
    }

    /// The node's matrix now, the unit matrix without one; with a chunk,
    /// turned by the chunk's turn (`sceVu0MulMatrix(m, m, RotMatrix(unit,
    /// rot))`) and moved to the chunk's place under the node (the copy of
    /// `ApplyMatrix(m, pos)` lands on the matrix's last row).
    fn node(&self, w: &mut dyn WeaponWorld) -> M4 {
        let m = w.node_matrix(&self.info.node, self.ext).unwrap_or_else(unit_matrix);
        match (!self.info.chunk.is_empty()).then(|| w.chunk(&self.info.chunk)).flatten() {
            Some((pos, rot)) => {
                let p = apply_matrix(&m, pos);
                let mut m = mul_matrix(&m, &rot_matrix(&unit_matrix(), rot));
                m[3] = p;
                m
            }
            None => m,
        }
    }

    /// The flash's place among the edges `pos`: the last, or the middle of
    /// the last two (w 1).
    fn rad_point(&self, pos: &[V4; 4]) -> V4 {
        let n = self.edges();
        if n < 2 {
            return pos[n.saturating_sub(1)];
        }
        let mut p = vscale(vadd(pos[n - 1], pos[n - 2]), K_0_5);
        p[3] = ONE;
        p
    }

    /// `setCell()` (gcmn 0x0043db50).
    fn set_cell(&mut self, w: &mut dyn WeaponWorld) {
        let k = self.get_new_cell();
        if self.oldest.is_none() {
            self.oldest = Some(k);
        }
        if let Some(n) = self.newest {
            self.cells[n].newer = Some(k);
        }
        self.cells[k].newer = None;
        self.cells[k].older = self.newest;
        self.newest = Some(k);
        self.cells[k].life = self.info.life as i16;
        self.cells[k].alpha = ONE;
        let m = self.node(w);
        for i in 0..self.edges() {
            self.cells[k].vert[i].pos = apply_matrix(&m, self.info.pts[i]);
        }
        if self.flags & (flag::BEZIER | flag::SPLINE) != 0 && self.count >= 4 {
            self.interpolate_cell(w);
        }
        if self.flags & flag::RADIATE == 0 {
            return;
        }
        let pos = self.cells[k].vert.map(|v| v.pos);
        let p = self.rad_point(&pos);
        if !self.on {
            return;
        }
        let r = &mut self.rad;
        r.init();
        r.pos = p;
        r.attr = self.attr as i16;
        r.life = self.rad_life as i16;
        r.scale = self.info.scale;
        r.rad_flag = true;
        self.on = false;
    }

    /// `interpolateCell()` (gcmn 0x0043d3b0): between the second and third
    /// newest cells, `between` new ones at t = i / (between + 1), their life
    /// the second's less t of the difference (at least 1), each edge a
    /// bezier over the four newest (flag 0x20) or `eneSplineH` between the
    /// two, the four's points put into Kite's frame first.
    fn interpolate_cell(&mut self, w: &mut dyn WeaponWorld) {
        let mut q = [0usize; 4];
        let mut c = self.newest;
        for f in 0..4 {
            let Some(k) = c else { return };
            q[3 - f] = k;
            c = self.cells[k].older;
        }
        let n = i32::from(self.info.between) + 1;
        let step = div(ONE, from_int(n));
        let (p1, p2, p3) = (q[1], q[2], q[3]);
        let mut s0 = p2;
        for fp in 1..n {
            let k = self.get_new_cell();
            self.cells[s0].older = Some(k);
            self.cells[k].newer = Some(s0);
            self.cells[k].older = Some(p1);
            self.cells[p1].newer = Some(k);
            let (l0, l1) = (i32::from(self.cells[s0].life), i32::from(self.cells[p1].life));
            let t = mul(step, from_int(fp));
            let mut l = sub(from_int(l0), mul(t, from_int(l0 - l1)));
            if lt(l, ONE) {
                l = ONE;
            }
            self.cells[k].life = fptosi(l) as i16;
            self.cells[k].between = true;
            for e in 0..self.edges() {
                for &c in &q {
                    self.cells[c].vert[e].pos = w.fw2lw(self.cells[c].vert[e].pos);
                }
                let [a, b, c, d] = [p3, p2, p1, q[0]].map(|c| self.cells[c].vert[e].pos);
                self.cells[k].vert[e].pos = if self.flags & flag::BEZIER != 0 {
                    bezier(a, b, c, d, t)
                } else {
                    spline_h(a, b, c, d, t, self.info.tension)
                };
            }
            s0 = k;
        }
    }

    /// `ctrlCell()` (gcmn 0x0043de90): each live cell from the newest fades
    /// by 0.4 / life (not below 0), each edge's alpha times the cell's
    /// (truncated, so it compounds), and loses a frame; out of life, it is
    /// taken off the list.
    fn ctrl_cell(&mut self) {
        let dec = div(K_0_4, from_int(i32::from(self.info.life)));
        let mut c = self.newest;
        for _ in 0..self.count {
            let Some(k) = c else { break };
            let edges = self.edges();
            let cell = &mut self.cells[k];
            cell.alpha = sub(cell.alpha, dec);
            if lt(cell.alpha, 0) {
                cell.alpha = 0;
            }
            for v in &mut cell.vert[..edges] {
                let a = fptoui(mul(from_int((v.color >> 24) as i32), cell.alpha));
                v.color = (v.color & 0x00ff_ffff) | (a << 24);
            }
            cell.life -= 1;
            let (newer, older) = (cell.newer, cell.older);
            if cell.life <= 0 {
                cell.life = 0;
                self.count -= 1;
                match newer {
                    Some(n) => self.cells[n].older = older,
                    None => {
                        if let Some(o) = older {
                            self.cells[o].newer = None;
                        }
                        self.newest = older;
                    }
                }
                match older {
                    Some(o) => self.cells[o].newer = newer,
                    None => {
                        if let Some(n) = newer {
                            self.cells[n].older = None;
                        }
                        self.oldest = newer;
                    }
                }
            }
            c = older;
        }
    }

    /// `dispCell()` (gcmn 0x0043e110): with more than one cell listed,
    /// every point not yet drawn this frame, the packets' buffers flipped,
    /// and for each pair of edges each live cell from the newest through
    /// `makePacketCell`.
    fn disp_cell(&mut self, me: usize, w: &mut dyn WeaponWorld, out: &mut Vec<WeaponOut>) {
        if self.newest == self.oldest {
            return;
        }
        let edges = self.edges();
        for c in &mut self.cells {
            for v in &mut c.vert[..edges] {
                v.drawn = false;
            }
        }
        let mut polys = (self.flags & flag::POLY != 0).then(Vec::new);
        let mut lines = (self.flags & flag::LINE != 0).then(Vec::new);
        if polys.is_some() {
            self.packet[0] = (self.packet[0] + 1) & 1;
        }
        if lines.is_some() {
            self.packet[1] = (self.packet[1] + 1) & 1;
        }
        for e in 0..edges.saturating_sub(1) {
            let mut c = self.newest;
            for k in 0..self.count {
                let Some(i) = c else { break };
                self.make_packet_cell(i, e, k != 0, w, &mut polys, &mut lines);
                c = self.cells[i].older;
            }
        }
        if polys.is_some() || lines.is_some() {
            out.push(WeaponOut::Trail { weapon: me, polys, lines });
        }
    }

    /// `makePacketCell(v, cont)` (gcmn 0x0043d720): edges e and e + 1 of
    /// cell `i`, each put into Kite's frame in place the first time this
    /// frame (`ccTransPosFW2LW`; its screen place `sceVu0RotTransPers` on
    /// the active layer's view is the renderer's); into the strip packet as
    /// they are and into the line packet with their alphas 1.1 times (held
    /// to 0-255).
    fn make_packet_cell(
        &mut self,
        i: usize,
        e: usize,
        cont: bool,
        w: &mut dyn WeaponWorld,
        polys: &mut Option<Vec<Pair>>,
        lines: &mut Option<Vec<Pair>>,
    ) {
        for v in &mut self.cells[i].vert[e..e + 2] {
            if !v.drawn {
                v.pos = w.fw2lw(v.pos);
                v.drawn = true;
            }
        }
        let [a, b] = [self.cells[i].vert[e], self.cells[i].vert[e + 1]];
        let pair = Pair { pos: [a.pos, b.pos], color: [a.color, b.color], cont };
        if let Some(p) = polys {
            p.push(pair);
        }
        if let Some(l) = lines {
            let brighter = |c: u32| {
                let x = fptosi(mul(from_int((c >> 24) as i32), K_1_1)).clamp(0, 255) as u32;
                (x << 24) | (c & 0x00ff_ffff)
            };
            l.push(Pair { color: pair.color.map(brighter), ..pair });
        }
    }

    /// `ctrlRadiate()` (gcmn 0x0043e3f0): a skill's flash kept at the
    /// node's edges; while it shines its frame (`ccEnemyWeaponRad::ctrl`),
    /// `create` and `disp`.
    fn ctrl_radiate(&mut self, w: &mut dyn WeaponWorld, out: &mut Vec<WeaponOut>) {
        if self.skill {
            let m = self.node(w);
            let pos: [V4; 4] = std::array::from_fn(|i| apply_matrix(&m, self.info.pts[i]));
            self.rad.pos = self.rad_point(&pos);
        }
        if !self.rad.rad_flag {
            return;
        }
        let mut o = Vec::new();
        rad_ctrl(&mut self.rad, w, &mut o);
        self.rad.create(w);
        if !self.rad.parts.is_empty() {
            self.rad.disp(w, &mut o);
        }
        out.extend(o.into_iter().map(WeaponOut::Rad));
    }
}

/// `ccEnemyWeaponCtrl` (8 bytes): +0 how many, +4 the first weapon (each
/// the next's +0x70).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WeaponCtrl {
    pub weapons: Vec<Weapon>,
}

impl WeaponCtrl {
    /// `new ccEnemyWeaponCtrl(n, info, obj)` (gcmn 0x0043e760): a weapon a
    /// row.
    pub fn new(rows: Vec<WpInfo>) -> WeaponCtrl {
        WeaponCtrl { weapons: rows.into_iter().map(Weapon::new).collect() }
    }

    /// `ctrl(obj)` (gcmn 0x0043e900), the enemy as `at` says.
    pub fn ctrl(&mut self, at: &WeaponAt, w: &mut dyn WeaponWorld, out: &mut Vec<WeaponOut>) {
        for (i, wp) in self.weapons.iter_mut().enumerate() {
            wp.ctrl_cell();
            if at.disp_sw {
                if wp.check_weapon(at.act_num, at.atk_num) {
                    if at.act_cnt == 0 {
                        wp.init(at.attr);
                    }
                    wp.set_cell(w);
                }
                wp.disp_cell(i, w, out);
            }
            wp.ctrl_radiate(w, out);
        }
    }

    /// `note(obj, note)` (gcmn 0x0043eae0): a normal attack's hit (0x8005
    /// in attacks 0, 1, 4, 5) flashes weapon `param` (1 at least, the last
    /// at most) for 20 frames; a skill's start (0x8003 in attacks 2, 3)
    /// every weapon for 80, following the node; each only where
    /// `checkWeapon` holds.
    pub fn note(&mut self, at: &WeaponAt, note: Note) {
        let mode = match at.atk_num {
            0 | 1 | 4 | 5 if note.event == 0x8005 => 1,
            2 | 3 if note.event == 0x8003 => 2,
            _ => 0,
        };
        match mode {
            1 => {
                let n = (note.param as i32).max(1).min(self.weapons.len() as i32);
                let Some(wp) = usize::try_from(n - 1).ok().and_then(|k| self.weapons.get_mut(k)) else { return };
                if wp.check_weapon(at.act_num, at.atk_num) {
                    wp.on = true;
                    wp.skill = false;
                    wp.rad_life = 20;
                }
            }
            2 => {
                for wp in &mut self.weapons {
                    if wp.check_weapon(at.act_num, at.atk_num) {
                        wp.on = true;
                        wp.skill = true;
                        wp.rad_life = 80;
                    }
                }
            }
            _ => {}
        }
    }
}

/// `ccEnemyWeaponRad::ctrl()` (gcmn 0x0043c600), the flash's frame: act 0
/// takes the element's colours (`ccFractionalHsv(hsv, 1.0, 0.7)` inside,
/// `(hsv, 0, 0)` outside), length and zoom 0, width 2 pi / pnum, and goes
/// on at once to act 1; act 1 turns it about x, swells it with `param[0]`
/// (alpha sin, the light 1.5 sin in the scene's group at its place in
/// Kite's frame, length 40 sin, width 20 sin, zoom 10 sin; pi / life a
/// frame) and after `life + 2` frames goes to act 2, which puts it out.
pub fn rad_ctrl(r: &mut Radiate, w: &mut dyn RadWorld, out: &mut Vec<RadOut>) {
    match r.act_num {
        0 => {
            let hsv = ATTR_HSV[usize::try_from(r.attr).unwrap_or(0) & 7] | 0x8000_0000;
            let c0 = fractional_hsv(hsv, ONE, K_0_7);
            let c1 = fractional_hsv(hsv, 0, 0);
            r.set_color(c0, c1);
            r.length = 0;
            r.fzoom = 0;
            r.width = div(TWO_PI, from_int(i32::from(r.pnum)));
            r.param[0] = 0;
            r.param[1] = 0;
            r.act_cnt = 0;
            r.act_num += 1;
            shine(r, w, out);
        }
        1 => shine(r, w, out),
        2 => {
            r.rad_flag = false;
            if r.lgt_flag {
                out.push(RadOut::LightOff);
                r.lgt_flag = false;
            }
        }
        _ => {}
    }

    fn shine(r: &mut Radiate, w: &mut dyn RadWorld, out: &mut Vec<RadOut>) {
        let x = add(r.rot[0], ROT_STEP);
        r.rot[0] = x;
        if !le(x, PI) {
            r.rot[0] = sub(x, TWO_PI);
        }
        if lt(r.rot[0], geom::NEG_PI) {
            r.rot[0] = add(r.rot[0], TWO_PI);
        }
        let s = sinf(r.param[0]);
        r.alpha = s;
        r.light.intensity = mul(K_1_5, s);
        r.pos = w.fw2lw(r.pos);
        r.light.matrix = trans_matrix(&unit_matrix(), r.pos);
        r.light.mat_calc = true;
        out.push(RadOut::LightOn(r.light));
        r.lgt_flag = true;
        r.length = mul(K_40, s);
        r.width = mul(K_20, s);
        r.fzoom = mul(K_10, s);
        r.param[0] = add(r.param[0], div(PI, from_int(i32::from(r.life))));
        let c = r.act_cnt;
        r.act_cnt = c.wrapping_add(1);
        if r.life < c {
            r.act_num += 1;
            r.act_cnt = 0;
        }
    }
}

/// The bezier of `interpolateCell` over the four newest cells' points
/// (`a` the newest), each lane `((a u^3 + b 3u^2 t) + c 3u t^2) + d t^3` as
/// the FPU's accumulator takes it; w 1.
fn bezier(a: V4, b: V4, c: V4, d: V4, t: F) -> V4 {
    let u = sub(ONE, t);
    let f5 = mul(u, mul(u, u));
    let u3 = mul(K_3, u);
    let f4 = mul(t, mul(u3, u));
    let f3 = mul(t, mul(u3, t));
    let f2 = mul(t, mul(t, t));
    let mut p: V4 = std::array::from_fn(|i| {
        let f1 = add(mul(a[i], f5), mul(b[i], f4));
        let acc = add(mul(c[i], f3), f1);
        add(acc, mul(d[i], f2))
    });
    p[3] = ONE;
    p
}

/// `eneSplineH(out, a, b, c, d, t, tension)` (gcmn 0x004382a0): the
/// Hermite curve from `b` to `c`, its tangents `((b - a) + c - b) k` and
/// `((c - b) + d - c) k` (x, y, z), `k = (1 - tension) / 2`; w 1.
pub fn spline_h(a: V4, b: V4, c: V4, d: V4, t: F, tension: F) -> V4 {
    let t2 = mul(t, t);
    let t3 = mul(t2, t);
    let k = div(sub(ONE, tension), K_2);
    let xyz = |v: V4, s: F| [mul(v[0], s), mul(v[1], s), mul(v[2], s), v[3]];
    let m0 = xyz(vsub(vadd(vsub(b, a), c), b), k);
    let m1 = xyz(vsub(vadd(vsub(c, b), d), c), k);
    let t2x3 = mul(K_3, t2);
    let h00 = add(ONE, sub(mul(K_2, t3), t2x3));
    let h10 = add(t, sub(t3, mul(K_2, t2)));
    let h11 = sub(t3, t2);
    let h01 = add(mul(K_M2, t3), t2x3);
    let mut p = vadd(vadd(vadd(xyz(b, h00), xyz(m0, h10)), xyz(m1, h11)), xyz(c, h01));
    p[3] = ONE;
    p
}

/// `dptofp(dpdiv(fptodp(v), fptodp(max)))`: `v / max` in doubles; 0 / 0
/// as libgcc's soft float gives it.
fn dp_ratio(v: F, max: F) -> F {
    let (a, b) = (f64::from(f32::from_bits(v)), f64::from(f32::from_bits(max)));
    if b == 0.0 {
        return DP_NAN;
    }
    ((a / b) as f32).to_bits()
}

/// `dptofp(dpdiv(0, 0))` as the game's libgcc soft float (fp-bit) gives
/// it: 1.0019531 (a one-edge row's share; its colour is never drawn).
const DP_NAN: F = 0x3f80_4000;

#[cfg(test)]
mod tests {
    use super::*;

    fn row(flags: u32, edges: u16, life: u16, between: u16) -> WpInfo {
        let pts = [[0, 0, 0, ONE], [0, geom::k(20.0), 0, ONE], [0, geom::k(50.0), 0, ONE], [0, geom::k(100.0), 0, ONE]];
        WpInfo { chunk: String::new(), node: "OBJ_x".into(), flags, scale: ONE, tension: 0, between, life, edges, pts }
    }

    #[test]
    fn ext_names_are_the_animations() {
        assert!(is_ext_name("EXT_evb1body01"));
        assert!(!is_ext_name("OBJ_t0 l hand"));
        assert!(!is_ext_name("EXTRA_x"));
    }

    #[test]
    fn a_weapon_shows_on_its_attacks() {
        let w = Weapon::new(row(0x0280 | flag::HSV, 4, 16, 1));
        assert!(w.check_weapon(6, 0) && w.check_weapon(6, 2));
        assert!(!w.check_weapon(6, 1) && !w.check_weapon(5, 0) && !w.check_weapon(6, 6) && !w.check_weapon(6, -1));
        // life x (between + 1) cells; the shares along y.
        assert_eq!(w.cells.len(), 32);
        assert_eq!(w.rate, [0, geom::k(0.2), geom::k(0.5), ONE]);
    }

    #[test]
    fn the_spline_runs_from_the_second_point_to_the_third() {
        let p = |y: f32| [0, geom::k(y), 0, ONE];
        let (a, b, c, d) = (p(0.0), p(10.0), p(20.0), p(30.0));
        assert_eq!(spline_h(a, b, c, d, 0, 0), b);
        assert_eq!(spline_h(a, b, c, d, ONE, 0), c);
    }

    #[test]
    fn a_hit_flashes_the_weapon_it_names() {
        let mut c = WeaponCtrl::new(vec![row(0x1f80 | 0x12, 1, 1, 0), row(0x1f80 | 0x12, 1, 1, 0)]);
        let at = WeaponAt { disp_sw: true, act_num: 6, act_cnt: 3, atk_num: 0, attr: 2 };
        c.note(&at, Note { event: 0x8005, param: 9 });
        assert!(!c.weapons[0].on && c.weapons[1].on && c.weapons[1].rad_life == 20);
        c.note(&WeaponAt { atk_num: 3, ..at }, Note { event: 0x8003, param: 0 });
        assert!(c.weapons.iter().all(|w| w.on && w.skill && w.rad_life == 80));
    }
}
