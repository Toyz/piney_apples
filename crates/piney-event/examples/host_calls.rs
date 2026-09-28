//! Which `Host` methods each block of a disc's scripts reaches: every block
//! played at level 2 on a host that ports nothing, so each call falls to the
//! trait's default and names itself. One line per block, `event block | tags ;
//! conditions | methods`, the open conditions as block `open` and the tags the
//! block's own. For checking the port's hosts against what the scripts need
//! (`cargo run -p piney-event --example host_calls [ISO]`).

use std::path::PathBuf;
use std::sync::Arc;

use piney_data::iso::Iso;
use piney_event::host::take_unported;
use piney_event::{Host, Library, SaveData, Vm, official, text};

struct Null {
    save: SaveData,
}

impl Host for Null {
    fn save(&mut self) -> &mut SaveData {
        &mut self.save
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let iso = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| root.join("work/infection/infection.iso"));
    let mut disc = Iso::open(&iso)?;
    let events = official::events(&mut disc)?;
    let lib = Arc::new(Library::new(disc.volume()?, events.clone()));
    for e in &events {
        let n = i32::from(e.number);
        let open: Vec<String> = e.script.open.iter().map(text::print_cond).collect();
        println!("{n} open | ; {} |", open.join(" & "));
        for (b, block) in e.script.blocks.iter().enumerate() {
            let mut vm = Vm::new(lib.clone());
            let mut host = Null { save: SaveData::default() };
            take_unported();
            vm.play_block(n, b, &mut host);
            let mut used = take_unported();
            used.sort();
            let tags: Vec<String> = block.tags.iter().map(text::print_tag).collect();
            let conds: Vec<String> = block.conds.iter().map(text::print_cond).collect();
            println!("{n} {b} | {} ; {} | {}", tags.join(" & "), conds.join(" & "), used.join(" "));
        }
    }
    Ok(())
}
