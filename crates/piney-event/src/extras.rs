//! What the port adds to the game's scripts, applied after they are loaded
//! ([`apply`]) so `official::events` stays the game's. Infection's event 1 (the
//! new game) sends one more mail, Helba's ([`HELBA_MAIL`], its text in
//! `piney_desktop::extras`), between Yasuhiko's (mail 4) and CC Corporation's
//! two (5 and 320), so the mailer, newest first, lists it between them.

use piney_data::volume::Volume;

use crate::ir::{Event, Op};

/// The port's mail from Helba: the save's last mail slot.
pub const HELBA_MAIL: i16 = 511;

/// Applies the port's patches to a volume's events.
pub fn apply(events: &mut [Event], volume: Volume) {
    if volume != Volume::Inf {
        return;
    }
    let Some(e) = events.iter_mut().find(|e| e.number == 1) else { return };
    let Some(block) = e.script.blocks.first_mut() else { return };
    if block.ops.contains(&Op::Mail { mail: HELBA_MAIL }) {
        return;
    }
    if let Some(i) = block.ops.iter().position(|o| *o == Op::Mail { mail: 5 }) {
        block.ops.insert(i, Op::Mail { mail: HELBA_MAIL });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A volume's events read from its disc image, None without it.
    fn events(dir: &str) -> Option<Vec<crate::ir::Event>> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{dir}/{dir}.iso"));
        let mut iso = piney_data::iso::Iso::open(path).ok()?;
        Some(crate::official::events(&mut iso).unwrap())
    }

    #[test]
    fn helba_lists_below_cc_corporation() {
        let (Some(mut events), Some(mut mutation)) = (events("infection"), events("mutation")) else { return };
        apply(&mut events, Volume::Inf);
        apply(&mut events, Volume::Inf);
        let ops = &events.iter().find(|e| e.number == 1).unwrap().script.blocks[0].ops;
        let mails: Vec<i16> =
            ops.iter().filter_map(|o| if let Op::Mail { mail } = o { Some(*mail) } else { None }).collect();
        assert_eq!(mails, [4, HELBA_MAIL, 5, 320]);
        // The other volumes are left alone.
        let before = mutation.clone();
        apply(&mut mutation, Volume::Mut);
        assert_eq!(mutation, before);
    }
}
