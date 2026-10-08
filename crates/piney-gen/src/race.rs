//! The Flag Race's tables (Mutation on), found through its code: the race's
//! task is the function that names it `PG_RACE`; from it the calls lead to
//! the object's functions, and each function's data addresses (its
//! `lui`/`addiu` pairs into the overlay's data, strings aside) are its
//! tables, in the order the code first builds them. None on Infection.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::volume::{Ctx, Vol};

/// Every table address the race reads (MUT gcmn addresses in the docs).
#[derive(Clone, Debug, Default)]
pub struct RaceAddrs {
    /// The set-up (MUT 0x005fd820): Kite's start, his heading, the ride's
    /// handling by kind and by server.
    pub kite_pos: u32,
    pub kite_rot: u32,
    pub ride_kind: u32,
    pub ride_server: u32,
    /// The intro (0x005fdaf0): the camera by kind, then by server.
    pub intro_pos_kind: u32,
    pub intro_view_kind: u32,
    pub intro_pos_server: u32,
    pub intro_view_server: u32,
    /// The task's loop (0x005fded0): the countdown's height, Kite's place
    /// after.
    pub count_height: u32,
    pub end_pos: u32,
    /// The finish (0x005fe290): the Grunty's place and heading, the camera,
    /// the cups' height.
    pub ride_end_pos: u32,
    pub ride_end_rot: u32,
    pub finish_cam_pos: u32,
    pub finish_cam_view: u32,
    pub cup_height: u32,
    /// The rank's two clips by rank (0x005fe830).
    pub cup_anms: u32,
    pub place_anms: u32,
    /// The timer's cells (0x005fee10).
    pub timer_cells: u32,
    /// The flags' markers by town (MUT 0x006d80b0, `ccSetChibiGuso`'s
    /// call 0x005fca40) and their palettes (0x006d80c8).
    pub flag_markers: u32,
    pub flag_cluts: u32,
    /// Where the chosen Grunty walks in the intro (`ccPGuso::adultMain`):
    /// by kind, then by server.
    pub walk_kind: u32,
    pub walk_server: u32,
    /// Flag Race's menu (88, MUT 0x0058a4f0): its greeting's record, its
    /// results' records, the prizes, the other prizes, the wallpapers.
    pub greet: u32,
    pub results: u32,
    pub prizes: u32,
    pub prizes_gone: u32,
    pub wallpapers: u32,
}

thread_local! {
    static FOUND: RefCell<HashMap<Vol, Result<RaceAddrs, String>>> = RefCell::new(HashMap::new());
}

/// The overlay's code as (first address, words).
fn code(c: &Ctx) -> Result<(u32, Vec<u32>), String> {
    let (lo, hi) = c.p.code_range(true);
    Ok((lo, (lo..hi).step_by(4).map(|a| c.p.u32(a)).collect::<Result<Vec<u32>, String>>()?))
}

fn sext16(w: u32) -> u32 {
    w as u16 as i16 as i32 as u32
}

/// The address a `lui rt`, `addiu rt, rt` pair at `code[i..]` builds.
fn pair(code: &[u32], i: usize) -> Option<u32> {
    let (w, x) = (*code.get(i)?, *code.get(i + 1)?);
    let r = (w >> 16) & 31;
    let ok = w >> 26 == 0x0f && x >> 26 == 0x09 && (x >> 21) & 31 == r && (x >> 16) & 31 == r;
    ok.then(|| ((w & 0xffff) << 16).wrapping_add(sext16(x)))
}

/// The start of the function holding `code[i]`: the `addiu $sp, $sp, -N`
/// at or before it.
fn start_of(lo: u32, code: &[u32], i: usize) -> Option<u32> {
    (0..=i).rev().find(|&k| code[k] >> 16 == 0x27bd && code[k] & 0x8000 != 0).map(|k| lo + 4 * k as u32)
}

/// The words of the function at `f`, up to its `jr $ra` and the delay slot.
fn body(lo: u32, code: &[u32], f: u32) -> &[u32] {
    let s = ((f - lo) / 4) as usize;
    let n = code[s..].iter().position(|&w| w == 0x03e0_0008).map_or(0, |k| k + 2);
    &code[s..(s + n).min(code.len())]
}

/// Whether `va` holds a short text (a name the code passes), not a
/// pointer whose bytes happen to read as one.
fn is_text(c: &Ctx, va: u32) -> bool {
    let pointer = va.is_multiple_of(4) && c.p.u32(va).is_ok_and(|w| w != 0 && c.p.mapped(w));
    !pointer && c.p.cstr(va, 32).is_ok_and(|t| t.len() >= 2 && t.iter().all(|&b| (0x20..0x7f).contains(&b)))
}

/// The function's calls (`jal` targets) in order.
fn calls(f: &[u32]) -> Vec<u32> {
    f.iter().filter(|&&w| w >> 26 == 3).map(|&w| (w & 0x03ff_ffff) << 2).collect()
}

/// The calls to functions no carried name covers, in order.
fn unnamed_calls(c: &Ctx, f: &[u32]) -> Vec<u32> {
    calls(f).into_iter().filter(|&t| !matches!(c.p.symbol_at(t, 0), Some((_, 0)))).collect()
}

/// The function's data addresses (strings and code aside), each once, in
/// the order the code first builds them.
fn tables(c: &Ctx, lo: u32, f: &[u32]) -> Vec<u32> {
    let (_, end) = c.p.code_range(true);
    let mut out = Vec::new();
    for i in 0..f.len() {
        if let Some(a) = pair(f, i)
            && !(lo..end).contains(&a)
            && !is_text(c, a)
            && !out.contains(&a)
        {
            out.push(a);
        }
    }
    out
}

/// Whether `a` is the overlay's or main's initialised data (not code, not
/// .bss).
fn data(c: &Ctx, a: u32) -> bool {
    let ov = c.p.overlay.as_ref();
    let in_ov = ov.is_some_and(|o| a >= o.text + o.text_size && a < o.text + o.text_size + o.data_size);
    let (mlo, mhi) = c.p.code_range(false);
    in_ov || ((mlo..mhi).contains(&a) && !c.p.symbol_at(a, 0).is_some_and(|(s, _)| s.kind == crate::elf::STT_FUNC))
}

/// The named function `name`'s words.
fn named<'a>(c: &Ctx, lo: u32, code: &'a [u32], name: &str) -> Result<&'a [u32], String> {
    let f = c.p.symbol_named(name).ok_or_else(|| format!("no {name}"))?.value;
    Ok(body(lo, code, f))
}

/// The handler of menu `n`: `ccThMenu`'s jump table (after its `sltiu`
/// against the count) holds menu `n`'s case at `n + 1`; the case's first
/// call.
fn menu_handler(c: &Ctx, lo: u32, code: &[u32], n: u32) -> Result<u32, String> {
    let t = named(c, lo, code, "ccThMenu__FPv")?;
    let at = t.iter().position(|&w| w >> 16 == 0x2c41).ok_or("ccThMenu has no sltiu")?;
    let table = (at..t.len()).find_map(|i| pair(t, i)).ok_or("ccThMenu builds no table")?;
    let case = c.p.u32(table + 4 * (n + 1))?;
    let k = ((case - lo) / 4) as usize;
    let jal = code[k..].iter().take(8).find(|&&w| w >> 26 == 3).ok_or("menu case calls nothing")?;
    Ok((jal & 0x03ff_ffff) << 2)
}

/// The `n`th unnamed call of `f`.
fn nth(c: &Ctx, f: &[u32], n: usize, what: &str) -> Result<u32, String> {
    unnamed_calls(c, f).get(n).copied().ok_or_else(|| format!("the race's {what}: too few calls"))
}

fn find(c: &Ctx) -> Result<RaceAddrs, String> {
    let (lo, code) = code(c)?;
    // The task: the function that builds the address of "PG_RACE".
    let (_, end) = c.p.code_range(true);
    let names = (0..code.len()).find_map(|i| {
        let a = pair(&code, i)?;
        (a >= end && c.p.cstr(a, 16).ok()? == b"PG_RACE").then_some(i)
    });
    let i = names.ok_or("no code names PG_RACE")?;
    let task = start_of(lo, &code, i).ok_or("PG_RACE's task has no start")?;
    let t = body(lo, &code, task);
    // ctor, main, pcgs check, dtor.
    let main = nth(c, t, 1, "task")?;
    let m = body(lo, &code, main);
    let setup = nth(c, m, 0, "loop")?;
    let s = body(lo, &code, setup);
    let intro = nth(c, s, 0, "set-up")?;
    let ib = body(lo, &code, intro);
    // The loop's calls: set-up, countdown, the camera's place, finish, quit,
    // HUD, restart.
    let finish = nth(c, m, 3, "loop")?;
    let hud = nth(c, m, 5, "loop")?;
    let fb = body(lo, &code, finish);
    // The finish's calls: the ride's place, the rank, the camera's place.
    let rank = nth(c, fb, 1, "finish")?;
    let rb = body(lo, &code, rank);
    let pick = |v: &[u32], k: usize, what: &str| -> Result<u32, String> {
        v.get(k).copied().ok_or_else(|| format!("the race's {what}: too few tables ({v:x?})"))
    };
    let st = tables(c, lo, s);
    let it = tables(c, lo, ib);
    let mt = tables(c, lo, m);
    let ft = tables(c, lo, fb);
    let rt = tables(c, lo, rb);
    let ht = tables(c, lo, body(lo, &code, hud));
    // The flags: ccSetChibiGuso's one unnamed call makes them; their
    // palettes' table holds "CLT_xpflag01c1" second.
    let chibi = named(c, lo, &code, "ccSetChibiGuso__Fv")?;
    let entry = nth(c, chibi, 0, "flags")?;
    let et = tables(c, lo, body(lo, &code, entry));
    let clut_name = find_text(c, b"CLT_xpflag01c1").ok_or("no CLT_xpflag01c1")?;
    let flag_cluts = find_word(c, clut_name).ok_or("no table holds CLT_xpflag01c1")? - 4;
    let at = tables(c, lo, named(c, lo, &code, "adultMain__7ccPGusoFv")?);
    // Flag Race's menu: its data tables (main's texts and gcmn's records).
    let menu = body(lo, &code, menu_handler(c, lo, &code, 88)?);
    let mn: Vec<u32> = tables(c, lo, menu).into_iter().filter(|&a| data(c, a)).collect();
    Ok(RaceAddrs {
        flag_markers: pick(&et, 0, "flags")?,
        flag_cluts,
        // adultMain's first table is its jump table.
        walk_kind: pick(&at, 1, "adultMain")?,
        walk_server: pick(&at, 2, "adultMain")?,
        // The menu's: greeting, the texts (helpStr's next), results,
        // prizes, other prizes, wallpapers.
        greet: pick(&mn, 0, "menu 88")?,
        results: pick(&mn, 2, "menu 88")?,
        prizes: pick(&mn, 3, "menu 88")?,
        prizes_gone: pick(&mn, 4, "menu 88")?,
        wallpapers: pick(&mn, 5, "menu 88")?,
        kite_pos: pick(&st, 0, "set-up")?,
        kite_rot: pick(&st, 1, "set-up")?,
        ride_kind: pick(&st, 2, "set-up")?,
        ride_server: pick(&st, 3, "set-up")?,
        // The intro's first table is its jump table.
        intro_pos_kind: pick(&it, 1, "intro")?,
        intro_view_kind: pick(&it, 2, "intro")?,
        intro_pos_server: pick(&it, 3, "intro")?,
        intro_view_server: pick(&it, 4, "intro")?,
        count_height: pick(&mt, 0, "loop")?,
        end_pos: pick(&mt, 1, "loop")?,
        // The finish's first table is its jump table.
        ride_end_pos: pick(&ft, 1, "finish")?,
        ride_end_rot: pick(&ft, 2, "finish")?,
        finish_cam_pos: pick(&ft, 3, "finish")?,
        finish_cam_view: pick(&ft, 4, "finish")?,
        cup_height: pick(&ft, 5, "finish")?,
        cup_anms: pick(&rt, 0, "rank")?,
        place_anms: pick(&rt, 1, "rank")?,
        // The HUD builds the cells' second words first.
        timer_cells: pick(&ht, 1, "HUD")?,
    })
}

/// The overlay's data address of the text `t` (NUL-terminated).
fn find_text(c: &Ctx, t: &[u8]) -> Option<u32> {
    let o = c.p.overlay.as_ref()?;
    let mut needle = t.to_vec();
    needle.push(0);
    let lo = (o.text + o.text_size - o.base) as usize;
    let hi = (o.text + o.text_size + o.data_size - o.base) as usize;
    let d = o.data.get(lo..hi.min(o.data.len()))?;
    d.windows(needle.len()).position(|w| w == needle.as_slice()).map(|k| o.base + (lo + k) as u32)
}

/// The overlay's data address of a word equal to `v`.
fn find_word(c: &Ctx, v: u32) -> Option<u32> {
    let o = c.p.overlay.as_ref()?;
    let lo = (o.text + o.text_size - o.base) as usize;
    let hi = (o.text + o.text_size + o.data_size - o.base) as usize;
    let d = o.data.get(lo..hi.min(o.data.len()))?;
    (0..d.len() / 4)
        .find(|&k| u32::from_le_bytes(d[4 * k..4 * k + 4].try_into().unwrap()) == v)
        .map(|k| o.base + (lo + 4 * k) as u32)
}

/// The race's table addresses on the volume (Mutation on).
pub fn addrs(c: &Ctx) -> Result<RaceAddrs, String> {
    if c.volume == Vol::Inf {
        return Err("Infection has no Flag Race".into());
    }
    if let Some(r) = FOUND.with(|m| m.borrow().get(&c.volume).cloned()) {
        return r;
    }
    let r = find(c);
    FOUND.with(|m| m.borrow_mut().insert(c.volume, r.clone()));
    r
}
