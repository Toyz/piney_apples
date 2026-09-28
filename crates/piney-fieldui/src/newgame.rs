//! What `ccSetupNewGame` (main 0x001687a0) does to the save as The World
//! starts, after the overlay (GCMN.PRG) is in: `ccSaveData::InitSpcParam`
//! (main 0x00175d30), `newGameFlag` (+0x6770) = 1, `SetSpcBaseMsg`
//! (0x00176200, [`crate::menus::talk::set_spc_base_msg`]).
//!
//! `InitSpcParam` gives a new game its starting kit, and does nothing once
//! `newGameFlag` is 1:
//!
//! ```text
//! newGameFlag 0 or 2: for member i in 1..18 (2: only those not in partyMemberFlag's bit i)
//!     ChangeEquipment(i, ccGetJobWeaponCategory(spcParam[i].job), -1)
//!     ChangeEquipment(i, 6, -1) .. (i, 9, -1)
//!     spcDefaultItemList[i][0..10] (gcmn 0x00647d80, 40 bytes a member):
//!         AddItem(i, category, id, num) for each whose category is not negative
//! newGameFlag 0 only, Kite:
//!     ChangeEquipment(0, ccGetJobWeaponCategory(spcParam[0].job), -1), then 6 .. 9
//!     AddItem(0, ...) for each of playerDefaultItemList[24] (0x00647cc0) and
//!         playerDefaultImportantItemList[24] (0x00647d20), whatever the category
//!     parodyFlag: spcParam[0].level = 1, 30, 50, 70 and ccSetLevelParam(spcParam[0],
//!         20, 50, 70, 90) by volumeNum 1-4; AddItem(0, 15, k, 50) for k in 0..12
//! ```
//!
//! `ChangeEquipment(sid, cat, -1)` changes no piece: it adds the skills of
//! the piece `NewGame` put in that slot (from `charTbl`) and the sets' to
//! the member's `skillList`, which is where the starting skills come from.
//! `ccGetJobWeaponCategory(job)` (gcmn 0x005711f0) is the job for 0-5 and 0
//! for anything else.

use piney_battle::Tables;
use piney_battle::item::{self as bitem, CategoryOrder};
use piney_data::save::{SaveData, offset};
use piney_desktop::SaveState;

use crate::menus::equip::{self, EquipRaw};

/// `ccSpcParam.equipment[6]` and `job`, from the record's start.
const SPC_EQUIPMENT: usize = 0xc8;
const SPC_JOB: usize = 0xd8;
/// `ccSpcParam.base.level` (a short here: `InitSpcParam` stores it with
/// `sh`).
const SPC_LEVEL: usize = 0x0e;
const IMPORTANT: i32 = 15;

fn spc_at(sid: usize) -> usize {
    piney_data::save::by_id::spc_param(sid)
}

/// `ccSaveData::ChangeEquipment(sid, cat, id)` (main 0x00177010):
/// `ccChangeEquipment(&spcParam[sid].equipment, skillList[sid], cat, id,
/// &spcParam[sid])` ([`equip::change_equipment`]) on the save.
pub fn change_equipment(save: &mut SaveData, raw: &EquipRaw, sid: usize, cat: i32, id: i32) {
    let at = spc_at(sid);
    let list = piney_data::save::by_id::skill_list(sid);
    let mut eq: [i16; 6] = std::array::from_fn(|k| save.i16(at + SPC_EQUIPMENT + 2 * k));
    let mut sk: [i16; 20] = std::array::from_fn(|k| save.i16(list + 2 * k));
    let job = save.i16(at + SPC_JOB);
    equip::change_equipment(raw, &mut eq, &mut sk, cat, id, job);
    for (k, v) in eq.iter().enumerate() {
        save.set_i16(at + SPC_EQUIPMENT + 2 * k, *v);
    }
    for (k, v) in sk.iter().enumerate() {
        save.set_i16(list + 2 * k, *v);
    }
}

/// Every slot of member `sid` through `ChangeEquipment(sid, cat, -1)`:
/// the job's weapon category, then 6 (head) .. 9 (leg).
fn wear_all(save: &mut SaveData, raw: &EquipRaw, sid: usize) {
    let job = save.i16(spc_at(sid) + SPC_JOB);
    change_equipment(save, raw, sid, equip::job_weapon(i32::from(job)).0, -1);
    for cat in 6..=9 {
        change_equipment(save, raw, sid, cat, -1);
    }
}

/// `ccSaveData::InitSpcParam()` (main 0x00175d30), for volume `volume`.
pub fn init_spc_param(save: &mut SaveData, t: &Tables, raw: &EquipRaw, order: &CategoryOrder, volume: i32) {
    let flag = save.u8(offset::NEW_GAME_FLAG) as i8;
    if flag == 0 || flag == 2 {
        for i in 1..18usize {
            if flag != 0 && save.i32(offset::PARTY_MEMBER_FLAG) & (1 << i) != 0 {
                continue;
            }
            wear_all(save, raw, i);
            for e in t.spc_default_items.get(i).map_or(&[][..], Vec::as_slice) {
                if e.category >= 0 {
                    bitem::add_item(save, order, i as i32, i32::from(e.category), i32::from(e.id), i32::from(e.num));
                }
            }
        }
    }
    if flag != 0 {
        return;
    }
    wear_all(save, raw, 0);
    for e in t.player_default_items.iter().chain(&t.player_default_important) {
        bitem::add_item(save, order, 0, i32::from(e.category), i32::from(e.id), i32::from(e.num));
    }
    if save.u8(offset::PARODY_FLAG) == 0 {
        return;
    }
    let (level, lvs) = match volume {
        4 => (70, 90),
        3 => (50, 70),
        2 => (30, 50),
        _ => (1, 20),
    };
    save.set_i16(spc_at(0) + SPC_LEVEL, level);
    let mut p = piney_battle::param::SpcParam::from_save(save, 0);
    piney_battle::chara::set_level_param(t, &mut p, lvs);
    p.store(save, 0);
    for k in 0..12 {
        bitem::add_item(save, order, 0, IMPORTANT, k, 50);
    }
}

/// A new game's save as the title hands it on: the boot's
/// ([`SaveState::fresh_with`], `ccSaveData::Init`'s text from the disc),
/// `NewGame(1)` as `ccSetupDemo` runs it, `parodyFlag` set when the title's
/// Parody was chosen (`PlayParodyGame`), `NewGame(0)` as New Game runs it;
/// the player named Kite when the name is empty. What `--mode world`
/// starts from, before [`crate::FieldUi::setup_new_game`].
pub fn new_game_save(disc: &mut piney_data::iso::Iso, parody: bool) -> Result<SaveState, String> {
    use piney_demo::newgame::{NewGameTables, SAVE_VA, new_game};
    let read = |disc: &mut piney_data::iso::Iso| -> piney_data::Result<(piney_desktop::InitText, NewGameTables)> {
        Ok((piney_desktop::InitText::from_disc(disc)?, NewGameTables::of(disc.volume()?)))
    };
    let (text, tables) = read(disc).map_err(|e| format!("the new game's tables: {e}"))?;
    let mut state = SaveState::fresh_with(&text);
    new_game(&mut state.save, 1, &tables, SAVE_VA);
    if parody {
        state.save.set_u8(offset::PARODY_FLAG, 1);
    }
    new_game(&mut state.save, 0, &tables, SAVE_VA);
    if state.save.name().is_empty() {
        for (i, &b) in b"Kite\0".iter().enumerate() {
            state.save.set_u8(offset::PL_NAME + i, b);
        }
    }
    Ok(state)
}

/// `ccSetupNewGame`'s save as The World starts (Log in): `InitSpcParam`,
/// `newGameFlag` = 1, `SetSpcBaseMsg`, with the volume's tables.
pub fn setup_new_game_with(
    state: &mut SaveState,
    volume: piney_data::volume::Volume,
    t: &Tables,
    raw: &EquipRaw,
    order: &CategoryOrder,
) {
    init_spc_param(&mut state.save, t, raw, order, volume.number());
    state.save.set_u8(offset::NEW_GAME_FLAG, 1);
    crate::menus::talk::set_spc_base_msg(state, volume);
}

/// [`setup_new_game_with`] for the disc's volume.
pub fn setup_new_game(state: &mut SaveState, disc: &mut piney_data::iso::Iso) -> piney_data::Result<()> {
    let volume = disc.volume()?;
    let t = Tables::of(volume);
    let raw = EquipRaw::of(volume, &t);
    let order = piney_data::tables::fieldui::of(volume).category_order();
    let order: CategoryOrder = std::array::from_fn(|k| (order[k][0], order[k][1]));
    setup_new_game_with(state, volume, &t, &raw, &order);
    Ok(())
}

impl crate::FieldUi {
    /// [`setup_new_game_with`] with the tables the menus read.
    pub fn setup_new_game(&self, state: &mut SaveState) {
        let p = &self.texts().pers;
        setup_new_game_with(state, self.texts().volume, &p.battle, &p.equip_raw, &p.category_order);
    }
}
