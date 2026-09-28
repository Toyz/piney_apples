//! The event scripts' windows in the field: `ccMsg`, the `ccMessage` the
//! menu task draws. In The World (`game.status` 5, phase 4 on) the event
//! task calls the window directly, with no dim menu as on the desktop:
//! `message` waits for menu type -1 or 62 and `Change`s the window, `info`
//! for -1 and `ChangeInfo`s it, `teach_camera1`-`3` `Open` event 3's
//! messages. These are the [`piney_event::host::Host`] calls for
//! [`Place::Field`]; the VM supplies the frame counts (docs/engine/event-vm.md)
//! and [`crate::FieldUi`] the window (docs/engine/field-ui.md).

use piney_event::host::{Announce, MessageCall, MessageKind};

use crate::Request;
use crate::ctrl::MenuCtrl;

/// The lines of a record as the window takes them (`ccKanjiStrSeparate`
/// 0..2 of its text; missing lines empty).
fn record_lines(call: &MessageCall<'_>) -> (i32, Option<Vec<u8>>, [Vec<u8>; 3]) {
    let Some(r) = call.record else {
        return (0, None, Default::default());
    };
    let mut lines: [Vec<u8>; 3] = Default::default();
    for (i, l) in r.lines.iter().take(3).enumerate() {
        lines[i] = l.as_bytes().to_vec();
    }
    (r.mode, r.name.as_ref().map(|n| n.as_bytes().to_vec()), lines)
}

/// `Host::message_open` in the field.
pub fn open(m: &mut MenuCtrl, call: &MessageCall<'_>, names: &piney_desktop::kanji::Names, req: &mut Vec<Request>) {
    let (mode, name, lines) = record_lines(call);
    let grp = i32::from(call.event);
    let msg = i32::from(call.msg);
    match call.kind {
        MessageKind::Speech => {
            let l = [Some(&lines[0][..]), Some(&lines[1][..]), Some(&lines[2][..])];
            m.msg.change(mode, name.as_deref(), l, names);
            req.push(Request::VoiceRequest { grp, msg });
        }
        MessageKind::Info | MessageKind::InfoNow => {
            // Empty lines go to ChangeInfo as null pointers.
            let l = |i: usize| Some(&lines[i][..]).filter(|s| !s.is_empty());
            m.msg.change_info([l(0), l(1), l(2), None], names);
            req.push(Request::VoiceRequest { grp, msg });
            if call.kind == MessageKind::InfoNow {
                m.msg.hide_frame();
            }
        }
        MessageKind::Teach(_) => {
            let l = [Some(&lines[0][..]), Some(&lines[1][..]), Some(&lines[2][..])];
            m.msg.open(mode, name.as_deref(), l, names);
            req.push(Request::VoiceRequest { grp, msg });
        }
    }
    m.note(|| format!("[\"msg_event\",{grp},{msg},\"{:?}\"]", call.kind));
}

/// `Host::message_open` on the set-up's screen: the event's own window
/// (a tutorial prompt opens as speech), its voice asked as it opens.
pub fn open_setup(screen: &mut piney_desktop::setup::SetupScreen, call: &MessageCall<'_>, req: &mut Vec<Request>) {
    use piney_desktop::message::MessageKind as Window;
    let (mode, name, lines) = record_lines(call);
    let refs: Vec<&[u8]> = lines.iter().map(|l| &l[..]).collect();
    let window = match call.kind {
        MessageKind::Info => Window::Info,
        MessageKind::InfoNow => Window::InfoNow,
        MessageKind::Speech | MessageKind::Teach(_) => Window::Speech,
    };
    screen.open_message(window, mode, name.as_deref(), &refs);
    req.push(Request::VoiceRequest { grp: i32::from(call.event), msg: i32::from(call.msg) });
}

/// `ccEvent::DispInfo`'s lines for an announcement in the field (the
/// member's name from `spcParam[pc].base.name`).
pub fn announce_lines(a: Announce, member_name: &[u8], texts: &AnnounceTexts) -> [Option<Vec<u8>>; 4] {
    match a {
        Announce::Member { .. } => {
            let mut l1 = b"#Y".to_vec();
            l1.extend_from_slice(member_name);
            l1.extend_from_slice(&texts.member_tail);
            [Some(texts.you_now_have.clone()), Some(l1), None, None]
        }
        // The gate addresses and desktop items are composed elsewhere.
        _ => [None, None, None, None],
    }
}

/// The pieces of `getItemMenuStr` the announcements use.
#[derive(Clone, Debug, Default)]
pub struct AnnounceTexts {
    /// Line 0: "You now have ".
    pub you_now_have: Vec<u8>,
    /// Line 6: "'s member address!".
    pub member_tail: Vec<u8>,
}
