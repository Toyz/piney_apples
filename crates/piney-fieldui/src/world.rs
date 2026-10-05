//! What the field UI reads of the world each frame. `ccMenuCtrl` reaches
//! into the game through globals: `game`, `ccPartyManager`, `cmndTarget` /
//! `cmndTargetPrev`, `cmndSortRoot`, `plw`, `eventMng` and `ccSys.count`.
//! The runtime fills a [`World`] with the same facts before each
//! [`crate::FieldUi::step`]; the UI never changes it. Screen positions of
//! characters are the world's to compute (`ccCalcTagPosChar` /
//! `ccCalcTagPos` project through the field camera).

/// `ccGame` as the menus read it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Game {
    /// +0x00 `status`: 5 in The World.
    pub status: i32,
    /// +0x14 `area`: 0 town, 1 field, 2 dungeon.
    pub area: i32,
    /// +0x20 `town`.
    pub town: i32,
    /// +0x24 `field`.
    pub field: i32,
    /// +0x28 `dungeon`.
    pub dungeon: i32,
    /// +0x2c `floor`: the dungeon's floor (the boxes' draws go up by it).
    pub floor: i32,
    /// +0x1c `server`: 0-4 (the Chaos Gate's lists are by server).
    pub server: i32,
    /// +0x78 `areaLevel`: the keyword screen writes it (when it is 0 or in
    /// a town) with the battle level it shows.
    pub area_level: i32,
    /// +0x58 `inBattle`: 0 none, 1 a battle, 2 a boss's.
    pub in_battle: i32,
    /// +0x5c `inBattleCnt`: frames since the battle started (the banner).
    pub in_battle_cnt: i32,
    /// +0x64 `gameCnt[3]`: play-time counters in sixtieths
    /// (`ccAddPlayTime`); `ChangeRequest` zeroes [0] at every change,
    /// [1] when the scene is replaced, [2] when leaving a town: [2] is the
    /// time since the Chaos Gate (the Zeit statue's).
    pub game_cnt: [i32; 3],
    /// `WORLD_MAN::GetDungeonType()` (8 and 9 are the special floors).
    pub dungeon_type: i32,
    /// `worldman->wordparam->areaLevel` (+0x20): the spring's kind (4 on).
    pub word_level: i32,
    /// `WORLD_MAN::GetBG()` (+0xc, `bgnum`): what the spring changes an
    /// item into.
    pub bgnum: i32,
}

/// A character as the HUD reads it (`ccChar` and its `ccCharBaseParam`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CharInfo {
    /// Identity for the runtime (pointer equality in the game); never 0.
    pub handle: u32,
    /// `base->type`: the character's type bits (0x07 party, 0x60 enemies,
    /// 0x80 bosses, 0x18 NPCs, 0x0f001f1f the talkable ones ...).
    pub types: u32,
    /// `base->id`.
    pub id: i16,
    /// `ccCheckObjectSize(c)` (gcmn 0x0042e120): the size class of an
    /// enemy's (`enemyTbl` row +0x70), gimmick's or NPC's table row: 1
    /// large, 3 middle, 4 small (the enemies have no other).
    pub object_size: i32,
    /// `base->name`.
    pub name: Vec<u8>,
    /// `base->height`.
    pub height: f32,
    /// +0x70 `HP`, +0x72 `SP`, +0x74 `maxHP`, +0x76 `maxSP`.
    pub hp: i16,
    pub sp: i16,
    pub max_hp: i16,
    pub max_sp: i16,
    /// +0x08 `condition`: dead, hold, drainHP, drainSP, critical, dying,
    /// invincible, regeneSP, curse, sleep, confusion, charm, regeneHP,
    /// poison, paralysis, speed; then `speedValue`.
    pub condition: [i16; 16],
    pub speed_value: f32,
    /// The battle abilities' changes the icons show (the `time` element:
    /// pAtk, pDef, pHit, pEva, mAtk, mDef, mHit, mEva), and the six
    /// elements'.
    pub ability: [i16; 8],
    pub elements: [i16; 6],
    /// +0xc4 `cmndDist`: the distance to the player.
    pub cmnd_dist: f32,
    /// +0x14c of a treasure box (`ccObj`): its item (`category << 16 |
    /// id`), -1 for a draw from the area's list.
    pub item: i32,
    /// +0x150 of a treasure box or idol: its entry's `param[2]`, the trap
    /// (0-2; -1 none, 3 disarmed).
    pub trap: i32,
    /// +0x7c `skillID`: a trapped box's trap skill (1, 156, 162).
    pub skill: i16,
    /// +0x1d4 of a gimmick: its `actNum` (the spring's state, which
    /// `FountainMenu3` waits on).
    pub act_num: i32,
    /// `ccChar::CheckCharAttribute(0)`: an enemy's element, -1 none.
    pub attribute: i32,
    /// An enemy's or boss's table row `Exdefense` (+0x64): its immunities,
    /// bit 0x1 pDef, 0x2 mDef, 0x4-0x80 the elements; 0 for the rest.
    pub exdefense: i16,
    /// The same bits for the defences now below the row's (`personality`'s
    /// `real` under the table's): those immunities are broken.
    pub exdefense_lowered: i16,
    /// The target cursor's point: `ccCalcTagPosChar(c, p, (0, 0, 0.45
    /// height), 0)`, `None` when it fails.
    pub tag: Option<(i32, i32)>,
    /// The life bar's point: `ccCalcTagPosChar(c, p, (0, 0, 160), 1)`'s
    /// result (1 on screen, 0 off it) and `ccCalcTagPos(pos, p, (0, 0, 0.9
    /// height))`'s point.
    pub bar_res: i32,
    pub bar: (i32, i32),
    /// `personality +98` of an enemy or boss: the protect break's count
    /// (a mark shows while it is between 1 and 239).
    pub pp: i16,
    /// The off-screen arrow's point (`ccGetDirc` from the player to the
    /// character, turned by the camera, 256 - 384 sin / 256 - 384 cos).
    pub arrow: (i32, i32),
    /// `base->width`: added to a skill's reach.
    pub width: f32,
    /// `ccCheckCameraDeg(pos, 0x1400)`: in front of the camera (the skill
    /// and target menus take only these).
    pub in_view: bool,
    /// +0x50 `posP` (x, y, z): where an area skill measures from.
    pub pos_p: [f32; 3],
    /// A party member's chat settings (`personality` +200 on):
    /// `chatMember`, `chatAction`, `chatSkill`, `equipSpcNum`,
    /// `chatEquipStatus[2]`.
    pub chat: [i16; 6],
    /// A party member's `actNum` (+0xee): the action (animation) playing.
    pub act: i16,
}

impl CharInfo {
    /// `base->type & mask`.
    pub fn is(&self, mask: u32) -> bool {
        self.types & mask != 0
    }

    /// The immunity the target window names (`Disp` 0x0051e960 -
    /// 0x0051ec3c): an enemy's or boss's lowest `Exdefense` bit as its
    /// `kyviaStatusStr` piece (0 "Physical Tol." to 7 "Darkness Tol."),
    /// and whether that defence is lowered (the immunity broken).
    pub fn tolerance(&self) -> Option<(usize, bool)> {
        if !self.is(0xe0) {
            return None;
        }
        let k = (0..8).find(|&k| self.exdefense & (1 << k) != 0)?;
        Some((k, self.exdefense_lowered & (1 << k) != 0))
    }

    /// `ccSpcChar::CheckChangeEquip()` (gcmn 0x0059f3c0): why the member
    /// cannot change equipment now, 0 when it can. 1-5 a condition (held,
    /// asleep, paralysed, charmed, confused), 7 a skill under way
    /// (`skillID` +0x7c 2 or more), 8 acts 12 and 13, 9 act 14.
    pub fn check_change_equip(&self) -> i16 {
        let c = &self.condition;
        if c[1] != 0 {
            1
        } else if c[9] != 0 {
            2
        } else if c[14] != 0 {
            3
        } else if c[11] != 0 {
            4
        } else if c[10] != 0 {
            5
        } else if self.skill >= 2 {
            7
        } else {
            match self.act {
                12 | 13 => 8,
                14 => 9,
                _ => 0,
            }
        }
    }
}

/// The field around the menus, one frame's worth.
#[derive(Clone, Debug, Default)]
pub struct World {
    pub game: Game,
    /// Where the chat balloons' speakers are this frame (for those of
    /// `FieldUi::chat_speakers`).
    pub chat_at: Vec<crate::chat_msg::ChatAt>,
    /// `ccPartyManager.memberChar[3]` and `memberID[3]` (-1 empty).
    pub party: [Option<CharInfo>; 3],
    pub party_id: [i32; 3],
    /// `cmndTarget` when `ccCheckTarget` passes it.
    pub target: Option<CharInfo>,
    /// `cmndTargetPrev` when `ccCheckTarget` returns 1 for it.
    pub target_prev: Option<CharInfo>,
    /// The characters on `cmndSortRoot`'s chain, in order.
    pub sorted: Vec<CharInfo>,
    /// `plw.pw->condition.dead` (the player is down).
    pub player_dead: bool,
    /// `ccSkillCheck(plw.pw)`: the player is using a skill.
    pub player_skill: i32,
    /// `plw.pw`'s byte +0xe0 bit 2 (`attackSW`: an attack running).
    pub player_attacking: bool,
    /// `compulsionGameOver`.
    pub game_over: bool,
    /// `eventMng->status`: an event block is running.
    pub event_status: i32,
    /// `eventMng->bossEntry`, -1 none.
    pub boss_entry: i32,
    /// `pgRideFlag`: riding a Grunty.
    pub pg_ride: bool,
    /// `ccPuccigusoStart` has not returned yet (its fades run).
    pub pg_starting: bool,
    /// A Fairy's Orb's `WORLD_MAN::ShowMap` has not reported done yet
    /// (a dungeon's builds one room a frame).
    pub map_showing: bool,
    /// `cmndPcRoot`, `cmndEneRoot`, `cmndObjRoot`: the party members, the
    /// enemies and bosses, and the rest, each chained by `cmndLink`; an
    /// area skill's other targets come from the chain of its target's kind.
    pub pc_chain: Vec<CharInfo>,
    pub ene_chain: Vec<CharInfo>,
    pub obj_chain: Vec<CharInfo>,
    /// `ccEvent +0x180 areaCode[16]`: the story areas the events bar
    /// (mode 0) or allow alone (mode 1) at the Chaos Gate; (-1, -1) free.
    pub area_codes: [(i16, i16); 16],
    /// `WORLD_MAN::GetFieldAttrb()`: the area's element (0-5), which of
    /// the item lists a box draws from.
    pub field_attr: i32,
    /// `WORLD_MAN +0x120 eventAreaNumber`: the story area the field is, 0
    /// none (a box in a story field draws by the area's item level).
    pub event_area: i32,
    /// The area's merged `WORDPARAM` (`WORLD_MAN +0x158`): `areaLevel`
    /// and `itemOfs`.
    pub area_word: (i32, i32),
    /// `ccPgAdultCheck(game.server, k)` (gcmn 0x00510650) for the three
    /// Grunty slots: `Some(n)` when an adult Grunty can be called there
    /// (the Grunty Flute), `None` for the check's -1.
    pub pg_adult: [Option<i32>; 3],
    /// `checkPartyAnnihilation()` (gcmn 0x0059d080): every member down.
    pub party_annihilated: bool,
    /// `ccSpcManager.registry[registryNum]` (gcmn 0x00730340): the members
    /// whose characters are loaded, as (id, `bootParam`); `CheckSpc(id)` is
    /// the place in it.
    pub spc: Vec<(i32, i32)>,
    /// The people on `g_entCtrl`'s NPC list (`ccEntryObj` +0x124, their
    /// `npcTbl` rows): book III's "Online" for a person.
    pub npcs: Vec<i32>,
    /// A Grunty in a breeder's pen (`ccPGuso`): what Give Food reads of
    /// `cmndTargetPrev` when it is this character.
    pub grunty: Option<crate::menus::breeder::Grunty>,
}

impl World {
    /// `ccPartyManager.num`: the members in the party.
    pub fn party_num(&self) -> i16 {
        self.party.iter().flatten().count() as i16
    }

    /// `ccParty::CheckMemberID(id)` (gcmn 0x0059cfe0): the party slot of
    /// member `id`, -1 when not in the party.
    pub fn member_slot(&self, id: i32) -> i32 {
        self.party_id.iter().position(|&v| v == id).map_or(-1, |k| k as i32)
    }
}

impl World {
    /// `plw.pw`, the player's character: party slot 0.
    pub fn player(&self) -> Option<&CharInfo> {
        self.party[0].as_ref()
    }
}
