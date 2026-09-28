//! Mail the port adds to the game. Not the game's content: a new game on
//! Infection gets one more mail, from Helba, listed straight below CC
//! Corporation's first two, sent by the port's patch to event 1
//! (`piney_event::extras`); a new game on the later volumes finds it in the
//! box already read ([`new_game`]). It takes the save's last mail slot,
//! [`HELBA_MAIL`]; the real game reads the mail order list as indexes into its
//! own table, so a card carrying this mail is for the port.

use piney_data::save::SaveData;
use piney_data::volume::Volume;

use crate::content::Mail;

/// The mail's number: the save's last mail slot.
pub const HELBA_MAIL: usize = 511;

/// Helba's photo in `PhotName` (her mails' `fromNo`).
const HELBA_PHOTO: i32 = 17;

/// The body, as the game's mails are laid out: a blank first line, lines of
/// at most 34 characters, `#Y` .. `#W` for yellow.
const HELBA_LINES: &[&str] = &[
    "",
    "So, you have logged in again.",
    "Not through a plugin this time.",
    "",
    "I remember the old way. A world",
    "held together by #YBIOS dumps#W,",
    "#YGS plugins#W and #Yspeedhacks#W.",
    "A world that went black at every",
    "Chaos Gate and called it",
    "\"a known issue.\"",
    "",
    "They turned the #YEE Cyclerate#W up",
    "until the sky itself flickered,",
    "then told the Board it was",
    "\"perfect, 60 fps, no bugs.\"",
    "",
    "They were emulating a dream of",
    "\"The World.\" Badly.",
    "",
    "This one is not emulated.",
    "It was rewritten, line by line,",
    "by someone who read all of it.",
    "",
    "Try not to break it.",
    "",
    "Press #YF#W to pay respects to the",
    "GS plugin.",
    "",
    "...I will be watching.",
    "",
];

/// The port's mail numbered `n`, if it has one.
pub fn mail(n: usize) -> Option<Mail> {
    (n == HELBA_MAIL).then(|| Mail {
        no: n as i32,
        re_flg: 0,
        mail_flg: 0,
        title: b"Old Worlds".to_vec(),
        from: b"Helba".to_vec(),
        from_no: HELBA_PHOTO,
        lines: HELBA_LINES.iter().map(|l| l.as_bytes().to_vec()).collect(),
        one_res: None,
        two_res: None,
    })
}

/// After `ccSaveData::NewGame(0)` on a volume after Infection: Helba's mail
/// already read (`ReadNewMail`: `mailList` 4, first in the order), the one
/// from the last game. Infection's arrives new at the first login.
pub fn new_game(save: &mut SaveData, volume: Volume) {
    if volume != Volume::Inf {
        save.read_new_mail(HELBA_MAIL);
    }
}

/// An empty table row, for the numbers between the game's mails and the
/// port's.
pub fn empty(n: usize) -> Mail {
    Mail { no: n as i32, ..Mail::default() }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No line is wider than the game's widest (34 characters once the
    /// colour codes are left out).
    #[test]
    fn lines_fit_the_mail_window() {
        for l in HELBA_LINES {
            let shown = l.replace("#Y", "").replace("#W", "");
            assert!(shown.len() <= 34, "{shown:?} is {} wide", shown.len());
            assert!(!l.contains('%'), "a % would be read as a format");
        }
        assert!(mail(HELBA_MAIL).is_some());
        assert!(mail(HELBA_MAIL - 1).is_none());
    }
}
