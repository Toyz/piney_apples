//! The interpreter: `ccEvent`, `eventSub`, `ccEventFlagSet` and the event task
//! `ccThEvent`, as Infection runs them, with the later volumes' pass order
//! (`Vm::next_event`). [`Vm`] holds the unsaved [`EventMng`] and where the task
//! is; scripts come from a [`Library`], the save and the rest from the [`Host`].
//! A block runs at level 0 (stepped over), 1 (bookkeeping only, as
//! `ccEventFlagSet`) or 2 (played). [`Vm::frame`] is one frame of `ccThEvent`: a
//! pass at phase 0, one at 2, then one a frame from 4; a mode's setup moves the
//! phase with [`Vm::enable`] and waits on [`Vm::enable_settled`].

mod cond;
mod exec;

use std::sync::Arc;

use crate::host::{CharRef, Host};
use crate::ir::{Event, MAX_BLOCKS, Message, Script, Tag};
use crate::state::{CLOSED, DONE, ScriptSave};
use piney_data::volume::Volume;

/// Scripts and message tables by event number (0-499), and the volume
/// they are (`volumeNum`, which the scripts and the task test).
#[derive(Clone, Debug, Default)]
pub struct Library {
    events: Vec<Option<Event>>,
    pub volume: Volume,
}

pub const EVENTS: usize = 500;

impl Library {
    pub fn new(volume: Volume, events: impl IntoIterator<Item = Event>) -> Self {
        let mut lib = Library { events: vec![None; EVENTS], volume };
        for e in events {
            if let Some(slot) = lib.events.get_mut(e.number as usize) {
                *slot = Some(e);
            }
        }
        lib
    }
    pub fn event(&self, n: i32) -> Option<&Event> {
        usize::try_from(n).ok().and_then(|i| self.events.get(i)).and_then(|e| e.as_ref())
    }
    pub fn script(&self, n: i32) -> Option<&Script> {
        self.event(n).map(|e| &e.script)
    }
    /// Message `msg` of event `n`, from the Parody Mode table when `parody`.
    pub fn message(&self, n: i32, msg: i32, parody: bool) -> Option<&Message> {
        let e = self.event(n)?;
        let t = if parody { &e.parody } else { &e.messages };
        usize::try_from(msg).ok().and_then(|i| t.get(i))
    }
}

/// The five story stages of `ccThEvent`'s pass on a volume (1-4): the
/// side stories, with the volume's main story before its own side story.
fn story_stages(volume: i32) -> [(i32, i32); 5] {
    let v = volume.clamp(1, 4);
    let mut out = [(0, 0); 5];
    let mut i = 0;
    for k in 1..=4 {
        let base = 100 * (k - 1);
        if k == v {
            out[i] = (base, base + 50);
            i += 1;
        }
        out[i] = (base + 50, base + 100);
        i += 1;
    }
    out
}

/// `ccEvPos`: an event position (`+0x1c0`, 16 of 32 bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EvPos {
    pub floor: i16,
    pub block: i16,
    /// -1 when the slot is free.
    pub num: i32,
    /// Radians.
    pub dirc: f32,
    pub pos: [f32; 4],
}

/// `ccEvPoint`: an event point (`+0x3c0`, 16 of 8 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvPoint {
    pub floor: i16,
    pub block: i16,
    /// -1 when the slot is free.
    pub num: i32,
}

/// `ccEvCurrentOpen` (`+0x750`): the precondition settings in force while
/// an event is walked. -1 fields are unset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CurrentOpen {
    pub phase: i16,
    pub phase_comp: i16,
    pub status: i16,
    /// area, town, field, dungeon, floor, block.
    pub scene: [i16; 6],
    pub flag: i16,
    pub status_index: i16,
    pub status_num: i16,
    /// The comparison, or the upper bound when `range`.
    pub status_comp: i16,
    pub range: bool,
}

impl CurrentOpen {
    pub const CLEAR: CurrentOpen = CurrentOpen {
        phase: -1,
        phase_comp: -1,
        status: -1,
        scene: [-1; 6],
        flag: -1,
        status_index: -1,
        status_num: -1,
        status_comp: -1,
        range: false,
    };
}

/// `ccEvent`, the event manager (`eventMng`), less the camera and the task
/// pointers, which are the host's. Offsets are Infection's.
#[derive(Clone, Debug, PartialEq)]
pub struct EventMng {
    /// `+0x004 status`: 1 while an instruction list runs.
    pub status: i32,
    /// `+0x00c enablePhase`: -1 disabled; 0, 2, 4 requested by a mode's
    /// setup; 1, 3 after the pass at 0 or 2; 5 during play.
    pub enable_phase: i32,
    /// `+0x010 msgSelect`: the last answer.
    pub msg_select: i32,
    /// `+0x014 msgNum`: the message the last answer was to.
    pub msg_num: i32,
    /// `+0x018 entryNpcNum`.
    pub entry_npc_num: i32,
    /// `+0x01c registNpcNum`.
    pub regist_npc_num: i32,
    /// `+0x040 target[16]`: (type, code) of characters whose talk goes to the event; -1 free.
    pub targets: [(i16, i16); 16],
    /// `+0x080 entry[16]`: type, code, marker, param; type -1 free.
    pub entries: [[i16; 4]; 16],
    /// `+0x100 entryMc[16]`.
    pub entries_mc: [[i16; 4]; 16],
    /// `+0x180 areaCode[16]`: (code, except); -1 free.
    pub area_codes: [(i16, i16); 16],
    /// `+0x1c0 evPos[16]`.
    pub positions: [EvPos; 16],
    /// `+0x3c0 evPoint[16]`.
    pub points: [EvPoint; 16],
    /// `+0x750 currentOpen`.
    pub current: CurrentOpen,
    /// `+0x770 operate`: bit per player operation the event intercepts
    /// (0-17 desktop and field, 19-28 with the `_sf` instructions). The
    /// desktop and the menus test it through [`Vm::check_operate`].
    pub operate: u64,
    /// `+0x778 operateSet`: the first operation the player tried this frame
    /// (-1 none). Reset at the top of every event-task frame.
    pub operate_set: i16,
    /// `+0x77a areaCodeSet[3]`: the words entered at the Chaos Gate (-1 unset); reset every frame.
    pub area_code_set: [i16; 3],
    /// `+0x780 operateTarget`: the character talked to (operation 9); reset every frame.
    pub operate_target: Option<CharRef>,
    /// `+0x784 crisis`.
    pub crisis: i32,
    /// `+0x788 fadeNum`.
    pub fade_num: i32,
    /// `+0x78c puppetShow`.
    pub puppet_show: i32,
    /// `+0x000 actEvent`.
    pub act_event: i32,
}

impl Default for EventMng {
    fn default() -> Self {
        let mut m = EventMng {
            status: 0,
            enable_phase: -1,
            msg_select: 0,
            msg_num: 0,
            entry_npc_num: 0,
            regist_npc_num: 0,
            targets: [(-1, -1); 16],
            entries: [[-1, -1, -1, 0]; 16],
            entries_mc: [[-1, -1, -1, 0]; 16],
            area_codes: [(-1, -1); 16],
            positions: [EvPos { floor: -1, block: -1, num: -1, dirc: 0.0, pos: [0.0; 4] }; 16],
            points: [EvPoint { floor: -1, block: -1, num: -1 }; 16],
            current: CurrentOpen::CLEAR,
            operate: 0,
            operate_set: -1,
            area_code_set: [-1; 3],
            operate_target: None,
            crisis: 0,
            fade_num: -1,
            puppet_show: 0,
            act_event: -1,
        };
        m.init();
        m
    }
}

impl EventMng {
    /// `ccEvent::Init` (`INF 0x001a6ca0`). The entries keep their `param`,
    /// the positions their direction and point, as in the game.
    pub fn init(&mut self) {
        self.enable_phase = -1;
        for e in self.entries.iter_mut().chain(self.entries_mc.iter_mut()) {
            e[0] = -1;
            e[1] = -1;
            e[2] = -1;
        }
        self.targets = [(-1, -1); 16];
        for p in self.positions.iter_mut() {
            p.floor = -1;
            p.block = -1;
            p.num = -1;
        }
        self.points = [EvPoint { floor: -1, block: -1, num: -1 }; 16];
        self.area_codes = [(-1, -1); 16];
        self.current = CurrentOpen::CLEAR;
        self.entry_npc_num = 0;
        self.regist_npc_num = 0;
        self.operate_set = -1;
        self.operate = 0;
        self.operate_target = None;
        self.area_code_set = [-1; 3];
        self.msg_select = 0;
        self.msg_num = 0;
        self.crisis = 0;
        self.fade_num = -1;
        self.puppet_show = 0;
        self.act_event = -1;
        self.status = 0;
    }

    /// `ccEvent::AddOperate(num, except, sf)`.
    pub fn add_operate(&mut self, num: i32, except: i32, sf: bool) {
        let (lo, hi) = if sf { (19, 29) } else { (0, 18) };
        if except == 0 {
            if num < 0 {
                (lo..hi).for_each(|b| self.operate |= sllv64(b));
            } else {
                self.operate |= sllv64(num);
            }
        } else {
            (lo..hi).filter(|&b| b != num).for_each(|b| self.operate |= sllv64(b));
        }
    }

    /// `ccEvent::DelOperate(num, except, sf)`.
    pub fn del_operate(&mut self, num: i32, except: i32, sf: bool) {
        let (lo, hi) = if sf { (19, 29) } else { (0, 18) };
        let clear = |b: i32| (!(1u32 << (b & 31))) as i32 as i64 as u64;
        if except == 0 {
            if num < 0 {
                (lo..hi).for_each(|b| self.operate &= clear(b));
            } else {
                self.operate &= clear(num);
            }
        } else {
            (lo..hi).filter(|&b| b != num).for_each(|b| self.operate &= clear(b));
        }
    }

    /// First event point with this number.
    pub fn point(&self, num: i32) -> Option<EvPoint> {
        self.points.iter().copied().find(|p| p.num == num)
    }

    /// First event position with this number.
    pub fn position(&self, num: i32) -> Option<EvPos> {
        self.positions.iter().copied().find(|p| p.num == num)
    }

    /// `ccEvent::SetEventPoint(floor, block, num)` (main 0x001b3620): the
    /// first free point (`num` < 0) of the 16 takes them, floor and block
    /// as shorts; none free, nothing.
    pub fn set_event_point(&mut self, floor: i32, block: i32, num: i32) {
        if let Some(p) = self.points.iter_mut().find(|p| p.num < 0) {
            *p = EvPoint { floor: floor as i16, block: block as i16, num };
        }
    }

    /// `ccEvent::SetEventPos(floor, block, num, dirc, pos)` (main
    /// 0x001b3590): the first free position (`num` < 0) of the 16 takes
    /// them, `pos` copied with w 1.0; none free, nothing.
    pub fn set_event_pos(&mut self, floor: i32, block: i32, num: i32, dirc: f32, pos: [f32; 4]) {
        if let Some(p) = self.positions.iter_mut().find(|p| p.num < 0) {
            *p = EvPos { floor: floor as i16, block: block as i16, num, dirc, pos: [pos[0], pos[1], pos[2], 1.0] };
        }
    }
}

/// A 32-bit `sllv` sign-extended to 64 bits: the mask the game builds to
/// test or set bit `n` of the 64-bit `operate` word.
pub fn sllv64(n: i32) -> u64 {
    (1u32 << (n & 31)) as i32 as i64 as u64
}

/// Something the game would have crashed on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault {
    pub event: i32,
    pub block: i32,
    pub what: &'static str,
}

/// A trace line, for comparing the scheduler with the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trace {
    /// A pass began at this phase.
    Pass { frame: u64, phase: i32 },
    /// A block began to play.
    Block { frame: u64, event: i32, block: i32 },
}

// ---------------------------------------------------------------------------
// Where the event task is.

/// An instruction in the middle of running.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct OpRun {
    pub step: u8,
    pub n: i32,
    pub v: i32,
}

/// A block being executed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Exec {
    block: usize,
    op: usize,
    set_bit: bool,
    run: OpRun,
}

/// One event being walked by `eventSub`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Walk {
    event: i32,
    /// The next block to look at.
    block: usize,
    exec: Option<Exec>,
}

/// Where the pass is: the group being walked and the next event in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Pass {
    stage: u8,
    next: i32,
    /// The `ML` gate, decided when the pass reaches it.
    ml: i32,
    walk: Option<Walk>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Task {
    /// The top of the task's loop: reset the per-frame fields, then Breath.
    Top,
    /// Just after the Breath.
    Awake,
    Pass(Pass),
    /// Waiting while the phase is 1 or 3.
    PhaseWait(i32),
}

/// What happened in a step of the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Done,
    Yield,
}

/// The interpreter.
pub struct Vm {
    lib: Arc<Library>,
    pub mng: EventMng,
    task: Task,
    /// Frames run so far.
    pub frames: u64,
    /// Things the game would have crashed on.
    pub faults: Vec<Fault>,
    /// When set, passes and blocks are recorded here.
    pub trace: Option<Vec<Trace>>,
}

impl Vm {
    /// A fresh event manager (`new ccEvent` then `ccEvent::Init`), task at the top of its loop.
    /// `volumeNum`: the library's volume, 1-4.
    pub fn volume(&self) -> i32 {
        self.lib.volume.number()
    }

    pub fn new(lib: Arc<Library>) -> Self {
        Vm { lib, mng: EventMng::default(), task: Task::Top, frames: 0, faults: Vec::new(), trace: None }
    }

    pub fn library(&self) -> &Arc<Library> {
        &self.lib
    }

    /// `eventMng.enablePhase`.
    pub fn phase(&self) -> i32 {
        self.mng.enable_phase
    }

    /// `eventMng.status`: an instruction list is running.
    pub fn executing(&self) -> bool {
        self.mng.status != 0
    }

    /// The event and block being played, when the task is inside one.
    pub fn playing(&self) -> Option<(i32, usize)> {
        match self.task {
            Task::Pass(Pass { walk: Some(Walk { event, exec: Some(e), .. }), .. }) => Some((event, e.block)),
            _ => None,
        }
    }

    /// [`Vm::playing`] with the instruction the block is on.
    pub fn playing_op(&self) -> Option<(i32, usize, usize)> {
        match self.task {
            Task::Pass(Pass { walk: Some(Walk { event, exec: Some(e), .. }), .. }) => Some((event, e.block, e.op)),
            _ => None,
        }
    }

    /// `eventMng.operate` (+0x770): bit per player operation the events
    /// intercept. The desktop tests icon `n` and `n + 19`; see
    /// [`Vm::check_operate`], which is how the game asks.
    pub fn operate(&self) -> u64 {
        self.mng.operate
    }

    /// `eventMng.operateSet` (+0x778): the first operation the player tried
    /// this frame, -1 for none. The task resets it at the top of every
    /// frame; [`Vm::check_operate`] with `flag` 0 records it.
    pub fn operate_set(&self) -> i16 {
        self.mng.operate_set
    }

    pub fn set_operate_set(&mut self, v: i16) {
        self.mng.operate_set = v;
    }

    /// `ccEvent::Init`.
    pub fn init(&mut self) {
        self.mng.init();
    }

    // --- Boot and bookkeeping -----------------------------------------------------------

    /// `ccStartEvent(vol, flag)` (`INF 0x001b5380`), which the boot task runs
    /// as `ccStartEvent(1, 0)`: count the earlier volumes as cleared, bring
    /// their events' flags forward with [`Vm::flag_set`], and mark the event
    /// that opens this volume done (event 0 for volume 1).
    pub fn start_event<H: Host + ?Sized>(&mut self, vol: i32, flag: i32, host: &mut H) {
        for _ in 1..vol {
            let s = host.save();
            s.set_clear_flag(s.clear_flag().wrapping_add(1));
        }
        let mut first = 0;
        let groups = [(2, 0, 62, 50), (3, 100, 163, 150), (4, 200, 263, 250)];
        for (need, main, skip, side) in groups {
            if vol < need {
                break;
            }
            if need == 3 {
                host.save().set_u8(piney_data::save::offset::CRISIS, 1);
            }
            for n in main..main + 50 {
                self.flag_set(n, host);
            }
            if flag == 0 {
                host.save().update_flags(skip, |f| f | DONE);
            } else {
                for n in side..side + 50 {
                    self.flag_set(n, host);
                }
            }
            first = main + 100;
        }
        host.save().update_flags(first, |f| f | DONE);
    }

    /// `ccStartEventConvert` (`INF 0x001b55f0`): a save carried from the
    /// previous volume marks event 100, 200 or 300 done (0 on volume 1).
    pub fn start_event_convert<H: Host + ?Sized>(&mut self, host: &mut H) {
        let n = match self.volume() {
            2 => 100,
            3 => 200,
            4 => 300,
            _ => 0,
        };
        host.save().update_flags(n, |f| f | DONE);
    }

    /// `ccEventFlagSet(n)` (`INF 0x001b6160`): unless the event is done or
    /// closed, run every block at level 1, whatever its conditions. The
    /// precondition settings are applied (they only change the event
    /// manager) and each block that ends marks itself.
    pub fn flag_set<H: Host + ?Sized>(&mut self, n: i32, host: &mut H) {
        if host.save().flags(n) & (DONE | CLOSED) != 0 {
            return;
        }
        let lib = self.lib.clone();
        let Some(script) = lib.script(n) else { return };
        for (b, block) in script.blocks.iter().enumerate().take(MAX_BLOCKS) {
            for t in &block.tags {
                self.set_current(t);
            }
            self.execute_now(host, n, b, 1, &block.ops);
        }
    }

    /// Runs a block's instructions at level 0 or 1, which never wait.
    pub(crate) fn execute_now<H: Host + ?Sized>(
        &mut self,
        host: &mut H,
        n: i32,
        b: usize,
        lv: i32,
        ops: &[crate::ir::Op],
    ) {
        assert!(lv < 2);
        self.mng.status = 1;
        let mut set_bit = lv > 0;
        for op in ops {
            let mut run = OpRun::default();
            let s = self.op(host, n, b, lv, *op, &mut run, &mut set_bit);
            debug_assert_eq!(s, Step::Done);
        }
        if set_bit {
            host.save().update_flags(n, |f| f | 1u64 << (b & 63));
        }
        self.mng.status = 0;
    }

    /// `SetCurrentOpen` for one setting.
    pub fn set_current(&mut self, t: &Tag) {
        let c = &mut self.mng.current;
        match *t {
            Tag::Phase { phase, comp } => {
                c.phase = phase;
                c.phase_comp = comp.to_short();
            }
            Tag::GameStatus { status } => {
                c.status = status;
                if status != 5 {
                    c.scene = [-1; 6];
                }
            }
            Tag::Scene { area, town, field, dungeon, floor, block } => {
                c.scene = [area, town, field, dungeon, floor, block];
            }
            Tag::InTown { town } => c.scene = [0, town, -1, -1, -1, -1],
            Tag::InField { town, field } => c.scene = [1, town, field, -1, -1, -1],
            Tag::InDungeon { town, field, dungeon } => c.scene = [2, town, field, dungeon, -1, -1],
            Tag::BlockDone { num } => c.flag = num,
            Tag::InPoint { num } => {
                if let Some(p) = self.mng.points.iter().find(|p| p.num == num as i32) {
                    c.scene[4] = p.floor;
                    c.scene[5] = p.block;
                }
            }
            Tag::Status { index, num, comp } => {
                c.status_index = index;
                c.status_num = num;
                c.status_comp = comp.to_short();
                c.range = false;
            }
            Tag::StatusRange { index, lo, hi } => {
                c.status_index = index;
                c.status_num = lo;
                c.status_comp = hi;
                c.range = true;
            }
        }
    }

    /// The header and each block's `CheckOpen` at level 2, walking the
    /// settings in as `eventSub` does but running nothing: what each block
    /// would decide if the walk reached it now. For tests and tools.
    pub fn open_results<H: Host + ?Sized>(&mut self, n: i32, host: &mut H) -> Option<Vec<bool>> {
        let lib = self.lib.clone();
        let script = lib.script(n)?;
        let mut out = vec![self.check_open(host, n, -1, 2, &script.open)];
        self.mng.current = CurrentOpen::CLEAR;
        for (b, block) in script.blocks.iter().enumerate().take(MAX_BLOCKS) {
            for t in &block.tags {
                self.set_current(t);
            }
            out.push(self.check_open(host, n, b as i32, 2, &block.conds));
        }
        Some(out)
    }

    /// `eventSub(n)` run to its end outside the event task, as if every
    /// frame boundary passed at once: the number of frames it would take.
    /// For tests and tools; the task itself is [`Vm::frame`].
    pub fn run_event<H: Host + ?Sized>(&mut self, n: i32, host: &mut H) -> u64 {
        let Some(mut walk) = self.begin_walk(host, n) else { return 0 };
        let mut frames = 0;
        while self.run_walk(host, &mut walk) == Step::Yield {
            frames += 1;
        }
        frames
    }

    /// Plays block `b` of event `n` (its settings, then its instructions at
    /// level 2 whatever its conditions), as if every frame boundary passed
    /// at once: the number of frames it would take. The block marks itself
    /// as run unless it is repeatable. For tests and tools.
    pub fn play_block<H: Host + ?Sized>(&mut self, n: i32, b: usize, host: &mut H) -> u64 {
        let lib = self.lib.clone();
        let Some(block) = lib.script(n).and_then(|s| s.blocks.get(b)) else { return 0 };
        for t in &block.tags {
            self.set_current(t);
        }
        self.mng.status = 1;
        let mut ex = Exec { block: b, op: 0, set_bit: true, run: OpRun::default() };
        let mut frames = 0;
        while ex.op < block.ops.len() {
            if self.op(host, n, b, 2, block.ops[ex.op], &mut ex.run, &mut ex.set_bit) == Step::Yield {
                frames += 1;
                continue;
            }
            ex.op += 1;
            ex.run = OpRun::default();
        }
        if ex.set_bit {
            host.save().update_flags(n, |f| f | 1u64 << (b & 63));
        }
        self.mng.status = 0;
        frames
    }

    /// `CheckOpen` at level 2 for a list of open conditions of event `n`
    /// (no precondition settings), as an event's header is tested.
    pub fn check_conditions<H: Host + ?Sized>(&mut self, n: i32, conds: &[crate::ir::Cond], host: &mut H) -> bool {
        self.check_open(host, n, -1, 2, conds)
    }

    // --- The event task ------------------------------------------------------------------

    /// `ccStartThEvent` (`INF 0x001b5230`), which each mode's setup calls
    /// first: reset the event manager, clear the gate hack, and make every
    /// closed event (bit 63) among 0-449 done (bit 62) as well.
    pub fn start_thread<H: Host + ?Sized>(&mut self, host: &mut H) {
        self.mng.regist_npc_num = 0;
        self.mng.init();
        host.clear_gate_hack();
        for n in 0..450 {
            host.save().update_flags(n, |f| if f & CLOSED != 0 { f | DONE } else { f });
        }
    }

    /// `ccEnableThEvent(phase)`: request a pass at `phase`. For 0 and 2 the
    /// game's caller then waits (a frame at a time) until
    /// [`Vm::enable_settled`].
    pub fn enable(&mut self, phase: i32) {
        self.mng.enable_phase = phase;
    }

    /// The wait in `ccEnableThEvent`: over once the pass at 0 or 2 has been
    /// made (the phase reads 1 or 3) or events are disabled.
    pub fn enable_settled(&self, requested: i32) -> bool {
        match requested {
            0 => self.mng.enable_phase == 1 || self.mng.enable_phase < 0,
            2 => self.mng.enable_phase == 3 || self.mng.enable_phase < 0,
            _ => true,
        }
    }

    /// `ccDisableThEvent`, which `ccGame::ChangeRequest` calls.
    pub fn disable(&mut self) {
        self.mng.enable_phase = -1;
    }

    /// `ccEvent::CheckOperate(num, flag)` (`INF 0x001b32f0`), which the
    /// desktop and the field menus call before doing operation `num`: true
    /// to go ahead, false when the event intercepts it. With `flag` 0 the
    /// first operation of the frame is recorded in `operate_set`; for
    /// operation 9 (talk) the command target becomes `operate_target`, and
    /// talking to a registered target is intercepted too.
    pub fn check_operate<H: Host + ?Sized>(&mut self, num: i32, flag: i32, host: &mut H) -> bool {
        if num < 0 {
            return true;
        }
        let m = &mut self.mng;
        if flag == 0 && m.operate_set < 0 {
            m.operate_set = num as i16;
        }
        let mut go = true;
        if m.operate != 0 && m.operate & sllv64(num) != 0 {
            go = false;
        }
        if num == 9
            && let Some(t) = host.command_target()
        {
            m.operate_target = Some(t);
            if m.targets.iter().any(|&(ty, code)| t.types & (1u32 << (ty as i32 & 31)) != 0 && t.code == code) {
                return false;
            }
        }
        go
    }

    /// One frame of `ccThEvent` (`INF 0x001b5a60`).
    pub fn frame<H: Host + ?Sized>(&mut self, host: &mut H) {
        self.frames += 1;
        loop {
            match self.task {
                Task::Top => {
                    self.mng.operate_set = -1;
                    self.mng.operate_target = None;
                    self.mng.area_code_set = [-1; 3];
                    self.task = Task::Awake;
                    return;
                }
                Task::Awake => {
                    let game = host.game();
                    if game.status == 5 && self.mng.enable_phase >= 4 && host.game_over() {
                        self.task = Task::Top;
                        continue;
                    }
                    if game.status == 5 && game.cnt_stop == 0 && self.mng.enable_phase >= 4 {
                        self.party_time(host);
                    }
                    if self.mng.enable_phase < 0 {
                        self.task = Task::Top;
                        continue;
                    }
                    self.mng.act_event = -1;
                    if let Some(t) = self.trace.as_mut() {
                        t.push(Trace::Pass { frame: self.frames, phase: self.mng.enable_phase });
                    }
                    self.task = Task::Pass(Pass { stage: 0, next: -1, ml: 0, walk: None });
                }
                Task::Pass(mut pass) => {
                    let r = self.run_pass(host, &mut pass);
                    self.task = Task::Pass(pass);
                    if r == Step::Yield {
                        return;
                    }
                    self.task = match self.mng.enable_phase {
                        4 => {
                            host.play_pass_done();
                            self.mng.enable_phase = 5;
                            Task::Top
                        }
                        2 => {
                            self.mng.enable_phase = 3;
                            Task::PhaseWait(3)
                        }
                        0 => {
                            self.mng.enable_phase = 1;
                            Task::PhaseWait(1)
                        }
                        _ => Task::Top,
                    };
                }
                Task::PhaseWait(k) => {
                    if self.mng.enable_phase == k {
                        return;
                    }
                    self.task = Task::Top;
                }
            }
        }
    }

    /// The companions' time in the party, and a friendship point every 7560
    /// frames of it (in The World, during play, while time counts);
    /// characters 18-20's in the save's extension (Mutation's
    /// `AddPartyTime` 0x0017ace0 and `GetPartyTime` 0x0017ac60).
    fn party_time<H: Host + ?Sized>(&mut self, host: &mut H) {
        let party = host.party();
        let rate = host.frame_rate();
        let volume = self.volume();
        for slot in 1..3 {
            let pc = party.ids[slot];
            if pc <= 0 {
                continue;
            }
            let at = piney_data::save::by_id::party_time(pc as usize);
            let s = host.save();
            let mut t = s.i32(at).wrapping_add(rate);
            if 0x0cdf_e5c4 < t {
                t = 0x0cdf_e5c4;
            }
            s.set_i32(at, t);
            if t % 7560 == 0 {
                s.add_friendship(pc, 1, volume);
            }
        }
    }

    /// The pass's order: the four side stories S1 to S4 (50-99, 150-199,
    /// 250-299, 350-399) with the disc's own main story walked just before
    /// the side story of its number (M1 0-49 before S1 on Infection, M2
    /// 100-149 before S2 on Mutation, M3 200-249 before S3 on Outbreak, M4
    /// 300-349 before S4 on Quarantine), then the ML events while their gate
    /// allows. The later volumes' `ccThEvent` also leaves the story stages
    /// before any event once the phase has gone negative (an event ended
    /// the pass); the ML stage does not check.
    fn next_event<H: Host + ?Sized>(&mut self, host: &mut H, pass: &mut Pass) -> Option<i32> {
        let volume = self.volume();
        loop {
            if pass.stage < 5 && volume >= 2 && self.mng.enable_phase < 0 {
                pass.stage = 5;
                pass.next = -1;
            }
            let (lo, hi) = match pass.stage {
                0..=4 => story_stages(volume)[pass.stage as usize],
                5 => {
                    if pass.next < 0 {
                        pass.ml = self.ml_gate(host);
                    }
                    if pass.ml == 0 {
                        return None;
                    }
                    (400, 500)
                }
                _ => return None,
            };
            let n = if pass.next < 0 { lo } else { pass.next };
            let n = if pass.stage == 5 && pass.ml == 2 && (407..411).contains(&n) { 411 } else { n };
            if n >= hi {
                pass.stage += 1;
                pass.next = -1;
                continue;
            }
            pass.next = n + 1;
            return Some(n);
        }
    }

    /// Whether the ML events (400-499) are walked this pass: never in
    /// Parody Mode; 0 while volume 2 or 3 has started and not reached its
    /// point; 2 (skipping 407-410) after event 203 until 308.
    fn ml_gate<H: Host + ?Sized>(&self, host: &mut H) -> i32 {
        let s = host.save();
        if s.parody_on() {
            return 0;
        }
        let mut g = 1;
        if (s.event_done(100) && !s.event_done(101)) || (s.event_done(200) && !s.event_done(203)) {
            g = 0;
        }
        if s.event_done(203) && !s.event_done(308) {
            g = 2;
        }
        g
    }

    fn run_pass<H: Host + ?Sized>(&mut self, host: &mut H, pass: &mut Pass) -> Step {
        loop {
            if pass.walk.is_none() {
                let Some(n) = self.next_event(host, pass) else { return Step::Done };
                pass.walk = self.begin_walk(host, n);
                if pass.walk.is_none() {
                    continue;
                }
            }
            let mut walk = pass.walk.unwrap();
            let r = self.run_walk(host, &mut walk);
            pass.walk = Some(walk);
            if r == Step::Yield {
                return Step::Yield;
            }
            pass.walk = None;
        }
    }

    /// `eventSub(n)` up to the first block: skip a done or closed event or
    /// one without a script; test the open conditions; clear the settings.
    fn begin_walk<H: Host + ?Sized>(&mut self, host: &mut H, n: i32) -> Option<Walk> {
        if host.save().flags(n) & (DONE | CLOSED) != 0 {
            return None;
        }
        let lib = self.lib.clone();
        let script = lib.script(n)?;
        if !self.check_open(host, n, -1, 2, &script.open) {
            return None;
        }
        self.mng.current = CurrentOpen::CLEAR;
        Some(Walk { event: n, block: 0, exec: None })
    }

    /// The rest of `eventSub(n)`: each block's settings, then its run bit or
    /// conditions decide whether it plays.
    fn run_walk<H: Host + ?Sized>(&mut self, host: &mut H, walk: &mut Walk) -> Step {
        let lib = self.lib.clone();
        let script = lib.script(walk.event).expect("walked event has a script");
        let n = walk.event;
        loop {
            if let Some(mut ex) = walk.exec {
                let ops = &script.blocks[ex.block].ops;
                while ex.op < ops.len() {
                    let op = ops[ex.op];
                    let r = self.op(host, n, ex.block, 2, op, &mut ex.run, &mut ex.set_bit);
                    if r == Step::Yield {
                        walk.exec = Some(ex);
                        return Step::Yield;
                    }
                    ex.op += 1;
                    ex.run = OpRun::default();
                }
                if ex.set_bit {
                    host.save().update_flags(n, |f| f | 1u64 << (ex.block & 63));
                }
                self.mng.status = 0;
                walk.exec = None;
            }
            let b = walk.block;
            if b >= MAX_BLOCKS || b >= script.blocks.len() {
                return Step::Done;
            }
            walk.block += 1;
            let block = &script.blocks[b];
            for t in &block.tags {
                self.set_current(t);
            }
            if host.save().flags(n) & (1u64 << b) != 0 {
                continue;
            }
            if self.check_open(host, n, b as i32, 2, &block.conds) {
                if let Some(t) = self.trace.as_mut() {
                    t.push(Trace::Block { frame: self.frames, event: n, block: b as i32 });
                }
                self.mng.status = 1;
                walk.exec = Some(Exec { block: b, op: 0, set_bit: true, run: OpRun::default() });
            }
        }
    }

    pub(crate) fn fault(&mut self, event: i32, block: usize, what: &'static str) {
        self.faults.push(Fault { event, block: block as i32, what });
    }
}

#[cfg(test)]
mod tests {
    use super::story_stages;

    /// Each volume's `ccThEvent` (`slti` bounds and its `volumeNum` test).
    #[test]
    fn each_volume_walks_its_own_main_story() {
        let s = |a, b| (a, b);
        assert_eq!(story_stages(1), [s(0, 50), s(50, 100), s(150, 200), s(250, 300), s(350, 400)]);
        assert_eq!(story_stages(2), [s(50, 100), s(100, 150), s(150, 200), s(250, 300), s(350, 400)]);
        assert_eq!(story_stages(3), [s(50, 100), s(150, 200), s(200, 250), s(250, 300), s(350, 400)]);
        assert_eq!(story_stages(4), [s(50, 100), s(150, 200), s(250, 300), s(300, 350), s(350, 400)]);
    }
}
