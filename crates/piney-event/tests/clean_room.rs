//! A script set of our own, written in the text form, run by the same
//! interpreter as the game's: nothing here comes from a disc.

use std::sync::Arc;

use piney_data::save::SaveData;
use piney_event::host::{Game, Host, MessageCall, MessageKind};
use piney_event::state::{CLOSED, ScriptSave};
use piney_event::text;
use piney_event::vm::{Library, Vm};

const SCRIPTS: &str = r#"
# The desktop's first visit: a greeting, a mail, a news item and a post,
# and every desktop operation but the mailer held back until the mail is read.
event 1 label="WELCOME"
open
  if event_done event=0
  if game_status status=2
block 0
  set phase phase=0 comp=eq
  message msg=0
  add_operate num=1 except=1
  mail mail=10
  news_add news=3
  bbs_post thread=5 post=0
block 1
  set phase phase=4 comp=ge
  if operate num=-1 except=1
  message msg=1
  repeatable
block 2
  if mail_got mail=10
  del_operate num=1 except=1
  wallpaper_add num=2
  end_event grp=-1
end
messages
  0 mode=0 name="Guide" "Welcome to the test desktop."
  1 mode=0 "Read your mail first."
end
"#;

#[derive(Default)]
struct Desk {
    save: SaveData,
    shown: Vec<(u16, i16)>,
}

impl Host for Desk {
    fn save(&mut self) -> &mut SaveData {
        &mut self.save
    }
    fn game(&self) -> Game {
        Game { status: 2, ..Game::default() }
    }
    fn message_open(&mut self, c: &MessageCall<'_>) {
        assert_eq!(c.kind, MessageKind::Speech);
        assert!(c.record.is_some());
        self.shown.push((c.event, c.msg));
    }
}

fn frames(vm: &mut Vm, host: &mut Desk, n: usize) {
    for _ in 0..n {
        vm.frame(host);
    }
}

#[test]
fn our_own_script_runs_the_desktop() {
    let events = text::parse_events(SCRIPTS).unwrap();
    assert_eq!(text::parse_events(&text::print_event(&events[0])).unwrap(), events);
    let mut vm = Vm::new(Arc::new(Library::new(piney_data::volume::Volume::Inf, events)));
    let mut host = Desk::default();
    for i in 0..512 {
        host.save.set_mail_order(i, -1);
    }

    // Boot, then the desktop's setup: phases 0, 2 and 4.
    vm.start_event(1, 0, &mut host);
    vm.start_thread(&mut host);
    for phase in [0, 2, 4] {
        vm.enable(phase);
        let mut guard = 0;
        while !vm.enable_settled(phase) {
            vm.frame(&mut host);
            guard += 1;
            assert!(guard < 1000);
        }
    }
    frames(&mut vm, &mut host, 3);
    assert_eq!(host.shown, [(1, 0)]);
    assert_eq!(host.save.mail(10), 1);
    assert_eq!(host.save.webnews(3), 1);
    assert_eq!(host.save.bbs(5, 0), 1);

    // The desktop asks before opening the address book (3): held back, and
    // the event answers. The mailer (1) is allowed.
    assert!(!vm.check_operate(3, 0, &mut host));
    frames(&mut vm, &mut host, 30);
    assert_eq!(host.shown, [(1, 0), (1, 1)]);
    assert!(vm.check_operate(1, 0, &mut host));

    // Reading the mail closes the event and unlocks a wallpaper.
    host.save.set_mail(10, 4);
    frames(&mut vm, &mut host, 3);
    assert!(host.save.flags(1) & CLOSED != 0);
    assert_eq!(host.save.dt_wallpaper_list(0) & 4, 4);
    assert!(vm.check_operate(3, 0, &mut host));
}

/// Instructions whose rules changed after Infection, run from one script
/// on Infection's rules and Mutation's (worklog 325).
const LATER: &str = r#"
event 60 label="LATER"
open
  if game_status status=2
block 0
  set phase phase=0 comp=eq
  call_on pc=19
  call_on pc=2
  call_lock
  friendship pc=19 num=10
  talk_num pc=19 num=3
  desktop_item type=2 id=1
  mode num=5
end
"#;

#[derive(Default)]
struct Later {
    save: SaveData,
    changes: Vec<String>,
}

impl Host for Later {
    fn save(&mut self) -> &mut SaveData {
        &mut self.save
    }
    fn game(&self) -> Game {
        Game { status: 2, ..Game::default() }
    }
    fn change_request(&mut self, num: i32, sf: i32) {
        self.changes.push(format!("request {num} {sf}"));
    }
    fn change_area(&mut self, a: i32, n: i32) {
        self.changes.push(format!("area {a} {n}"));
    }
}

fn run_later(volume: piney_data::volume::Volume) -> Later {
    use piney_data::save::offset as off;
    let mut vm = Vm::new(Arc::new(Library::new(volume, text::parse_events(LATER).unwrap())));
    let mut host = Later::default();
    host.save.set_u8(off::PARODY_FLAG, 1);
    host.save.set_u8(off::LAST_TOWN, 2);
    // A side story's number (50-99), which every volume's pass walks.
    vm.start_event(60, 0, &mut host);
    vm.start_thread(&mut host);
    vm.enable(0);
    for _ in 0..200 {
        vm.frame(&mut host);
    }
    host
}

/// `call_lock` clears characters 0-17 on Infection and 0-20 from Mutation
/// on; `mode 5` returns to the last town only on Infection; a movie is not
/// given in Parody Mode from Mutation on; characters 18-20's friendship and
/// talk count are the extension's.
#[test]
fn later_volumes_rules() {
    use piney_data::save::{by_id, offset as off};
    let inf = run_later(piney_data::volume::Volume::Inf);
    let mutation = run_later(piney_data::volume::Volume::Mut);
    let call = |h: &Later| h.save.i32(off::PARTY_MEMBER_CALL) as u32;
    let store = |h: &Later| h.save.i32(off::PARTY_MEMBER_CALL_STORE) as u32;
    assert_eq!(store(&inf), 1 << 19 | 1 << 2);
    assert_eq!(call(&inf), 0x8000_0000 | 1 << 19);
    assert_eq!(call(&mutation), 0x8000_0000);
    assert_eq!(inf.changes, ["request 5 8", "area 0 2"]);
    assert_eq!(mutation.changes, ["request 5 7"]);
    assert_eq!(mutation.save.i32(off::DT_STR_LIST) & 1, 0);
    assert_eq!(mutation.save.i16(by_id::spc_param(19) + off::SPC_FRIENDSHIP), 10);
    assert_eq!(mutation.save.u8(by_id::talk_num(19)), 3);
}
