//! Runs a volume's executable from `_start` under `piney_eemu::sys`, the
//! IOP's services stood in for on the program's library functions:
//!
//!   cargo run --release -p piney-eemu --example ee_run -- ISO ELF SYMS VBLANKS
//!
//! Prints the threads and the heap's state as it goes.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::rc::Rc;

use piney_eemu::heap::Heap;
use piney_eemu::sys::{Hooked, State, Sys};

/// A heap block `_ccMalloc` handed out.
#[derive(Clone, Debug)]
struct Block {
    ptr: u32,
    size: u32,
    req: u32,
    flags: u32,
    ra: u32,
    at: u64,
    freed: Option<u64>,
}

/// State the hooks share.
type Shared<T> = Rc<RefCell<T>>;

/// `EE_HEAP_MODEL`'s state: off, waiting for the first handout (the heap
/// is set up by then), or running with the handouts matched so far.
enum Model {
    Off,
    Waiting,
    On(Heap, usize),
}

/// `name -> va` from a `.syms` file (main's and the overlays').
fn symbols(path: &str) -> HashMap<String, (String, u32)> {
    let text = std::fs::read_to_string(path).expect("the symbols");
    let mut out = HashMap::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() >= 5
            && let Ok(va) = u32::from_str_radix(f[1], 16)
        {
            out.entry(f[4].to_string()).or_insert((f[0].to_string(), va));
        }
    }
    out
}

fn load_elf(sys: &mut Sys, data: &[u8]) -> u32 {
    let u32_at = |o: usize| u32::from_le_bytes(data[o..o + 4].try_into().unwrap());
    let u16_at = |o: usize| u16::from_le_bytes(data[o..o + 2].try_into().unwrap());
    let (phoff, n) = (u32_at(0x1c) as usize, u16_at(0x2c) as usize);
    for i in 0..n {
        let p = phoff + 32 * i;
        let (ty, off, va, fsz, msz) = (
            u32_at(p),
            u32_at(p + 4) as usize,
            u32_at(p + 8) as usize,
            u32_at(p + 16) as usize,
            u32_at(p + 20) as usize,
        );
        if ty != 1 || fsz == 0 {
            continue;
        }
        sys.ram[va..va + fsz].copy_from_slice(&data[off..off + fsz]);
        sys.ram[va + fsz..va + msz.max(fsz)].fill(0);
    }
    u32_at(0x18)
}

/// The disc: ISO 9660 lookups and sector reads, and the stream's place.
struct Disc {
    file: File,
    iso: piney_data::iso::Iso,
    stream: u32,
}

impl Disc {
    fn read(&mut self, lsn: u32, sectors: u32) -> Vec<u8> {
        let mut buf = vec![0; sectors as usize * 2048];
        self.file.seek(SeekFrom::Start(u64::from(lsn) * 2048)).unwrap();
        let _ = self.file.read(&mut buf).unwrap();
        buf
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (iso_path, elf, syms, vblanks) = (&args[1], &args[2], &args[3], args[4].parse::<u64>().unwrap());
    let syms = symbols(syms);
    let sym = |n: &str| syms.get(n).map(|s| s.1);
    let mut sys = Sys::new();
    sys.log = std::env::var_os("EE_LOG").is_some();
    // EE_SPIN=n: give up after n instructions with no vertical blank.
    if let Some(n) = std::env::var("EE_SPIN").ok().and_then(|v| v.parse().ok()) {
        sys.spin_limit = n;
    }
    let entry = load_elf(&mut sys, &std::fs::read(elf).unwrap());
    let disc = Rc::new(RefCell::new(Disc {
        file: File::open(iso_path).unwrap(),
        iso: piney_data::iso::Iso::open(iso_path).unwrap(),
        stream: 0,
    }));

    let ret = |sys: &mut Sys, names: &[&str], v: u32| {
        for n in names {
            if let Some(a) = sym(n) {
                sys.stub(a, v);
            }
        }
    };
    // The IOP: modules load, RPC servers answer at once.
    ret(
        &mut sys,
        &["sceSifInitRpc", "sceSifExitRpc", "sceSifInitCmd", "sceSifExitCmd", "sceSifResetIop", "sceSifSyncIop"],
        1,
    );
    ret(&mut sys, &["sceSifRebootIop", "sceSifInitIopHeap", "sceSifLoadFileReset", "sceFsReset"], 1);
    ret(
        &mut sys,
        &["sceSifLoadModule", "sceSifLoadStartModule", "sceSifLoadModuleBuffer", "sceSifLoadStartModuleBuffer"],
        1,
    );
    ret(&mut sys, &["sceSifCheckStatRpc", "sceSifWriteBackDCache"], 0);
    // The sound driver's data loads (commands 0x9210, 0x9310 on `gCd`):
    // done at once, `sbuff[0]` reporting the load's size as the IOP does.
    if let (Some(a), Some(gcd), Some(sbuff), Some(load)) =
        (sym("sceSifCallRpc"), sym("gCd"), sym("sbuff"), sym("dataLoad"))
    {
        sys.hook(a, move |sys, a| {
            if a[0] == gcd && a[1] & 0x9000 == 0x9000 {
                let n = sys.r32(load + 0xc);
                sys.w32(sbuff, n);
            }
            Hooked::Ret(0)
        });
    }
    ret(&mut sys, &["sceSifAllocIopHeap", "sceSifAllocSysMemory"], 0x0010_0000);
    ret(&mut sys, &["sceSifIsAliveIop"], 1);
    if let Some(a) = sym("sceSifBindRpc") {
        sys.hook(a, |sys, a| {
            sys.w32(a[0] + 36, 0x0001_0000);
            Hooked::Ret(0)
        });
    }
    // The GS and DMA: nothing waits.
    ret(&mut sys, &["sceDmaReset"], 0);
    ret(&mut sys, &["sceGsSyncPath", "sceGsExecLoadImage", "sceGsResetGraph", "sceGsResetPath", "sceDmaWatch"], 0);
    if let Some(a) = sym("sceGsSyncV") {
        sys.hook(a, |sys, _| {
            if let Err(e) = sys.spin_vsync() {
                eprintln!("vblank: {e}");
            }
            Hooked::Ret(0)
        });
    }
    if let Some(a) = sym("sceDmaGetChan") {
        sys.hook(a, |_, a| Hooked::Ret(0x1000_8000 + 0x1000 * a[0].min(9)));
    }
    // The disc.
    ret(
        &mut sys,
        &["sceCdInit", "sceCdMmode", "sceCdStInit", "sceCdStStop", "sceCdStPause", "sceCdStResume", "sceCdInitEeCB"],
        1,
    );
    ret(&mut sys, &["sceCdSync", "sceCdSyncS", "sceCdGetError", "sceCdStatus", "sceCdStStat"], 0);
    ret(&mut sys, &["sceCdDiskReady", "sceCdNcmdDiskReady"], 2);
    {
        let d = disc.clone();
        if let Some(a) = sym("sceCdSearchFile") {
            sys.hook(a, move |sys, a| {
                let name = sys.cstr(a[1]).replace('\\', "/");
                let name = name.trim_start_matches('/').trim_end_matches(";1").to_string();
                if sys.log {
                    eprintln!("search {name}");
                }
                match d.borrow_mut().iso.find(&name) {
                    Ok(e) => {
                        sys.w32(a[0], e.lba);
                        sys.w32(a[0] + 4, e.size);
                        Hooked::Ret(1)
                    }
                    Err(_) => {
                        eprintln!("sceCdSearchFile: no {name}");
                        Hooked::Ret(0)
                    }
                }
            });
        }
        let d = disc.clone();
        if let Some(a) = sym("sceCdRead") {
            sys.hook(a, move |sys, a| {
                let buf = d.borrow_mut().read(a[0], a[1]);
                sys.write(a[2], &buf);
                Hooked::Ret(1)
            });
        }
        let d = disc.clone();
        for n in ["sceCdStStart", "sceCdStSeek", "sceCdStSeekF"] {
            let d = d.clone();
            if let Some(a) = sym(n) {
                sys.hook(a, move |_, a| {
                    d.borrow_mut().stream = a[0];
                    Hooked::Ret(1)
                });
            }
        }
        let d = disc.clone();
        if let Some(a) = sym("sceCdStRead") {
            sys.hook(a, move |sys, a| {
                let mut d = d.borrow_mut();
                let lsn = d.stream;
                let buf = d.read(lsn, a[0]);
                d.stream += a[0];
                drop(d);
                sys.write(a[1], &buf);
                if a[3] != 0 {
                    sys.w32(a[3], 0);
                }
                Hooked::Ret(a[0])
            });
        }
    }
    // The IOP's file system on the disc: sceOpen("cdrom0:\\PATH;1").
    {
        let files: Shared<HashMap<u32, (u32, u32, u32)>> = Rc::new(RefCell::new(HashMap::new()));
        ret(&mut sys, &["sceFsInit", "sceFsReset"], 0);
        let (d, f) = (disc.clone(), files.clone());
        if let Some(a) = sym("sceOpen") {
            sys.hook(a, move |sys, a| {
                let name = sys.cstr(a[0]);
                let path = name.split(':').nth(1).unwrap_or(&name).replace('\\', "/");
                let path = path.trim_start_matches('/').trim_end_matches(";1").to_string();
                if sys.log {
                    eprintln!("sceOpen {name}");
                }
                match d.borrow_mut().iso.find(&path) {
                    Ok(e) => {
                        let fd = 3 + f.borrow().len() as u32 + 16 * (sys.vblanks as u32 & 0xff);
                        f.borrow_mut().insert(fd, (e.lba, e.size, 0));
                        Hooked::Ret(fd)
                    }
                    Err(_) => Hooked::Ret(u32::MAX),
                }
            });
        }
        let (d, f) = (disc.clone(), files.clone());
        if let Some(a) = sym("sceRead") {
            sys.hook(a, move |sys, a| {
                let Some(&(lba, size, pos)) = f.borrow().get(&a[0]) else { return Hooked::Ret(u32::MAX) };
                let n = a[2].min(size.saturating_sub(pos));
                let first = lba + pos / 2048;
                let sectors = (pos % 2048 + n).div_ceil(2048);
                let buf = d.borrow_mut().read(first, sectors);
                let o = (pos % 2048) as usize;
                sys.write(a[1], &buf[o..o + n as usize]);
                f.borrow_mut().insert(a[0], (lba, size, pos + n));
                Hooked::Ret(n)
            });
        }
        let f2 = files.clone();
        if let Some(a) = sym("sceLseek") {
            sys.hook(a, move |_, a| {
                let Some(&(lba, size, pos)) = f2.borrow().get(&a[0]) else { return Hooked::Ret(u32::MAX) };
                let p = match a[2] {
                    0 => a[1],
                    1 => pos.wrapping_add(a[1]),
                    _ => size.wrapping_add(a[1]),
                };
                f2.borrow_mut().insert(a[0], (lba, size, p));
                Hooked::Ret(p)
            });
        }
        let f3 = files;
        if let Some(a) = sym("sceClose") {
            sys.hook(a, move |_, a| {
                f3.borrow_mut().remove(&a[0]);
                Hooked::Ret(0)
            });
        }
    }
    // The pads: connected, nothing pressed.
    ret(
        &mut sys,
        &[
            "scePadInit",
            "scePadInit2",
            "scePadPortOpen",
            "scePadSetMainMode",
            "scePadSetActAlign",
            "scePadSetActDirect",
        ],
        1,
    );
    ret(&mut sys, &["scePadGetReqState", "scePadEnterPressMode", "scePadExitPressMode", "scePadSetButtonInfo"], 0);
    ret(&mut sys, &["scePadGetState"], 6);
    if let Some(a) = sym("scePadInfoMode") {
        sys.hook(a, |_, a| Hooked::Ret(if a[2] == 1 || a[2] == 2 { 7 } else { 1 }));
    }
    if let Some(a) = sym("scePadRead") {
        sys.hook(a, |sys, a| {
            let mut d = [0u8; 32];
            d[1] = 0x73;
            d[2] = 0xff;
            d[3] = 0xff;
            d[4..8].fill(0x80);
            sys.write(a[2], &d);
            Hooked::Ret(32)
        });
    }
    // The memory cards: a formatted PS2 card in slot 1, the same card.
    ret(&mut sys, &["sceMcInit", "sceMcOpen", "sceMcGetDir", "sceMcChdir"], 0);
    if let Some(a) = sym("sceMcGetInfo") {
        sys.hook(a, |sys, a| {
            let t0 = sys.cpu.r[8] as u32;
            for (at, v) in [(a[2], 2u32), (a[3], 8000), (t0, 1)] {
                if at != 0 {
                    sys.w32(at, v);
                }
            }
            Hooked::Ret(0)
        });
    }
    if let Some(a) = sym("sceMcSync") {
        sys.hook(a, |sys, a| {
            if a[1] != 0 {
                sys.w32(a[1], 0);
            }
            if a[2] != 0 {
                sys.w32(a[2], 0);
            }
            Hooked::Ret(1)
        });
    }

    // VU0's bounding-box microprogram: every box in view, unclipped.
    ret(&mut sys, &["_ccCheckBoundingBoxEx__FPA4_fP6ccViewPA4_fi"], 1);
    // Overlays' functions, guarded by their first word in the PRG.
    let prg_word = |d: &Rc<RefCell<Disc>>, file: &str, addr: u32| -> u32 {
        let data = d.borrow_mut().iso.read_path(file).expect("the overlay");
        let load = u32::from_le_bytes(data[8..12].try_into().unwrap());
        let o = (addr - load) as usize;
        u32::from_le_bytes(data[o..o + 4].try_into().unwrap())
    };
    // The movies (the logos): over at once.
    if let Some(a) = sym("ccDecodeMpeg__FPcUi") {
        let w = prg_word(&disc, "DATA/DEMO.PRG", a);
        sys.hook_overlay(a, w, |_, _| Hooked::Ret(0));
    }

    // The heap: every block _ccMalloc hands out (its header's place and
    // size, the caller, when) and when _ccFree takes it back.
    let heap: Rc<RefCell<Vec<Block>>> = Rc::new(RefCell::new(Vec::new()));
    // The live blocks by payload address: their index in `heap`.
    let live: Rc<RefCell<HashMap<u32, usize>>> = Rc::new(RefCell::new(HashMap::new()));
    // A call's (n, flags, ra, align) by thread until it returns.
    let pending: Shared<HashMap<u32, (u32, u32, u32, u32)>> = Rc::new(RefCell::new(HashMap::new()));
    // EE_TRACE=path: every handout and free, one line each, in order.
    let trace: Rc<RefCell<Option<std::io::BufWriter<File>>>> = Rc::new(RefCell::new(
        std::env::var("EE_TRACE").ok().map(|p| std::io::BufWriter::new(File::create(p).unwrap())),
    ));
    // EE_HEAP_MODEL: piney_eemu::heap::Heap beside the game, every handout
    // compared; the first it would place elsewhere is reported and it stops.
    let model: Rc<RefCell<Model>> =
        Rc::new(RefCell::new(if std::env::var_os("EE_HEAP_MODEL").is_some() { Model::Waiting } else { Model::Off }));
    let (heap_top, heap_end) = (sym("heapTop__6ccHeap").unwrap(), sym("heapEnd__6ccHeap").unwrap());
    // EE_FILL=word: at the first `_ccMalloc` the region's one hole (past
    // its record, to `heapEnd`) filled with `word`, so a word no one wrote
    // since `InitFirst` shows as it. RAM here starts zeroed; the console's
    // is what the boot left.
    let fill: Option<u32> =
        std::env::var("EE_FILL").ok().and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok());
    let filled = Rc::new(RefCell::new(fill.is_none()));
    if std::env::var_os("EE_HEAP").is_some() {
        let p = pending.clone();
        sys.watch(sym("_ccMalloc__FUiUiUii").unwrap(), move |sys, a| {
            if !*filled.borrow() {
                *filled.borrow_mut() = true;
                let (top, end) = (sys.r32(heap_top), sys.r32(heap_end));
                eprintln!("filling {:#x}..{end:#x} with {:#010x}", top + 32, fill.unwrap_or(0));
                for at in (top + 32..end).step_by(4) {
                    sys.w32(at, fill.unwrap_or(0));
                }
            }
            p.borrow_mut().insert(sys.current_id(), (a[1], a[3], sys.cpu.r[31] as u32, a[0]));
        });
        let (p, h, l, t, m) = (pending, heap.clone(), live.clone(), trace.clone(), model.clone());
        // `_ccMalloc`'s `jr ra` (+0x2ac, main 0x0010098c on every volume).
        sys.watch(sym("_ccMalloc__FUiUiUii").unwrap() + 0x2ac, move |sys, _| {
            let v = sys.cpu.r[2] as u32;
            let (req, flags, ra, align) = p.borrow_mut().remove(&sys.current_id()).unwrap_or((0, 0, 0, 0));
            let mut m = m.borrow_mut();
            if let Model::Waiting = *m {
                *m = Model::On(Heap::new(sys.r32(heap_top), sys.r32(heap_end)), 0);
            }
            if let Model::On(heap, n) = &mut *m {
                let want = heap.malloc(align, req, flags).unwrap_or(0);
                if want == v {
                    *n += 1;
                } else {
                    eprintln!(
                        "HEAP MODEL: handout {n} ({req:#x} from {ra:#x}) at {v:#x}, the model's at {want:#x}; it stops"
                    );
                    *m = Model::Off;
                }
            }
            if v != 0 {
                let size = sys.r32(v - 16 + 8);
                let mut h = h.borrow_mut();
                l.borrow_mut().insert(v, h.len());
                h.push(Block { ptr: v - 16, size, req, flags, ra, at: sys.vblanks, freed: None });
            }
            if let Some(w) = t.borrow_mut().as_mut() {
                use std::io::Write;
                let size = if v != 0 { sys.r32(v - 16 + 8) } else { 0 };
                let id = sys.current_id();
                writeln!(w, "{} A {:x} {:x} {:x} {} {:x} {} {:x}", sys.vblanks, v, size, req, flags, ra, id, align)
                    .unwrap();
            }
        });
        let (h, l, t) = (heap.clone(), live, trace.clone());
        let dump_freed: Option<(u32, String)> = std::env::var("EE_DUMP_FREED").ok().and_then(|v| {
            let (size, path) = v.split_once(':')?;
            Some((u32::from_str_radix(size.trim_start_matches("0x"), 16).ok()?, path.to_string()))
        });
        // ccMemReduce(p, n) shrinks a block in place, the rest a hole.
        let (t2, m) = (t.clone(), model.clone());
        sys.watch(sym("ccMemReduce__FPvUi").unwrap(), move |sys, a| {
            if let Model::On(heap, _) = &mut *m.borrow_mut() {
                heap.reduce(a[0], a[1]);
            }
            if let Some(w) = t2.borrow_mut().as_mut() {
                use std::io::Write;
                writeln!(w, "{} R {:x} {:x}", sys.vblanks, a[0], a[1]).unwrap();
            }
        });
        let m = model.clone();
        sys.watch(sym("_ccFree__FPv").unwrap(), move |sys, a| {
            if let Model::On(heap, _) = &mut *m.borrow_mut() {
                heap.free(a[0]);
            }
            if let Some(w) = t.borrow_mut().as_mut() {
                use std::io::Write;
                writeln!(w, "{} F {:x} {:x}", sys.vblanks, a[0], sys.cpu.r[31] as u32).unwrap();
            }
            if let Some(k) = l.borrow_mut().remove(&a[0]) {
                let mut h = h.borrow_mut();
                h[k].freed = Some(sys.vblanks);
                // A block of the size asked for, as it is the moment it is freed.
                if let Some((size, path)) = dump_freed.as_ref()
                    && h[k].req == *size
                {
                    let b = &h[k];
                    let bytes: Vec<u8> = (0..b.size).map(|i| sys.r8(b.ptr + i)).collect();
                    std::fs::write(path, &bytes).unwrap();
                    eprintln!("dumped the freed {:#x}-byte block at {:#x} to {path}", b.size, b.ptr);
                }
            }
        });
    }
    // A boss fight: the boss effect manager's `operator new(0x1038)` in
    // ccThBossEffect returns 0x0c bytes into it (OUT gcmn 0x00472eac, QUA
    // 0x003657bc).
    if let Some(at) = sym("ccThBossEffect__FPv") {
        let h = heap;
        sys.watch(at + 0x1c, move |sys, _| {
            let m = sys.cpu.r[2] as u32;
            let exit = sys.r32(m + 0x2c);
            eprintln!("MANAGER at {m:#x} (vblank {}): m_exit word {exit:#010x}", sys.vblanks);
            let h = h.borrow();
            let target = m + 0x2c;
            for b in h.iter().rev().filter(|b| b.ptr <= target && target < b.ptr + b.size).take(6) {
                eprintln!(
                    "  block {:#x}+{:#x} (asked {:#x}, flags {}) from {:#x} at vblank {}, freed {:?}",
                    b.ptr, b.size, b.req, b.flags, b.ra, b.at, b.freed
                );
            }
            // The last freed block under m_exit: its bytes as they lie, and
            // the blocks handed out inside it since it was freed.
            if let Some(b) = h.iter().rev().find(|b| b.ptr <= target && target < b.ptr + b.size && b.freed.is_some()) {
                if let Ok(path) = std::env::var("EE_DUMP") {
                    let bytes: Vec<u8> = (0..b.size).map(|i| sys.r8(b.ptr + i)).collect();
                    std::fs::write(&path, &bytes).unwrap();
                    eprintln!("  dumped {:#x} bytes at {:#x} to {path}", b.size, b.ptr);
                }
                let freed = b.freed.unwrap();
                for x in h.iter().filter(|x| x.at >= freed && x.ptr >= b.ptr && x.ptr < b.ptr + b.size) {
                    eprintln!(
                        "    then {:#x}+{:#x} (asked {:#x}) from {:#x} at vblank {}, freed {:?}",
                        x.ptr, x.size, x.req, x.ra, x.at, x.freed
                    );
                }
            }
        });
    }
    // EE_PRINT=name,0xaddr,...: each call of these printed with its vblank,
    // its first four arguments and the active ccLayer (its priority).
    if let Ok(list) = std::env::var("EE_PRINT") {
        let active = sym("active__7ccLayer").unwrap();
        for w in list.split(',').filter(|w| !w.is_empty()) {
            let at = match u32::from_str_radix(w.trim_start_matches("0x"), 16) {
                Ok(a) if w.starts_with("0x") => a,
                _ => sym(w).unwrap_or_else(|| panic!("no symbol {w}")),
            };
            let name = w.to_string();
            sys.watch(at, move |sys, a| {
                let lay = sys.r32(active);
                let pri = if lay != 0 { sys.r32(lay + 4) as i16 } else { 0 };
                eprintln!(
                    "PRINT {} {name} {:#x} {:#x} {:#x} {:#x} active {lay:#x} pri {pri}",
                    sys.vblanks, a[0], a[1], a[2], a[3]
                );
            });
        }
    }
    let save = std::env::var("EE_SAVE").ok().map(|p| std::fs::read(p).unwrap());
    let go_at: u64 = std::env::var("EE_GO").ok().and_then(|v| v.parse().ok()).unwrap_or(240);
    let scene: Vec<i32> = std::env::var("EE_SCENE")
        .ok()
        .map(|v| v.split(',').map(|x| x.parse().unwrap()).collect())
        .unwrap_or_else(|| vec![1, 3, 5, -1, -1, -1]);

    // Later steps: EE_THEN="vblank:a,t,f,d,fl,b;vblank:..." - a scene, a
    // member's id (an invite), three words (a warp) or 118,n (`area n`).
    let then: Vec<(u64, Vec<i32>)> = std::env::var("EE_THEN")
        .ok()
        .map(|v| {
            v.split(';')
                .map(|e| {
                    let (at, sc) = e.split_once(':').unwrap();
                    (at.parse().unwrap(), sc.split(',').map(|x| x.parse().unwrap()).collect())
                })
                .collect()
        })
        .unwrap_or_default();

    sys.boot(entry);
    let step = 60.min(vblanks);
    let mut done = 0;
    // `_start`'s `$gp` (OUT 0x0038ddf0, QUA 0x00280770): the `lui` and
    // `addiu` on `$a0` before its `move $gp, $a0`.
    let gp = {
        let w = |a: u32| sys.r32(a);
        let at = (entry..entry + 0x100).step_by(4).find(|&a| w(a) == 0x0080_e025).expect("_start's move $gp");
        let ops: Vec<u32> = (entry..at).step_by(4).map(w).collect();
        let hi = ops.iter().rfind(|&&o| o >> 16 == 0x3c04).expect("lui $a0") << 16;
        let lo = ops.iter().rfind(|&&o| o >> 16 == 0x2484).expect("addiu $a0") & 0xffff;
        hi.wrapping_add(lo as u16 as i16 as u32)
    };
    eprintln!("gp {gp:#x}");
    while done < vblanks {
        if let Some(bytes) = save.as_ref()
            && done == go_at
        {
            let sd = sys.r32(sym("saveData").unwrap());
            sys.write(sd, bytes);
            // ccThDemo's `ccSaveData::LoadGame()` after a load: the names'
            // and CCS names' pointers rebuilt (the save holds the port's
            // stand-in for character 0's), velocities, default words.
            sys.call(sym("LoadGame__10ccSaveDataFv").unwrap(), &[sd], gp).unwrap();
            let game = sys.r32(sym("game").unwrap());
            eprintln!("injecting the save at {sd:#x}; game {game:#x}; scene {scene:?}");
            // EE_AREA=n: the event instruction `area n` for a story area
            // (1-13) before the scene: `WORLD_MAN.eventAreaNumber` (+0x120).
            if let Some(n) = std::env::var("EE_AREA").ok().and_then(|v| v.parse::<u32>().ok()) {
                let wm = sys.r32(sym("worldman").unwrap());
                sys.w32(wm + 0x120, n);
            }
            sys.call(sym("ChangeRequest__6ccGameFii").unwrap(), &[game, 5, 8], gp).unwrap();
            let sc: Vec<u32> = std::iter::once(game).chain(scene.iter().map(|&v| v as u32)).collect();
            sys.call(sym("ChangeScene__6ccGameFiiiiii").unwrap(), &sc, gp).unwrap();
        }
        for (at, sc) in &then {
            if *at == done {
                let game = sys.r32(sym("game").unwrap());
                if sc.len() == 1 {
                    // A member invited as PartyInMenu does: the task
                    // ccThPartyAdd (priority 34, a 4096-byte stack), its
                    // param[0] 1 and param[1] the member, which runs
                    // ccParty::AddMember.
                    eprintln!("member {} joins at vblank {}", sc[0], sys.vblanks);
                    let start = sym("ccStartThread__FPFPv_vii").unwrap();
                    let task = sym("ccThPartyAdd__FP6ccTscb").unwrap();
                    let t = sys.call(start, &[task, 34, 4096], gp).unwrap();
                    sys.w32(t + 0x14, 1);
                    sys.w32(t + 0x18, sc[0] as u32);
                    continue;
                }
                if let [169, list, func] = sc[..] {
                    // Quarantine's `ending_kanji` from outside the event
                    // task: the file list added, then its thread (66).
                    // A task of its own (code written at 0x80000) does
                    // what the instruction does after the flag: the list
                    // added (it breathes), then `func`, which starts the
                    // thread and breathes until it is done.
                    let game = sys.r32(sym("game").unwrap());
                    sys.w32(game + 0x7c, 1);
                    eprintln!("ending_kanji at vblank {}", sys.vblanks);
                    let jal = |t: u32| 0x0c00_0000 | ((t >> 2) & 0x03ff_ffff);
                    let (list, func) = (list as u32, func as u32);
                    let lo = list & 0xffff;
                    let hi = (list >> 16).wrapping_add(u32::from(lo & 0x8000 != 0)) & 0xffff;
                    let code = [
                        0x3c04_0000 | hi,
                        0x2484_0000 | lo,
                        jal(sym("ccLoadFLAdd__FP10ccFileList").unwrap()),
                        0,
                        jal(func),
                        0,
                        0x2404_0001,
                        jal(sym("ccBreathThread__Fi").unwrap()),
                        0,
                        0x1000_fffc,
                        0,
                    ];
                    for (k, w) in code.iter().enumerate() {
                        sys.w32(0x0008_0000 + 4 * k as u32, *w);
                    }
                    let start = sym("ccStartThread__FPFPv_vii").unwrap();
                    sys.call(start, &[0x0008_0000, 32, 4096], gp).unwrap();
                    continue;
                }
                if let [118, n] = sc[..] {
                    // The event instruction `area n` (1-13).
                    let wm = sys.r32(sym("worldman").unwrap());
                    eprintln!("area {n} at vblank {}", sys.vblanks);
                    sys.w32(wm + 0x120, n as u32);
                    continue;
                }
                if sc.len() == 3 {
                    // The Chaos Gate's warp: SetGenerateCode on three words.
                    let wm = sys.r32(sym("worldman").unwrap());
                    eprintln!("warp {sc:?} at vblank {}", sys.vblanks);
                    let args = [wm, sc[0] as u32, sc[1] as u32, sc[2] as u32];
                    sys.call(sym("SetGenerateCode__9WORLD_MANFiii").unwrap(), &args, gp).unwrap();
                    continue;
                }
                eprintln!("scene {sc:?} at vblank {}", sys.vblanks);
                let sc: Vec<u32> = std::iter::once(game).chain(sc.iter().map(|&v| v as u32)).collect();
                sys.call(sym("ChangeScene__6ccGameFiiiiii").unwrap(), &sc, gp).unwrap();
            }
        }
        if let Err(e) = sys.run_vblanks(step) {
            eprintln!("stopped: {e} at pc {:#x} ra {:#x}", sys.run.pc, sys.cpu.r[31] as u32);
            let sp = sys.cpu.r[29] as u32;
            // The words above sp that could be return addresses, by offset.
            let up: Vec<String> = (0..256)
                .map(|i| (4 * i, sys.r32(sp + 4 * i)))
                .filter(|&(_, v)| (0x100000..0x700000).contains(&v) && v & 3 == 0)
                .map(|(o, v)| format!("+{o:x}:{v:x}"))
                .collect();
            eprintln!("  sp {sp:#x} stack {}", up.join(" "));
            let regs: Vec<String> = (1..32).map(|i| format!("r{i} {:x}", sys.cpu.r[i] as u32)).collect();
            eprintln!("  regs {}", regs.join(" "));
            eprintln!("intc {:?} enabled {:#x}", sys.handlers, sys.enabled_intc);
            eprintln!("dmac {:?} enabled {:#x}", sys.dmac, sys.enabled_dmac);
            let io = sys.io.borrow();
            let mut regs: Vec<_> = io.regs.iter().filter(|(a, _)| (0x1000_8000..0x1000_f000).contains(*a)).collect();
            regs.sort();
            eprintln!("dma regs {regs:x?}");
            break;
        }
        done += step;
        let live: Vec<String> = sys
            .threads
            .iter()
            .filter(|t| t.state != State::Dormant)
            .map(|t| format!("{}:{:x}@{}{:?}", t.id, t.func, t.prio, t.state))
            .collect();
        eprintln!("vblank {} threads {}", sys.vblanks, live.join(" "));
        for k in 0..sys.threads.len() {
            if Some(sys.threads[k].func) == sym("ccStream_PlaySceneThread__FP6ccTscb")
                && sys.threads[k].state != State::Dormant
                && let Some(st) = sys.thread_reg(k, 21)
            {
                let h = |o: u32| sys.r32(st + o) & 0xffff;
                eprintln!(
                    "  stream {st:#x}: 380 {:#x} 382 {:#x} frame {} sub {} speed {} end {}",
                    h(380),
                    sys.r32(st + 380) >> 16,
                    sys.r32(st + 356),
                    h(368) as i16,
                    sys.r32(st + 368) >> 16,
                    sys.r32(st + 360)
                );
            }
        }
        if sys.log {
            for k in 0..sys.threads.len() {
                if let Some((pc, ra)) = sys.thread_pc(k) {
                    let sp = sys.thread_sp(k).unwrap_or(0);
                    let up: Vec<String> = (0..24)
                        .map(|i| sys.r32(sp + 4 * i))
                        .filter(|&v| (0x100000..0x700000).contains(&v))
                        .map(|v| format!("{v:x}"))
                        .collect();
                    eprintln!("  thread {} pc {pc:#x} ra {ra:#x} stack {}", sys.threads[k].id, up.join(" "));
                }
            }
        }
    }
    let mut syscalls: Vec<_> = sys.unknown_syscalls.iter().collect();
    syscalls.sort();
    if let Some(w) = trace.borrow_mut().as_mut() {
        use std::io::Write;
        w.flush().unwrap();
    }
    if let Model::On(_, n) = &*model.borrow() {
        eprintln!("HEAP MODEL: all {n} handouts where the game put them");
    }
    eprintln!("unknown syscalls {syscalls:x?}; {} instructions", sys.executed);
}
