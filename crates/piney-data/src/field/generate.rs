//! `WORLD::Generate` (`INF gcmn.prg:0x005a6da0`) and what it calls, as
//! `tools/field.py`'s `Field` and `Fractal` model them, operation for
//! operation: every float is an EE float's bits and goes through [`ee`].

use super::ee::{self, bits};
use super::render;
use super::{
    CHIPS, Cover, FIELD_TYPES, Field, Flat, GenerateError, Hill, Kind, MAP, Object, ObjectInfo, Params, Rng, Tables,
};

const N: usize = MAP;
const C: usize = CHIPS;
const ZERO: u32 = 0;
const TWO: u32 = bits(2.0);
const CHIP: u32 = bits(1200.0);
const CELL: u32 = bits(600.0);

/// `FIELD::MakeField(0)`: every height is `fieldrand(64)`.
const NOISE: u32 = 64;
/// A story area takes the chips around (20, 20), flattens them and starts
/// there.
const STORY_CENTRE: u32 = 20;
/// `MakeHill` gives up on a hill after this many sites.
const HILL_TRIES: u32 = 100;
/// The object loops end after this many failed sites in one call.
const MAX_FAILS: u32 = 300;
/// `FIELD::CheckAreaHeight`: corner heights spanning more than this fail.
const SLOPE_SPAN: u32 = bits(64.0);
const SLOPE_LO: u32 = bits(65535.0);
const SLOPE_HI: u32 = bits(-65535.0);
/// `SetDungeonEnter` (0x005abd20): a story area's entrance is at least
/// this far from the start - or, in the first story area, closer.
const ENTRANCE_FAR: u32 = bits(8000.0);
const FIRST_STORY_AREA: i32 = 14;
/// The start: a free 2 x 2 site passing `fieldrand(100) >= 96`; in a
/// protected story area more than 7,000 units from the entrance.
const START_ROLL: u32 = 100;
const START_PASS: u32 = 96;
const START_FAR: u32 = bits(7000.0);
/// `SetLake` (0x005ac7e0): a 3 x 3 chip site at `fieldrand(37)`, 9 x 9
/// cells flattened, the lake object in the middle chip.
const LAKE_SITES: u32 = 37;
const LAKE_CHIPS: u32 = 3;
const LAKE_FLAT: u32 = 9;
/// `SetTreeObject_B`: rows of `fieldrand(3) + 3` pieces: a left end (row 0
/// or 1), middles (2 or 3), the right end (4).
const ROW_RANGE: u32 = 3;
const ROW_MIN: u32 = 3;
const ROW_MIDDLE: usize = 2;
const ROW_RIGHT: usize = 4;
/// `SetCover`: four corners per chip.
const QUADRANTS: u32 = 4;
/// Loops the game would run forever, cut short.
const GIVE_UP: u32 = 1 << 24;

/// FRACTAL2 (fractal.cpp): a size x size hill from a 5 x 5 key grid,
/// interpolated by rows then columns with a natural cubic spline.
const KEYS: usize = 5;
/// Key heights: `fieldrand(128)` on the grid's rim, `fieldrand(768)` inside.
const KEY_RIM: u32 = 128;
const KEY_INSIDE: u32 = 768;
const LAST_SPAN: u32 = bits(10.0);
const THREE: u32 = bits(3.0);
const MINUS_THREE: u32 = bits(-3.0);

#[derive(Clone, Copy, Default)]
struct SplineKey {
    val: u32,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    span: u32,
    frame: i32,
}

struct Fractal {
    size: usize,
    height: Vec<u32>,
    keys: [SplineKey; KEYS],
    now_frame: i32,
    now_key: usize,
}

impl Fractal {
    fn new(size: usize) -> Fractal {
        Fractal { size, height: vec![ZERO; size * size], keys: [SplineKey::default(); KEYS], now_frame: 0, now_key: 0 }
    }

    /// `FRACTAL2::InitNaturalSpline` (0x005b3d50), operation for operation.
    fn init_spline(&mut self) {
        let n = KEYS;
        let size = self.size as i32;
        let step = size / (n as i32 - 1);
        self.now_frame = 0;
        self.now_key = 0;
        let keys = &mut self.keys;
        let mut val = [ZERO; KEYS + 2];
        for (v, k) in val.iter_mut().zip(keys.iter()) {
            *v = k.val;
        }
        val[n] = keys[n - 1].val;
        val[n + 1] = keys[n - 1].val;
        for (i, k) in keys.iter_mut().enumerate() {
            k.frame = if i == n - 1 { size } else { step * i as i32 };
        }
        for i in 0..n {
            keys[i].span = if i < n - 1 { ee::from_int(keys[i + 1].frame - keys[i].frame) } else { LAST_SPAN };
        }
        for i in 0..n {
            let (f4, f5) = (val[i], val[i + 1]);
            let f2 = if i == 0 {
                ee::sub(f5, f4)
            } else {
                let (span, prev) = (keys[i].span, keys[i - 1].span);
                let acc = ee::mul(span, ee::sub(f4, val[i - 1]));
                let f1 = ee::add(acc, ee::mul(prev, ee::sub(f5, f4)));
                ee::div(f1, ee::add(prev, span))
            };
            let f3 = if i == n - 1 {
                ee::sub(val[i + 2], f5)
            } else {
                let (next, span) = (keys[i + 1].span, keys[i].span);
                let acc = ee::mul(span, ee::sub(val[i + 2], f5));
                let f1 = ee::add(acc, ee::mul(next, ee::sub(f5, f4)));
                ee::div(f1, ee::add(span, next))
            };
            let acc = ee::mul(TWO, f4);
            let f0 = ee::sub(acc, ee::mul(TWO, f5));
            keys[i].a = ee::add(f3, ee::add(f2, f0));
            let acc = ee::add(ee::mul(MINUS_THREE, f4), ee::mul(THREE, f5));
            keys[i].b = ee::sub(ee::sub(acc, ee::mul(TWO, f2)), f3);
            keys[i].c = f2;
            keys[i].d = f4;
        }
    }

    /// One frame along the spline: `d + t(c + t(b + t a))` as the game
    /// orders it.
    fn step(&mut self) -> u32 {
        self.now_frame += 1;
        self.advance();
        let k = self.keys[self.now_key];
        let t = ee::div(ee::from_int(self.now_frame - k.frame), k.span);
        let t2 = ee::mul(t, t);
        let t3 = ee::mul(t2, t);
        let acc = ee::add(ee::mul(t3, k.a), ee::mul(t2, k.b));
        ee::add(k.d, ee::add(acc, ee::mul(t, k.c)))
    }

    /// Move to the next key once past this one's span.
    fn advance(&mut self) {
        let k = self.keys[self.now_key];
        if !ee::lt(ee::from_int(self.now_frame), ee::add(ee::from_int(k.frame), k.span)) {
            self.now_key += 1;
        }
    }

    /// `FRACTAL2::Generate` (0x005b4070).
    fn generate(&mut self, rng: &mut Rng) {
        let (n, size) = (KEYS, self.size);
        let mut rows = vec![ZERO; n * size];
        for r in 0..n {
            for k in 0..n {
                let rim = k == 0 || k == n - 1 || r == 0 || r == n - 1;
                self.keys[k].val = ee::from_int(rng.below(if rim { KEY_RIM } else { KEY_INSIDE }) as i32);
            }
            self.init_spline();
            for s in 0..size {
                rows[r * size + s] = self.step();
            }
        }
        for col in 0..size {
            for k in 0..n {
                self.keys[k].val = rows[k * size + col];
            }
            self.init_spline();
            self.advance();
            for s in 0..size {
                let h = self.step();
                self.height[col * size + s] = if ee::lt(h, ZERO) { ZERO } else { h };
            }
        }
    }
}

struct Gen<'a> {
    t: &'a Tables,
    p: Params,
    rng: Rng,
    map: Vec<u32>,
    quads: Vec<u32>,
    check: Vec<u8>,
    check3: Vec<u8>,
    mnt: Vec<u8>,
    hills: Vec<Hill>,
    objects: Vec<Object>,
    covers: Vec<Cover>,
    start_pos: Option<[u32; 3]>,
    entrance: Option<[u32; 2]>,
    dungeon_pos: Option<[u32; 3]>,
}

/// Which chip grid a test reads.
#[derive(Clone, Copy)]
enum Grid {
    Check,
    Mount,
}

fn wrap(i: usize) -> usize {
    if i >= N { i - N } else { i }
}

/// A chip's corner and size in world units: `1200 x + 1200 w / 2`.
fn centre(x: u32, w: u32) -> u32 {
    ee::add(ee::mul(CHIP, ee::from_int(x as i32)), ee::div(ee::mul(CHIP, ee::from_int(w as i32)), TWO))
}

impl Gen<'_> {
    fn rand(&mut self, n: u32) -> u32 {
        self.rng.below(n)
    }

    // FIELD --------------------------------------------------------------

    /// `FIELD::SetHeight`: wraps at 80, clamps below 0.
    fn set_height(&mut self, x: usize, y: usize, z: u32) {
        self.map[wrap(x) * N + wrap(y)] = if ee::lt(z, ZERO) { ZERO } else { z };
    }

    /// `FIELD::Flat` (0x005ae810): zero the cells [2x, 2x+w) x [2y, 2y+h).
    fn flat(&mut self, x: u32, y: u32, w: u32, h: u32) {
        for j in 2 * y..2 * y + h {
            for i in 2 * x..2 * x + w {
                self.map[wrap(i as usize) * N + wrap(j as usize)] = ZERO;
            }
        }
    }

    /// `FIELD::Hide`: no ground cover on these chips.
    fn hide(&mut self, x: u32, y: u32, w: u32, h: u32) {
        for j in y..y + h {
            for i in x..x + w {
                self.check3[i as usize * C + j as usize] = 1;
            }
        }
    }

    /// `FIELD::CheckArea` / `CheckMount`: any chip of [x, x2) x [y, y2) taken.
    fn taken(&self, grid: Grid, x: u32, y: u32, x2: u32, y2: u32) -> bool {
        let g = match grid {
            Grid::Check => &self.check,
            Grid::Mount => &self.mnt,
        };
        (y..y2).any(|j| (x..x2).any(|i| g[i as usize * C + j as usize] != 0))
    }

    /// `FIELD::CheckAreaHeight` over the quads of the last `InitQuad`: the
    /// corner heights span more than 64.
    fn too_steep(&self, x: u32, y: u32, w: u32, h: u32) -> bool {
        let (mut hi, mut lo) = (SLOPE_HI, SLOPE_LO);
        let at = |i: usize, j: usize| self.quads[wrap(i) * N + wrap(j)];
        for j in 2 * y as usize..2 * (y + h) as usize {
            for i in 2 * x as usize..2 * (x + w) as usize {
                for z in [at(i, j), at(i + 1, j), at(i, j + 1), at(i + 1, j + 1)] {
                    if !ee::le(z, hi) {
                        hi = z;
                    }
                    if ee::lt(z, lo) {
                        lo = z;
                    }
                }
            }
        }
        !ee::le(ee::sub(hi, lo), SLOPE_SPAN)
    }

    /// `FIELD::InitQuad`: the quads `CheckAreaHeight` reads, from the heights
    /// as they are now.
    fn init_quad(&mut self) {
        self.quads.clone_from(&self.map);
    }

    /// `FOBJECT(2)::SetPosition2`'s footprint: the chips taken, the world
    /// centre and its height cell.
    fn set_position(&mut self, x: u32, y: u32, w: u32, h: u32) -> ([u32; 2], [i32; 2]) {
        for j in y..y + h {
            for i in x..x + w {
                self.check[i as usize * C + j as usize] = 1;
            }
        }
        let (wx, wy) = (centre(x, w), centre(y, h));
        ([wx, wy], [ee::to_int(ee::div(wx, CELL)), ee::to_int(ee::div(wy, CELL))])
    }

    /// Places an object; its z is 0 when `level`, else `WORLD::GetHeight` at
    /// its centre on the heights as they are now.
    fn place(&mut self, kind: Kind, index: usize, info: &'static ObjectInfo, x: u32, y: u32, level: bool) {
        let (pos, cell) = self.set_position(x, y, info.w, info.h);
        let z = if level { ZERO } else { render::get_height(&self.map, &self.check3, pos[0], pos[1]) };
        self.objects.push(Object { kind, index, info, x, y, w: info.w, h: info.h, pos, cell, level, z });
    }

    /// `FIELD::CalcWorldMeshPosition`: the centre of chip (x, y).
    fn world_pos(x: u32, y: u32) -> [u32; 3] {
        let at = |v: u32| ee::add(CELL, ee::mul(CHIP, ee::from_int(v as i32)));
        [at(x), at(y), ZERO]
    }

    // WORLD --------------------------------------------------------------

    /// `WORLD::MakeHill` (0x005a6b00): true when the hill found a place.
    fn make_hill(&mut self, size: u32) -> bool {
        let mut fr = Fractal::new(size as usize);
        fr.generate(&mut self.rng);
        let size = size as usize;
        for _ in 0..HILL_TRIES {
            let x = self.rand((N - size) as u32);
            let y = self.rand((N - size) as u32);
            let (cx, cy, half) = (x / 2, y / 2, size as u32 / 2);
            if self.taken(Grid::Mount, cx, cy, cx + half, cy + half) {
                continue;
            }
            let (cx, cy) = (cx as usize, cy as usize);
            for i in 0..size {
                for j in 0..size {
                    self.set_height(i + cx * 2, j + cy * 2, fr.height[j * size + i]);
                    self.mnt[(i / 2 + cx) * C + j / 2 + cy] = 1;
                }
            }
            self.hills.push(Hill { x, y, size: size as u32 });
            return true;
        }
        false
    }

    /// `ccGetDist`: |a - b|. The difference and the inner product are VU0
    /// macro code, modelled with the FPU rules as `field.py` does; the
    /// square root is the executable's own.
    fn dist(&self, a: [u32; 3], b: [u32; 3]) -> u32 {
        let d = [ee::sub(a[0], b[0]), ee::sub(a[1], b[1]), ee::sub(a[2], b[2])];
        let s = ee::add(ee::add(ee::mul(d[0], d[0]), ee::mul(d[1], d[1])), ee::mul(d[2], d[2]));
        self.t.sqrt.apply(s)
    }

    /// `WORLD::SetDungeonEnter` (0x005abd20).
    fn set_dungeon_enter(&mut self, info: &'static ObjectInfo) -> Result<(), GenerateError> {
        let (w, h) = (info.w, info.h);
        let mut tries = 0;
        let (x, y) = loop {
            tries += 1;
            if tries > GIVE_UP {
                return Err(GenerateError::NoSite("dungeon entrance"));
            }
            let x = self.rand(C as u32 - w);
            let y = self.rand(C as u32 - h);
            if self.taken(Grid::Mount, x, y, x + w, y + h) || self.taken(Grid::Check, x, y, x + w, y + h) {
                continue;
            }
            if self.p.event != 0 {
                let pos = [centre(x, w), centre(y, h), ZERO];
                let near = ee::lt(self.dist(self.start_pos.unwrap_or([ZERO; 3]), pos), ENTRANCE_FAR);
                if near != (self.p.event == FIRST_STORY_AREA) {
                    continue;
                }
            }
            break (x, y);
        };
        if info.flat.contains(Flat::LEVEL) {
            self.flat(x, y, 3 * w, 3 * h);
            self.hide(x, y, w, h);
        }
        self.place(Kind::Entrance, 0, info, x, y, true);
        let pos = self.objects.last().unwrap().pos;
        self.entrance = Some([x, y]);
        self.dungeon_pos = Some([pos[0], pos[1], ZERO]);
        Ok(())
    }

    /// `WORLD::SetLake` (0x005ac7e0).
    fn set_lake(&mut self, table: &'static [ObjectInfo]) -> Result<(), GenerateError> {
        let idx = self.rand(table.len() as u32) as usize;
        let info = &table[idx];
        let mut tries = 0;
        let (x, y) = loop {
            tries += 1;
            if tries > GIVE_UP {
                return Err(GenerateError::NoSite("lake"));
            }
            let x = self.rand(LAKE_SITES);
            let y = self.rand(LAKE_SITES);
            let (x2, y2) = (x + LAKE_CHIPS, y + LAKE_CHIPS);
            if !(self.taken(Grid::Mount, x, y, x2, y2) || self.taken(Grid::Check, x, y, x2, y2)) {
                break (x, y);
            }
        };
        self.hide(x + 1, y + 1, 1, 1);
        self.flat(x, y, LAKE_FLAT, LAKE_FLAT);
        for j in y..y + LAKE_CHIPS {
            for i in x..x + LAKE_CHIPS {
                self.check[i as usize * C + j as usize] = 1;
            }
        }
        self.place(Kind::Lake, idx, info, x + 1, y + 1, true);
        Ok(())
    }

    /// `SetKeyObject` (0x005ac120), `SetSubObject`, `SetBaseObject`,
    /// `SetTreeObject`: the same loop; sub objects also hide and level for
    /// [`Flat::LAKE`], hiding first.
    fn set_objects(&mut self, kind: Kind, table: &'static [ObjectInfo], count: u32) {
        let mut fails = 0;
        for _ in 0..count {
            let idx = self.rand(table.len() as u32) as usize;
            let info = &table[idx];
            let (w, h, fl) = (info.w, info.h, info.flat);
            let (x, y) = loop {
                let x = self.rand(C as u32 - w);
                let y = self.rand(C as u32 - h);
                let bad = if fl.intersects(Flat::LEVEL | Flat::LAKE) {
                    self.taken(Grid::Mount, x, y, x + w, y + h) || self.taken(Grid::Check, x, y, x + w, y + h)
                } else if fl.contains(Flat::SLOPE) {
                    self.taken(Grid::Mount, x, y, x + w, y + h)
                        || self.too_steep(x, y, w, h)
                        || self.taken(Grid::Check, x, y, x + w, y + h)
                } else {
                    self.taken(Grid::Check, x, y, x + w, y + h)
                };
                if !bad {
                    break (x, y);
                }
                fails += 1;
                if fails == MAX_FAILS {
                    return;
                }
            };
            let level = if kind == Kind::Sub {
                let level = fl.intersects(Flat::LEVEL | Flat::LAKE);
                if level {
                    self.hide(x, y, w, h);
                    self.flat(x, y, 3 * w, 3 * h);
                }
                level
            } else {
                let level = fl.contains(Flat::LEVEL);
                if level {
                    self.flat(x, y, 3 * w, 3 * h);
                    self.hide(x, y, w, h);
                }
                level
            };
            self.place(kind, idx, info, x, y, level);
        }
    }

    /// `WORLD::SetTreeObject_B` (0x005ad250): rows of 3-5 pieces.
    fn set_tree_rows(&mut self, table: &'static [ObjectInfo], count: u32) {
        let mut fails = 0;
        for _ in 0..count {
            let length = self.rand(ROW_RANGE) + ROW_MIN;
            let (x, y) = loop {
                let x = self.rand(C as u32 - length);
                let y = self.rand(C as u32 - 1);
                if !(self.taken(Grid::Mount, x, y, x + length, y + 1)
                    || self.taken(Grid::Check, x, y, x + length, y + 1))
                {
                    break (x, y);
                }
                fails += 1;
                if fails == MAX_FAILS {
                    return;
                }
            };
            self.flat(x, y, 3 * length, 3);
            self.hide(x, y, length, 1);
            let mut pieces = vec![(x, self.rand(2) as usize)];
            for s in 0..length - 2 {
                pieces.push((x + s + 1, self.rand(2) as usize + ROW_MIDDLE));
            }
            pieces.push((x + length - 1, ROW_RIGHT));
            for (px, idx) in pieces {
                self.place(Kind::Tree, idx, &table[idx], px, y, true);
            }
        }
    }

    /// `WORLD::SetCover` (0x005aba50): cover on every chip not hidden.
    fn set_cover(&mut self) {
        let meshes = self.t.small_mesh[self.p.field_type as usize];
        for i in 0..C {
            for j in 0..C {
                if self.check[C * C + i * C + j] == 1 || self.check3[i * C + j] != 0 {
                    continue;
                }
                let quadrant = self.rand(QUADRANTS);
                let mesh = meshes[self.rand(meshes.len() as u32) as usize];
                self.covers.push(Cover { x: i as u32, y: j as u32, quadrant, mesh });
            }
        }
    }

    /// The start: the scan at the end of `Generate`, repeated until a chip
    /// passes. It reads `check` across into `check2`, as the game does.
    fn find_start(&mut self) -> Result<([u32; 2], [u32; 3]), GenerateError> {
        for _ in 0..GIVE_UP {
            for y in 0..C {
                for x in 0..C {
                    let at = x * C + y;
                    let chk = &self.check;
                    if chk[at] != 0 || chk[at + C] != 0 || chk[at + 1] != 0 || chk[at + C + 1] != 0 {
                        continue;
                    }
                    if self.rand(START_ROLL) < START_PASS {
                        continue;
                    }
                    let pos = Self::world_pos(x as u32, y as u32);
                    if !(self.p.event != 0 && self.p.protect) {
                        return Ok(([x as u32, y as u32], pos));
                    }
                    if !ee::le(self.dist(pos, self.dungeon_pos.unwrap_or([ZERO; 3])), START_FAR) {
                        return Ok(([x as u32, y as u32], pos));
                    }
                }
            }
        }
        Err(GenerateError::NoSite("start"))
    }
}

/// A field from `fieldSeed` and the area's attributes: `WORLD::Init`'s
/// draws, then `WORLD::Generate` up to the start position.
pub fn generate(t: &Tables, p: &Params) -> Result<Field, GenerateError> {
    let ft = p.field_type;
    if ft as usize >= FIELD_TYPES {
        return Err(GenerateError::FieldType(ft));
    }
    let Some(&hills) = t.hills.get(p.ground as usize) else {
        return Err(GenerateError::Ground(p.ground));
    };
    let mut g = Gen {
        t,
        p: *p,
        rng: Rng::new(p.seed),
        map: vec![ZERO; N * N],
        quads: Vec::new(),
        check: vec![0; 2 * C * C],
        check3: vec![0; C * C],
        mnt: vec![0; C * C],
        hills: Vec::new(),
        objects: Vec::new(),
        covers: Vec::new(),
        start_pos: None,
        entrance: None,
        dungeon_pos: None,
    };
    let init_draws = if p.skip_init { 0 } else { t.init_draws(ft, p.weather) };
    for _ in 0..init_draws {
        g.rng.advance();
    }
    let ftu = ft as usize;

    // FIELD::MakeField: white noise.
    for i in 0..N * N {
        g.map[i] = ee::from_int((g.rand(NOISE) & 0xff) as i32);
    }
    if p.event != 0 {
        let c = STORY_CENTRE as usize;
        for a in c - 1..=c + 1 {
            for b in c - 1..=c + 1 {
                g.check[b * C + a] = 1;
                g.mnt[b * C + a] = 1;
            }
        }
        g.flat(STORY_CENTRE, STORY_CENTRE, 3, 3);
        g.start_pos = Some(Gen::world_pos(STORY_CENTRE, STORY_CENTRE));
    }

    // WORLD::MakeHill: the large hills, then the small ones, each retried
    // until it finds a place.
    for (mut left, size) in [(hills.large, t.large_hill), (hills.small, t.small_hill)] {
        let mut tries = 0;
        while left > 0 {
            tries += 1;
            if tries > GIVE_UP {
                return Err(GenerateError::NoSite("hill"));
            }
            let n = g.rand(size.range) + size.base;
            if g.make_hill(n) {
                left -= 1;
            }
        }
    }

    if !t.no_entrance.contains(&p.event) {
        g.set_dungeon_enter(&t.enter.by_type[ftu][0])?;
    }
    if t.lake_fields.contains(&ft) {
        g.set_lake(t.lake.by_type[ftu])?;
    }
    let count = |kind| t.count(ft, kind, p.object);
    let (key, sub, base, tree) = (count(Kind::Key), count(Kind::Sub), count(Kind::Base), count(Kind::Tree));
    g.init_quad();
    g.set_objects(Kind::Key, t.key.by_type[ftu], key);
    g.set_objects(Kind::Sub, t.sub.by_type[ftu], sub);
    g.set_objects(Kind::Base, t.base.by_type[ftu], base);
    if t.tree_rows.contains(&ft) {
        g.set_tree_rows(t.tree.by_type[ftu], tree);
    } else {
        g.set_objects(Kind::Tree, t.tree.by_type[ftu], tree);
    }
    g.init_quad();
    g.set_cover();
    let (mut start, mut start_pos) = g.find_start()?;
    if p.event != 0 {
        // A story area starts at the flattened centre; the scan above still
        // ran and drew from the RNG.
        start = [STORY_CENTRE; 2];
        start_pos = Gen::world_pos(STORY_CENTRE, STORY_CENTRE);
    }
    Ok(Field {
        params: *p,
        init_draws,
        map: g.map,
        check: g.check,
        check3: g.check3,
        mnt: g.mnt,
        hills: g.hills,
        objects: g.objects,
        covers: g.covers,
        entrance: g.entrance,
        dungeon_pos: g.dungeon_pos,
        start,
        start_pos,
        rng: g.rng,
    })
}
