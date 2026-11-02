//! Not the game's: the options kept across the four parts. Each volume keeps
//! its options in its own save, so a fresh volume came up with others; the
//! port keeps one copy in `settings.toml` in its folder
//! (`piney_data::pack::home`), put into every save as it is made or loaded and
//! written back whenever a menu changes one. One `name = number` a line, each
//! the save's own value, then the port's own options, which no save holds
//! ([`PORT_KEYS`]): `store` keeps their lines as they are.

use std::path::{Path, PathBuf};

use piney_data::save::{SaveData, offset};

/// A field of the option block: its name in the file, where it is in the
/// save, and whether it is a short (else a byte).
struct Field {
    name: &'static str,
    at: usize,
    short: bool,
}

const FIELDS: [Field; 13] = [
    Field { name: "main_volume", at: offset::MAIN_VOL, short: true },
    Field { name: "se_volume", at: offset::SE_VOL, short: true },
    Field { name: "bgm_volume", at: offset::BGM_VOL, short: true },
    Field { name: "sound_output", at: offset::OUTPUT, short: true },
    Field { name: "screen_x", at: offset::SCREEN_X, short: true },
    Field { name: "screen_y", at: offset::SCREEN_Y, short: true },
    Field { name: "vibration", at: offset::VIBRATION, short: false },
    Field { name: "controller_type", at: offset::CAM_TYPE, short: false },
    Field { name: "voice", at: offset::VOICE, short: false },
    Field { name: "message_window", at: offset::STR_WIN_MODE, short: false },
    Field { name: "data_drain_movie", at: offset::DRAIN_DEMO, short: false },
    Field { name: "field_camera", at: offset::CAMERA_MODE, short: false },
    Field { name: "map_mode", at: offset::MAP_MODE, short: false },
];

/// `disc_speed = N`: loads between areas take the PS2 disc's time at N times
/// DVD 1x, as `--dvd N` (`crate::dvd`), for runs started without the flag;
/// 0, the default, leaves them instant (issue #81).
pub const DISC_SPEED: &str = "disc_speed";

/// The port's own keys, with the value `store` writes when the file has none.
const PORT_KEYS: [(&str, &str); 1] = [(DISC_SPEED, "0")];

/// A line's `name = value`, comments and blanks left out.
fn entry(line: &str) -> Option<(&str, &str)> {
    let line = line.split('#').next().unwrap_or("").trim();
    let (name, value) = line.split_once('=')?;
    Some((name.trim(), value.trim()))
}

/// The disc's speed `path` asks for, when above 0.
pub fn disc_speed(path: &Path) -> Option<f64> {
    let text = std::fs::read_to_string(path).ok()?;
    let speed = text.lines().filter_map(entry).find(|&(n, _)| n == DISC_SPEED)?.1;
    speed.parse::<f64>().ok().filter(|&v| v > 0.0)
}

/// The options, in [`FIELDS`]' order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings([i16; FIELDS.len()]);

impl Settings {
    /// The options a save holds.
    pub fn of(save: &SaveData) -> Settings {
        Settings(FIELDS.map(|f| if f.short { save.i16(f.at) } else { i16::from(save.u8(f.at) as i8) }))
    }

    /// The main volume (`mainVol`).
    pub fn main_volume(&self) -> i16 {
        self.0[0]
    }

    /// Into a save.
    pub fn apply(&self, save: &mut SaveData) {
        for (f, &v) in FIELDS.iter().zip(&self.0) {
            if f.short {
                save.set_i16(f.at, v);
            } else {
                save.set_u8(f.at, v as u8);
            }
        }
    }

    /// Read from `path` over `base` (a new save's): the names the file
    /// has; None when there is no file.
    pub fn load(path: &Path, base: &SaveData) -> Option<Settings> {
        let text = std::fs::read_to_string(path).ok()?;
        let mut s = Settings::of(base);
        for (name, value) in text.lines().filter_map(entry) {
            if let (Some(i), Ok(v)) = (FIELDS.iter().position(|f| f.name == name), value.parse::<i16>()) {
                s.0[i] = v;
            }
        }
        Some(s)
    }

    /// Written to `path` (its folder made if need be), the port's own
    /// keys' lines kept as the file had them.
    pub fn store(&self, path: &Path) {
        let old = std::fs::read_to_string(path).unwrap_or_default();
        let mut text = String::from("# piney's options, kept for all four parts.\n");
        for (f, v) in FIELDS.iter().zip(&self.0) {
            text.push_str(&format!("{} = {v}\n", f.name));
        }
        for (key, default) in PORT_KEYS {
            match old.lines().find(|l| entry(l).is_some_and(|(n, _)| n == key)) {
                Some(line) => text.push_str(line),
                None => text.push_str(&format!("{key} = {default}")),
            }
            text.push('\n');
        }
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(path, text) {
            tracing::warn!("{}: {e}", path.display());
        }
    }
}

/// Where the settings live: `settings.toml` in the port's folder.
pub fn path() -> Option<PathBuf> {
    piney_data::pack::home().map(|h| h.join("settings.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A save's options through the file and into another save.
    #[test]
    fn options_round_trip() {
        let mut a = SaveData::new();
        a.set_i16(offset::MAIN_VOL, 200);
        a.set_i16(offset::SCREEN_X, -3);
        a.set_u8(offset::VOICE, 1);
        a.set_u8(offset::STR_WIN_MODE, 2);
        let dir = std::env::current_exe().unwrap().parent().unwrap().join("settings-test");
        let path = dir.join("settings.toml");
        Settings::of(&a).store(&path);
        let got = Settings::load(&path, &SaveData::new()).unwrap();
        let mut b = SaveData::new();
        got.apply(&mut b);
        assert_eq!(Settings::of(&b), Settings::of(&a));
        assert_eq!(b.i16(offset::SCREEN_X), -3);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Issue #81: `disc_speed` is off until the file asks (written as 0),
    /// and an option stored again keeps the line as the player wrote it.
    #[test]
    fn the_disc_speed_survives_a_store() {
        let dir = std::env::current_exe().unwrap().parent().unwrap().join("settings-disc-test");
        let path = dir.join("settings.toml");
        let _ = std::fs::remove_dir_all(&dir);
        let mut a = SaveData::new();
        Settings::of(&a).store(&path);
        assert_eq!(disc_speed(&path), None, "off by default");
        assert!(std::fs::read_to_string(&path).unwrap().contains("disc_speed = 0"));
        let text = std::fs::read_to_string(&path).unwrap().replace("disc_speed = 0", "disc_speed = 2.5 # a slow drive");
        std::fs::write(&path, text).unwrap();
        a.set_i16(offset::MAIN_VOL, 100);
        Settings::of(&a).store(&path);
        assert_eq!(disc_speed(&path), Some(2.5));
        assert!(std::fs::read_to_string(&path).unwrap().contains("disc_speed = 2.5 # a slow drive"));
        assert_eq!(Settings::load(&path, &SaveData::new()).unwrap().main_volume(), 100);
        let _ = std::fs::remove_dir_all(dir);
    }
}
