//! The Chaos Gate's keywords as the gate menus read them
//! (`docs/engine/area-words.md`): the twelve `WORDPARAM` tables, the story
//! areas (`eventAreaInfo`), and the parts of `WORLD_MAN` the menus call -
//! `GetWordParamPtr` / `Part` / `ID`, `SimGenerateCode`, `GetFieldAttrb` -
//! with `ccAnalyzeEnemyList` and `ccEvent::CheckAreaCode`.
//!
//! The tables are the volume's generated ones (`piney_data::area`,
//! `piney_data::tables::battle`'s enemy lists).

/// "Unset" in a word's fields (`dungeonSize` uses 0).
pub const UNSET: i32 = 255;

/// A `WORDPARAM`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Word {
    pub text: Vec<u8>,
    /// Which part of an address: 0 A, 1 B, 2 C.
    pub slot: i32,
    pub id: i32,
    pub pri: i32,
    /// +0x0c `fieldType`, `dungeonSize`, `weather`, `ground`, `object`,
    /// `areaLevel`, `enemyOfs`, `itemOfs`, `circleOfs` (+0x2c).
    pub fields: [i32; 9],
}

impl Word {
    pub fn field_type(&self) -> i32 {
        self.fields[0]
    }
    pub fn weather(&self) -> i32 {
        self.fields[2]
    }
    pub fn area_level(&self) -> i32 {
        self.fields[5]
    }
    pub fn enemy_ofs(&self) -> i32 {
        self.fields[6]
    }
}

/// An `EVENTAREA_INFO`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventArea {
    pub code: i32,
    /// The address, `None` for records without one.
    pub words: Option<[Vec<u8>; 3]>,
    pub server: i32,
    pub circle: i32,
    pub enemy: i32,
    pub item: i32,
    pub model: i32,
    pub flag: i32,
    pub kind: i32,
    pub bgnum: i32,
    pub dungeon_num: i32,
    pub protect: [i32; 8],
}

/// What `SimGenerateCode` leaves in `WORLD_MAN`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Generated {
    /// The three word IDs.
    pub ids: [i32; 3],
    /// The merged `WORDPARAM` (+0x158): the attributes of the area.
    pub base: Word,
    /// +0x120 `eventAreaNumber`: the story area the words name, 0 none.
    pub event: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Words {
    /// `volumeNum`.
    pub volume: i32,
    pub words: Vec<Word>,
    pub events: Vec<EventArea>,
    pub dungeon: Vec<(i32, i32)>,
    /// `ccEnemyListInfo[server][type]`: the enemies' levels, in list order.
    pub enemy_levels: Vec<Vec<Vec<i32>>>,
}

impl Words {
    /// The volume's keywords (`word_a1` .. `word_c4` in that order, the slot
    /// the table's place mod 3), story areas and dungeon sizes
    /// (`piney_data::area`), and the enemies' levels by the spawn lists.
    pub fn of(volume: piney_data::volume::Volume, battle: &piney_battle::Tables) -> Words {
        let area = piney_data::area::AreaTables::of(volume);
        let mut rows: Vec<_> = area.words.iter().collect();
        rows.sort_by_key(|w| (w.group, w.slot));
        let words = rows
            .iter()
            .map(|w| {
                let a = &w.attrs;
                Word {
                    // One char a byte (the executable's bytes as Latin-1).
                    text: w.text.chars().map(|c| c as u8).collect(),
                    slot: w.slot as i32,
                    id: w.id,
                    pri: w.pri,
                    fields: [
                        a.field_type,
                        a.dungeon_size,
                        a.weather,
                        a.ground,
                        a.object,
                        a.area_level,
                        a.enemy_ofs,
                        a.item_ofs,
                        a.circle_ofs,
                    ],
                }
            })
            .collect();
        let latin = |t: &str| t.chars().map(|c| c as u8).collect::<Vec<u8>>();
        let events = area
            .events
            .iter()
            .map(|e| EventArea {
                code: e.code,
                words: e.words[0].map(|_| e.words.map(|w| w.map_or_else(Vec::new, latin))),
                server: e.server,
                circle: e.circle,
                enemy: e.enemy,
                item: e.item,
                model: e.model,
                flag: e.flag,
                kind: e.field_type,
                bgnum: e.bgnum,
                dungeon_num: e.dungeon_num,
                protect: e.protect,
            })
            .collect();
        let dungeon = area.dungeon_data.iter().map(|d| (d.level_max, d.room_max)).collect();
        let level = |id: &i32| {
            usize::try_from(*id).ok().and_then(|i| battle.enemies.get(i)).map_or(0, |r| i32::from(r.param.base.level))
        };
        let enemy_levels = piney_data::tables::battle::of(volume)
            .enemy_lists()
            .iter()
            .map(|server| server.iter().map(|l| l.list.unwrap_or_default().iter().map(level).collect()).collect())
            .collect();
        Words { volume: volume.number(), words, events, dungeon, enemy_levels }
    }

    /// `GetWordParamPtr(id)`: the word with this ID.
    pub fn word(&self, id: i32) -> Option<&Word> {
        self.words.iter().find(|w| w.id == id)
    }

    /// `GetWordParamPart(id)`: its slot, -1 for none.
    pub fn part(&self, id: i32) -> i32 {
        self.word(id).map_or(-1, |w| w.slot)
    }

    /// `GetWordParamID(text)`: the first word with this text, -1 none.
    pub fn id_of(&self, text: &[u8]) -> i32 {
        self.words.iter().find(|w| w.text == text).map_or(-1, |w| w.id)
    }

    /// `ccGetEventAreaInfo(code)`.
    pub fn event(&self, code: i32) -> Option<&EventArea> {
        self.events.iter().find(|e| e.code == code)
    }

    /// `GetWordParamFromEvCode(code, slot)`'s ID: the story area's word in
    /// that slot, -1 none.
    pub fn event_word_id(&self, code: i32, slot: i32) -> i32 {
        let Some(e) = self.event(code) else { return -1 };
        let Some(ws) = &e.words else { return -1 };
        let text = &ws[slot.clamp(0, 2) as usize];
        self.words.iter().find(|w| w.slot == slot && &w.text == text).map_or(-1, |w| w.id)
    }

    /// `ccCheckEventAreaNum(a, b, c)`: the story area on `server` whose
    /// address is these words (compared as text), 0 none.
    pub fn event_number(&self, ids: [i32; 3], server: i32) -> i32 {
        let ws: Vec<Option<&Word>> = ids.iter().map(|&i| self.word(i)).collect();
        for e in &self.events {
            let Some(t) = &e.words else { continue };
            if e.server != server {
                continue;
            }
            if (0..3).all(|k| ws[k].is_some_and(|w| w.text == t[k])) {
                return e.code;
            }
        }
        0
    }

    /// `WORLD_MAN::SimGenerateCode(a, b, c)` (main 0x0019e5c0).
    /// `flag71` is `saveData + 0x5bc0` bit 62.
    pub fn generate(&self, ids: [i32; 3], server: i32, flag71: bool) -> Generated {
        let blank = Word { fields: [UNSET, 0, UNSET, UNSET, UNSET, UNSET, UNSET, UNSET, UNSET], ..Word::default() };
        let get = |i: i32| self.word(i).cloned().unwrap_or_else(|| blank.clone());
        let (wa, wb, wc) = (get(ids[0]), get(ids[1]), get(ids[2]));
        let code = (wa.id as u32)
            .wrapping_mul(1_000_000)
            .wrapping_add((wb.id as u32).wrapping_mul(1000))
            .wrapping_add(wc.id as u32);
        let mut seed = code;
        let mut rand = || {
            seed = seed.wrapping_mul(109).wrapping_add(1021) % 0xffff_fffe;
            seed
        };
        // fieldSeed and the three dungeon seeds.
        for _ in 0..4 {
            rand();
        }
        let mut order = [wa.clone(), wb, wc];
        for i in 1..3 {
            let mut j = i;
            while j > 0 && order[j].pri < order[j - 1].pri {
                order.swap(j, j - 1);
                j -= 1;
            }
        }
        let mut base = Word { id: code as i32, ..Word::default() };
        let ft = (rand() % 11) as i32;
        base.fields[0] = ft;
        base.fields[2] = clamp_weather(ft, (rand() % 10) as i32);
        base.fields[1] = (rand() % 10) as i32 + 1;
        base.fields[3] = (rand() % 3) as i32;
        base.fields[4] = (rand() % 3) as i32;
        base.fields[5] = (rand() % 5) as i32 + 1;
        base.fields[6] = (rand() % 20) as i32 - 9;
        base.fields[7] = (rand() % 20) as i32 - 9;
        base.fields[8] = (rand() % 3) as i32;
        let mut merged = order[0].clone();
        merge(&mut merged, &order[1]);
        merge(&mut merged, &order[2]);
        merge(&mut base, &merged);
        let event = self.event_number(ids, server);
        if event != 0
            && let Some(e) = self.event(event)
        {
            for (v, k) in [(e.circle, 8), (e.enemy, 6), (e.item, 7), (e.kind, 0), (e.bgnum, 2)] {
                if v != UNSET {
                    base.fields[k] = v;
                }
            }
            if event == 71 && flag71 {
                base.fields[6] = 119;
                base.fields[7] = 119;
            }
            if event == 47 && self.volume >= 3 {
                base.fields[6] = 78;
                base.fields[7] = 122;
            }
        }
        base.fields[2] = clamp_weather(base.fields[0], base.fields[2]);
        if base.fields[1] >= 11 {
            base.fields[1] = 1;
        }
        Generated { ids, base, event }
    }
}

/// The weather a field type allows (the jump tables at 0x003555f0 and
/// 0x003555c0).
pub fn clamp_weather(field_type: i32, w: i32) -> i32 {
    match field_type {
        0..=4 => {
            if w < 4 {
                w
            } else {
                0
            }
        }
        5 | 6 => match w {
            8 => 4,
            9 => 5,
            w if w < 4 => w,
            _ => 0,
        },
        7..=10 => {
            if w < 8 {
                w
            } else {
                0
            }
        }
        _ => w,
    }
}

/// `CopyWordParam(dst, src)` (0x0019e500): the fields `src` sets win.
fn merge(dst: &mut Word, src: &Word) {
    for k in [0, 2, 3, 4, 5, 6, 7, 8] {
        if src.fields[k] != UNSET {
            dst.fields[k] = src.fields[k];
        }
    }
    if src.fields[1] != 0 {
        dst.fields[1] = src.fields[1];
    }
}

/// `WORLD_MAN::GetFieldAttrb()` (main 0x001a3830): the enemy list type
/// (and the element the menu shows) by field type and weather.
pub fn field_attr(field_type: i32, weather: i32) -> i32 {
    match field_type {
        0..=3 => 2,
        4 => {
            if weather < 6 {
                3
            } else {
                4
            }
        }
        5 | 6 => 1,
        7 => 0,
        8 => {
            if weather < 6 {
                5
            } else {
                4
            }
        }
        9 => {
            if weather < 4 {
                3
            } else if weather < 6 {
                1
            } else {
                4
            }
        }
        10 => {
            if weather < 4 {
                3
            } else if weather < 6 {
                5
            } else {
                4
            }
        }
        _ => 2,
    }
}

impl Words {
    /// `ccAnalyzeEnemyList(server, type, rank)` (gcmn 0x0042f290): the
    /// highest level of the three enemies registered from `rank` (staying
    /// on the list's last). From Mutation on (0x00444c20) a start too near
    /// the end moves back so that all three fit (`ccRegisterEnemyRange`,
    /// which `ccInitRegisterEnemy` sets to 3 and nothing changes), as
    /// `piney_battle::entry::analyze_enemy_list` has it.
    pub fn analyze_enemies(&self, server: i32, kind: i32, rank: i32) -> i32 {
        let Some(list) =
            self.enemy_levels.get(server.clamp(0, 4) as usize).and_then(|r| r.get(kind.clamp(0, 6) as usize))
        else {
            return 0;
        };
        let num = list.len() as i32;
        let later = self.volume > 1;
        let mut at = if later && num < rank + 3 { num - 3 } else { rank };
        let mut best = 0;
        for _ in 0..3 {
            let lv = usize::try_from(at).ok().and_then(|i| list.get(i)).copied().unwrap_or(0);
            best = best.max(lv);
            at += 1;
            if !later && at == num {
                at -= 1;
            }
        }
        best
    }

    /// `ccEvent::CheckAreaCode(a, b, c)` (main 0x001b3100): 1 if the events
    /// let these words through. `area_codes` is `ccEvent +0x180`'s sixteen
    /// (story area, mode): mode 0 bars that area's address, mode 1 bars
    /// every other.
    pub fn check_area_code(&self, ids: [i32; 3], server: i32, area_codes: &[(i16, i16); 16]) -> bool {
        let mut ok = true;
        for &(code, mode) in area_codes {
            let (mut wa, mut wb, mut wc, mut sv) = (-1, -1, -1, -1);
            if code >= 0 {
                let c = i32::from(code);
                sv = self.event(c).map_or(-1, |e| e.server);
                wa = self.event_word_id(c, 0);
                wb = self.event_word_id(c, 1);
                wc = self.event_word_id(c, 2);
            }
            let same = wa == ids[0] && wb == ids[1] && wc == ids[2] && sv == server;
            match mode {
                0 if same => ok = false,
                1 if !same => ok = false,
                _ => {}
            }
        }
        ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_and_attr() {
        assert_eq!(clamp_weather(2, 5), 0);
        assert_eq!(clamp_weather(5, 8), 4);
        assert_eq!(clamp_weather(9, 7), 7);
        assert_eq!(field_attr(9, 5), 1);
        assert_eq!(field_attr(10, 4), 5);
        assert_eq!(field_attr(11, 0), 2);
    }
}
