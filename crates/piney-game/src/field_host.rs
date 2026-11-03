//! What the event scripts' host ([`crate::ev_host`]) holds of The World
//! besides the world and the UI: `ccGame`, the faders (`ccMenu`'s and
//! `scFadeDef`), `piros_colour`'s run, the call log, the `area` and `scene`
//! the scripts asked for; the instructions are in docs/engine/events.md.

use std::collections::HashMap;
use std::sync::Arc;

use piney_data::save::{SaveData, offset};
use piney_desktop::anm::Ctx;
use piney_event::host::{Game, StoryArea};
use piney_input::Pad;

use crate::mode::Event;

/// `ccGame.status` in The World.
pub const STATUS_WORLD: i32 = 5;

/// `ccScFade` (fade.cpp): four fading elements (`status`, `cnt`, `tcnt`, two
/// more counts, the rectangle, `col0`, `col1`) and a layer; the events use
/// `ccMenu`'s. `EntryFade` (0x00160400) takes the first free element (its
/// number the event keeps, `eventMng.fadeNum`), `ContinueFade` (0x00160490)
/// goes on from the old colour, `SendPacket` (0x0015fb80) draws and counts
/// each (docs/engine/event-vm.md).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScFade {
    /// +0x00: nothing is sent while it is set.
    pub off: i32,
    pub elems: [FadeElem; 4],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FadeElem {
    pub status: i16,
    pub cnt: i16,
    pub tcnt: i16,
    /// +0x06, +0x08: the chained fades' counts.
    pub tcnt_back: i16,
    pub tcnt_hold: i16,
    pub col0: u32,
    pub col1: u32,
}

/// The menu layer (`ccMenu`'s sprites, its fader).
pub const MENU_LAYER: i16 = 242;

impl ScFade {
    /// `EntryFade`: the element's number, -1 when all four are busy.
    pub fn entry(&mut self, n: i16, col0: u32, col1: u32) -> i32 {
        match self.elems.iter().position(|e| e.status == 0) {
            Some(i) => {
                let e = &mut self.elems[i];
                e.status = 1;
                e.tcnt = n;
                e.cnt = 0;
                e.col0 = col0;
                e.col1 = col1;
                i as i32
            }
            None => -1,
        }
    }

    /// `EntryFlash(n, col, ...)` (0x00160240): status 3, from `col` to
    /// `col` with alpha 0 over `n`; the element's number, -1 when all four
    /// are busy.
    pub fn flash(&mut self, n: i16, col: u32) -> i32 {
        let i = self.entry(n, col, col & 0x00ff_ffff);
        if let Some(e) = usize::try_from(i).ok().and_then(|i| self.elems.get_mut(i)) {
            e.status = 3;
        }
        i
    }

    /// `EntryFlash2(t0, t1, col, ...)` (0x001602d0; MUT 0x001603d0): status
    /// 5, in from `col` with alpha 0 over `t0`, then out over `t1`.
    pub fn flash2(&mut self, t0: i16, t1: i16, col: u32) -> i32 {
        self.chained(5, t0, t1, None, col)
    }

    /// `EntryFlash3(t0, t1, t2, col, ...)` (0x00160360; MUT 0x00160460):
    /// status 9, in over `t0`, held for `t2`, out over `t1`.
    pub fn flash3(&mut self, t0: i16, t1: i16, t2: i16, col: u32) -> i32 {
        self.chained(9, t0, t1, Some(t2), col)
    }

    /// The flashes' element: from `col` with alpha 0 to `col` over `t0`,
    /// the chained counts kept for [`ScFade::count`] (`EntryFlash2` leaves
    /// the hold's as it was).
    fn chained(&mut self, status: i16, t0: i16, back: i16, hold: Option<i16>, col: u32) -> i32 {
        let i = self.entry(t0, col & 0x00ff_ffff, col);
        if let Some(e) = usize::try_from(i).ok().and_then(|i| self.elems.get_mut(i)) {
            e.status = status;
            e.tcnt_back = back;
            if let Some(h) = hold {
                e.tcnt_hold = h;
            }
        }
        i
    }

    /// The event's `fade` (case 145) and `fade_more` (146) on element
    /// `*num` (`eventMng.fadeNum`). From Mutation on (`reset`), `fade`
    /// empties the fader first (`ccScFade::Init`), and `fade_more` with no
    /// element (`*num` outside 0-3) starts a fade from `alpha` to nothing
    /// (MUT SLUS_205.62:0x001c6f50, 0x001c6fc0).
    pub fn event(&mut self, num: &mut i32, count: i16, alpha: i16, more: bool, reset: bool) {
        let col = (alpha as u32 & 0xff) << 24;
        let own = (0..4).contains(num);
        if more && (own || !reset) {
            self.continue_fade(*num, count, col);
            return;
        }
        if reset {
            self.elems.iter_mut().for_each(|e| e.status = 0);
        }
        *num = if more { self.entry(count, col, 0) } else { self.entry(count, 0, col) };
    }

    /// `ContinueFade(i, n, col1)`.
    pub fn continue_fade(&mut self, i: i32, n: i16, col1: u32) {
        let Some(e) = usize::try_from(i).ok().and_then(|i| self.elems.get_mut(i)) else { return };
        e.status = 1;
        e.tcnt = n;
        e.cnt = 0;
        e.col0 = e.col1;
        e.col1 = col1;
    }

    /// `SendPacket`'s counting, after each live element is drawn.
    pub fn count(&mut self) {
        if self.off != 0 {
            return;
        }
        for e in self.elems.iter_mut().filter(|e| e.status & 1 != 0) {
            e.cnt = e.cnt.wrapping_add(1);
            if e.cnt <= e.tcnt {
                continue;
            }
            if e.status & 2 != 0 {
                e.status = 0;
            } else if e.status & 4 != 0 {
                e.status = (e.status & !4) | 2;
                e.cnt = 0;
                e.tcnt = e.tcnt_back;
                e.col0 = e.col1;
                e.col1 &= 0x00ff_ffff;
            } else if e.status & 8 != 0 {
                e.status = (e.status & !8) | 4;
                e.cnt = 0;
                e.tcnt = e.tcnt_hold;
                e.col0 = e.col1;
            } else {
                e.cnt = e.tcnt;
            }
        }
    }

    /// `SendPacket`: draw each live element on `layer`, then count.
    pub fn send(&mut self, ctx: &mut Ctx, layer: i16) {
        if self.off != 0 {
            return;
        }
        for e in self.elems.iter().filter(|e| e.status & 1 != 0) {
            let cnt = u32::try_from(e.cnt).unwrap_or(0);
            let tcnt = u32::try_from(e.tcnt).unwrap_or(0).max(1);
            piney_desktop::fade::draw_on(ctx, layer, e.col0, e.col1, cnt, tcnt);
        }
        self.count();
    }

    /// The alpha of the first live element, for the title and the checks.
    pub fn alpha(&self) -> Option<u8> {
        self.elems.iter().find(|e| e.status & 1 != 0).map(|e| {
            let cnt = u32::try_from(e.cnt).unwrap_or(0);
            let tcnt = u32::try_from(e.tcnt).unwrap_or(0).max(1);
            piney_desktop::fade::colour(e.col0, e.col1, cnt, tcnt)[3]
        })
    }
}

/// `ccGame` (0x88 bytes) as far as the scene goes: +0x14 `area` ...
/// +0x30 `block`, the `*Prev` copies +0x34-+0x48, and the battle state
/// `ChangeScene` resets (+0x58 `inBattle`, +0x5c `inBattleCnt`, +0x60
/// `inBattleDist`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CcGame {
    pub status: i32,
    pub area: i32,
    pub area_prev: i32,
    pub server: i32,
    pub town: i32,
    pub field: i32,
    pub dungeon: i32,
    pub floor: i32,
    pub block: i32,
    pub server_prev: i32,
    pub town_prev: i32,
    pub field_prev: i32,
    pub dungeon_prev: i32,
    pub floor_prev: i32,
    pub block_prev: i32,
    pub in_battle: i32,
    pub in_battle_cnt: i32,
    /// Bits.
    pub in_battle_dist: u32,
    pub cnt_stop: i32,
}

/// `ChangeScene`'s server by town (0x00306dc0, 8 ints).
pub const SERVERS: [i32; 8] = [0, 1, 2, 3, 4, 0, 1, 2];

impl CcGame {
    /// `ccGame::ChangeScene(a, t, fd, d, f, b)` (0x00167380), before its
    /// `ChangeRequest(6, 7)`: each `*Prev` takes the old value and each of
    /// area, town, field, dungeon, floor, block the new one unless it is
    /// below -1; a town also goes into `saveData.lastTown` (+0x8426) and
    /// picks the server (a town of 8 or more would read past the table: not
    /// modelled, the server is kept); the battle state is cleared, the
    /// distance 2200.
    pub fn change_scene(&mut self, save: &mut SaveData, [a, t, fd, d, f, b]: [i32; 6]) {
        self.town_prev = self.town;
        if t >= -1 {
            self.town = t;
            save.set_u8(offset::LAST_TOWN, t as u8);
        }
        self.server_prev = self.server;
        if self.town >= 0
            && let Some(&s) = SERVERS.get(self.town as usize)
        {
            self.server = s;
        }
        for (v, prev, n) in [
            (&mut self.field, &mut self.field_prev, fd),
            (&mut self.dungeon, &mut self.dungeon_prev, d),
            (&mut self.floor, &mut self.floor_prev, f),
            (&mut self.block, &mut self.block_prev, b),
        ] {
            *prev = *v;
            if n >= -1 {
                *v = n;
            }
        }
        self.area_prev = self.area;
        if a >= -1 {
            self.area = a;
        }
        self.in_battle = 0;
        self.in_battle_cnt = 0;
        self.in_battle_dist = 0x4509_8000;
    }

    /// What the scripts read of it.
    pub fn view(&self) -> Game {
        Game {
            status: self.status,
            area: self.area,
            area_prev: self.area_prev,
            server: self.server,
            town: self.town,
            field: self.field,
            dungeon: self.dungeon,
            floor: self.floor,
            block: self.block,
            cnt_stop: self.cnt_stop,
        }
    }
}

/// `ccGame` as an area's scene holds it.
pub fn game_of(scene: &piney_world::area::Scene) -> CcGame {
    CcGame {
        status: STATUS_WORLD,
        area: scene.area,
        area_prev: scene.area_prev,
        server: scene.server,
        town: scene.town,
        field: scene.field,
        dungeon: scene.dungeon,
        floor: scene.floor,
        block: scene.block,
        server_prev: scene.server_prev,
        town_prev: scene.town_prev,
        field_prev: scene.field_prev,
        dungeon_prev: scene.dungeon_prev,
        floor_prev: scene.floor_prev,
        block_prev: scene.block_prev,
        ..CcGame::default()
    }
}

/// A mode change the scripts asked for, which the field does not follow
/// yet: `ccGame::ChangeScene` and the `area` before it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneChange {
    /// area, town, field, dungeon, floor, block.
    pub scene: [i16; 6],
    /// The `area` instruction's story area and its words (the field
    /// `WORLD_MAN::SimGenerateCode` would build), if one came first.
    pub area: Option<(i16, [Option<i32>; 3])>,
}

/// What the runtime answers the scripts with in The World, apart from the
/// world and the UI.
pub struct FieldState {
    /// This frame's pad (`ccSys.pad[0]`).
    pub pad: Pad,
    /// The frame, for the call log.
    pub frame: u64,
    /// `ccGame`: status 5, the scene `ChangeArea(0, lastTown)` set.
    pub game: CcGame,
    /// What the scripts asked of the sound.
    pub events: Vec<Event>,
    /// `ccMenu`'s fader.
    pub fade: ScFade,
    /// `eventMng.fadeNum`.
    pub fade_num: i32,
    /// `scFadeDef`, the screen's own fader (the flashes of
    /// `piros_colour`), on the font layer.
    pub fade_def: ScFade,
    /// A `piros_colour` running ([`crate::piros`]).
    pub piros: Option<crate::piros::Sequence>,
    /// Every host call with its frame.
    pub calls: Vec<(u64, String)>,
    /// `eventAreaInfo` by code.
    pub stories: HashMap<i16, StoryArea>,
    /// The `area` instruction's last request.
    pub area: Option<(i16, [Option<i32>; 3])>,
    /// `ChangeScene`, once asked: the event task sleeps in it.
    pub change: Option<SceneChange>,
    /// The windows opened, (event, message).
    pub shown: Vec<(u16, i16)>,
    /// `DispInfo`'s gate-address and desktop-item lines, as the desktop
    /// composes them (None: those windows open empty).
    pub announcements: Option<Arc<crate::story::Announcements>>,
    /// `ccSystem::SetFrameRate` from the scripts, over the town's own.
    pub frame_rate: Option<u32>,
}

impl FieldState {
    pub fn new(save: &SaveData, stories: HashMap<i16, StoryArea>) -> FieldState {
        let town = i32::from(save.u8(offset::LAST_TOWN) as i8);
        FieldState {
            pad: Pad::default(),
            frame: 0,
            game: {
                // ChangeRequest(5, 8)'s InitScene (every scene field and
                // *Prev -1), then ChangeArea(0, lastTown)'s ChangeScene.
                let mut g = CcGame {
                    status: STATUS_WORLD,
                    area: -1,
                    area_prev: -1,
                    server: -1,
                    town: -1,
                    field: -1,
                    dungeon: -1,
                    floor: -1,
                    block: -1,
                    server_prev: -1,
                    town_prev: -1,
                    field_prev: -1,
                    dungeon_prev: -1,
                    floor_prev: -1,
                    block_prev: -1,
                    ..CcGame::default()
                };
                let mut scratch = save.clone();
                g.change_scene(&mut scratch, [0, town, -1, -1, -1, -1]);
                g
            },
            events: Vec::new(),
            fade: ScFade::default(),
            fade_num: -1,
            fade_def: ScFade::default(),
            piros: None,
            calls: Vec::new(),
            stories,
            area: None,
            change: None,
            shown: Vec::new(),
            announcements: None,
            frame_rate: None,
        }
    }

    pub(crate) fn log(&mut self, s: String) {
        self.calls.push((self.frame, s));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The event's `fade` and `fade_more` with the fader's four elements
    /// busy: Infection's find none (-1) and the `fade_more` after it does
    /// nothing; from Mutation on the fader is emptied first, and a
    /// `fade_more` with no element starts a fade from its alpha to 0.
    #[test]
    fn event_fades_by_volume() {
        let busy = || ScFade { elems: [FadeElem { status: 1, ..FadeElem::default() }; 4], ..ScFade::default() };
        let (mut inf, mut n) = (busy(), 0);
        inf.event(&mut n, 10, 0x80, false, false);
        assert_eq!(n, -1);
        let before = inf.elems;
        inf.event(&mut n, 10, 0x40, true, false);
        assert_eq!(inf.elems, before);

        let (mut later, mut n) = (busy(), 0);
        later.event(&mut n, 10, 0x80, false, true);
        assert_eq!(n, 0);
        assert_eq!((later.elems[0].col0, later.elems[0].col1), (0, 0x8000_0000));
        assert!(later.elems[1..].iter().all(|e| e.status == 0));
        later.event(&mut n, 5, 0x40, true, true);
        assert_eq!((later.elems[0].tcnt, later.elems[0].col0, later.elems[0].col1), (5, 0x8000_0000, 0x4000_0000));
        let mut n = -1;
        later.event(&mut n, 7, 0x40, true, true);
        assert_eq!(n, 0);
        assert_eq!((later.elems[0].tcnt, later.elems[0].col0, later.elems[0].col1), (7, 0x4000_0000, 0));
    }

    fn fixture() -> Option<String> {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/field_host_fixture.txt");
        std::fs::read_to_string(p).ok()
    }

    fn nums(s: &str) -> Vec<i64> {
        s.split_whitespace().map(|t| t.parse().unwrap()).collect()
    }

    fn fade_from(v: &[i64]) -> ScFade {
        let mut f = ScFade { off: v[0] as i32, ..ScFade::default() };
        for (i, e) in f.elems.iter_mut().enumerate() {
            let w = &v[1 + 7 * i..8 + 7 * i];
            *e = FadeElem {
                status: w[0] as i16,
                cnt: w[1] as i16,
                tcnt: w[2] as i16,
                tcnt_back: w[3] as i16,
                tcnt_hold: w[4] as i16,
                col0: w[5] as u32,
                col1: w[6] as u32,
            };
        }
        f
    }

    /// `EntryFade`, `ContinueFade`, `EntryFlash`, `EntryFlash2`,
    /// `EntryFlash3` and `SendPacket`'s counting against the game's
    /// (tools/test_field_host.py): every fader after every call.
    #[test]
    fn the_fader_counts_as_the_game_does() {
        let Some(text) = fixture() else { return };
        let mut fade = ScFade::default();
        let mut n = 0;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("fade ") {
                fade = fade_from(&nums(rest));
                continue;
            }
            let Some(rest) = line.strip_prefix("op ") else { continue };
            let (op, want) = rest.split_once(" -> ").unwrap();
            let want = nums(want);
            let mut w = op.split_whitespace();
            let r = match w.next().unwrap() {
                "entry" => {
                    let v: Vec<i64> = w.map(|t| t.parse().unwrap()).collect();
                    i64::from(fade.entry(v[0] as i16, v[1] as u32, v[2] as u32))
                }
                "continue" => {
                    let v: Vec<i64> = w.map(|t| t.parse().unwrap()).collect();
                    fade.continue_fade(v[0] as i32, v[1] as i16, v[2] as u32);
                    0
                }
                "flash" => {
                    let v: Vec<i64> = w.map(|t| t.parse().unwrap()).collect();
                    i64::from(fade.flash(v[0] as i16, v[1] as u32))
                }
                "flash2" => {
                    let v: Vec<i64> = w.map(|t| t.parse().unwrap()).collect();
                    i64::from(fade.flash2(v[0] as i16, v[1] as i16, v[2] as u32))
                }
                "flash3" => {
                    let v: Vec<i64> = w.map(|t| t.parse().unwrap()).collect();
                    i64::from(fade.flash3(v[0] as i16, v[1] as i16, v[2] as i16, v[3] as u32))
                }
                _ => {
                    fade.count();
                    0
                }
            };
            assert_eq!(r, want[0], "{line}");
            assert_eq!(fade, fade_from(&want[1..]), "{line}");
            n += 1;
        }
        assert!(n > 500, "{n} calls");
    }

    /// `ccGame::ChangeScene` against the game's: every field of `ccGame`
    /// and `lastTown`.
    #[test]
    fn change_scene_as_the_game_does() {
        let Some(text) = fixture() else { return };
        let mut n = 0;
        for line in text.lines() {
            let Some(rest) = line.strip_prefix("scene ") else { continue };
            let (args, want) = rest.split_once(" -> ").unwrap();
            let (a, want) = (nums(args), nums(want));
            let w = |k: usize| a[k] as i32;
            let mut g = CcGame {
                status: w(0),
                area: w(5),
                area_prev: w(6),
                server: w(7),
                town: w(8),
                field: w(9),
                dungeon: w(10),
                floor: w(11),
                block: w(12),
                server_prev: w(13),
                town_prev: w(14),
                field_prev: w(15),
                dungeon_prev: w(16),
                floor_prev: w(17),
                block_prev: w(18),
                in_battle: w(22),
                in_battle_cnt: w(23),
                in_battle_dist: w(24) as u32,
                cnt_stop: w(29),
            };
            let mut save = SaveData::new();
            save.set_u8(offset::LAST_TOWN, a[34] as u8);
            g.change_scene(&mut save, [35, 36, 37, 38, 39, 40].map(w));
            let got = [
                g.status,
                g.area,
                g.area_prev,
                g.server,
                g.town,
                g.field,
                g.dungeon,
                g.floor,
                g.block,
                g.server_prev,
                g.town_prev,
                g.field_prev,
                g.dungeon_prev,
                g.floor_prev,
                g.block_prev,
                g.in_battle,
                g.in_battle_cnt,
                g.in_battle_dist as i32,
                g.cnt_stop,
            ];
            let idx = [0, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 22, 23, 24, 29];
            let want_fields: Vec<i32> = idx.iter().map(|&k| want[k] as i32).collect();
            assert_eq!(got.to_vec(), want_fields, "{line}");
            assert_eq!(i64::from(save.u8(offset::LAST_TOWN) as i8), want[34], "{line}");
            // Nothing else of ccGame moves.
            for (k, (&x, &y)) in a[..34].iter().zip(&want[..34]).enumerate() {
                if !idx.contains(&k) {
                    assert_eq!(x, y, "ccGame word {k}: {line}");
                }
            }
            n += 1;
        }
        assert!(n >= 120, "{n} scenes");
    }
}
