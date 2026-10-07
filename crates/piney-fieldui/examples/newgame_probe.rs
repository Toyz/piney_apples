//! Answers `tools/test_save_init_rs.py`'s new game: the save `--mode world`
//! enters The World with, built as the runtime builds it
//! ([`piney_fieldui::newgame::new_game_save`], then at Log in
//! `ccSetupNewGame`'s [`piney_fieldui::newgame::setup_new_game`]). One
//! request a line: `save PARODY` (the whole ccSaveData in hex; PARODY 1 sets
//! parodyFlag before NewGame(0)) and `newgame PARODY` (the save after the
//! title's part alone, as the disc's volume's slot file); `convgame HEX`
//! (CONVERT's: `ccStartEventConvert` and `ConvGame` on that record, its
//! extension after it, answered the same way).

use std::io::BufRead;

use piney_data::iso::Iso;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path)?;
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        let w: Vec<&str> = line.split_whitespace().collect();
        match w.as_slice() {
            ["save", parody] => {
                let mut state = piney_fieldui::newgame::new_game_save(&mut iso, *parody == "1")?;
                piney_fieldui::newgame::setup_new_game(&mut state, &mut iso)?;
                let hex: String = state.save.bytes().iter().map(|b| format!("{b:02x}")).collect();
                println!("{hex}");
            }
            ["newgame", parody] => {
                let state = piney_fieldui::newgame::new_game_save(&mut iso, *parody == "1")?;
                let hex: String = state.save.slot_bytes(iso.volume()?).iter().map(|b| format!("{b:02x}")).collect();
                println!("{hex}");
            }
            ["convgame", hex] => {
                let bytes: Vec<u8> = (0..hex.len() / 2)
                    .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16))
                    .collect::<Result<_, _>>()?;
                let mut save = piney_data::save::SaveData::from_bytes(&bytes)?;
                let volume = iso.volume()?;
                let tables = piney_demo::newgame::NewGameTables::of(volume);
                piney_demo::newgame::start_event_convert(&mut save, volume);
                piney_demo::newgame::conv_game(&mut save, &tables, piney_demo::newgame::SAVE_VA);
                let hex: String = save.slot_bytes(volume).iter().map(|b| format!("{b:02x}")).collect();
                println!("{hex}");
            }
            _ => panic!("bad request: {line}"),
        }
    }
    Ok(())
}
