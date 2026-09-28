//! The tables the field generator reads (`piney_data::field::Tables`,
//! `docs/engine/field.md`): engine data read from gcmn.prg. The
//! `FOBJECT_TABLE`s (enter, lake, key, sub, base, tree objects per field type),
//! `fieldccs(2)`, the ground and cover tiles, what `WORLD::Generate`'s code
//! holds, the lake's water, the `fieldrand` draws before `Generate`, which
//! square root `ccGetDist` takes, the height cells `SetMESH2` and
//! `SetSmallMESH` read (run in piney-eemu), and the backgrounds. The object
//! rows are written once per array and named by index, as the game shares them.

use std::collections::{BTreeMap, HashMap};

use piney_eemu::cpu::Features;
use piney_eemu::machine::Machine;

use super::{Out, latin1};
use crate::program::Program;
use crate::volume::{Vol, ctx};
use crate::xfer;

const FIELD_TYPES: u32 = 11;
/// `clamp_weather` keeps an area's weather in 0-9.
const WEATHERS: u32 = 10;
const TABLE_KINDS: [&str; 6] =
    ["EnterObjTABLE", "LakeObjTABLE", "KeyObjTABLE", "SubObjTABLE", "BaseObjTABLE", "TreeObjTABLE"];
/// `FOBJECT_INFO_TABLE`.
const OBJECT_ROW: u32 = 28;
/// `BG_INFO_TABLE`.
const BG_ROW: u32 = 52;
/// WORLD's object counts (maxKeyObj, maxBaseObj, maxSubObj, maxTreeObj), by
/// offset, as `piney_data::field::Kind` numbers them.
const COUNT_FIELDS: [(i32, u32); 4] = [(0, KEY), (4, BASE), (8, SUB), (12, TREE)];
const KEY: u32 = 1;
const SUB: u32 = 2;
const BASE: u32 = 3;
const TREE: u32 = 4;
/// Signed-division magic numbers the compiler emits (mult, mfhi, add the
/// sign).
const MAGIC: [(u32, u32); 2] = [(0x5555_5556, 3), (0x2AAA_AAAB, 6)];

const S0: u32 = 16;
const V0: u32 = 2;
const A0: u32 = 4;
const A1: u32 = 5;

// code reading ------------------------------------------------------------------

/// One function's words, with the addresses its lui/lo pairs form.
struct Code<'a> {
    p: &'a Program,
    va: u32,
    words: Vec<u32>,
    addrs: HashMap<usize, u32>,
}

impl<'a> Code<'a> {
    fn new(p: &'a Program, name: &str) -> Result<Code<'a>, String> {
        let s = p.symbol_named(name).ok_or_else(|| format!("no symbol {name}"))?;
        let b = p.read(s.value, (s.size / 4 * 4) as usize)?;
        let words: Vec<u32> = b.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect();
        let addrs = xfer::addresses(&words, p.gp);
        Ok(Code { p, va: s.value, words, addrs })
    }

    fn at(&self, va: u32) -> usize {
        (va.wrapping_sub(self.va) / 4) as usize
    }

    fn word(&self, i: usize) -> u32 {
        self.words.get(i).copied().unwrap_or(0)
    }

    /// Indexes of every `jal callee`.
    fn jal(&self, callee: &str) -> Result<Vec<usize>, String> {
        let t = self.p.symbol_named(callee).ok_or_else(|| format!("no symbol {callee}"))?.value;
        let want = 0x0C00_0000 | (t >> 2 & 0x03FF_FFFF);
        Ok(self.words.iter().enumerate().filter(|&(_, &w)| w == want).map(|(i, _)| i).collect())
    }

    /// Branch target of instruction i.
    fn target(&self, i: usize) -> u32 {
        let imm = (self.words[i] & 0xFFFF) as i16 as i32;
        self.va.wrapping_add(4 * (i as u32 + 1)).wrapping_add((4 * imm) as u32)
    }
}

/// An instruction's fields: op, rs, rt, rd, sa, funct and the signed
/// immediate.
fn fields(w: u32) -> (u32, u32, u32, u32, u32, u32, i32) {
    (w >> 26, (w >> 21) & 31, (w >> 16) & 31, (w >> 11) & 31, (w >> 6) & 31, w & 63, (w & 0xFFFF) as i16 as i32)
}

/// (register, value) of `addiu rt, $zero, imm`.
fn li(w: u32) -> Option<(u32, i32)> {
    let (op, rs, rt, _, _, _, simm) = fields(w);
    (op == 0x09 && rs == 0).then_some((rt, simm))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Branch {
    Beq,
    Bne,
    Beqz,
}

/// (index, value, branch, target) for `li r, V` then a branch on r, and
/// `beqz`, in instructions lo..hi.
fn compares(c: &Code, lo: usize, hi: usize) -> Vec<(usize, i32, Branch, u32)> {
    let mut out = Vec::new();
    for i in lo..hi {
        let w = c.words[i];
        let (op, rs, rt) = (w >> 26, (w >> 21) & 31, (w >> 16) & 31);
        // beqz; not b (beq $zero, $zero)
        if op == 0x04 && rt == 0 && rs != 0 {
            out.push((i, 0, Branch::Beqz, c.target(i)));
            continue;
        }
        if let Some((reg, v)) = li(w)
            && i + 1 < hi
        {
            let n = c.words[i + 1];
            let (nop, nrs, nrt) = (n >> 26, (n >> 21) & 31, (n >> 16) & 31);
            if matches!(nop, 0x04 | 0x05) && (reg == nrs || reg == nrt) && (nrs, nrt) != (0, 0) {
                out.push((i, v, if nop == 0x04 { Branch::Beq } else { Branch::Bne }, c.target(i + 1)));
            }
        }
    }
    out
}

fn one<T>(mut items: Vec<T>, what: &str) -> Result<T, String> {
    if items.len() != 1 {
        return Err(format!("{what}: found {}, expected 1", items.len()));
    }
    Ok(items.remove(0))
}

/// The saved register the prologue moves `this` ($a0) into: $s0 in
/// Infection and Mutation, $s2 in Outbreak and Quarantine.
fn this_reg(c: &Code) -> u32 {
    for &w in c.words.iter().take(16) {
        let (op, rs, rt, rd, _, func, _) = fields(w);
        if op == 0
            && matches!(func, 0x21 | 0x25 | 0x2D)
            && (rs == A0 || rt == A0)
            && (rs == 0 || rt == 0)
            && (16..=23).contains(&rd)
        {
            return rd;
        }
    }
    S0
}

/// What `WORLD::Generate`'s code says.
#[derive(Debug, PartialEq)]
struct Constants {
    no_entrance: Vec<i32>,
    lake_fields: Vec<i32>,
    tree_rows: Vec<i32>,
    /// By `Kind` number.
    counts: BTreeMap<u32, i32>,
    /// (field type, kind, divisor).
    divide: Vec<(i32, u32, u32)>,
    /// Float bits by `object` 0, 1, and any other.
    percent: [u32; 3],
    /// (small, large) by `ground`.
    hills: [(i32, i32); 3],
    /// (range, base).
    large_hill: (i32, i32),
    small_hill: (i32, i32),
}

fn generate_constants(p: &Program) -> Result<Constants, String> {
    let c = Code::new(p, "Generate__5WORLDFv")?;
    let this = this_reg(&c);
    let enter = one(c.jal("SetDungeonEnter__5WORLDFiP13FOBJECT_TABLE")?, "SetDungeonEnter calls")?;
    let lake = one(c.jal("SetLake__5WORLDFP13FOBJECT_TABLE")?, "SetLake calls")?;
    let init_quad = *c.jal("InitQuad__5FIELDFv")?.first().ok_or("no InitQuad call")?;
    let base_call = one(c.jal("SetBaseObject__5WORLDFP13FOBJECT_TABLEi")?, "SetBaseObject calls")?;
    let rows_call = one(c.jal("SetTreeObject_B__5WORLDFP13FOBJECT_TABLEi")?, "SetTreeObject_B calls")?;
    let hill_calls = c.jal("MakeHill__5WORLDFi")?;
    if hill_calls.len() < 2 {
        return Err(format!("{} MakeHill calls, expected 2", hill_calls.len()));
    }

    // eventAreaNumber tests that skip SetDungeonEnter: li/beq pairs
    // branching past the call.
    let mut past = enter + 2;
    while c.word(past) == 0 {
        // Outbreak leaves a nop after the delay slot
        past += 1;
    }
    let after = c.va + 4 * past as u32;
    let no_entrance = compares(&c, 0, enter)
        .into_iter()
        .filter(|&(_, _, k, t)| k == Branch::Beq && t == after)
        .map(|x| x.1)
        .collect();
    // fieldType tests between the two calls: SetLake's field types.
    let lake_fields = compares(&c, enter + 2, lake).into_iter().map(|x| x.1).collect();
    // fieldType tests between SetBaseObject and SetTreeObject_B: the types
    // whose trees stand in rows.
    let tree_rows = compares(&c, base_call + 2, rows_call).into_iter().map(|x| x.1).collect();

    // maxKeyObj..maxTreeObj: li then sw off($s0), after SetLake.
    let count_kind = |off: i32| COUNT_FIELDS.iter().find(|&&(o, _)| o == off).map(|&(_, k)| k);
    let mut regs: HashMap<u32, i32> = HashMap::new();
    let mut counts = BTreeMap::new();
    let mut seen = Vec::new();
    let mut i = lake + 2;
    loop {
        let w = *c.words.get(i).ok_or("the counts run off Generate's end")?;
        let (op, rs, rt, _, _, _, simm) = fields(w);
        if op == 0x23 && rs == this && simm == 16 {
            // lw fieldType: the stores are done
            break;
        }
        if let Some((r, v)) = li(w) {
            regs.insert(r, v);
        } else if op == 0x2B
            && rs == this
            && let Some(kind) = count_kind(simm)
            && !seen.contains(&simm)
        {
            seen.push(simm);
            counts.insert(kind, *regs.get(&rt).ok_or_else(|| format!("count at +{simm} from an unknown register"))?);
        }
        i += 1;
    }

    // Per field type, `li $v0, FT; bne fieldType, $v0, skip`, then each
    // count divided: sra 1 (with the sign fix-up) halves, a magic mult
    // divides.
    let mut divide = Vec::new();
    for (ci, ft, kind, t) in compares(&c, i, init_quad) {
        if kind != Branch::Bne || !(0..FIELD_TYPES as i32).contains(&ft) {
            continue;
        }
        let mut divisor = None;
        for k in ci + 2..c.at(t) {
            let (op, rs, _, _, sa, func, simm) = fields(c.words[k]);
            if op == 0 && func == 3 && sa == 1 {
                // sra rd, rt, 1
                divisor = Some(2);
            } else if op == 0x0D && c.words[k - 1] >> 26 == 0x0F {
                // lui + ori
                let magic = (c.words[k - 1] & 0xFFFF) << 16 | (c.words[k] & 0xFFFF);
                let d = MAGIC.iter().find(|&&(m, _)| m == magic).ok_or_else(|| format!("magic 0x{magic:08x}"))?;
                divisor = Some(d.1);
            } else if op == 0x2B
                && rs == this
                && let Some(kind) = count_kind(simm)
                && let Some(d) = divisor
            {
                divide.push((ft, kind, d));
                divisor = None;
            }
        }
    }

    // The `object` percentage: a float register (Infection's $f20,
    // Outbreak's $f1) is 1.0, then per case a float built by lui (+ ori)
    // and moved to it.
    let freg = (i + 1..init_quad)
        .find(|&k| c.words[k] >> 21 == 0x224 && c.words[k - 1] >> 26 == 0x0F)
        .map(|k| fields(c.words[k]).3)
        .unwrap_or(20);
    // The float lui/ori builds and mtc1 moves to the register from index k.
    let float_at = |k: usize| -> Option<u32> {
        let (mut hi, mut lo) = (None, 0);
        for j in k..k + 4 {
            let w = c.word(j);
            let (op, _, _, rd, _, _, _) = fields(w);
            if op == 0x0F {
                hi = Some((w & 0xFFFF) << 16);
                lo = 0;
            } else if op == 0x0D {
                lo = w & 0xFFFF;
            } else if w >> 21 == 0x224
                && rd == freg
                && let Some(h) = hi
            {
                // mtc1 rt, freg
                return Some(h | lo);
            }
        }
        None
    };
    let start =
        (i..init_quad).find(|&k| float_at(k).is_some() && c.words[k] >> 26 == 0x0F).ok_or("no `object` percentage")?;
    let default = float_at(start).unwrap();
    let mut cases = HashMap::new();
    for (_, v, _, t) in compares(&c, start, init_quad) {
        if let Some(f) = float_at(c.at(t)) {
            cases.insert(v, f);
        }
    }
    let percent = [*cases.get(&0).unwrap_or(&default), *cases.get(&1).unwrap_or(&default), default];

    // Hills: a switch on ground (li $s3 small; $s4 large) before the loops,
    // each loop `fieldrand(range) + base` into MakeHill, then its counter
    // decremented. Each loop's counter is the register decremented after
    // its call; the first loop makes the large hills ($s4 in Infection, $s3
    // in Outbreak), the second the small.
    let first = hill_calls[0];
    let mut counters = Vec::new();
    for &call in &hill_calls {
        let r = (call + 1..call + 8)
            .map(|k| fields(c.word(k)))
            .find(|f| f.0 == 0x09 && f.6 == -1)
            .ok_or("a MakeHill loop without its counter")?
            .2;
        counters.push(r);
    }
    let (large, small) = (counters[0], counters[1]);
    let mut by_ground = HashMap::new();
    for (_, v, _, t) in compares(&c, 0, first) {
        // every case is reached with $v0 = 1 (li before beq)
        let regs: HashMap<u32, i32> = HashMap::from([(V0, 1)]);
        let mut k = c.at(t);
        let mut got: HashMap<u32, i32> = HashMap::new();
        while k < first && got.len() < 2 {
            let w = c.words[k];
            if let Some((r, v)) = li(w) {
                got.insert(r, v);
            } else if w & 0xFC1F_07FF == 0x0000_002D && [small, large].contains(&((w >> 11) & 31)) {
                // move rd, rs
                let rs = (w >> 21) & 31;
                got.insert((w >> 11) & 31, *regs.get(&rs).ok_or("a hill count from an unknown register")?);
            }
            k += 1;
        }
        if let (Some(&s), Some(&l)) = (got.get(&small), got.get(&large)) {
            by_ground.insert(v, (s, l));
        }
    }
    let hills = [0, 1, 2].map(|g| by_ground.get(&g).copied());
    let [Some(h0), Some(h1), Some(h2)] = hills else {
        return Err(format!("hills by ground: {hills:?}"));
    };
    let mut sizes = HashMap::new();
    for (&call, &counter) in hill_calls.iter().zip(&counters) {
        let (mut base, mut range) = (None, None);
        for k in call.saturating_sub(8)..call {
            let (op, rs, rt, _, _, _, simm) = fields(c.words[k]);
            if op == 0x09 && rs == V0 && rt == A1 {
                // addiu $a1, $v0, base
                base = Some(simm);
            } else if let Some((A0, v)) = li(c.words[k]) {
                range = Some(v);
            }
        }
        let (Some(range), Some(base)) = (range, base) else {
            return Err("a MakeHill call without its size".into());
        };
        sizes.insert(counter == large, (range, base));
    }
    Ok(Constants {
        no_entrance,
        lake_fields,
        tree_rows,
        counts,
        divide,
        percent,
        hills: [h0, h1, h2],
        large_hill: sizes[&true],
        small_hill: sizes[&false],
    })
}

/// The constants `tools/field.py` and the port's generator were written
/// against; a volume whose code says otherwise stops the generator.
fn check_constants(k: &Constants) -> Result<(), String> {
    let want = Constants {
        no_entrance: vec![113, 112, 111, 110, 109, 82, 81, 80, 79, 78, 57, 56, 55, 54, 53, 42, 41, 40, 39, 28],
        lake_fields: vec![10, 2, 3, 9, 8],
        tree_rows: vec![1],
        counts: BTreeMap::from([(KEY, 20), (BASE, 35), (SUB, 30), (TREE, 35)]),
        divide: vec![(7, KEY, 2), (7, BASE, 3), (7, TREE, 2), (1, TREE, 6)],
        percent: [0.8f32.to_bits(), 0.9f32.to_bits(), 1.0f32.to_bits()],
        hills: [(3, 1), (5, 1), (8, 1)],
        large_hill: (10, 20),
        small_hill: (10, 10),
    };
    if *k != want {
        return Err(format!("WORLD::Generate says {k:?}, the generator expects {want:?}"));
    }
    Ok(())
}

/// (effect CCS file, the lake's water-surface animation): the string
/// WORLD::Init loads into WORLD.effccs (+0x6154), and the animation SetLake
/// takes from it.
fn water(p: &Program) -> Result<(String, String), String> {
    let c = Code::new(p, "Init__5WORLDFv")?;
    let this = this_reg(&c);
    let store = c
        .words
        .iter()
        .position(|&w| w >> 26 == 0x2B && (w >> 21) & 31 == this && w & 0xFFFF == 0x6154)
        .ok_or("WORLD::Init stores no effccs")?;
    let last = c.addrs.keys().filter(|&&i| i < store).max().ok_or("no address before effccs")?;
    let ccs = latin1(&p.cstr(c.addrs[last], 64)?);
    let c = Code::new(p, "SetLake__5WORLDFP13FOBJECT_TABLE")?;
    let mut names: Vec<String> = c
        .addrs
        .values()
        .filter(|&&a| p.mapped(a))
        .filter_map(|&a| p.cstr(a, 64).ok())
        .filter(|s| s.starts_with(b"ANM_"))
        .map(|s| latin1(&s))
        .collect();
    names.sort();
    names.dedup();
    Ok((ccs, one(names, "SetLake's animations")?))
}

// the meshes' vertex tables -----------------------------------------------------

const MAP: u32 = 80;
const MESH_FIELD: u32 = 0x0100_0000;
const MESH_OBJ: u32 = 0x0110_0000;
const MESH_MODEL: u32 = 0x0110_0100;
const MESH_MMAT: u32 = 0x0110_0200;
const MESH_VERTS: u32 = 0x0110_1000;
const MESH_COLS: u32 = 0x0110_2000;
const MESH_SENTINEL: u16 = 0x7777;
const COL_SENTINEL: u32 = 0x7777_7777;

/// A vertex rewritten from a height cell: (vertex, cell offset x, y).
type MeshCell = (u8, i8, i8);

fn put(m: &mut Machine, a: u32, b: &[u8]) {
    m.ram[a as usize..a as usize + b.len()].copy_from_slice(b);
}

fn get(m: &Machine, a: u32, n: usize) -> &[u8] {
    &m.ram[a as usize..a as usize + n]
}

/// Which height cell `FIELD_MESH::SetMESH2` (chip (x, y): cells (2x + dx,
/// 2y + dy)) and `FIELD::SetSmallMESH` (cell (x, y): cells (x + dx, y +
/// dy)) write into each vertex's z and colour: both run over a FIELD whose
/// every height and colour names its own cell. (tile, tile colour, cover,
/// cover colour).
fn mesh_tables(p: &Program) -> Result<[Vec<MeshCell>; 4], String> {
    let mut m = crate::sinit::machine(p, Features::default());
    let fld = MESH_FIELD;
    for x in 0..MAP {
        for y in 0..MAP {
            let cell = x * MAP + y;
            put(&mut m, fld + 0xAF02C + 4 * cell, &(cell as f32).to_bits().to_le_bytes());
            put(&mut m, fld + 0xB5430 + 16 * cell, &(x as f32).to_bits().to_le_bytes());
            put(&mut m, fld + 0xB5430 + 16 * cell + 4, &(y as f32).to_bits().to_le_bytes());
        }
    }
    let reset = |m: &mut Machine| {
        put(m, MESH_MODEL + 8, &MESH_MMAT.to_le_bytes());
        // vertexScale 4096: s16 = height
        put(m, MESH_MODEL + 12, &4096f32.to_bits().to_le_bytes());
        put(m, MESH_MMAT + 16, &MESH_VERTS.to_le_bytes());
        put(m, MESH_MMAT + 24, &MESH_COLS.to_le_bytes());
        for k in 0..256 {
            put(m, MESH_VERTS + 6 * k + 4, &MESH_SENTINEL.to_le_bytes());
            put(m, MESH_COLS + 4 * k, &COL_SENTINEL.to_le_bytes());
        }
    };
    let read = |m: &Machine, ox: i32, oy: i32| {
        let (mut z, mut col) = (Vec::new(), Vec::new());
        for k in 0..256u32 {
            let v = u16::from_le_bytes(get(m, MESH_VERTS + 6 * k + 4, 2).try_into().unwrap());
            if v != MESH_SENTINEL {
                let v = i32::from(v);
                z.push((k as u8, (v / MAP as i32 - ox) as i8, (v % MAP as i32 - oy) as i8));
            }
            let c = get(m, MESH_COLS + 4 * k, 4);
            if u32::from_le_bytes(c.try_into().unwrap()) != COL_SENTINEL {
                col.push((k as u8, (i32::from(c[0]) - ox) as i8, (i32::from(c[1]) - oy) as i8));
            }
        }
        (z, col)
    };
    let sym = |n: &str| p.symbol_named(n).map(|s| s.value).ok_or_else(|| format!("no symbol {n}"));
    reset(&mut m);
    // FIELD_MESH.parent, .model
    put(&mut m, MESH_OBJ, &fld.to_le_bytes());
    put(&mut m, MESH_OBJ + 76, &MESH_MODEL.to_le_bytes());
    m.call(sym("SetMESH2__10FIELD_MESHFiiii")?, &[MESH_OBJ, 10, 10, 2, 2], 10_000_000)
        .map_err(|e| format!("SetMESH2: {e:?}"))?;
    let (tile, tile_colour) = read(&m, 20, 20);
    reset(&mut m);
    m.call(sym("SetSmallMESH__5FIELDFiiP7ccModel")?, &[fld, 21, 21, MESH_MODEL], 10_000_000)
        .map_err(|e| format!("SetSmallMESH: {e:?}"))?;
    let (cover, cover_colour) = read(&m, 21, 21);
    Ok([tile, tile_colour, cover, cover_colour])
}

// the file ----------------------------------------------------------------------

/// `WORLD::Init`'s fieldrand calls before `Generate`, by field type and
/// weather: weather particles (100 SNOW of 7 calls, or 200 of 8 when
/// GetWeather() is 5) for the snow fields 5 and 6, 16 SNOW of 7 for type 0,
/// smoke (1 + 5 x 4) for 2, 3, 5 and 6. Measured by running WORLD::Init in
/// tools/eemu.py for every field type and weather, the same on all four
/// volumes; tools/test_field.py re-derives it.
fn init_draws(field_type: u32, weather: u32) -> u16 {
    let mut n = match field_type {
        0 => 112,
        5 | 6 if weather >= 4 => 1600,
        5 | 6 => 700,
        _ => 0,
    };
    if matches!(field_type, 2 | 3 | 5 | 6) {
        n += 21;
    }
    n
}

fn text(p: &Program, va: u32) -> Result<Option<String>, String> {
    if va == 0 {
        return Ok(None);
    }
    Ok(Some(latin1(&p.cstr(va, 64)?)))
}

fn named(p: &Program, va: u32, what: &str) -> Result<String, String> {
    text(p, va)?.ok_or_else(|| format!("{what}: a NULL name"))
}

fn sym(p: &Program, name: &str) -> Result<(u32, u32), String> {
    p.symbol_named(name).map(|s| (s.value, s.size)).ok_or_else(|| format!("no symbol {name}"))
}

/// A table of names (`fieldccs`), NULL as None.
fn strings(p: &Program, name: &str) -> Result<Vec<Option<String>>, String> {
    let (va, size) = sym(p, name)?;
    (0..size / 4).map(|i| text(p, p.u32(va + 4 * i)?)).collect()
}

/// A per-field-type `{ptr, num}` table.
fn per_type(p: &Program, name: &str) -> Result<Vec<(u32, u32)>, String> {
    let (base, _) = sym(p, name)?;
    (0..FIELD_TYPES)
        .map(|ft| {
            let num = p.u32(base + 8 * ft + 4)? as i32;
            let num = u32::try_from(num).map_err(|_| format!("{name}[{ft}]: {num} rows"))?;
            Ok((p.u32(base + 8 * ft)?, num))
        })
        .collect()
}

fn opt(out: &mut Out, s: &Option<String>) {
    out.some(s.is_some());
    if let Some(s) = s {
        out.str(s);
    }
}

/// A per-type table of name lists (`BaseMeshName`).
fn names_by_type(p: &Program, name: &str, out: &mut Out) -> Result<(), String> {
    for (ptr, num) in per_type(p, name)? {
        out.count(num as usize);
        for k in 0..num {
            out.str(&named(p, p.u32(ptr + 4 * k)?, name)?);
        }
    }
    Ok(())
}

/// One `FOBJECT_INFO_TABLE` row as `ObjectInfo`.
fn object_row(p: &Program, va: u32, out: &mut Out) -> Result<(), String> {
    let w: Vec<u32> =
        p.read(va, OBJECT_ROW as usize)?.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect();
    let where_ = format!("the object row at 0x{va:08x}");
    opt(out, &text(p, w[0])?);
    out.str(&named(p, w[1], &where_)?);
    if w[2] > 3 {
        return Err(format!("{where_}: type {} is not 0-3", w[2] as i32));
    }
    out.u8(w[2] as u8);
    for v in [w[3], w[4]] {
        if (v as i32) < 0 {
            return Err(format!("{where_}: size {}", v as i32));
        }
        out.u32(v);
    }
    let rot = u8::try_from(w[5]).map_err(|_| format!("{where_}: rotflag {} does not fit a u8", w[5] as i32))?;
    out.u8(rot);
    out.u32(w[6]);
    Ok(())
}

/// One `BG_INFO_TABLE` row as `BgRow`.
fn bg_row(p: &Program, va: u32, out: &mut Out) -> Result<(), String> {
    let w: Vec<u32> = p.read(va, BG_ROW as usize)?.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect();
    let where_ = format!("the background row at 0x{va:08x}");
    opt(out, &text(p, w[0])?);
    for &c in &w[1..7] {
        out.str(&named(p, c, &where_)?);
    }
    for &f in &w[7..13] {
        out.u32(f);
    }
    Ok(())
}

/// Field type 6's aurora clumps: `WORLD::Init` copies a 24-byte table of
/// clump names to its stack and loads the one for the weather.
fn aurora(p: &Program) -> Result<Vec<String>, String> {
    let c = Code::new(p, "Init__5WORLDFv")?;
    let mut at: Vec<(usize, u32)> = c.addrs.iter().map(|(&i, &a)| (i, a)).collect();
    at.sort();
    for (_, va) in at {
        let Ok(first) = p.u32(va) else { continue };
        if first == 0 {
            continue;
        }
        let Ok(name) = p.cstr(first, 64) else { continue };
        if name.starts_with(b"CMP_") && latin1(&name).contains("aur") {
            return (0..6).map(|k| named(p, p.u32(va + 4 * k)?, "the aurora")).collect();
        }
    }
    Err("WORLD::Init names no aurora".into())
}

/// The volume's `piney_data::field::Tables`.
pub fn bytes(v: Vol) -> Result<Vec<u8>, String> {
    let c = ctx(v, Some("gcmn"));
    let p = &c.p;
    let mut out = Out::default();
    out.u32(p.u32(sym(p, "volumeNum")?.0)?);
    for name in ["fieldccs", "fieldccs2"] {
        let names = strings(p, name)?;
        if names.len() != FIELD_TYPES as usize {
            return Err(format!("{name} has {} entries", names.len()));
        }
        for s in &names {
            opt(&mut out, s);
        }
    }
    names_by_type(p, "BaseMeshName", &mut out)?;
    names_by_type(p, "SmallMeshName", &mut out)?;

    // The row arrays, each once, then each table's by index.
    let mut arrays: Vec<(u32, u32)> = Vec::new();
    let mut tables = Vec::new();
    for name in TABLE_KINDS {
        let mut refs = Vec::new();
        for (ft, (ptr, num)) in per_type(p, name)?.into_iter().enumerate() {
            let k = match arrays.iter().position(|&(a, _)| a == ptr) {
                Some(k) if arrays[k].1 != num => {
                    return Err(format!("{name}[{ft}]: 0x{ptr:08x} read with {} and {num} rows", arrays[k].1));
                }
                Some(k) => k,
                None => {
                    arrays.push((ptr, num));
                    arrays.len() - 1
                }
            };
            refs.push(k as u32);
        }
        tables.push((name, refs));
    }
    out.count(arrays.len());
    for &(ptr, num) in &arrays {
        out.count(num as usize);
        for k in 0..num {
            object_row(p, ptr + OBJECT_ROW * k, &mut out)?;
        }
    }
    for (name, refs) in tables {
        out.str(name);
        for k in refs {
            out.u32(k);
        }
    }

    for ft in 0..FIELD_TYPES {
        for w in 0..WEATHERS {
            out.u16(init_draws(ft, w));
        }
    }
    let k = generate_constants(p)?;
    check_constants(&k)?;
    for list in [&k.no_entrance, &k.lake_fields, &k.tree_rows] {
        out.count(list.len());
        for &e in list {
            out.i32(e);
        }
    }
    for kind in [KEY, SUB, BASE, TREE] {
        out.i32(k.counts[&kind]);
    }
    out.count(k.divide.len());
    for &(ft, kind, by) in &k.divide {
        out.i32(ft);
        out.u32(kind);
        out.u32(by);
    }
    for b in k.percent {
        out.u32(b);
    }
    for (s, l) in k.hills {
        out.i32(s);
        out.i32(l);
    }
    for (range, base) in [k.large_hill, k.small_hill] {
        out.i32(range);
        out.i32(base);
    }
    let (effect, water) = water(p)?;
    out.str(&effect);
    out.str(&water);
    // ccGetDist's square root: a call of newlib's sqrtf, which rounds to
    // nearest (INF, MUT), or the FPU's sqrt.s inline (OUT, QUA).
    let dist = Code::new(p, "ccGetDist__FPfPf")?;
    out.u8(u8::from(dist.words.iter().any(|&w| w & 0xFFE0_003F == 0x4600_0004)));
    for cells in mesh_tables(p)? {
        out.count(cells.len());
        for (k, dx, dy) in cells {
            out.u8(k);
            out.u8(dx as u8);
            out.u8(dy as u8);
        }
    }

    // The backgrounds.
    let (back, back2) = (strings(p, "backccs")?, strings(p, "backccs2")?);
    if back.len() != FIELD_TYPES as usize || back2.len() != FIELD_TYPES as usize {
        return Err(format!("backccs has {} entries, backccs2 {}", back.len(), back2.len()));
    }
    for s in back {
        out.str(&s.ok_or("backccs: a NULL name")?);
    }
    for s in &back2 {
        opt(&mut out, s);
    }
    for name in ["BGTBL", "BGTBL2"] {
        for (ptr, num) in per_type(p, name)? {
            out.count(num as usize);
            for k in 0..num {
                bg_row(p, ptr + BG_ROW * k, &mut out)?;
            }
        }
    }
    names_by_type(p, "BgMatName", &mut out)?;
    names_by_type(p, "BgMatName2", &mut out)?;
    let aurora = aurora(p)?;
    out.count(aurora.len());
    for a in &aurora {
        out.str(a);
    }
    Ok(out.0)
}
