//! The tables the dungeon generator reads (`piney_data::dungeon::Tables`,
//! `docs/engine/dungeon.md`): engine data only, each table found through the
//! code that uses it so one reader runs on all four executables. The
//! `ROOM_INFO` tables, `symroom`, `DungeonName(2)`, `SetAllGim`'s dummy
//! patterns, `dungeonData`, the dungeon types by field type, `EditDungeon`'s
//! story dungeons, the CLUT lists, the doors, the fog tables, and the
//! texType / clutType pairs run in piney-eemu. Shared tables are written once
//! and named by index; the room models' dummies are read from the CCS file.

use std::collections::{BTreeMap, HashMap};

use piney_eemu::cpu::Features;

use super::{Out, latin1};
use crate::program::Program;
use crate::volume::{Vol, ctx};
use crate::xfer;

/// Dungeon type -> room table suffix (MakeFloor's jump tables).
const TABLE_SUFFIX: [&str; 10] = ["A", "B", "C", "4", "E0", "E1", "E2", "E4", "X", "X"];
/// SetAllGim's searches, in order: (name, prefix search, `Gim` number in
/// `piney_data::dungeon::Gim`'s order).
const GIM_PATTERNS: [(&str, bool, u8); 8] = [
    ("OBJ_0ppi", true, 0),
    ("OBJ_0ppm", true, 1),
    ("OBJ_0ppg", false, 2),
    ("OBJ_o_fountain_l0_", false, 3),
    ("OBJ_0ps0", true, 4),
    ("OBJ_0ps1", true, 5),
    ("OBJ_0ps2", true, 6),
    ("OBJ_0ps3", true, 7),
];
/// `ROOM_INFO.r`: the four angles every table uses, by float bits, as
/// `Rotation` numbers them.
const ROTATIONS: [(u32, u8); 4] = [(0x0000_0000, 0), (0x3FC9_0FDB, 1), (0x4049_0FDB, 2), (0xBFC9_0FDB, 3)];
/// Field types SetDungeonTypeFromField has cases for; one row more for "any
/// other".
const FIELD_TYPES: u32 = 11;
/// Servers SetDungeonTexClut has cases for (Delta .. Omega).
const SERVERS: u32 = 5;
/// A dungeonFog* row: nine floats; each table has one row per dungeon type
/// 0-3 (plain), 4-7 (hacked) or background (forest).
const FOG_ROW: u32 = 0x24;
const FOG_ROWS: u32 = 4;
/// DUNGEON fields the constructor's fog block writes, and WORLD_MAN's it
/// reads.
const FOG_PARAM: u32 = 0x144;
const FOG_INDEX: u32 = 0x148;
const WM_CLUT_TYPE: u32 = 0x140;
const WM_TEX_TYPE: u32 = 0x144;
const OP_ADDIU: u32 = 0x09;
const OP_ORI: u32 = 0x0D;
const OP_SLTIU: u32 = 0x0B;
const OP_LW: u32 = 0x23;
const OP_SW: u32 = 0x2B;
const OP_BNE: u32 = 0x05;
const OP_JAL: u32 = 0x03;
const R_ZERO: u32 = 0;
const R_V0: u32 = 2;
const R_S0: u32 = 16;

/// Story areas with a dungeon type of their own, and the same on the
/// random path (tools/dungeon.py's FIXED_TYPES, PLAIN_FIXED_TYPES).
const FIXED_TYPES: [(i32, u8); 6] = [(120, 3), (66, 5), (91, 7), (16, 7), (18, 4), (21, 6)];
const PLAIN_FIXED_TYPES: [(i32, u8); 6] = [(120, 3), (66, 1), (91, 3), (16, 3), (18, 0), (21, 2)];

/// (opcode, rs, rt, immediate) of an I-type word.
fn fields(w: u32) -> (u32, u32, u32, u32) {
    (w >> 26, (w >> 21) & 31, (w >> 16) & 31, w & 0xFFFF)
}

/// One function's words and the addresses they form.
struct Code {
    va: u32,
    words: Vec<u32>,
    addrs: HashMap<usize, u32>,
}

impl Code {
    fn new(p: &Program, name: &str) -> Result<Code, String> {
        let s = p.symbol_named(name).ok_or_else(|| format!("no symbol {name}"))?;
        let size = match p.function_at(s.value) {
            Some(f) if f.value == s.value && f.size != 0 => f.size,
            _ => s.size,
        };
        let b = p.read(s.value, (size / 4 * 4) as usize)?;
        let words: Vec<u32> = b.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect();
        let addrs = xfer::addresses(&words, p.gp);
        Ok(Code { va: s.value, words, addrs })
    }

    /// The indexes that form an address, in order.
    fn sorted(&self) -> Vec<usize> {
        let mut k: Vec<usize> = self.addrs.keys().copied().collect();
        k.sort();
        k
    }

    fn at(&self, va: u32) -> usize {
        (va.wrapping_sub(self.va) / 4) as usize
    }
}

fn text(p: &Program, va: u32) -> Result<String, String> {
    Ok(latin1(&p.cstr(va, 64)?))
}

/// The C string at va, or "" where nothing is mapped (a field offset formed
/// with lui).
fn text_or_empty(p: &Program, va: u32) -> String {
    p.cstr(va, 64).map(|b| latin1(&b)).unwrap_or_default()
}

fn sym(p: &Program, name: &str) -> Result<u32, String> {
    p.symbol_named(name).map(|s| s.value).ok_or_else(|| format!("no symbol {name}"))
}

/// The symbol starting at va when its name is an identifier, else
/// `{what}_{va:08x}`.
fn name_of(p: &Program, va: u32, what: &str) -> String {
    let ident = |n: &str| {
        let mut c = n.chars();
        c.next().is_some_and(|f| f == '_' || f.is_alphabetic()) && c.all(|x| x == '_' || x.is_alphanumeric())
    };
    match p.symbol_at(va, 0) {
        Some((s, 0)) if ident(&s.name) => s.name.clone(),
        _ => format!("{what}_{va:08x}"),
    }
}

/// `"OBJ_x*"` -> `"OBJ_x"`: ccAnm::GetSubstAdrs' trailing '*' matches any
/// rest.
fn obj_pattern(s: &str, what: &str) -> Result<String, String> {
    match s.strip_suffix('*') {
        Some(x) if s.starts_with("OBJ_") => Ok(x.to_string()),
        _ => Err(format!("{what}: {s:?} is not an OBJ_ prefix search")),
    }
}

/// A room table: (name, va, rows (exits, rotation bits, models)).
type RoomTable = (String, u32, Vec<(u8, u32, Vec<String>)>);

/// MakeFloor's random branch picks a ROOM_INFO table through one of four
/// jump tables indexed by dungeon type (`sltiu $at, $v1, 10`): small,
/// medium, large, and the large rooms' second set. Each case passes the
/// table in $a3 and its row count in $t0 to DUNGEON::MakeRoom. Returns the
/// tables, each once, and the jump tables as [4][10] indexes into them; a
/// table is named by its first slot, as Infection's symbols name them.
fn room_tables(p: &Program) -> Result<(Vec<RoomTable>, Vec<[u32; 10]>), String> {
    let c = Code::new(p, "MakeFloor__7DUNGEONFv")?;
    let make_room = sym(p, "MakeRoom__7DUNGEONFiP4FOOTP9ROOM_INFOi")?;
    let mut jumps = Vec::new();
    for (i, &w) in c.words.iter().enumerate() {
        if w >> 26 == OP_SLTIU && w & 0xFFFF == 10 {
            let jt = (i + 1..i + 8).find_map(|j| c.addrs.get(&j)).ok_or("MakeFloor: a switch without its table")?;
            jumps.push(*jt);
        }
    }
    if jumps.len() != 4 {
        return Err(format!("MakeFloor: {} room-table jump tables, expected 4", jumps.len()));
    }
    let mut tables: Vec<(String, u32, u32)> = Vec::new();
    let mut index = Vec::new();
    for (n, &jt) in jumps.iter().enumerate() {
        let mut row = [0u32; 10];
        for (dtype, slot) in row.iter_mut().enumerate() {
            let mut k = c.at(p.u32(jt + 4 * dtype as u32)?);
            let (mut table, mut count) = (None, None);
            loop {
                let w = *c.words.get(k).ok_or("MakeFloor: a case runs off the end")?;
                let (op, rs, rt, imm) = fields(w);
                if rt == 7
                    && let Some(&a) = c.addrs.get(&k)
                {
                    table = Some(a);
                }
                if op == OP_ADDIU && rs == 0 && rt == 8 {
                    count = Some(imm);
                }
                if op == OP_JAL {
                    let callee = ((c.va + 4 * k as u32 + 4) & 0xF000_0000) | ((w & 0x03FF_FFFF) << 2);
                    if callee != make_room {
                        return Err(format!("MakeFloor: case at 0x{:x} calls something else", c.va + 4 * k as u32));
                    }
                    let next = c.words[k + 1];
                    if (next >> 16) & 31 == 7
                        && let Some(&a) = c.addrs.get(&(k + 1))
                    {
                        table = Some(a);
                    }
                    if next >> 26 == OP_ADDIU && (next >> 16) & 31 == 8 {
                        count = Some(next & 0xFFFF);
                    }
                    break;
                }
                k += 1;
            }
            let (Some(table), Some(count)) = (table, count) else {
                return Err(format!("MakeFloor: jump table {n} type {dtype} without its table or count"));
            };
            *slot = match tables.iter().position(|t| t.1 == table) {
                Some(i) => i as u32,
                None => {
                    let name = format!(
                        "{}room_info{}{}",
                        ["s", "m", "l"][n.min(2)],
                        TABLE_SUFFIX[dtype],
                        if n == 3 { "_2" } else { "" }
                    );
                    tables.push((name, table, count));
                    tables.len() as u32 - 1
                }
            };
        }
        index.push(row);
    }
    let mut out = Vec::new();
    for (name, va, count) in tables {
        let mut rows = Vec::new();
        for k in 0..count {
            let b = p.read(va + 16 * k, 16)?;
            let w = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
            let (objs, num, exit, r) = (w(0), w(4), b[8], w(12));
            let models = (0..num).map(|j| text(p, p.u32(objs + 4 * j)?)).collect::<Result<Vec<_>, _>>()?;
            rows.push((exit, r, models));
        }
        out.push((name, va, rows));
    }
    Ok((out, index))
}

/// MakeRoom copies its symroom[10] initialiser (40 bytes: lq, lq, ld 32) to
/// the stack; the address formed just before the ld.
fn symroom(p: &Program) -> Result<u32, String> {
    let c = Code::new(p, "MakeRoom__7DUNGEONFiP4FOOTP9ROOM_INFOi")?;
    for i in c.sorted() {
        if c.words.iter().skip(i + 1).take(4).any(|&w| w >> 26 == 0x37 && w & 0xFFFF == 32) {
            return Ok(c.addrs[&i]);
        }
    }
    Err("MakeRoom: no symroom initialiser".into())
}

/// DungeonName[] alternates "sd1.ccs", "sd1"; the constructor passes the
/// second of each pair to ccStream::GetCCSAdrs.
fn name_list(p: &Program, table: &str) -> Result<Vec<String>, String> {
    let base = sym(p, table)?;
    (0..10).map(|i| text(p, p.u32(base + 8 * i + 4)?)).collect()
}

/// The C strings SetAllGim builds, in order, checked against the searches
/// the port's generator knows.
fn gim_patterns(p: &Program) -> Result<(), String> {
    let c = Code::new(p, "SetAllGim__7DUNGEONFi")?;
    let mut out = Vec::new();
    for i in c.sorted() {
        let s = text_or_empty(p, c.addrs[&i]);
        if s.starts_with("OBJ_") {
            out.push((s.trim_end_matches('*').to_string(), s.ends_with('*')));
        }
    }
    let want: Vec<(String, bool)> = GIM_PATTERNS.iter().map(|&(n, w, _)| (n.to_string(), w)).collect();
    if out != want {
        return Err(format!("SetAllGim searches {out:?}, the generator knows {want:?}"));
    }
    Ok(())
}

/// EditDungeon's length: the bound of the DUNGEON constructor's search
/// (`sltiu` after the table's address; 88 in Infection, 90 later).
fn edit_count(p: &Program) -> Result<u32, String> {
    let c = Code::new(p, "__ct__7DUNGEONFi")?;
    let base = sym(p, "EditDungeon")?;
    let i = c.addrs.iter().filter(|&(_, &a)| a == base).map(|(&i, _)| i).min().ok_or("no EditDungeon address")?;
    c.words
        .iter()
        .skip(i)
        .take(32)
        .find(|&&w| w >> 26 == OP_SLTIU)
        .map(|&w| w & 0xFFFF)
        .ok_or_else(|| "DUNGEON::DUNGEON: no EditDungeon bound".to_string())
}

/// (floors, {event area: floors}): how many floors DUNGEON::Generate runs
/// MakeRealMap over. Infection: the constant 10 in Generate's loop (`li $s0,
/// 10`). From Mutation on Generate reads DUNGEON.floors (+0x430), which the
/// constructor sets to 10, or 15 for event area 125.
fn edit_floors(p: &Program) -> Result<(u32, BTreeMap<i32, u32>), String> {
    let c = Code::new(p, "Generate__7DUNGEONFv")?;
    let mut field = None;
    for &w in &c.words {
        let (op, rs, rt, imm) = fields(w);
        if rt == 16 && op == OP_ADDIU && rs == 0 {
            return Ok((imm, BTreeMap::new()));
        }
        if rt == 16 && op == OP_LW {
            field = Some(imm);
            break;
        }
    }
    let field = field.ok_or("Generate: no floor count")?;
    let c = Code::new(p, "__ct__7DUNGEONFi")?;
    let mut li: HashMap<u32, i32> = HashMap::new();
    let (mut default, mut special, mut compared) = (None, BTreeMap::new(), None);
    for &w in &c.words {
        let (op, rs, rt, imm) = fields(w);
        if op == OP_ADDIU && rs == 0 {
            li.insert(rt, imm as i16 as i32);
        } else if op == OP_SW && imm == field && li.contains_key(&rt) {
            if default.is_none() {
                default = Some(li[&rt]);
            } else if let Some(e) = compared {
                special.insert(e, li[&rt] as u32);
                break;
            }
        } else if op == OP_BNE && default.is_some() && (li.contains_key(&rs) || li.contains_key(&rt)) {
            compared = li.get(&rt).or_else(|| li.get(&rs)).copied();
        } else if op == 0 && !matches!(w & 0x3F, 0x08 | 0x09) {
            li.remove(&((w >> 11) & 31));
        } else if (0x08..=0x0F).contains(&op) || (0x20..=0x27).contains(&op) || op == 0x37 {
            li.remove(&rt);
        }
    }
    Ok((default.ok_or("DUNGEON::DUNGEON: no floor count")? as u32, special))
}

/// What the save flag does in WORLD_MAN::SetDungeonTypeFromField: the first
/// saveData field read after the eventAreaInfo search. Infection reads the
/// byte at saveData+0x6772 inside the story areas' cases and takes the "E"
/// type like EVENTAREA_INFO.flag 3 (`None` bit). Mutation on read bit 62 of
/// the u64 at saveData+0x5ec8 before the story areas' cases and, when set,
/// give a story area a random area's type. (offset, bit).
fn save_flag(p: &Program) -> Result<(u32, Option<u32>), String> {
    let c = Code::new(p, "SetDungeonTypeFromField__9WORLD_MANFv")?;
    let save = sym(p, "saveData")?;
    let info = sym(p, "eventAreaInfo")?;
    let start = c.addrs.iter().filter(|&(_, &a)| a == info).map(|(&i, _)| i).min().ok_or("no eventAreaInfo")?;
    let mut base = Vec::new();
    for i in start..c.words.len() {
        let (op, rs, rt, imm) = fields(c.words[i]);
        if op == OP_LW && c.addrs.get(&i) == Some(&save) {
            base.push(rt);
        } else if matches!(op, 0x20 | 0x24) && base.contains(&rs) {
            return Ok((imm, None));
        } else if op == 0x37 && base.contains(&rs) {
            for j in i + 1..(i + 6).min(c.words.len() - 1) {
                let (w2, w3) = (c.words[j], c.words[j + 1]);
                // lui; dsll32
                if w2 >> 26 == 0x0F && w3 & 0xFFE0_003F == 0x0000_003C {
                    return Ok((imm, Some(48 + (32 - (w2 & 0xFFFF).leading_zeros()) - 1)));
                }
            }
        }
    }
    Err("SetDungeonTypeFromField: no save flag".into())
}

/// tools/dungeon.py's model of WORLD_MAN::SetDungeonTypeFromField:
/// `dungeonType[0..1]`. `plain` is the rule from Mutation on.
fn dungeon_type(field_type: u32, event: i32, event_flag: i32, save_flag: bool, plain: bool) -> [u8; 2] {
    let mut types = [0u8, 0];
    if event == 0 || (plain && save_flag) {
        if let Some(&(_, t)) = PLAIN_FIXED_TYPES.iter().find(|&&(e, _)| e == event) {
            types[0] = t;
            return types;
        }
        types[0] = match field_type {
            0 => 2,
            1 => 1,
            2 | 3 => 3,
            4 => 8,
            5 | 6 => 1,
            7 => 0,
            8 => 3,
            9 => 2,
            _ => 0,
        };
        if field_type == 4 {
            types[1] = 2;
        }
        return types;
    }
    if let Some(&(_, t)) = FIXED_TYPES.iter().find(|&&(e, _)| e == event) {
        types[0] = t;
        return types;
    }
    if event == -1 {
        return types;
    }
    let alt = event_flag == 3 || (!plain && save_flag);
    let pair = match field_type {
        0 | 9 => (2, 6),
        1 | 5 | 6 => (1, 5),
        2 | 3 | 8 => (3, 7),
        4 => (8, 9),
        7 | 10 => (0, 4),
        _ => {
            types[0] = 4;
            return types;
        }
    };
    types[0] = if alt { pair.1 } else { pair.0 };
    if field_type == 4 {
        types[1] = if alt { 6 } else { 2 };
    }
    types
}

/// DUNGEON::SetDoor's names: (door dummy prefix, gate wall prefix, door
/// animation per type). It asks the room's animation for its door dummies
/// with doorname[0], a string read through a $gp word; puts the door
/// animation its jump table on DUNGEON.type (`sltiu $at, $v0, 10`) picks at
/// each; and, for a room an event bans, blocks the door or the gate wall
/// (the one other OBJ_ search) nearest the next room.
fn doors(p: &Program) -> Result<(String, String, Vec<String>), String> {
    let c = Code::new(p, "SetDoor__7DUNGEONFii")?;
    let order = c.sorted();
    let first = *order.first().ok_or("SetDoor forms no address")?;
    let dummy = obj_pattern(&text(p, p.u32(c.addrs[&first])?)?, "SetDoor doorname")?;
    let gates: Vec<usize> =
        order.iter().copied().filter(|i| text_or_empty(p, c.addrs[i]).starts_with("OBJ_")).collect();
    if gates.len() != 1 {
        return Err(format!("SetDoor: {} OBJ_ searches besides doorname, expected 1", gates.len()));
    }
    let gate = obj_pattern(&text_or_empty(p, c.addrs[&gates[0]]), "SetDoor gate")?;
    let sw: Vec<usize> =
        (0..c.words.len()).filter(|&k| fields(c.words[k]).0 == OP_SLTIU && fields(c.words[k]).3 == 10).collect();
    if sw.len() != 1 {
        return Err(format!("SetDoor: {} switches on the dungeon type, expected 1", sw.len()));
    }
    let jt = *(sw[0] + 1..sw[0] + 8).find_map(|k| c.addrs.get(&k)).ok_or("SetDoor: a switch without its table")?;
    let mut anime = Vec::new();
    for dtype in 0..10 {
        let k = c.at(p.u32(jt + 4 * dtype)?);
        let name = (k..k + 16)
            .filter_map(|j| c.addrs.get(&j))
            .map(|&a| text_or_empty(p, a))
            .find(|s| s.starts_with("ANM_"))
            .ok_or_else(|| format!("SetDoor: no door animation for type {dtype}"))?;
        anime.push(name);
    }
    Ok((dummy, gate, anime))
}

/// DUNGEON::SetClutList's name lists: the CLT_ objects whose palettes
/// SetRoom swaps. It copies five arrays of string pointers to its stack
/// (each formed with lui/addiu, in the order the cases use them) and, by
/// dungeon type, sets a count (`addiu $s1, $zero, N`; `$s2` in Outbreak and
/// Quarantine): types 0, 1, 2, 3, then 8 and 9.
fn clut_lists(p: &Program) -> Result<Vec<Vec<String>>, String> {
    let c = Code::new(p, "SetClutList__7DUNGEONFPc")?;
    // a field offset formed with lui (+0xd3884) points at nothing
    let points_at_clut = |va: u32| p.u32(va).is_ok_and(|a| text_or_empty(p, a).starts_with("CLT_"));
    let arrays: Vec<u32> = c.sorted().into_iter().map(|i| c.addrs[&i]).filter(|&a| points_at_clut(a)).collect();
    // The count's register: the saved register given a constant five times.
    let mut by_reg: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut order = Vec::new();
    for &w in &c.words {
        let (op, rs, rt, imm) = fields(w);
        if op == OP_ADDIU && rs == R_ZERO && (16..=23).contains(&rt) {
            if !by_reg.contains_key(&rt) {
                order.push(rt);
            }
            by_reg.entry(rt).or_default().push(imm);
        }
    }
    let counts = order.iter().map(|r| &by_reg[r]).find(|v| v.len() == 5).cloned().unwrap_or_default();
    if arrays.len() != 5 || counts.len() != 5 {
        return Err(format!(
            "SetClutList: {} name arrays and {} counts, expected 5 of each",
            arrays.len(),
            counts.len()
        ));
    }
    let mut out = Vec::new();
    for (va, n) in arrays.into_iter().zip(counts) {
        let names = (0..n).map(|k| text(p, p.u32(va + 4 * k)?)).collect::<Result<Vec<_>, _>>()?;
        if !names.iter().all(|x| x.starts_with("CLT_")) {
            return Err(format!("SetClutList: the array at 0x{va:08x} holds a name that is not a CLT_"));
        }
        out.push(names);
    }
    Ok(out)
}

/// The DUNGEON constructor's fog block: (default table, [(clutType,
/// texType, plain, hacked, forest)]), by table address. It sets fogParam
/// (+0x144) to dungeonFog and fogIndex (+0x148) to 0, then per WORLD_MAN
/// (clutType, texType) - `lw 320(wm); li $v0, C; bne`, then `lw 324(wm)`
/// tested by `bnez` (texType 0) or `li $v0, 1; bne` (texType 1) - stores
/// three tables: one whose index comes from WORLD_MAN::GetBG (types 8, 9),
/// one with index type - 4 (types 4-7, `addiu -4`) and one with index type.
type FogBlock = (u32, u32, u32, u32, u32);

fn fog(p: &Program) -> Result<(u32, Vec<FogBlock>), String> {
    let c = Code::new(p, "__ct__7DUNGEONFi")?;
    let get_bg = sym(p, "GetBG__9WORLD_MANFv")?;
    let w = &c.words;
    let (mut reg, mut stores, mut loads) = (HashMap::new(), Vec::new(), Vec::new());
    for (k, &word) in w.iter().enumerate() {
        let (op, rs, rt, imm) = fields(word);
        if let Some(&a) = c.addrs.get(&k)
            && matches!(op, OP_ADDIU | OP_ORI)
        {
            reg.insert(rt, a);
        }
        if op == OP_SW && rs == R_S0 && imm == FOG_PARAM {
            stores.push((k, reg.get(&rt).copied()));
        } else if op == OP_LW && !stores.is_empty() && (imm == WM_CLUT_TYPE || imm == WM_TEX_TYPE) && rs != R_S0 {
            loads.push((k, imm));
        }
    }
    let &(first, Some(default)) = stores.first().ok_or("DUNGEON::DUNGEON: no fogParam store")? else {
        return Err("DUNGEON::DUNGEON: the default fogParam is unread".into());
    };
    if w[first + 1] != (OP_SW << 26) | (R_S0 << 21) | FOG_INDEX {
        return Err("DUNGEON::DUNGEON: the default fogIndex is not `sw $zero, 0x148($s0)`".into());
    }
    let last = stores.last().unwrap().0;
    loads.retain(|&(k, _)| k < last);
    if loads.iter().enumerate().any(|(n, &(_, what))| what != [WM_CLUT_TYPE, WM_TEX_TYPE][n % 2])
        || loads.len() % 2 != 0
    {
        return Err("DUNGEON::DUNGEON: the fog block's tests are not clutType, texType pairs".into());
    }
    let mut blocks = Vec::new();
    for (n, &(k, what)) in loads.iter().enumerate() {
        if what != WM_CLUT_TYPE {
            continue;
        }
        let li = fields(w[k + 1]);
        if (li.0, li.1, li.2) != (OP_ADDIU, R_ZERO, R_V0) {
            return Err(format!("DUNGEON::DUNGEON: clutType test at {k} is not `li $v0, C`"));
        }
        let tk = loads[n + 1].0;
        let nxt = fields(w[tk + 1]);
        let tex = if nxt.0 == OP_BNE && nxt.2 == R_ZERO {
            0
        } else if (nxt.0, nxt.1, nxt.2) == (OP_ADDIU, R_ZERO, R_V0) {
            nxt.3
        } else {
            return Err(format!("DUNGEON::DUNGEON: texType test at {tk} not understood"));
        };
        let end = loads.get(n + 2).map_or(w.len(), |l| l.0);
        let (mut plain, mut hacked, mut forest) = (None, None, None);
        for &(sk, table) in &stores {
            if !(tk < sk && sk < end) {
                continue;
            }
            // What goes to fogIndex next says which table this is.
            let idx = (sk + 1..sk + 12)
                .find(|&j| {
                    let f = fields(w[j]);
                    (f.0, f.1, f.3) == (OP_SW, R_S0, FOG_INDEX)
                })
                .ok_or_else(|| format!("DUNGEON::DUNGEON: no fogIndex after {sk}"))?;
            let body = &w[sk + 1..idx];
            let slot = if body.iter().any(|&x| x >> 26 == OP_JAL && (x & 0x03FF_FFFF) << 2 == get_bg & 0x0FFF_FFFF) {
                &mut forest
            } else if body.iter().any(|&x| fields(x).0 == OP_ADDIU && fields(x).3 == 0xFFFC) {
                &mut hacked
            } else {
                &mut plain
            };
            if slot.is_some() || table.is_none() {
                return Err(format!("DUNGEON::DUNGEON: block at {k} stores a table twice or unread"));
            }
            *slot = table;
        }
        let (Some(plain), Some(hacked), Some(forest)) = (plain, hacked, forest) else {
            return Err(format!("DUNGEON::DUNGEON: block at {k} lacks a table"));
        };
        blocks.push((li.3, tex, plain, hacked, forest));
    }
    Ok((default, blocks))
}

/// WORLD_MAN::SetDungeonTexClut for a random area (eventAreaNumber 0), run
/// in piney-eemu: [server][field type] -> (clutType, texType).
fn tex_clut(v: Vol) -> Result<Vec<Vec<(u32, u32)>>, String> {
    let c = ctx(v, None);
    let p = &c.p;
    let mut m = crate::sinit::machine(p, Features::default());
    let (wm, game) = (0x0120_0000u32, 0x0120_2000u32);
    let put = |m: &mut piney_eemu::machine::Machine, a: u32, v: u32| {
        m.ram[a as usize..a as usize + 4].copy_from_slice(&v.to_le_bytes());
    };
    let get = |m: &piney_eemu::machine::Machine, a: u32| {
        u32::from_le_bytes(m.ram[a as usize..a as usize + 4].try_into().unwrap())
    };
    put(&mut m, sym(p, "worldman")?, wm);
    put(&mut m, sym(p, "game")?, game);
    let f = sym(p, "SetDungeonTexClut__9WORLD_MANFv")?;
    let mut out = Vec::new();
    for server in 0..SERVERS {
        let mut row = Vec::new();
        for ft in 0..=FIELD_TYPES {
            m.ram[wm as usize..wm as usize + 0x4E0].fill(0);
            put(&mut m, wm + 0x10, ft);
            put(&mut m, game + 0x1C, server);
            m.call(f, &[wm], 5_000_000).map_err(|e| format!("SetDungeonTexClut: {e:?}"))?;
            row.push((get(&m, wm + WM_CLUT_TYPE), get(&m, wm + WM_TEX_TYPE)));
        }
        out.push(row);
    }
    Ok(out)
}

fn exits(bits: u32, what: &str) -> Result<u8, String> {
    if bits & !0x3F != 0 || bits & 0x3F == 0 {
        return Err(format!("{what}: exit mask 0x{bits:x} has bits the port does not name"));
    }
    Ok(bits as u8)
}

/// The volume's `piney_data::dungeon::Tables`.
pub fn bytes(v: Vol) -> Result<Vec<u8>, String> {
    let c = ctx(v, Some("gcmn"));
    let p = &c.p;
    let mut out = Out::default();
    out.u32(p.u32(sym(p, "volumeNum")?)?);

    // The room tables, each once, then MakeFloor's jump tables by index.
    let (tables, jumps) = room_tables(p)?;
    out.count(tables.len());
    for (name, va, rows) in &tables {
        out.str(name);
        out.u32(*va);
        out.count(rows.len());
        for (k, (exit, r, models)) in rows.iter().enumerate() {
            let what = format!("{name}[{k}]");
            out.u8(exits(u32::from(*exit), &what)?);
            let rot = ROTATIONS.iter().find(|&&(b, _)| b == *r).ok_or_else(|| format!("{what}: rotation 0x{r:08x}"))?;
            out.u8(rot.1);
            out.count(models.len());
            for m in models {
                out.str(m);
            }
        }
    }
    for row in jumps {
        for k in row {
            out.u32(k);
        }
    }

    let symroom = symroom(p)?;
    for i in 0..10 {
        out.str(&text(p, p.u32(symroom + 4 * i)?)?);
    }
    for list in ["DungeonName", "DungeonName2"] {
        for s in name_list(p, list)? {
            out.str(&s);
        }
    }
    gim_patterns(p)?;
    for (name, prefix, gim) in GIM_PATTERNS {
        out.str(name);
        out.u8(u8::from(prefix));
        out.u8(gim);
    }
    // dungeonData: eleven (levelMax, roomMax).
    let data = p.symbol_named("dungeonData").ok_or("no symbol dungeonData")?;
    if data.size != 88 {
        return Err(format!("dungeonData is {} bytes, the port's type 88", data.size));
    }
    for k in 0..22 {
        out.u32(p.u32(data.value + 4 * k)?);
    }

    // The dungeon-type rule.
    let (offset, bit) = save_flag(p)?;
    let plain = bit.is_some();
    // a story area without a type of its own
    let story = (1..).find(|e| !FIXED_TYPES.iter().any(|&(f, _)| f == *e)).unwrap();
    for (event, flag) in [(0, 0), (story, 0), (story, 3)] {
        for ft in 0..=FIELD_TYPES {
            for t in dungeon_type(ft, event, flag, false, plain) {
                out.u8(t);
            }
        }
    }
    for list in [&FIXED_TYPES[..], if plain { &PLAIN_FIXED_TYPES[..] } else { &[] }] {
        out.count(list.len());
        for &(e, t) in list {
            out.i32(e);
            out.u8(t);
        }
    }
    out.u8(u8::from(plain));
    out.u32(offset);
    if let Some(bit) = bit {
        out.u32(bit);
    }

    // EditDungeon: the row arrays, each once, then the entries by index.
    let base = sym(p, "EditDungeon")?;
    // (address, rows) of each array, first seen first
    let mut rooms: Vec<(u32, u32)> = Vec::new();
    let mut gims: Vec<(u32, u32)> = Vec::new();
    let mut edits = Vec::new();
    let place = |list: &mut Vec<(u32, u32)>, va: u32, n: u32| match list.iter().position(|&(a, _)| a == va) {
        Some(i) => i as u32,
        None => {
            list.push((va, n));
            list.len() as u32 - 1
        }
    };
    for i in 0..edit_count(p)? {
        let b = p.read(base + 24 * i, 24)?;
        let w = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let (ev, idx, r, nr, g, ng) = (w(0) as i32, w(4) as i32, w(8), w(12), w(16), w(20));
        edits.push((ev, idx, name_of(p, r, "rooms"), place(&mut rooms, r, nr), place(&mut gims, g, ng)));
    }
    out.count(rooms.len());
    for &(va, n) in &rooms {
        out.count(n as usize);
        for k in 0..n {
            let b = p.read(va + 76 * k, 76)?;
            let r: Vec<i32> = b.as_chunks::<4>().0.iter().map(|c| i32::from_le_bytes(*c)).collect();
            let what = format!("the room at 0x{:08x}", va + 76 * k);
            for &x in &r[0..5] {
                out.i32(x);
            }
            if !(0..=2).contains(&r[5]) {
                return Err(format!("{what}: room size {}", r[5]));
            }
            out.u8(r[5] as u8);
            for &x in &r[6..9] {
                out.i32(x);
            }
            out.u8(exits(r[9] as u32, &what)?);
            for &x in &r[10..19] {
                out.i32(x);
            }
        }
    }
    out.count(gims.len());
    for &(va, n) in &gims {
        out.count(n as usize);
        for k in 0..n {
            for j in 0..8 {
                out.u32(p.u32(va + 32 * k + 4 * j)?);
            }
        }
    }
    out.count(edits.len());
    for (ev, idx, name, r, g) in edits {
        out.i32(ev);
        out.i32(idx);
        out.str(&name);
        out.u32(r);
        out.u32(g);
    }
    let (floors, by_event) = edit_floors(p)?;
    out.u32(floors);
    out.count(by_event.len());
    for (e, n) in by_event {
        out.i32(e);
        out.u32(n);
    }

    let (dummy, gate, anime) = doors(p)?;
    out.str(&dummy);
    out.str(&gate);
    for a in &anime {
        out.str(a);
    }
    for names in clut_lists(p)? {
        out.count(names.len());
        for n in &names {
            out.str(n);
        }
    }

    // The fog tables, each once by address, then the rule by index.
    let (default, blocks) = fog(p)?;
    let mut fog_vas: Vec<u32> = std::iter::once(default).chain(blocks.iter().flat_map(|b| [b.2, b.3, b.4])).collect();
    fog_vas.sort();
    fog_vas.dedup();
    out.count(fog_vas.len());
    for &va in &fog_vas {
        if let Some((s, 0)) = p.symbol_at(va, 0)
            && s.value == va
            && s.size != 0
            && s.size != FOG_ROW * FOG_ROWS
        {
            return Err(format!("{}: {} bytes, expected {}", s.name, s.size, FOG_ROW * FOG_ROWS));
        }
        out.str(&name_of(p, va, "fog"));
        out.u32(va);
        for k in 0..FOG_ROWS * FOG_ROW / 4 {
            out.u32(p.u32(va + 4 * k)?);
        }
    }
    let at = |va: u32| fog_vas.iter().position(|&x| x == va).unwrap() as u32;
    out.u32(at(default));
    out.count(blocks.len());
    for (clut, tex, plain, hacked, forest) in blocks {
        out.u8(clut as u8);
        out.u8(tex as u8);
        out.u32(at(plain));
        out.u32(at(hacked));
        out.u32(at(forest));
    }
    for row in tex_clut(v)? {
        for (clut, tex) in row {
            out.u8(clut as u8);
            out.u8(tex as u8);
        }
    }
    Ok(out.0)
}
