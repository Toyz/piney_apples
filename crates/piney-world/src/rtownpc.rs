//! The walking PCs of a Root Town: `ccRtownPC` (gcmn `rtownnpc.cpp`,
//! 0x00506640-0x0050933c, 0x2f0 bytes over `ccGimmick`), the other "players"
//! wandering Mac Anu between its landmarks. Who: [`register_random_npc`]
//! (`ccRegisterRandomNpc`, main 0x001b7660) off the game's MT19937. Where:
//! `ccSetRtownPC` from [`Tables::mark_pos`], built by [`RtownPc::place`]. Each
//! frame [`RtownPc::routine`] and [`RtownPc::main`] (acts in `normalMode`,
//! routes by [`RtownPc::route_move`]); what it asks of the game comes out as
//! [`PcEvent`]s (docs/engine/field-game.md, "The walking PCs").

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::tables::world;
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;

use crate::body::{Body, TRALL};
use crate::camera::Cam;
use crate::char::{Char, w2p};
use crate::ee::{self, F, ONE, V4, add, atan2f, cosf, deg2rad, dot, lt, mul, rad2deg, sinf, sqrtf, sub, vadd, vsub};
use crate::entry::{Npc, NpcCtx};
use crate::hit::{self, Hits, WALL_MASK};
use crate::mt::Mt;
use crate::navi::{Navi, NaviMap};
use crate::npc::NpcRow;
use piney_event::host::NpcCommand;

/// `ccLandHitCheck`'s mask for entries: floors with bit 1 (every floor of
/// Mac Anu has it).
pub const ENTRY_LAND_MASK: u32 = 0x2000_0002;
/// The kind of a walking PC's body (`bodyHit.type`).
pub const KIND_PC: u32 = 2;
/// Ids of the PCs' bodies in the character list: this plus the order they
/// were placed in.
pub const BODY_ID: u32 = 0x100;

/// Distances of `normalMode` and `move`.
const SHOW: F = 0x4589_8000; // 4400
const SHOW_NEAR: F = 0x4583_4000; // 4200
const ARRIVED: F = 0x42c8_0000; // 100
const NEAR_KITE: F = 0x4396_0000; // 300, the walls-only mask
const CHAT_NEAR: F = 0x42c8_0000; // 100: says hello
const WALK_SPD: F = 0x4128_0000; // 10.5
const EV_WALK_SPD: F = 0x4060_0000; // 3.5
const EV_ARRIVED: F = 0x42a0_0000; // 80
const FADE_STEP: F = 0x3ca3_d70a; // 0.02
const HALF: F = 0x3f00_0000;
const PI_2: F = 0x3fc9_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const MINUS_ONE: F = 0xbf80_0000;
const DISP: F = 0x45da_c000; // 7000
const FREEZE: F = 0x461c_4000; // 10000
/// Most PCs shown at once (`eventMng+0x18`).
const MAX_SHOWN: i32 = 10;
/// The gate's landmark in Mac Anu: act 7.
const GATE_LANDMARK: i8 = 44;

/// `rtownnpc.cpp`'s tables, the volume's (`tables::world`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tables {
    /// `rtpcCcsName[49]`: the files by `ccsType`.
    pub ccs_names: Vec<String>,
    /// `rtpcWeaponName[98]` (rows 30-127), `tvpcWeaponName[9]` (159-167).
    pub weapon_names: Vec<String>,
    pub tvpc_weapon_names: Vec<String>,
    /// `rtpcAnmTbl[49]`: by `ccsType`, standing, walking, running (and for
    /// `ctw2` four more).
    pub anm_tbls: Vec<Vec<String>>,
    /// `rtpcChatTbl[4]`: what a PC says passing Kite.
    pub chat: [&'static str; 4],
    /// `rtpcChatMesShop[6][4]`: leaving a shop, by shop.
    pub chat_shop: [[&'static str; 4]; 6],
    /// `rtpcChatMes[4][13]`: a chat group's conversations.
    pub chat_mes: [[&'static str; 13]; 4],
    /// `markPosTbl[16]` -> `markerTbl[5 towns][4]`: start, first target,
    /// chat group, place in it.
    pub mark_pos: [[[i8; 4]; 5]; 16],
    /// `merchanNum[5 towns][5]`: each merchant's landmark.
    pub merchan_num: [[i8; 5]; 5],
}

impl Tables {
    pub fn of(volume: Volume) -> Tables {
        let w = world::of(volume);
        let names = |t: &[Option<&str>]| t.iter().map(|s| s.unwrap_or_default().to_owned()).collect::<Vec<_>>();
        let text = |t: &[Option<&'static str>], k: usize| t.get(k).copied().flatten().unwrap_or_default();
        let rows = |t: &'static [&'static [Option<&'static str>]], s: usize| t.get(s).copied().unwrap_or_default();
        Tables {
            ccs_names: names(w.rtpc_ccs_names()),
            weapon_names: names(w.rtpc_weapon_names()),
            tvpc_weapon_names: names(w.tvpc_weapon_names()),
            anm_tbls: w.rtpc_anms().iter().map(|r| names(r)).collect(),
            chat: std::array::from_fn(|k| text(w.rtpc_chat(), k)),
            chat_shop: std::array::from_fn(|s| std::array::from_fn(|k| text(rows(w.rtpc_chat_shop(), s), k))),
            chat_mes: std::array::from_fn(|s| std::array::from_fn(|k| text(rows(w.rtpc_chat_mes(), s), k))),
            mark_pos: std::array::from_fn(|m| w.mark_pos().get(m).copied().unwrap_or_default()),
            merchan_num: w.merchan_num(),
        }
    }

    /// The weapons `ccRtownPC::ccRtownPC` hangs on a PC of `npcTbl` row
    /// `id` drawn from file type `ccs_type`: (hand node, weapon file, model).
    pub fn weapons(&self, ccs_type: i32, id: i32) -> Vec<(&'static str, String, String)> {
        const L: &str = "OBJ_t0 l hand";
        const R: &str = "OBJ_t0 r hand";
        let rt = usize::try_from(id - 30).ok().and_then(|k| self.weapon_names.get(k)).cloned().unwrap_or_default();
        let tv =
            usize::try_from(id - 159).ok().and_then(|k| self.tvpc_weapon_names.get(k)).cloned().unwrap_or_default();
        let one = |hand: &'static str, w: &String| vec![(hand, w.clone(), format!("MDL_{w}"))];
        let two = |w: &String| vec![(L, w.clone(), format!("MDL_{w}l")), (R, w.clone(), format!("MDL_{w}r"))];
        match ccs_type {
            2 => one(L, &rt),
            9 | 10 => two(&rt),
            k if k < 32 => {
                if id >= 128 || rt.is_empty() {
                    Vec::new()
                } else {
                    one(R, &rt)
                }
            }
            39 | 40 => two(&tv),
            k if k < 41 => one(R, &tv),
            48 => one(R, &rt),
            _ => Vec::new(),
        }
    }
}

/// `ccRegisterRandomNpc(reserved)` (main 0x001b7660) in a town: the rows of
/// the walking PCs, by slot (-1 unused).
pub fn register_random_npc(mt: &mut Mt, reserved: i32) -> [i16; 16] {
    let mut rows = [-1i16; 16];
    let n = 16 - reserved;
    if n <= 0 {
        return rows;
    }
    let mut used = [false; 50];
    for slot in rows.iter_mut().take(n as usize) {
        let mut r = (mt.rand() % 50).unsigned_abs() as usize;
        while used[r] {
            r = if r + 1 < 50 { r + 1 } else { 0 };
        }
        used[r] = true;
        *slot = r as i16 + 30;
    }
    rows
}

/// `ccGetDirc(a, b)` (main 0x001d9ce0): the heading from `a` to `b` (0
/// faces -y), in (-pi, pi].
pub fn get_dirc(a: V4, b: V4) -> F {
    let dx = sub(b[0], a[0]);
    let dy = sub(b[1], a[1]);
    let mut r = add(PI_2, atan2f(dy, dx));
    if !ee::le(r, ee::PI) {
        r = sub(r, TWO_PI);
    }
    if lt(r, ee::neg(ee::PI)) {
        r = add(r, TWO_PI);
    }
    r
}

/// `ccGetDirc0(a, b)` (0x001d9c50): the same through the 16-bit angle.
pub fn get_dirc0(a: V4, b: V4) -> F {
    let dx = sub(b[0], a[0]);
    let dy = sub(b[1], a[1]);
    let s = rad2deg(atan2f(dy, dx));
    deg2rad((i32::from(s) + 16384) as i16)
}

/// `ccGetDist3D(a, b)` (0x001d9e40).
pub fn get_dist3d(a: V4, b: V4) -> F {
    let d = vsub(b, a);
    sqrtf(dot(d, d))
}

/// `ccGetDircChg(from, to, mode)` (0x001d9eb0): the turn toward `to`, the
/// short way; with a mode below 0x10000, `(to - from) / (mode / 16)`, at
/// least 1 (64: a quarter of the way), at most 24576.
pub fn get_dirc_chg(from: i16, to: i16, mode: i32) -> i32 {
    let d = i32::from(to) - i32::from(from);
    if d == 0 {
        return 0;
    }
    let mut x = i64::from((d & 0xffff) << 4);
    let neg = x >= 0x8_0001;
    if neg {
        x = 0x10_0000 - x;
    }
    if mode & 0xf_0000 == 0 {
        x /= i64::from(mode & 0xffff);
        if x == 0 {
            x = 1;
        }
    }
    if x >= 24577 {
        x = 24576;
    }
    if neg { -x as i32 } else { x as i32 }
}

/// `ccSetDirc(&r, to, mode)` (0x001da0b0): `r` turned toward `to`.
pub fn set_dirc(r: &mut F, to: F, mode: i32) {
    let s = rad2deg(*r);
    let chg = get_dirc_chg(s, rad2deg(to), mode);
    *r = deg2rad((i32::from(s) + chg) as i16);
}

/// `ccCheckCameraDeg(pos, deg)` (0x001da710): `pos` within `deg` of where
/// the active camera looks, in the plane, all taken relative to the player.
pub fn check_camera_deg(pos: V4, deg: i16, cam: &Cam, player: V4) -> bool {
    let c = w2p(cam.pos, player);
    let v = w2p(cam.view, player);
    let p = w2p(pos, player);
    let a = rad2deg(atan2f(sub(p[1], c[1]), sub(p[0], c[0])));
    let b = rad2deg(atan2f(sub(v[1], c[1]), sub(v[0], c[0])));
    let x = (i32::from(deg) + (i32::from(a) - i32::from(b))) as i16;
    x > 0 && i32::from(x) < 2 * i32::from(deg)
}

/// What a PC's frame asks of the rest of the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcEvent {
    /// `ccChatMsg::OpenChat(ccChat, pc, text)`: a chat bubble over it.
    Chat(&'static str),
    /// `effTransfer(pc)`: the gate's transfer effect.
    Transfer,
    /// `rtpcCheckNote` (gcmn 0x00509160) on a step note (1 or 2):
    /// `ccSeSetParamPC(param, pc, ccsType)`, the step's sound where the PC
    /// stands on ground `attribute`, and `ccEffPawSmoke(pc, +0x2c0)` at its
    /// feet (none when the model has no foot nodes), heading `dirc_z`.
    Step { param: u32, ccs_type: i32, pos: V4, attribute: u32, feet: Option<[V4; 2]>, dirc_z: F, speed: F },
}

/// The town's walking PCs' shared state: the globals of `rtownnpc.cpp` and
/// what they read of the rest of the game.
#[derive(Clone, Debug)]
pub struct TownPcs {
    pub tables: Tables,
    pub map: NaviMap,
    /// `game->area` (0) and `game->town` (0 Mac Anu).
    pub area: i32,
    pub town: i32,
    /// `DMY_merchant1`-`5` in the town's file (`whatShop`).
    pub merchants: [Option<V4>; 5],
    /// The game's `ccRand`.
    pub mt: Mt,
    /// `eventMng+0x18`: PCs shown.
    pub shown: i32,
    /// `eventMng+0x78c`: an event holds back the greetings.
    pub chat_hold: bool,
    /// `rtpcChatNum`, `rtpcChatCnt[3]`, `chatMesRandTbl` (which of
    /// `rtpcChatMes`), `talkFlagChatPC` (a chat group member is spoken to).
    pub chat_num: i32,
    pub chat_cnt: [i32; 3],
    pub chat_mes: usize,
    pub talk_flag: bool,
    /// PCs placed so far: each body's id in the character list is
    /// [`BODY_ID`] plus its number.
    pub placed: u32,
}

impl TownPcs {
    /// The state `ccThEntryCtrl` starts with in town `town` of the file
    /// `town_file` (its landmarks and merchants' dummies), `ccRand` as
    /// `mt` leaves it.
    pub fn new(volume: Volume, town: i32, town_file: &SceneFile, mt: Mt) -> Result<TownPcs> {
        let dummy = |name: &str| {
            town_file
                .ccs
                .find_object(name)
                .and_then(|o| town_file.scene.dummies.get(&o))
                .map(|d| [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE])
        };
        Ok(TownPcs {
            tables: Tables::of(volume),
            map: NaviMap::read(volume, town as usize, town_file)?,
            area: 0,
            town,
            merchants: std::array::from_fn(|k| dummy(&format!("DMY_merchant{}", k + 1))),
            mt,
            shown: 0,
            chat_hold: false,
            chat_num: 0,
            chat_cnt: [0; 3],
            chat_mes: 0,
            talk_flag: false,
            placed: 0,
        })
    }

    /// The same outside the towns (area 1 a field, 2 a dungeon), where
    /// only events place PCs: no landmarks, no merchants' dummies.
    pub fn outside(volume: Volume, area: i32, town: i32, mt: Mt) -> Result<TownPcs> {
        Ok(TownPcs {
            tables: Tables::of(volume),
            map: NaviMap::default(),
            area,
            town,
            merchants: [None; 5],
            mt,
            shown: 0,
            chat_hold: false,
            chat_num: 0,
            chat_cnt: [0; 3],
            chat_mes: 0,
            talk_flag: false,
            placed: 0,
        })
    }

    fn landmark(&self, num: i8, into: &mut V4) {
        if let Some(p) = self.map.landmark_pos(i32::from(num)) {
            *into = p;
        }
    }
}

/// A walking PC: `ccRtownPC` over `ccEntryObj` over `ccChar`.
#[derive(Clone)]
pub struct RtownPc {
    pub town: Rc<RefCell<TownPcs>>,
    /// Its `npcTbl` row (base parameters and entry).
    pub row: NpcRow,
    /// The model, position (+0x40), `posP`, heading, animation,
    /// transparency, ground attribute and `bodyHit` (+0x160).
    pub char: Char,
    /// `ccEntryObj` +0xe0: `objFlag`, `freezeFlag`, `dispSW`, `cmndFlag`.
    pub obj_flag: bool,
    pub freeze: bool,
    pub disp_sw: bool,
    pub cmnd_flag: bool,
    /// On the command list (`ccEntryCmnd`): it can be targeted and spoken to.
    pub listed: bool,
    /// +0xe4 `plDist` (in the plane), +0xe8 `plDirc` (the heading facing
    /// Kite).
    pub pl_dist: F,
    pub pl_dirc: F,
    /// +0xf0 `fadeFlag`, +0xf2 `fadeCnt`, +0xf4 `grotDeg`, +0xf6 `grotSpd`
    /// (events').
    pub fade_flag: i16,
    pub fade_cnt: i16,
    pub grot_deg: i16,
    pub grot_spd: i16,
    /// +0x100 `entParam.pos`.
    pub ent_pos: V4,
    /// +0x1e0 `thinkMode`: 0 its own (`normalMode`), 1 an event's.
    pub think_mode: u8,
    /// +0x1f0.
    pub navi: Navi,
    /// +0x260 `nextPos` (the landmark walked to), +0x270 `oldPos`, +0x280
    /// `evNextPos`.
    pub next_pos: V4,
    pub old_pos: V4,
    pub ev_next_pos: V4,
    /// +0x290 `chatGp`: `gpID`, `gpNum`.
    pub chat_gp: (i16, i16),
    /// +0x294 `transrate`, +0x298 `merchanDeg`.
    pub transrate: F,
    pub merchan_deg: F,
    /// +0x29c `npcNum` (the marker), +0x2a0 `ccsType`.
    pub npc_num: i32,
    pub ccs_type: i32,
    /// +0x2a4 `actNum`, +0x2a6 `actProcess`, +0x2a8 `evActNum`, +0x2aa
    /// `evActProcess`, +0x2ac `actCnt`.
    pub act_num: i16,
    pub act_process: i16,
    pub ev_act_num: i16,
    pub ev_act_process: i16,
    pub act_cnt: i32,
    /// +0x2b0 `bodyHitFlag`, +0x2b2 `bodyHitCnt`, +0x2b4 `posCnt`, +0x2b6
    /// `changeRouteCnt`, +0x2b8 `chatCnt`, +0x2ba `shopNum`.
    pub body_hit_flag: i16,
    pub body_hit_cnt: i16,
    pub pos_cnt: i16,
    pub change_route_cnt: i16,
    pub chat_cnt: i16,
    pub shop_num: i16,
    /// +0x2bc `dist` (|posP| as last measured), +0x2c0 `walkSpd`.
    pub dist: F,
    pub walk_spd: F,
    /// +0x2c4 `markPos[4]`, +0x2c8 `targetNum`, `targetNumOld`,
    /// `targetNumStart`.
    pub mark_pos: [i8; 4],
    pub target_num: i8,
    pub target_num_old: i8,
    pub target_num_start: i8,
    /// +0x2cc `anmTbl`.
    pub anm_tbl: Vec<String>,
    /// +0x2d0 `thinkType` (1 in a chat group), +0x2d1 `changeRouteFlag`.
    pub think_type: u8,
    pub change_route_flag: u8,
    /// The weapons on its hands: (hand, model).
    pub weapons: Vec<(String, String)>,
    /// `changeTEX`: the texture `MAT_tex` takes (`ccClump::ChangeTex`; the
    /// draw layer has no texture swap yet, so it draws with its own).
    pub tex_swap: Option<String>,
    /// What this frame asked for.
    pub events: Vec<PcEvent>,
}

/// A PC's model as `ccRtownPC::ccRtownPC` builds it: the file's `CMP_trall`,
/// the weapons on its hands, the texture swap (`changeTEX`).
pub struct PcModel {
    pub body: Rc<Body>,
    pub ccs_type: i32,
    pub weapons: Vec<(String, String)>,
    pub tex_swap: Option<String>,
}

/// Files read so far, by stem.
pub type Files = std::collections::HashMap<String, Rc<SceneFile>>;

fn file(archive: &Arc<Archive>, files: &mut Files, stem: &str) -> Result<Rc<SceneFile>> {
    if let Some(f) = files.get(stem) {
        return Ok(f.clone());
    }
    let f = Rc::new(SceneFile::read(archive, stem)?);
    files.insert(stem.to_string(), f.clone());
    Ok(f)
}

/// `ccRtownPC::ccRtownPC`'s model for `row`: the file, the weapons by its
/// `ccsType`, the palette swap; `files` keeps what was read.
pub fn load_model(archive: &Arc<Archive>, files: &mut Files, tables: &Tables, row: &NpcRow) -> Result<PcModel> {
    let stem = row.stem();
    let ccs_type = tables
        .ccs_names
        .iter()
        .position(|n| *n == stem)
        .ok_or_else(|| Error::NotFound(format!("{stem}: not a walking PC's file")))? as i32;
    let mut body = Body::of(file(archive, files, &stem)?, TRALL)?;
    let mut weapons = Vec::new();
    for (hand, wfile, model) in tables.weapons(ccs_type, i32::from(row.id)) {
        let Ok(w) = file(archive, files, &wfile.to_ascii_lowercase()) else { continue };
        if body.attach(hand, w, &model) {
            weapons.push((hand.to_string(), model));
        }
    }
    let tex_swap = (!row.clut.is_empty()).then(|| row.clut.clone());
    // changeTEX (gcmn 0x005076c0): the clump duplicated and `MAT_tex`
    // given the row's texture (`ccClump::ChangeTex`).
    if let Some(name) = &tex_swap {
        let ccs = &body.file.ccs;
        if let (Some(mat), Some(tex)) = (ccs.find_object("MAT_tex"), ccs.find_object(name)) {
            body.tex_swaps.push((mat, tex));
        }
    }
    Ok(PcModel { body: Rc::new(body), ccs_type, weapons, tex_swap })
}

impl RtownPc {
    /// `ccSetRtownPC(row, marker)`, `ccEntryCtrl::entryObject`,
    /// `ccRtownPC::ccRtownPC` and `ccEntryCtrl::initObject`: the PC on its
    /// start landmark (or, with a negative marker, at the origin for an
    /// event to place), its body in the character list; Kite at `player`.
    pub fn place(
        town: &Rc<RefCell<TownPcs>>,
        row: &NpcRow,
        model: &PcModel,
        marker: i32,
        player: V4,
        hits: &mut Hits,
    ) -> RtownPc {
        let mut t = town.borrow_mut();
        // ccEntryParamClear: pos and dirc (0, 0, 0, 1).
        let mut pos = ee::VF0;
        let mark_pos = usize::try_from(marker)
            .ok()
            .and_then(|m| t.tables.mark_pos.get(m))
            .and_then(|m| m.get(t.town as usize))
            .copied()
            .unwrap_or([0; 4]);
        if marker >= 0 {
            t.landmark(mark_pos[0], &mut pos);
        }
        // entryObject, one object, entParam+0x44 -1: on the ground.
        pos[2] = hits.land(pos, ENTRY_LAND_MASK);
        let anm_tbl = t.tables.anm_tbls.get(model.ccs_type as usize).cloned().unwrap_or_default();
        let height = row.height;
        let width = row.width;
        let first = anm_tbl.get(2).cloned().unwrap_or_default();
        let mut ch = Char::new(model.body.clone(), &first, pos, ee::VF0, height, width)
            .or_else(|| {
                let anim = model.body.file.anims.first()?.object;
                let name = model.body.file.ccs.object_name(anim)?.to_string();
                Char::new(model.body.clone(), &name, pos, ee::VF0, height, width)
            })
            .expect("a walking PC's file has animations");
        ch.dirc = [0, 0, 0, ONE];
        ch.hit = hit::Body {
            pos,
            radius: 0x420c_0000, // 35
            height: 0x42f0_0000, // 120
            mask2: WALL_MASK,
            kind: KIND_PC,
            id: BODY_ID + t.placed,
            ..hit::Body::default()
        };
        t.placed += 1;
        let mut pc = RtownPc {
            town: town.clone(),
            row: row.clone(),
            char: ch,
            obj_flag: false,
            freeze: false,
            disp_sw: true,
            cmnd_flag: false,
            listed: false,
            pl_dist: 0,
            pl_dirc: 0,
            fade_flag: 0,
            fade_cnt: 0,
            grot_deg: 0,
            grot_spd: 0,
            ent_pos: pos,
            think_mode: 0,
            navi: Navi::default(),
            next_pos: [0; 4],
            old_pos: pos,
            ev_next_pos: [0; 4],
            chat_gp: (0, 0),
            transrate: ONE,
            merchan_deg: 0,
            npc_num: marker,
            ccs_type: model.ccs_type,
            act_num: 3,
            act_process: 0,
            ev_act_num: 0,
            ev_act_process: 0,
            act_cnt: 0,
            body_hit_flag: 0,
            body_hit_cnt: 0,
            pos_cnt: 0,
            change_route_cnt: 0,
            chat_cnt: 0,
            shop_num: -1,
            dist: 0,
            walk_spd: WALK_SPD,
            mark_pos,
            target_num: 0,
            target_num_old: 0,
            target_num_start: 0,
            anm_tbl,
            think_type: 0,
            change_route_flag: 0,
            weapons: model.weapons.clone(),
            tex_swap: model.tex_swap.clone(),
            events: Vec::new(),
        };
        if marker >= 0 {
            pc.target_num = mark_pos[1];
            pc.target_num_old = pc.target_num;
            pc.target_num_start = mark_pos[0];
            pc.think_type = 0;
            if pc.target_num != -1 {
                pc.act_num = 6;
            } else {
                pc.act_num = 5;
                let a = deg2rad(((marker << 14) & 0xffff) as u16 as i16);
                let p = &mut pc.char.pos;
                p[0] = add(p[0], mul(0x4348_0000, sinf(a)));
                p[1] = sub(p[1], mul(0x4348_0000, cosf(a)));
                t.landmark(mark_pos[0], &mut pc.next_pos);
                pc.think_type = 1;
                pc.chat_gp = (i16::from(mark_pos[2]), i16::from(mark_pos[3]));
                if let Some(c) = t.chat_cnt.get_mut(pc.chat_gp.0 as usize) {
                    *c = 0;
                }
                if pc.chat_gp.1 == 0 {
                    t.chat_mes = (t.mt.rand() & 3) as usize;
                    t.talk_flag = false;
                }
            }
            pc.think_mode = 0;
            pc.char.dirc[2] = get_dirc(pc.char.pos, pc.next_pos);
        } else {
            pc.think_mode = 1;
            pc.ev_act_num = 2;
        }
        // deleteCmnd(1): off the list, and the entry control leaves it off.
        pc.cmnd_flag = true;
        pc.listed = false;
        pc.ent_pos = pc.char.pos;
        // initObject: in its area; the body in the list; on the ground.
        pc.obj_flag = true;
        hits.set_hit_sw(&mut pc.char.hit, true);
        let mut p = pc.char.pos;
        p[2] = hits.land(p, ENTRY_LAND_MASK);
        pc.ent_pos = p;
        pc.char.pos = p;
        pc.char.pos_p = w2p(p, player);
        pc
    }

    fn set_anm(&mut self, k: usize) {
        if let Some(name) = self.anm_tbl.get(k).cloned() {
            self.char.set_anim(&name);
        }
    }

    fn entry_cmnd(&mut self) {
        self.listed = true;
    }

    fn delete_cmnd_list(&mut self) {
        self.listed = false;
    }

    /// `ccEntryObj::deleteCmnd(flg)`.
    fn delete_cmnd(&mut self, flg: bool) {
        if !self.cmnd_flag {
            self.listed = false;
            self.cmnd_flag = flg;
        }
    }

    /// `ccEntryObj::routine` (gcmn 0x0042fa60) for Kite at `player`.
    pub fn routine(&mut self, player: V4) {
        if self.grot_spd != 0 {
            let mut d = self.char.dirc;
            let to = deg2rad(self.grot_deg);
            set_dirc(&mut d[2], to, i32::from(self.grot_spd));
            self.char.dirc = d;
            if f64::from(ee::f(sub(d[2], to))).abs() < f64::from_bits(0x3f68_9374_c000_0000) {
                self.grot_spd = 0;
            }
        }
        let p = w2p(self.char.pos, player);
        let v = [mul(p[0], MINUS_ONE), mul(p[1], MINUS_ONE), 0, ONE];
        self.pl_dist = sqrtf(dot(v, v));
        let mut a = add(PI_2, atan2f(v[1], v[0]));
        if !ee::le(a, ee::PI) {
            a = sub(a, TWO_PI);
        }
        if lt(a, ee::neg(ee::PI)) {
            a = add(a, TWO_PI);
        }
        self.pl_dirc = a;
        self.disp_sw = ee::le(self.pl_dist, DISP);
        if !ee::le(self.pl_dist, FREEZE) {
            self.freeze = true;
            // entParam+0x40 is -1 for a town's PCs.
            self.delete_cmnd(false);
        } else {
            self.freeze = false;
            if !self.cmnd_flag {
                self.entry_cmnd();
            }
        }
        match self.fade_flag {
            1 => {
                let mut t = add(self.char.set_transparency, ee::div(ONE, ee::from_int(i32::from(self.fade_cnt))));
                if !ee::le(t, ONE) {
                    t = ONE;
                    self.fade_flag = 0;
                }
                self.char.transparency = t;
                self.char.set_transparency = t;
            }
            2 => {
                let mut t = sub(self.char.set_transparency, ee::div(ONE, ee::from_int(i32::from(self.fade_cnt))));
                if lt(t, 0) {
                    t = 0;
                    self.fade_flag = 0;
                }
                self.char.transparency = t;
                self.char.set_transparency = t;
            }
            _ => {}
        }
    }

    /// `ccRtownPC::main` (gcmn 0x00507c20).
    pub fn main(&mut self, ctx: &mut NpcCtx) {
        self.char.pos_p = w2p(self.char.pos, ctx.player);
        let hidden = if self.think_mode == 0 { self.normal_mode(ctx) } else { self.event_mode(ctx) };
        if hidden {
            self.char.drawn = false;
            return;
        }
        self.char.hit.mask2 = if lt(self.pl_dist, NEAR_KITE) { WALL_MASK } else { 1 };
        self.char.hit.pos = self.char.pos;
        if ctx.hits.collision_detection(&mut self.char.hit) != 0 {
            if self.think_mode == 0 {
                let off = self.char.hit.offset;
                self.char.pos = vadd(self.char.pos, off);
                self.char.pos_p = vadd(self.char.pos_p, off);
                self.char.hit.pos = self.char.pos;
                ctx.hits.sync(&self.char.hit);
            }
            self.body_hit_flag = 1;
            self.body_hit_cnt = self.body_hit_cnt.wrapping_add(1);
        } else {
            self.body_hit_flag = 0;
            self.body_hit_cnt = 0;
        }
        if check_camera_deg(self.char.pos, 12288, ctx.cam, ctx.player) {
            // _AnimateForward, then NoteProcess through rtpcCheckNote.
            let (_, notes) = self.char.play.forward_notes(&self.char.body.file);
            for (event, param) in notes {
                if matches!(event, 1 | 2) {
                    let c = &self.char;
                    let worlds = c.body.worlds(&c.play, c.root());
                    let foot = |name: &str| {
                        let t = worlds.get(&c.body.file.ccs.find_object(name)?)?.w_axis;
                        Some([t.x.to_bits(), t.y.to_bits(), t.z.to_bits(), ONE])
                    };
                    let feet = foot("OBJ_t0 l foot").zip(foot("OBJ_t0 r foot")).map(|(l, r)| [l, r]);
                    self.events.push(PcEvent::Step {
                        param,
                        ccs_type: self.ccs_type,
                        pos: c.pos,
                        attribute: c.hit_attribute,
                        feet,
                        dirc_z: c.dirc[2],
                        speed: self.walk_spd,
                    });
                }
            }
            self.char.transparency = self.transrate;
            self.char.set_transparency = self.transrate;
            self.char.fade(&ctx.view);
        } else {
            self.char.drawn = false;
        }
    }

    /// |posP|: the distance to Kite in the plane and the PC's own height.
    fn pos_p_len(&self) -> F {
        sqrtf(dot(self.char.pos_p, self.char.pos_p))
    }

    /// `ccRtownPC::normalMode` (0x00507de0): true when the PC is hidden this
    /// frame (nothing collides or draws).
    fn normal_mode(&mut self, ctx: &mut NpcCtx) -> bool {
        let town = self.town.clone();
        match self.act_num {
            0 => {
                let t = town.borrow();
                if t.shown < MAX_SHOWN {
                    let d = self.pos_p_len();
                    self.dist = d;
                    if lt(d, SHOW) {
                        self.act_num = 1;
                        t.landmark(self.mark_pos[0], &mut self.char.pos);
                        drop(t);
                        self.change_route(false, ctx.hits);
                    }
                }
                true
            }
            1 => {
                let shown = town.borrow().shown;
                if shown < MAX_SHOWN {
                    let d = self.pos_p_len();
                    if lt(d, SHOW) && !ee::le(d, SHOW_NEAR) {
                        if lt(self.dist, d) {
                            self.dist = d;
                            return true;
                        }
                        self.act_num = 3;
                        self.act_process = 0;
                        ctx.hits.hit_enable(&mut self.char.hit);
                        town.borrow_mut().shown += 1;
                        self.entry_cmnd();
                        self.transrate = ONE;
                        return false;
                    }
                }
                self.act_num = 0;
                true
            }
            6 => {
                let mut goal = [0; 4];
                town.borrow().landmark(self.target_num, &mut goal);
                self.route_search(goal, ctx.hits);
                self.navi.landmark += 1;
                self.destination();
                self.act_num = 3;
                self.entry_cmnd();
                ctx.hits.hit_enable(&mut self.char.hit);
                town.borrow_mut().shown += 1;
                false
            }
            2 => {
                if self.act_process == 0 {
                    self.set_anm(0);
                    self.act_process += 1;
                }
                set_dirc(&mut self.char.dirc[2], self.pl_dirc, 64);
                false
            }
            3 => match self.act_process {
                0 => {
                    self.set_anm(2);
                    self.act_process += 1;
                    self.route_move(ctx);
                    false
                }
                1 => {
                    let d = self.pos_p_len();
                    self.dist = d;
                    if ee::le(d, SHOW) {
                        self.route_move(ctx);
                        false
                    } else {
                        self.act_num = 0;
                        self.act_process = 0;
                        ctx.hits.hit_disable(&mut self.char.hit);
                        town.borrow_mut().shown -= 1;
                        self.delete_cmnd_list();
                        true
                    }
                }
                _ => {
                    self.route_move(ctx);
                    false
                }
            },
            4 => {
                match self.act_process {
                    0 => {
                        if self.what_shop() {
                            self.set_anm(0);
                            self.act_process += 1;
                            self.act_cnt = 0;
                        } else {
                            self.act_process = 1;
                            self.act_num = 3;
                            self.act_cnt = 0;
                            self.change_route(true, ctx.hits);
                        }
                    }
                    1 => {
                        set_dirc(&mut self.char.dirc[2], self.merchan_deg, 64);
                        let c = self.act_cnt;
                        self.act_cnt += 1;
                        if c >= 61 {
                            self.act_cnt = 0;
                            self.change_route(true, ctx.hits);
                            self.act_process += 1;
                        }
                    }
                    2 => {
                        let c = self.act_cnt;
                        self.act_cnt += 1;
                        if c >= 31 {
                            self.act_process = 0;
                            self.act_num = 3;
                            let mut t = town.borrow_mut();
                            let r = (t.mt.rand() & 3) as usize;
                            let text = t.tables.chat_shop.get(self.shop_num as usize).map_or("", |s| s[r]);
                            self.events.push(PcEvent::Chat(text));
                        }
                    }
                    _ => {}
                }
                false
            }
            5 => {
                if town.borrow().talk_flag {
                    return false;
                }
                match self.act_process {
                    0 => {
                        ctx.hits.hit_enable(&mut self.char.hit);
                        let mut t = town.borrow_mut();
                        t.shown += 1;
                        self.entry_cmnd();
                        self.set_anm(0);
                        self.act_process += 1;
                        self.act_cnt = 0;
                        t.chat_num = 0;
                        false
                    }
                    1 => {
                        if lt(self.pl_dist, SHOW) {
                            let to = get_dirc0(self.char.pos, self.next_pos);
                            set_dirc(&mut self.char.dirc[2], to, 64);
                            let mut t = town.borrow_mut();
                            let (id, num) = self.chat_gp;
                            if num == 0 {
                                let g = id as usize;
                                let c = t.chat_cnt.get(g).copied().unwrap_or(0) + 1;
                                if let Some(x) = t.chat_cnt.get_mut(g) {
                                    *x = c;
                                }
                                if c >= 241 {
                                    if let Some(x) = t.chat_cnt.get_mut(g) {
                                        *x = 0;
                                    }
                                    self.act_cnt = 0;
                                } else if c == 120 {
                                    t.chat_num += 1;
                                    if t.chat_num >= 13 {
                                        t.chat_num = 0;
                                    } else if let Some(x) = t.chat_cnt.get_mut(g) {
                                        *x = 0;
                                    }
                                }
                            }
                            if t.chat_num % 3 == i32::from(num) {
                                if self.act_cnt == 0 {
                                    let text = t.tables.chat_mes[t.chat_mes][t.chat_num as usize];
                                    self.events.push(PcEvent::Chat(text));
                                    self.act_cnt = 1;
                                }
                            } else {
                                self.act_cnt = 0;
                            }
                        }
                        false
                    }
                    2 => {
                        let shown = town.borrow().shown;
                        if shown < MAX_SHOWN {
                            let d = self.pos_p_len();
                            if lt(d, SHOW) && !ee::le(d, SHOW_NEAR) {
                                if lt(self.dist, d) {
                                    self.dist = d;
                                    return true;
                                }
                                self.act_process = 0;
                            }
                        }
                        false
                    }
                    _ => false,
                }
            }
            7 => {
                match self.act_process {
                    0 => {
                        self.set_anm(0);
                        self.act_process += 1;
                    }
                    1 => {
                        ctx.hits.hit_disable(&mut self.char.hit);
                        town.borrow_mut().shown -= 1;
                        self.delete_cmnd_list();
                        self.events.push(PcEvent::Transfer);
                        self.act_process += 1;
                    }
                    2 => {
                        let t = sub(self.transrate, FADE_STEP);
                        self.transrate = t;
                        if lt(t, 0) {
                            self.transrate = 0;
                            self.act_process += 1;
                            self.act_cnt = 0;
                            self.change_route(true, ctx.hits);
                        }
                    }
                    3 => {
                        let c = self.act_cnt;
                        self.act_cnt += 1;
                        if c >= 91 {
                            self.act_process += 1;
                            self.events.push(PcEvent::Transfer);
                        }
                        let to = get_dirc0(self.char.pos, self.next_pos);
                        set_dirc(&mut self.char.dirc[2], to, 64);
                    }
                    4 => {
                        let t = add(self.transrate, FADE_STEP);
                        self.transrate = t;
                        if !ee::le(t, ONE) {
                            self.transrate = ONE;
                            self.act_process += 1;
                            self.act_cnt = 0;
                        }
                    }
                    5 => {
                        self.act_num = 3;
                        self.act_process = 0;
                        self.entry_cmnd();
                        ctx.hits.hit_enable(&mut self.char.hit);
                        town.borrow_mut().shown += 1;
                    }
                    _ => {}
                }
                false
            }
            _ => false,
        }
    }

    fn route_search(&mut self, goal: V4, hits: &mut Hits) {
        let town = self.town.borrow();
        self.navi.route_search(&town.map, self.char.pos, goal, hits);
    }

    fn destination(&mut self) {
        let town = self.town.borrow();
        self.navi.destination(&town.map, &mut self.next_pos);
    }

    /// `ccRtownPC::move` (0x005077f0).
    pub fn route_move(&mut self, ctx: &mut NpcCtx) {
        let d = get_dist3d(self.char.pos, self.next_pos);
        let mut a = get_dirc(self.char.pos, self.next_pos);
        let mut s1 = rad2deg(self.char.dirc[2]) as u16 as i32;
        let s2 = rad2deg(a) as u16 as i32;
        let chg = get_dirc_chg(s1 as u16 as i16, s2 as u16 as i16, 64);
        s1 = (s1 + (chg & 0xffff)) & 0xffff;
        self.char.dirc[2] = deg2rad(s1 as u16 as i16);
        let step = |pc: &mut RtownPc, a: F| {
            let p = &mut pc.char.pos;
            p[0] = add(p[0], mul(pc.walk_spd, sinf(a)));
            p[1] = sub(p[1], mul(pc.walk_spd, cosf(a)));
        };
        if (s2 - s1 + 4096) & 0xffff < 8192 {
            if !ee::le(d, ARRIVED) && self.pos_cnt < 61 {
                if self.body_hit_flag != 0 && self.change_route_flag == 0 {
                    let c = self.body_hit_cnt;
                    if c < 40 {
                        a = sub(a, HALF);
                    } else if (60..100).contains(&c) {
                        a = add(a, HALF);
                    } else if c >= 120 {
                        self.body_hit_cnt = 0;
                    }
                }
                step(self, a);
                if self.chat_cnt == 0 {
                    if lt(self.pl_dist, CHAT_NEAR) {
                        let mut t = self.town.borrow_mut();
                        let r = (t.mt.rand() & 3) as usize;
                        if !t.chat_hold {
                            let text = t.tables.chat[r];
                            self.events.push(PcEvent::Chat(text));
                        }
                        self.chat_cnt = 1;
                    }
                } else {
                    self.chat_cnt += 1;
                    if self.chat_cnt >= 61 {
                        self.chat_cnt = 0;
                    }
                }
            } else if self.pos_cnt >= 61 {
                self.change_route(false, ctx.hits);
                self.pos_cnt = 0;
            } else {
                self.navi.landmark += 1;
                if self.navi.landmark < self.navi.step {
                    self.destination();
                    self.pos_cnt = 0;
                    step(self, a);
                } else {
                    self.act_num = 4;
                    self.act_process = 0;
                    self.pos_cnt = 0;
                    let town = self.town.borrow();
                    if self.target_num == GATE_LANDMARK && town.town == 0 {
                        self.act_num = 7;
                    }
                }
            }
        }
        // ccTransPosW2M (no wrap in a town) and the ground.
        let mut p = self.char.pos;
        p[3] = ONE;
        self.char.pos[2] = ctx.hits.land(p, ENTRY_LAND_MASK);
        self.char.hit_attribute = ctx.hits.attribute();
        if self.pos_cnt == 60 {
            if ee::le(get_dist3d(self.char.pos, self.old_pos), ARRIVED) {
                self.return_route(ctx.hits);
                self.pos_cnt = 0;
                self.change_route_flag = 1;
            } else {
                self.pos_cnt = 0;
                self.change_route_cnt = 0;
                self.change_route_flag = 0;
            }
        }
        if self.pos_cnt == 0 {
            self.old_pos = self.char.pos;
        }
        self.pos_cnt += 1;
    }

    /// `ccRtownPC::whatShop` (0x00508e50): at a merchant's landmark, the
    /// heading to its dummy and the shop.
    fn what_shop(&mut self) -> bool {
        let town = self.town.borrow();
        let row = town.tables.merchan_num.get(town.town as usize).copied().unwrap_or([-1; 5]);
        let k = row.iter().position(|&m| m == self.target_num);
        self.shop_num = -1;
        let Some(k) = k else { return false };
        let dummy = town.merchants[k].unwrap_or([0, 0, 0, ONE]);
        self.merchan_deg = get_dirc0(self.char.pos, dummy);
        self.shop_num = k as i16;
        true
    }

    /// `ccRtownPC::changeRoute(changetype)` (0x00509000): a new target among
    /// the four landmarks, never the last one, and the route there;
    /// `changetype` skips its first landmark.
    fn change_route(&mut self, skip_first: bool, hits: &mut Hits) {
        self.navi.finish = 1;
        loop {
            let r = (self.town.borrow_mut().mt.rand() & 3) as usize;
            self.target_num = self.mark_pos[r];
            if self.target_num != self.target_num_old {
                break;
            }
        }
        self.target_num_start = self.target_num_old;
        self.target_num_old = self.target_num;
        let mut goal = [0; 4];
        self.town.borrow().landmark(self.target_num, &mut goal);
        self.route_search(goal, hits);
        if skip_first {
            self.navi.landmark += 1;
        }
        self.destination();
    }

    /// `ccRtownPC::returnRoute` (0x005090c0): back toward where it came from.
    fn return_route(&mut self, hits: &mut Hits) {
        self.navi.finish = 1;
        let mut goal = [0; 4];
        self.town.borrow().landmark(self.target_num_start, &mut goal);
        self.target_num = self.target_num_start;
        self.target_num_start = self.target_num_old;
        self.target_num_old = self.target_num;
        self.char.pos[3] = ONE;
        self.route_search(goal, hits);
        self.navi.landmark += 1;
        self.destination();
    }

    /// `ccRtownPC::eventMode` (0x00508760): the PC under an event's
    /// `evActNum` (+0x2a8), always drawn.
    fn event_mode(&mut self, ctx: &mut NpcCtx) -> bool {
        let town = self.town.clone();
        let s0 = rad2deg(self.char.dirc[2]) as u16 as i32;
        match self.ev_act_num {
            -2 | -1 => match self.ev_act_process {
                0 => {
                    if self.ev_act_num == -2 {
                        self.set_anm(1);
                        self.ev_act_process = 1;
                        self.walk_spd = EV_WALK_SPD;
                    } else {
                        self.set_anm(2);
                        self.walk_spd = WALK_SPD;
                        self.ev_act_process = 1;
                    }
                }
                1 => {
                    let d = get_dist3d(self.char.pos, self.ev_next_pos);
                    let a = get_dirc(self.char.pos, self.ev_next_pos);
                    let s2 = rad2deg(a);
                    let chg = get_dirc_chg(s0 as u16 as i16, s2, 64);
                    let s = (s0 + (chg & 0xffff)) & 0xffff;
                    self.char.dirc[2] = deg2rad(s as u16 as i16);
                    let p = &mut self.char.pos;
                    p[0] = add(p[0], mul(self.walk_spd, sinf(a)));
                    p[1] = sub(p[1], mul(self.walk_spd, cosf(a)));
                    if ee::le(d, EV_ARRIVED) {
                        self.set_anm(0);
                        self.ev_act_process = 2;
                    }
                }
                _ => {}
            },
            3 => match self.ev_act_process {
                0 => {
                    self.transrate = 0;
                    self.set_anm(0);
                    self.ev_act_process += 1;
                }
                1 => {
                    let t = add(self.transrate, FADE_STEP);
                    self.transrate = t;
                    if !ee::le(t, ONE) {
                        self.transrate = ONE;
                        self.ev_act_process += 1;
                        self.act_cnt = 0;
                    }
                }
                2 => {
                    self.entry_cmnd();
                    ctx.hits.hit_enable(&mut self.char.hit);
                    town.borrow_mut().shown += 1;
                }
                _ => {}
            },
            4 | 5 => match self.ev_act_process {
                0 => {
                    self.transrate = ONE;
                    ctx.hits.hit_disable(&mut self.char.hit);
                    town.borrow_mut().shown -= 1;
                    self.delete_cmnd_list();
                    self.events.push(PcEvent::Transfer);
                    self.ev_act_process += 1;
                    self.act_cnt = 0;
                }
                1 => {
                    let t = sub(self.transrate, FADE_STEP);
                    self.transrate = t;
                    if lt(t, 0) {
                        self.transrate = 0;
                    }
                    let c = self.act_cnt;
                    self.act_cnt += 1;
                    if c >= 141 {
                        if self.ev_act_num == 5 {
                            return true;
                        }
                        self.ev_act_process += 1;
                    }
                }
                _ => {}
            },
            6 => {
                if self.ev_act_process == 0 {
                    ctx.hits.hit_disable(&mut self.char.hit);
                    self.set_anm(0);
                    self.transrate = 0;
                }
            }
            7 => {
                self.transrate = ONE;
                self.ev_act_num = 2;
                ctx.hits.hit_enable(&mut self.char.hit);
            }
            // -3: row 139's drain (anmTbl[5], [6], effects), not a town PC's.
            -3 => {}
            _ => {
                if self.ev_act_process == 0 {
                    self.set_anm(0);
                    self.ev_act_process = 1;
                    ctx.hits.hit_enable(&mut self.char.hit);
                    town.borrow_mut().shown += 1;
                    self.entry_cmnd();
                }
            }
        }
        let mut p = self.char.pos;
        p[3] = ONE;
        self.char.pos[2] = ctx.hits.land(p, ENTRY_LAND_MASK);
        self.char.hit_attribute = ctx.hits.attribute();
        false
    }

    /// `rTownNPCInfluence(pc)` (0x005091c0), the PC's `affectFunc`, for the
    /// menu's command: 14 (`PcMenu` opening) and 15 (`TalkMenu`) turn it to
    /// face Kite (act 2; 14 also holds a chat group's talk), 0 (the menu
    /// closing) sends it back to walking or to its group.
    pub fn influence(&mut self, cmd: i16) {
        let mut t = self.town.borrow_mut();
        match cmd {
            0 => {
                if self.think_type == 0 {
                    self.act_num = 3;
                    self.act_process = 0;
                } else {
                    self.act_num = 5;
                    self.act_process = 1;
                    t.talk_flag = false;
                }
            }
            14 => {
                self.act_num = 2;
                self.act_process = 0;
                if self.think_type != 0 {
                    t.talk_flag = true;
                }
            }
            15 => {
                self.act_num = 2;
                self.act_process = 0;
            }
            _ => {}
        }
    }

    /// `ccEvent::Execute`'s NPC instructions on a walking PC (main
    /// 0x001acbe0-0x001ae7c0): `npc_act` sets `evActNum` (0-2 stand, 3 fade in, 4
    /// / 5 transfer out, 5 then for good, 6 hidden, 7 shown); the walks set
    /// `evActNum` -1 toward `evNextPos` (a point, a distance along a heading, or
    /// the marker); a gradual `npc_turn` / `npc_face` sets `grotDeg` and
    /// `grotSpd` for `routine`. Only a PC an event placed runs its event mode.
    /// False for what the world does itself (puts, instant turns).
    pub fn command(&mut self, c: &NpcCommand, marker: Option<(V4, F)>, target: Option<V4>) -> bool {
        const TEN: F = 0x4120_0000;
        let spd = |chg: i16| if chg == 1 { 64 } else { chg };
        let along = |from: V4, rot: i16, dist: i16| {
            let a = deg2rad(rot);
            let d = mul(TEN, ee::from_int(i32::from(dist)));
            let mut p = from;
            p[0] = add(p[0], mul(d, sinf(a)));
            p[1] = sub(p[1], mul(d, cosf(a)));
            p
        };
        match *c {
            NpcCommand::Act { param, .. } => {
                self.ev_act_num = param;
                self.ev_act_process = 0;
            }
            NpcCommand::WalkPos { x, y, z, .. } => {
                self.ev_act_num = -1;
                self.ev_act_process = 0;
                let t = |v: i16| mul(TEN, ee::from_int(i32::from(v)));
                self.ev_next_pos = [t(x), t(y), t(z), ONE];
            }
            NpcCommand::WalkDir { rot, dist, .. } => {
                self.ev_act_num = -1;
                self.ev_act_process = 0;
                self.ev_next_pos = along(self.char.pos, rot, dist);
            }
            NpcCommand::WalkMarker { .. } => {
                let Some((p, _)) = marker else { return false };
                self.ev_act_num = -1;
                self.ev_act_process = 0;
                self.ev_next_pos = [p[0], p[1], p[2], ONE];
            }
            NpcCommand::WalkChar { rot, dist, .. } => {
                let Some(t) = target else { return false };
                self.ev_act_num = -1;
                self.ev_act_process = 0;
                self.ev_next_pos = along(t, rot, dist);
            }
            NpcCommand::Turn { dirc, chg, .. } if chg != 0 => {
                self.grot_deg = dirc;
                self.grot_spd = spd(chg);
            }
            NpcCommand::Face { chg, .. } if chg != 0 => {
                let Some(t) = target else { return false };
                self.grot_deg = rad2deg(get_dirc(self.char.pos, t));
                self.grot_spd = spd(chg);
            }
            _ => return false,
        }
        true
    }
}

impl Npc for RtownPc {
    fn code(&self) -> i32 {
        i32::from(self.row.id)
    }

    fn flags(&self) -> u32 {
        self.row.flags
    }

    fn char(&self) -> &Char {
        &self.char
    }

    fn char_mut(&mut self) -> &mut Char {
        &mut self.char
    }

    /// `ccEntryObj::routine`, then `ccRtownPC::main`.
    fn step(&mut self, ctx: &mut NpcCtx) {
        self.events.clear();
        if !self.obj_flag {
            return;
        }
        self.routine(ctx.player);
        self.main(ctx);
    }

    fn listed(&self) -> bool {
        self.listed
    }

    fn influence(&mut self, cmd: i16) {
        RtownPc::influence(self, cmd);
    }

    /// `ccMenuCtrl::PcMenu` (gcmn 0x00541e90) for rows 30-79: the first
    /// entry of the row's message table (`base->msg[0]`).
    fn talk_line(&self) -> Option<(u32, i32)> {
        (self.row.msg != 0).then_some((self.row.msg, 0))
    }

    fn command(&mut self, c: &NpcCommand, marker: Option<(V4, F)>, target: Option<V4>) -> bool {
        RtownPc::command(self, c, marker, target)
    }
}

/// Mac Anu's walking PCs as `ccSetupGameCtrl` and `ccThEntryCtrl`'s set-up
/// make them: `ccRand` seeded with `count` (`ccSys+0x358`), `reserved`
/// rows kept back (`eventMng+0x1c`), each chosen row's model loaded and the
/// PC placed, Kite at `player`.
#[allow(clippy::too_many_arguments)]
pub fn enter(
    archive: &Arc<Archive>,
    volume: Volume,
    town_file: &SceneFile,
    town_no: i32,
    count: u32,
    reserved: i32,
    player: V4,
    hits: &mut Hits,
) -> Result<(Rc<RefCell<TownPcs>>, Vec<RtownPc>)> {
    let mut mt = Mt::init(count);
    let rows = register_random_npc(&mut mt, reserved);
    let town = Rc::new(RefCell::new(TownPcs::new(volume, town_no, town_file, mt)?));
    let mut pcs = Vec::new();
    let mut files = Files::new();
    for (slot, &r) in rows.iter().enumerate() {
        if r < 0 {
            continue;
        }
        let row = NpcRow::of(volume, r as usize)?;
        let model = {
            let t = town.borrow();
            load_model(archive, &mut files, &t.tables, &row)?
        };
        pcs.push(RtownPc::place(&town, &row, &model, slot as i32, player, hits));
    }
    Ok((town, pcs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_a_quarter_of_the_way() {
        assert_eq!(get_dirc_chg(0, 4000, 64), 1000);
        assert_eq!(get_dirc_chg(0, -4000, 64), -1000);
        assert_eq!(get_dirc_chg(30000, -30000, 64), 1384);
        assert_eq!(get_dirc_chg(5, 6, 64), 1);
        assert_eq!(get_dirc_chg(7, 7, 64), 0);
    }

    #[test]
    fn fifteen_distinct_rows() {
        let mut mt = Mt::init(1234);
        let rows = register_random_npc(&mut mt, 1);
        let chosen: Vec<i16> = rows.iter().copied().filter(|&r| r >= 0).collect();
        assert_eq!(chosen.len(), 15);
        assert!(chosen.iter().all(|&r| (30..80).contains(&r)));
        let mut s = chosen.clone();
        s.sort_unstable();
        s.dedup();
        assert_eq!(s.len(), 15);
        assert_eq!(rows[15], -1);
    }

    const ISO: &str = "../../work/infection/infection.iso";

    struct Mac {
        archive: Arc<Archive>,
        town: SceneFile,
        hits: Hits,
    }

    fn mac_anu() -> Option<Mac> {
        if !std::path::Path::new(ISO).exists() {
            eprintln!("skipped: no {ISO}");
            return None;
        }
        let mut iso = piney_data::iso::Iso::open(ISO).unwrap();
        let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
        let town = SceneFile::read(&archive, "town01").unwrap();
        let hits = Hits::new(crate::hit::HitModel::read(&town.ccs).unwrap());
        Some(Mac { archive, town, hits })
    }

    #[test]
    fn routes_between_landmarks() {
        // Checked against ccNavi::RouteSearchByMap in eemu.
        let Some(mut m) = mac_anu() else { return };
        let map = NaviMap::read(piney_data::volume::Volume::Inf, 0, &m.town).unwrap();
        assert_eq!(map.num, 72);
        let mut navi = Navi::default();
        navi.route_search(&map, crate::START_POS, [0xc500_0000, 0xc500_0000, 0, ONE], &mut m.hits);
        assert_eq!(navi.route[..13], [254, 44, 43, 12, 14, 53, 54, 15, 16, 17, 24, 51, 255]);
        assert_eq!((navi.step, navi.landmark, navi.name), (12, 1, 0));
        let from = [ee::k(-700.0), ee::k(-5700.0), ee::k(300.0), ONE];
        navi.route_search(&map, from, [ee::k(2450.0), ee::k(-2250.0), 0, ONE], &mut m.hits);
        assert_eq!(navi.route[..9], [254, 38, 36, 17, 16, 19, 22, 48, 255]);
        let mut dest = [0; 4];
        assert_eq!(navi.destination(&map, &mut dest), 1);
        assert_eq!(dest, [ee::k(-700.0), ee::k(-5900.0), ee::k(300.0), ONE]);
    }

    #[test]
    fn mac_anu_walking_pcs() {
        let Some(mut m) = mac_anu() else { return };
        // ccSys+0x358 0, Kite alone: the rows the game picks (eemu).
        let (town, mut pcs) = enter(&m.archive, Volume::Inf, &m.town, 0, 0, 1, crate::START_POS, &mut m.hits).unwrap();
        let rows: Vec<i16> = pcs.iter().map(|p| p.row.id).collect();
        assert_eq!(rows, [37, 39, 49, 57, 44, 45, 47, 38, 79, 56, 62, 58, 61, 50, 63]);
        // Markers 0-2: Mac Anu's chat group round landmark 38; the rest walk.
        let acts: Vec<i16> = pcs.iter().map(|p| p.act_num).collect();
        assert_eq!(acts[..4], [5, 5, 5, 6]);
        assert!(acts[3..].iter().all(|&a| a == 6));
        assert!(pcs.iter().all(|p| p.char.hit.sw && p.char.hit.kind == KIND_PC && !p.listed));
        assert_eq!(m.hits.chars.len(), 15);
        // Weapons on their hands: row 57 (Koji, ctwm3, a staff) in the right.
        let koji = &pcs[3];
        assert_eq!(koji.weapons, [("OBJ_t0 r hand".to_string(), "MDL_cw1hst04".to_string())]);
        assert_eq!(koji.tex_swap.as_deref(), Some("TEX_cwm3bod1v3"));
        let file = &koji.char.body.file;
        let swaps: Vec<_> =
            koji.char.body.tex_swaps.iter().map(|&(m, t)| (file.ccs.object_name(m), file.ccs.object_name(t))).collect();
        assert_eq!(swaps, [(Some("MAT_tex"), Some("TEX_cwm3bod1v3"))], "changeTEX");
        // Kite standing at the gate: the two walkers from the plaza head
        // south down the stairs, shown and on the command list.
        let cam = crate::camera::Camera::new(crate::START_POS, crate::START_DIRC, 3, crate::camera::Scheme::new(0));
        let mut rand = crate::Rand(1);
        for _ in 0..120 {
            let t = cam.tcam.clone();
            for pc in pcs.iter_mut() {
                let mut ctx = NpcCtx {
                    player: crate::START_POS,
                    player_dirc: crate::START_DIRC,
                    view: crate::char::View { player: crate::START_POS, cam: t.pos, deg1: t.deg[1], eye: false },
                    cam: &t,
                    hits: &mut m.hits,
                    rand: &mut rand,
                };
                Npc::step(pc, &mut ctx);
            }
        }
        for k in [3, 10] {
            let p = &pcs[k];
            assert_eq!((p.act_num, p.target_num), (3, 48));
            assert!(ee::f(p.char.pos[1]) < 3500.0 && p.listed && p.char.drawn, "{k}");
        }
        assert!(town.borrow().shown >= 2);
        // Spoken to: facing Kite; let go: walking on.
        pcs[3].influence(14);
        assert_eq!(pcs[3].act_num, 2);
        pcs[3].influence(0);
        assert_eq!((pcs[3].act_num, pcs[3].act_process), (3, 0));
    }
}
