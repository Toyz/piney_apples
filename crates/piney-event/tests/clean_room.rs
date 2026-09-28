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
