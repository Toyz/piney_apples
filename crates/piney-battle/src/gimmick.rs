//! The dungeon's objects: `ccGimBox` (treasure boxes, the breakables, the
//! virus crystal; gmbox.cpp, gcmn 0x00453400-0x00454838), `ccGimIdol` (the
//! Gott and Zeit statues, 0x00459620-0x00459d4c), `ccGimSymbol` (0x0045a0d0)
//! and `ccGimFood` (the Grunty foods, 0x00458280), with the setters that place
//! them (`DUNGEON::SetItemBox` 0x005beb30, `SetIDOL` 0x005be750,
//! `EntryBreakObject` 0x005bff10). The trap's damage and skill are the world's
//! [`Call::TrapDamage`] and [`Call::TrapSkill`]; sounds, effects, rays and
//! draws are [`Out`]s. The acts are in docs/engine/battle.md.

use crate::chara::{AffectFunc, Char, spc_flag};
use crate::enemy_ai::EntryParam;
use crate::enemy_motion::{At, Call};
use crate::entry::{
    Cx, DungeonGims, EntryCtrl, EntryObj, Obj, Out, Seam, delete_cmnd, delete_event_entry, entry_param_clear,
    gimmick_char, set_hit_sw,
};
use crate::geom::{self, F, HALF_PI, ONE, PI, TWO_PI, V4, add, div, from_int, le, lt, mul, sub};
use crate::prim::{self, BOX_RAD_INFO, Radiate};
use crate::tables::Tables;
use crate::world::AnmSlot;

// `ccEntryGimBox` (INF gcmn 0x00453420): `gimmickTbl[0..15].entry.func`;
// `ccEntryGimIdol` (0x00459620): rows 38-44.
/// `ccDestGimBox` (0x00453470), `ccDestGimIdol` (0x00459670): their
/// rays' destructors.
pub const DEST_BOX: u32 = 0x0045_3470;
pub const DEST_IDOL: u32 = 0x0045_9670;

// `ccEntryGimSymbol` (INF gcmn 0x0045a010): rows 17 (a story dungeon's
// symbol, `XGSYMBOL.CCS`) and 18 (the lakes' `SYMBOL_OBJ`), and
// `ccDestGimSymbol` (0x0045a060).
pub const DEST_SYMBOL: u32 = 0x0045_a060;
/// `symbolSkillTbl[16]` (gcmn 0x006aa280): the skill a symbol casts,
/// `ccRand() & 15`.
pub const SYMBOL_SKILLS: [i16; 16] = [296, 296, 297, 298, 299, 300, 301, 302, 303, 187, 188, 189, 190, 191, 192, 296];
/// Row 17's model and clip, and its fires' Eff chunk (`initFire`).
pub const SYMBOL_MODEL: &str = "CMP_xgsymbo0";
pub const SYMBOL_ANIM: &str = "ANM_xgsymbol";
pub const SYMBOL_FIRE: &str = "EFF_x008";
/// `ccGimSymbol` +0x1e4: its 40 `ccSymFire`s (0x20 bytes each).
pub const SYMBOL_FIRES: usize = 40;

/// `ccSpcMessageOpenTrapBox()`'s message (`ccAISysMsgSend(0x1000f, -1,
/// kite's AI, 0xffff, 0, 30)`).
pub const MSG_OPEN_TRAP_BOX: i32 = 0x1000f;

/// What a gimmick's class keeps beyond `ccGimmick`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Class {
    /// A plain `ccGimmick` (or a class not ported: its seam's).
    #[default]
    None,
    /// `ccGimBox` +0x1e0 `boxRadiate`.
    Box(Box<Radiate>),
    /// `ccGimIdol`.
    Idol(Box<Idol>),
    /// `ccGimSymbol`.
    Symbol(Box<Symbol>),
    /// `ccGimFood`.
    Food(Box<Food>),
    /// `ccGimEtc` (rows 19-21: [`crate::gimetc`]).
    Etc(Box<crate::gimetc::Etc>),
}

impl Class {
    /// The object kept across a room change came back as scene index
    /// `who` (`EntryCtrl::restore_kept`): its rays follow it there.
    pub fn moved_to(&mut self, who: usize) {
        match self {
            Class::None => {}
            Class::Box(r) => r.user = Some(who),
            Class::Idol(i) => i.rad.user = Some(who),
            Class::Symbol(_) | Class::Food(_) | Class::Etc(_) => {}
        }
    }
}

/// `ccGimIdol`'s members.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Idol {
    /// +0x1e0 `effsw`: 1 until the idol has been open 100 frames. The
    /// glow `effStatueOfGod(pos, &effsw)` starts keeps a pointer to it and
    /// runs while it is not 0.
    pub effsw: i32,
    /// +0x1e4 `idolId`: the row - 38.
    pub idol_id: i32,
    /// +0x1e8 `boxRadiate`.
    pub rad: Radiate,
}

/// One `ccSymFire` (0x20 bytes): an `EFF_x008` sprite rising from the
/// symbol and fading.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fire {
    /// +0x00: 0 free, 1 burning. +0x04: 0 rising and brightening, 1
    /// fading. +0x08: frames in the stage. +0x14: the clip done. +0x18: the
    /// pattern.
    pub state: i32,
    pub stage: i32,
    pub cnt: i32,
    pub done: i32,
    pub pat: i32,
    /// Its `ccEff`: +0x10 the position, +0x20 / +0x24 the scale, +0x2c
    /// the colour (r, g, b, a bytes, low first).
    pub pos: V4,
    pub scale: [F; 2],
    pub colour: u32,
}

// `ccEntryGimFood` (INF gcmn 0x00458220): `gimmickTbl[22..38].entry.func`;
// `ccDestGimFood` (0x00458270) does nothing.
pub const DEST_FOOD: u32 = 0x0045_8270;
/// The food rows (22 the Golden Egg).
pub const FOOD_ROWS: std::ops::RangeInclusive<i32> = 22..=37;
/// `ccGimFood`'s model (`GetChunkAdrsF(stream, "CMP_trall")`).
pub const FOOD_MODEL: &str = "CMP_trall";
/// `foodAnimTbl` (gcmn 0x006a9d70, 30 bytes a name): each food's clip.
pub const FOOD_ANIMS: [&str; 16] = [
    "ANM_xgfood00a",
    "ANM_xgfood01a",
    "ANM_xgfood02a",
    "ANM_xgfood03a",
    "ANM_xgfood04a",
    "ANM_xgfood05a",
    "ANM_xgfood06a",
    "ANM_xgfood07a",
    "ANM_xgfood08a",
    "ANM_xgfood09a",
    "ANM_xgfood0aa",
    "ANM_xgfood0ba",
    "ANM_xgfood0ca",
    "ANM_xgfood0da",
    "ANM_xgfood0ea",
    "ANM_xgfood0fa",
];

/// `ccGimFood`'s members (0x240 bytes over `ccGimmick`, whose `actNum`
/// - 0 lying, 1 taken, 2 gone - `actCnt` and `anmFlag` it uses).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Food {
    /// +0x1e0 bit 0: Kite near (within 1000); +0x1e4 the food (row - 22).
    pub near: bool,
    pub kind: i32,
    /// +0x1f0 the model's scale.
    pub scale: V4,
    /// +0x200, +0x204: the roll's speeds about x and y.
    pub roll_spd: [F; 2],
    /// +0x210 the wobble's amplitude, +0x214 its step, +0x218 its gain.
    pub amp: F,
    pub step: F,
    pub gain: F,
    /// +0x21c a roll running, +0x220 and +0x224 its rates.
    pub rolling: i32,
    pub roll_rate: [i32; 2],
    /// +0x228 a `ccRandS()` its first frame draws, +0x22c the bounce's
    /// angle (16-bit units) as it goes, +0x22a and +0x22e zero.
    pub rand_s: i16,
    pub bounce: i16,
    /// +0x230 its heading as placed.
    pub base_dirc: F,
}

/// `ccGimSymbol`'s members.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    /// +0x1d4 `act` (0 waiting, 1 cast, 2 spent), +0x1d8 its count.
    pub act: i32,
    pub cnt: i32,
    /// +0x1e0 bit 0: the fires made; +0x1e4 the fires.
    pub fires: Vec<Fire>,
    /// The fires' `patNum` (`EFF_x008` of the symbol's file).
    pub pat_num: u16,
    /// +0x6f0 its omni light's place (160 above the symbol, 60 for row
    /// 18), +0x700 bit 0 the light in the group.
    pub light_pos: V4,
    pub light_on: bool,
}

/// The model and first clip of box row `id` (`ccGimBox::ccGimBox`'s
/// switch): the scene file's `CMP_`/`ANM_` names.
pub fn box_model(id: i32) -> Option<(&'static str, &'static str)> {
    Some(match id {
        0 | 1 => ("CMP_xgbbox00", "ANM_xgbbox00"),
        2 | 3 | 7 => ("CMP_xgbcont0", "ANM_xgbcont0"),
        4 | 5 | 8 => ("CMP_xgbbutt0", "ANM_xgbbutt0"),
        6 => ("CMP_xgvirus0", "ANM_xgvirus0"),
        9 => ("CMP_xgbpot1", "ANM_xgbpot1"),
        10 => ("CMP_xgbpot0", "ANM_xgbpot0"),
        11 => ("CMP_xgbske1", "ANM_xgbske1"),
        12 => ("CMP_xgbske0", "ANM_xgbske0"),
        13 => ("CMP_xgbegg0", "ANM_xgbegg0"),
        14 => ("CMP_xgbegg1", "ANM_xgbegg1"),
        _ => return None,
    })
}

/// `tboxAnimTbl[2]`: the treasure box's lid opening.
pub const BOX_OPEN: &str = "ANM_xgbbox02";

/// The idol's model.
pub const IDOL_MODEL: &str = "CMP_trall";

/// `idolAnimTbl[7][3]` (gcmn 0x006a9ff0): each idol's standing, opening
/// and open clips.
pub const IDOL_ANIMS: [[&str; 3]; 7] = [
    ["ANM_xgsanut0", "ANM_xgsadwn0", "ANM_xgsanut1"],
    ["ANM_xgsfnut0", "ANM_xgsfdwn0", "ANM_xgsfnut1"],
    ["ANM_xgswnut0", "ANM_xgswdwn0", "ANM_xgswnut1"],
    ["ANM_xgslnut0", "ANM_xgsldwn0", "ANM_xgslnut1"],
    ["ANM_xgsenut0", "ANM_xgsedwn0", "ANM_xgsenut1"],
    ["ANM_xgsdnut0", "ANM_xgsddwn0", "ANM_xgsdnut1"],
    ["ANM_xgstnut0", "ANM_xgstdwn0", "ANM_xgstnut1"],
];

/// The clip of idol `idol` in act `act` (0-2).
pub fn idol_anim(idol: i32, act: i32) -> Option<&'static str> {
    let row = IDOL_ANIMS.get(usize::try_from(idol).ok()?)?;
    row.get(usize::try_from(act).ok()?).copied()
}

/// The trapped rows: the treasure box, the wooden box, the barrel.
fn trapped(id: i32) -> bool {
    matches!(id, 1 | 3 | 5)
}

/// `ccGimmick::ccGimmick` (gcmn 0x004533a0) with `ccEntryObj`'s: the
/// entry copied in, the character at its position (`posP` through the
/// player's frame) and heading, `gimId` the row, `SetBaseParam`, and the
/// `affectFunc` `ccGimmickAffect`.
fn gimmick_base(cx: &mut Cx, ent: &EntryParam) -> (Char, EntryObj) {
    let mut ch = gimmick_char(cx.st, ent.id);
    // +0x140, entParam.entRoot
    ch.ent_root = ent.ent_root as u32;
    ch.pos = ent.pos;
    ch.pos_p = cx.world.w2p(ent.pos);
    let o = EntryObj { ent: *ent, dirc: ent.dirc, gim_id: ent.id, ..EntryObj::default() };
    (ch, o)
}

/// `ccGimBox::ccGimBox(ent)` (gcmn 0x004534a0), see the module's header:
/// the body (`bodyHit` at its position, the base's width, half its
/// height, its type) switched on, act 0, the model's first clip
/// ([`World::anim_set`](crate::world::World::anim_set)), the trap, a fade
/// in from 0 over 10 frames, the rays (`entryBoxRadiate`), `ccDestGimBox`.
pub fn box_new(cx: &mut Cx, ent: &EntryParam, who: usize) -> (Char, EntryObj) {
    let (mut ch, mut o) = gimmick_base(cx, ent);
    let base = *ch.base();
    o.hit.pos = ch.pos;
    o.hit.radius = base.width;
    o.hit.height = div(base.height, geom::k(2.0));
    o.hit.kind = base.ty as u32;
    set_hit_sw(cx.world, who, &mut o.hit, true);
    o.act_num = 0;
    if let Some((_, anm)) = box_model(o.gim_id) {
        cx.world.anim_set(who, AnmSlot::Main, anm);
    }
    if trapped(o.gim_id) && !matches!(o.ent.param[2], 0..=2) {
        o.ent.param[2] = match cx.cc.rand() & 3 {
            3 => 2,
            2 => 1,
            _ => 0,
        };
    }
    match o.ent.param[2] {
        0 => {
            ch.skill_id = 1;
            ch.cond[crate::param::cond::DYING] = 100;
        }
        1 => ch.skill_id = 156,
        2 => ch.skill_id = 162,
        _ => {}
    }
    o.transparency = 0;
    o.set_transparency = 0;
    o.fade_flag = 1;
    o.fade_cnt = 10;
    o.class = Class::Box(Box::new(Radiate::new(BOX_RAD_INFO, Some(who))));
    o.dest_func = DEST_BOX;
    (ch, o)
}

/// `ccGimIdol::ccGimIdol(ent)` (gcmn 0x004596a0): `idolId` the row - 38,
/// `effsw` 1; an idol whose event entry is used (`param[2]` 0) stands
/// open (act 2, `effsw` 0, kept off the command list), one of `param[2]`
/// -1 at act 0; the act's clip; the rays; `ccDestGimIdol`.
pub fn idol_new(cx: &mut Cx, ent: &EntryParam, who: usize) -> (Char, EntryObj) {
    let (ch, mut o) = gimmick_base(cx, ent);
    let mut idol = Idol { effsw: 1, idol_id: o.gim_id - 38, rad: Radiate::new(BOX_RAD_INFO, Some(who)) };
    match o.ent.param[2] {
        0 => {
            o.act_num = 2;
            idol.effsw = 0;
            // deleteCmnd(1): not listed yet, so only cmndFlag.
            o.cmnd_flag = true;
        }
        -1 => o.act_num = 0,
        _ => {}
    }
    if let Some(a) = idol_anim(idol.idol_id, o.act_num) {
        cx.world.anim_set(who, AnmSlot::Main, a);
    }
    o.class = Class::Idol(Box::new(idol));
    o.dest_func = DEST_IDOL;
    (ch, o)
}

/// `ccGimSymbol::ccGimSymbol(ent)` (gcmn 0x0045a0d0): the entry copied
/// in, `trapNum` (+0x7c) `symbolSkillTbl[ccRand() & 15]`, an omni light
/// (orange, `ccSetColor(0x7fff)`; full at the flame, falling to nothing
/// 1000 away). Row 18 puts the light 60 above the symbol and stops there.
/// Row 17 puts it 160 above, switches its body on (the base's width, half
/// its height, its type), plays `ANM_xgsymbol` on `CMP_xgsymbo0`, makes
/// its fires (`initFire`) and runs them 20 frames ahead (`advanceFire`: a
/// new fire each, undrawn).
pub fn symbol_new(cx: &mut Cx, ent: &EntryParam, who: usize) -> (Char, EntryObj) {
    let (mut ch, mut o) = gimmick_base(cx, ent);
    o.act_num = 0;
    ch.skill_id = SYMBOL_SKILLS[(cx.cc.rand() & 15) as usize];
    let up = if o.gim_id == 17 { geom::k(160.0) } else { geom::k(60.0) };
    let mut light_pos = ch.pos;
    light_pos[2] = add(light_pos[2], up);
    let mut sym = Symbol { act: 0, cnt: 0, fires: Vec::new(), pat_num: 1, light_pos, light_on: false };
    if o.gim_id == 17 {
        let base = *ch.base();
        o.hit.pos = ch.pos;
        o.hit.radius = base.width;
        o.hit.height = div(base.height, geom::k(2.0));
        o.hit.kind = base.ty as u32;
        set_hit_sw(cx.world, who, &mut o.hit, true);
        cx.world.anim_set(who, AnmSlot::Main, SYMBOL_ANIM);
        sym.pat_num = cx.world.eff_pat_num(who, SYMBOL_FIRE).unwrap_or(1);
        sym.fires = vec![Fire::default(); SYMBOL_FIRES];
        for _ in 0..20 {
            set_fire(cx, &mut sym, 1, 1);
            for k in 0..SYMBOL_FIRES {
                if sym.fires[k].state != 0 {
                    fire_main(cx, &mut sym.fires[k], sym.pat_num, None);
                }
            }
        }
    }
    o.class = Class::Symbol(Box::new(sym));
    o.dest_func = DEST_SYMBOL;
    (ch, o)
}

/// `ccGimFood::ccGimFood(ent)` (gcmn 0x00458280): see the module's
/// header.
pub fn food_new(cx: &mut Cx, ent: &EntryParam, who: usize) -> (Char, EntryObj) {
    let (ch, mut o) = gimmick_base(cx, ent);
    let kind = o.gim_id - 22;
    let base = *ch.base();
    o.hit.pos = ch.pos;
    o.hit.radius = base.width;
    o.hit.height = div(base.height, geom::k(2.0));
    o.hit.kind = base.ty as u32;
    set_hit_sw(cx.world, who, &mut o.hit, true);
    if let Some(a) = usize::try_from(kind).ok().and_then(|k| FOOD_ANIMS.get(k)) {
        cx.world.anim_set(who, AnmSlot::Main, a);
    }
    let food = Food { kind, scale: [ONE; 4], base_dirc: o.dirc[2], ..Food::default() };
    o.class = Class::Food(Box::new(food));
    o.dest_func = DEST_FOOD;
    (ch, o)
}

/// `fabs((double) v) < 0.01`.
fn under_hundredth(v: F) -> bool {
    f64::from(f32::from_bits(v)).abs() < 0.01
}

/// `ccGimFood::actRolling()` (gcmn 0x00458580).
fn food_rolling(f: &mut Food, dirc: &mut V4) {
    let mut done = false;
    if under_hundredth(f.roll_spd[1]) {
        f.roll_spd[1] = geom::get_dirc_chg_f(dirc[1], 0, f.roll_rate[0]);
        done = under_hundredth(f.roll_spd[1]);
    } else {
        let mut y = add(dirc[1], f.roll_spd[1]);
        if !le(y, PI) {
            y = sub(y, TWO_PI);
        }
        if lt(y, geom::NEG_PI) {
            y = add(y, TWO_PI);
        }
        dirc[1] = y;
        f.roll_spd[1] = geom::set_dirc(f.roll_spd[1], 0, f.roll_rate[1]);
    }
    if done {
        f.roll_spd = [0, 0];
        dirc[1] = 0;
        dirc[0] = 0;
        f.rolling = 0;
    }
}

/// `ccGimFood::main()` (gcmn 0x00458710): see the module's header. True
/// when it is gone.
fn food_main(cx: &mut Cx, who: usize, o: &mut EntryObj) -> bool {
    const FAR: F = 0x447a_0000; // 1000
    const QUARTER_PI: F = 0x3f49_0fdb;
    if o.freeze_flag {
        return false;
    }
    fw2lw(cx, who);
    let Class::Food(mut f) = std::mem::take(&mut o.class) else { return false };
    let pos = cx.scene.chars[who].pos;
    if !le(o.pl_dist, FAR) {
        o.dirc[2] = geom::set_dirc(o.dirc[2], f.base_dirc, 128);
        let away = geom::rad2deg(o.pl_dirc) ^ i16::MIN;
        o.dirc[2] = geom::set_dirc(o.dirc[2], geom::deg2rad(away), 128);
        f.near = false;
    } else {
        if !f.near {
            f.rolling = 0;
            f.amp = 0;
            cx.out.push(Out::FoodVoice { kind: f.kind });
        }
        o.dirc[2] = geom::set_dirc(o.dirc[2], o.pl_dirc, 128);
        f.near = true;
    }
    if !geom::eq(f.amp, 0) {
        if lt(f.amp, 0x3c23_d70a) {
            f.amp = 0;
        } else {
            f.amp = sub(f.amp, f.step);
            let s = geom::sinf(mul(TWO_PI, f.amp));
            f.scale[2] = add(ONE, mul(f.gain, mul(f.amp, s)));
        }
    } else if !f.near {
        if cx.cc.rand() & 3 == 0 {
            f.amp = from_int(cx.cc.rand() & 3);
            f.gain = div(0x3e4c_cccd, f.amp);
            f.step = div(f.amp, 0x42b4_0000);
        }
    } else {
        if o.act_num != 2 {
            cx.out.push(Out::Se { se: 164, pos });
        }
        f.amp = from_int(cx.cc.rand() & 7);
        f.gain = div(0x3e99_999a, f.amp);
        f.step = div(f.amp, 0x41f0_0000);
    }
    if f.rolling != 0 {
        food_rolling(&mut f, &mut o.dirc);
    } else if !f.near {
        if cx.cc.rand() & 3 == 0 {
            f.roll_rate = [128, 256];
            f.rolling = 1;
            let a = crate::enemy_ai::rand_f(cx.cc, QUARTER_PI);
            f.roll_spd[0] = geom::get_dirc_chg_f(o.dirc[0], a, f.roll_rate[1]);
            let b = crate::enemy_ai::rand_f(cx.cc, QUARTER_PI);
            f.roll_spd[1] = geom::get_dirc_chg_f(o.dirc[1], b, f.roll_rate[1]);
        }
    } else {
        f.roll_rate = [48, 76];
        f.rolling = 1;
        let a = crate::enemy_ai::rand_f(cx.cc, QUARTER_PI);
        f.roll_spd[0] = geom::get_dirc_chg_f(o.dirc[0], a, f.roll_rate[1]);
        let b = crate::enemy_ai::rand_f(cx.cc, QUARTER_PI);
        f.roll_spd[1] = geom::get_dirc_chg_f(o.dirc[1], b, f.roll_rate[1]);
    }
    match o.act_num {
        0 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            if c == 0 {
                f.rand_s = crate::entry::rand_s(cx.rnds);
            }
            if o.affect_flag {
                o.affect_flag = false;
                if cx.scene.chars[who].affect.ty == 11 {
                    o.act_num = 1;
                    o.act_cnt = 0;
                }
            }
        }
        1 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            if c == 20 {
                o.act_num = 2;
                o.act_cnt = 0;
            } else if c == 0 {
                delete_cmnd(&mut o.cmnd_flag, cx.scene, cx.out, who, true);
                cx.out.push(Out::SeNote { se: 179, pos, note: 70 });
            }
        }
        2 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            if c == 30 {
                o.class = Class::Food(f);
                return true;
            }
            if c == 0 {
                cx.out.push(Out::OpenBox { pos });
                cx.out.push(Out::Se { se: 77, pos });
                o.fade_flag = 2;
                o.fade_cnt = 30;
                f.bounce = 0;
            }
            f.rolling = 0;
            f.amp = 0;
            f.bounce = f.bounce.wrapping_add(9029);
            let mut s = geom::sinf(geom::deg2rad(f.bounce));
            if lt(s, 0) {
                s = mul(s, 0xbf80_0000);
            }
            let n = from_int(o.act_cnt);
            let acc = add(0x3f00_0000, mul(0x3ca3_d70a, n));
            f.scale[2] = add(acc, mul(s, mul(0x3c23_d70a, n)));
        }
        _ => {}
    }
    if o.disp_sw {
        o.disp_sw = cx.world.camera_deg(pos, 0x3000);
        if o.disp_sw {
            o.anm_flag = i32::from(cx.world.anim_forward(who, AnmSlot::Main, 256));
            cx.out.push(Out::FoodDraw { who, dirc: o.dirc, scale: f.scale });
        }
    }
    o.class = Class::Food(f);
    false
}

/// `ccGimSymbol::setFire(state, n)` (gcmn 0x0045a600): `n` times, the first
/// free fire lit: cleared, its colour 0, at the light's place moved by
/// `ccRandF(10)` along a `ccRand()` heading and `ccRandF(10)` up.
fn set_fire(cx: &mut Cx, sym: &mut Symbol, state: i32, n: i32) {
    for _ in 0..n {
        let Some(f) = sym.fires.iter_mut().find(|f| f.state == 0) else { continue };
        *f = Fire { state, pos: geom::VF0, colour: 0, ..Fire::default() };
        let r = geom::deg2rad(cx.cc.rand() as u16 as i16);
        let m = geom::rot_matrix_z(&geom::unit_matrix(), r);
        let mut v = geom::VF0;
        v[0] = add(v[0], crate::enemy_ai::rand_f(cx.cc, geom::k(10.0)));
        let v = geom::apply_matrix(&m, v);
        let mut p = geom::vadd(sym.light_pos, v);
        p[2] = add(p[2], crate::enemy_ai::rand_f(cx.cc, geom::k(10.0)));
        f.pos = p;
    }
}

/// `ccSymFire::main(draw)` (gcmn 0x00459e50): rising 0.8 a frame at scale
/// 1.2 and brightening by 3 until its clip is done, then rising 1.6 and
/// darkening by 3 (a channel below 0 turns 255, as `fade` does) until
/// done again, then free; drawn (`ccEff::Draw(pattern)`) when `out` is
/// given.
fn fire_main(cx: &mut Cx, f: &mut Fire, pat_num: u16, out: Option<&mut Vec<Out>>) {
    let q = cx.world.w2p(f.pos);
    f.pos = cx.world.p2w(q);
    if f.state == 1 {
        if f.stage == 0 {
            if f.cnt == 0 {
                f.scale = [0x3f99_999a; 2];
                f.cnt += 1;
            }
            f.pos[2] = add(f.pos[2], 0x3f4c_cccd);
            if f.done != 0 {
                f.done = 0;
                f.stage = 1;
                f.cnt = 0;
                f.pat = 0;
            }
        } else if f.stage == 1 {
            f.pos[2] = add(f.pos[2], 0x3fcc_cccd);
        }
        f.colour = fade(f.colour, f.stage, 3);
    }
    if f.done == 1 {
        f.state = 0;
    }
    if f.state != 0 {
        if let Some(out) = out {
            out.push(Out::SymbolFire { pos: f.pos, pattern: f.pat as u16, scale: f.scale, colour: f.colour });
        }
        f.pat += 1;
        if f.pat == i32::from(pat_num) {
            f.done = 1;
        }
    }
}

/// `ccSymFire::fade(dir, step)` (gcmn 0x00459d50): each byte of the
/// colour up by `step` (dir 0, held at 255) or down (dir 1, a byte below 0
/// becoming 255).
fn fade(c: u32, dir: i32, step: i32) -> u32 {
    let mut b = c.to_le_bytes().map(i32::from);
    for v in &mut b {
        match dir {
            0 => *v = (*v + step).min(255),
            1 => {
                *v -= step;
                if *v < 0 {
                    *v = 255;
                }
            }
            _ => {}
        }
    }
    u32::from_le_bytes(b.map(|v| v as u8))
}

/// The constructor `gimmickTbl[ent.id].entry.func` names, when this module
/// has it (the boxes, the idols, the symbols and [`crate::gimetc`]'s).
pub fn construct(cx: &mut Cx, ent: &EntryParam, who: usize) -> Option<(Char, EntryObj)> {
    use piney_data::tables::types::EntryFunc;
    let func = usize::try_from(ent.id).ok().and_then(|i| cx.st.gimmicks.get(i)).and_then(|g| g.func)?;
    match func {
        EntryFunc::GimBox => Some(box_new(cx, ent, who)),
        EntryFunc::GimIdol => Some(idol_new(cx, ent, who)),
        EntryFunc::GimSymbol => Some(symbol_new(cx, ent, who)),
        EntryFunc::GimFood => Some(food_new(cx, ent, who)),
        EntryFunc::GimEtc => Some(crate::gimetc::etc_new(cx, ent, who)),
        _ => None,
    }
}

/// `ccGimmickAffect(ch)` (gcmn 0x00453400): the object's `affectFlag`.
/// Its `+0xe0` byte is [`EntryObj::affect_flag`]; [`crate::affect`] sets
/// the character's copy ([`spc_flag::AFFECT`]), which [`main`] moves over
/// before the object's frame.
pub fn take_affect(ch: &mut Char, o: &mut EntryObj) {
    if ch.affect.func == AffectFunc::Gimmick && ch.spc_char.flags & spc_flag::AFFECT != 0 {
        ch.spc_char.flags &= !spc_flag::AFFECT;
        o.affect_flag = true;
    }
}

/// The main of a gimmick this module made (vtable +8: `ccGimBox::main`,
/// `ccGimIdol::main`): true to delete it. None for any other object.
pub fn main(ctrl: &mut EntryCtrl, cx: &mut Cx, seam: &mut dyn Seam, in_battle: i32, who: usize) -> Option<bool> {
    let Obj::Gimmick(o) = ctrl.objs.get(who)? else { return None };
    if matches!(o.class, Class::None) {
        return None;
    }
    let Obj::Gimmick(mut o) = std::mem::take(&mut ctrl.objs[who]) else { unreachable!() };
    take_affect(&mut cx.scene.chars[who], &mut o);
    let r = match &o.class {
        Class::Box(_) => box_main(ctrl, cx, seam, in_battle, who, &mut o),
        Class::Symbol(_) => symbol_main(cx, who, &mut o),
        Class::Food(_) => food_main(cx, who, &mut o),
        Class::Etc(_) => crate::gimetc::main(cx, who, &mut o),
        _ => idol_main(cx, who, &mut o),
    };
    ctrl.objs[who] = Obj::Gimmick(o);
    Some(r)
}

/// The rays' frame (`ccPrimRadiate::main`), their outputs tagged.
fn rays(cx: &mut Cx, who: usize, rad: &mut Radiate) {
    let mut out = Vec::new();
    prim::box_main(rad, cx, &mut out);
    for r in out {
        cx.out.push(Out::Radiate { who, out: r });
    }
}

/// `ccTransPosW2P(posP, pos)`, `ccTransPosP2W(pos, posP)`.
fn fw2lw(cx: &mut Cx, who: usize) {
    let q = cx.world.w2p(cx.scene.chars[who].pos);
    cx.scene.chars[who].pos_p = q;
    cx.scene.chars[who].pos = cx.world.p2w(q);
}

/// `ccGimBox::main()` (gcmn 0x004545d0).
fn box_main(
    ctrl: &mut EntryCtrl,
    cx: &mut Cx,
    seam: &mut dyn Seam,
    in_battle: i32,
    who: usize,
    o: &mut EntryObj,
) -> bool {
    if o.freeze_flag {
        return !matches!(o.ent.ent_root, -1 | 0);
    }
    if let Class::Box(rad) = &mut o.class {
        rays(cx, who, rad);
    }
    let (r, layer) = match o.gim_id {
        0 | 1 => (box_act(ctrl, cx, seam, in_battle, who, o), 5),
        6 => (virus_act(cx, who, o), 3),
        2..=14 => (object_act(ctrl, cx, seam, who, o), 5),
        // Rows 15 on never have this class; the game's result is then its
        // register's left-over.
        _ => (false, 0),
    };
    o.anm_flag = i32::from(cx.world.anim_forward(who, AnmSlot::Main, 256));
    if o.disp_sw {
        let pos = cx.scene.chars[who].pos;
        o.disp_sw = cx.world.camera_deg(pos, 12288);
        if o.disp_sw {
            let t = cx.world.camera_transparency(pos, 0, 0, geom::k(7000.0), geom::k(600.0));
            let t = mul(t, o.set_transparency);
            // stored as a byte (`sb`)
            let alpha = ((crate::damage::fptosi(mul(geom::k(256.0), t)).wrapping_add(1) >> 1) & 0xff) as i32;
            let h = cx.scene.chars[who].base().height;
            let height = add(h, mul(geom::k(2.0), h));
            cx.out.push(Out::GimDraw { who, layer, transparency: t, alpha, height, dirc: o.dirc });
        }
    }
    r
}

/// The event entry of a used object (`deleteEventEntry` when it has an
/// item: `param[1]` not -1).
fn use_entry(cx: &mut Cx, o: &EntryObj) {
    if o.ent.param[1] != -1 {
        delete_event_entry(cx.save, cx.game.field, &o.ent);
    }
}

/// The trapped object's untrapped twin made in its place (`param[2]` 3,
/// row `id`), fully opaque; the old one leaves the list, `effRemoveTrap(pos,
/// -1, -1)`, sound 104.
fn disarm(ctrl: &mut EntryCtrl, cx: &mut Cx, seam: &mut dyn Seam, who: usize, o: &mut EntryObj, id: Option<i32>) {
    let mut ep = o.ent;
    ep.pos = cx.scene.chars[who].pos;
    ep.dirc = o.dirc;
    if let Some(id) = id {
        ep.id = id;
    }
    ep.param[2] = 3;
    if let Some(n) = ctrl.entry_object(cx, seam, &mut ep)
        && let Some(no) = ctrl.entry_obj_mut(n)
    {
        no.transparency = ONE;
        no.set_transparency = ONE;
    }
}

/// `ccGimBox::boxMain()` (gcmn 0x00453d80).
fn box_act(
    ctrl: &mut EntryCtrl,
    cx: &mut Cx,
    seam: &mut dyn Seam,
    in_battle: i32,
    who: usize,
    o: &mut EntryObj,
) -> bool {
    if o.affect_flag {
        o.affect_flag = false;
        match cx.scene.chars[who].affect.ty {
            11 => {
                o.act_num = 2;
                o.act_cnt = 0;
            }
            12 if o.gim_id == 1 => {
                disarm(ctrl, cx, seam, who, o, Some(0));
                o.fade_flag = 0;
                delete_cmnd(&mut o.cmnd_flag, cx.scene, cx.out, who, true);
                cx.out.push(Out::TrapRemoved { pos: cx.scene.chars[who].pos });
                cx.out.push(Out::Se2d(104));
                return true;
            }
            _ => {}
        }
    }
    fw2lw(cx, who);
    let pos = cx.scene.chars[who].pos;
    match o.act_num {
        0 => {
            if matches!(o.ent.ent_root, 1..=3) && in_battle == 0 {
                let c = o.act_cnt;
                o.act_cnt += 1;
                if c >= 901 {
                    o.act_num = 1;
                    o.act_cnt = 0;
                }
            }
        }
        1 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            if c == 0 {
                delete_cmnd(&mut o.cmnd_flag, cx.scene, cx.out, who, true);
                o.fade_flag = 2;
                o.fade_cnt = 30;
            }
            if o.act_cnt >= 31 {
                return true;
            }
        }
        2 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            if c == 0 {
                if o.gim_id == 1 {
                    invoke_trap(cx, who, o);
                }
                delete_cmnd(&mut o.cmnd_flag, cx.scene, cx.out, who, true);
                use_entry(cx, o);
                cx.world.anim_set(who, AnmSlot::Main, BOX_OPEN);
                o.anm_flag = 0;
                cx.out.push(Out::Se { se: 73, pos });
                cx.out.push(Out::OpenBox { pos });
                if let Class::Box(rad) = &mut o.class {
                    rad.rad_flag = true;
                }
            }
            if o.anm_flag != 0 && o.act_cnt >= 31 {
                o.act_num = 3;
                o.act_cnt = 0;
            }
        }
        3 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            if c == 0 {
                o.fade_flag = 2;
                o.fade_cnt = 30;
            }
            if o.act_cnt >= 31 {
                return true;
            }
        }
        _ => {}
    }
    o.hit.pos = cx.scene.chars[who].pos;
    false
}

/// `ccGimBox::objectMain()` (gcmn 0x00454140): the breakables.
fn object_act(ctrl: &mut EntryCtrl, cx: &mut Cx, seam: &mut dyn Seam, who: usize, o: &mut EntryObj) -> bool {
    if o.affect_flag {
        o.affect_flag = false;
        match cx.scene.chars[who].affect.ty {
            11 => {
                o.act_num = 1;
                o.act_cnt = 0;
            }
            12 if matches!(o.gim_id, 3 | 5) => {
                let id = if o.gim_id == 5 { 4 } else { 2 };
                disarm(ctrl, cx, seam, who, o, Some(id));
                delete_cmnd(&mut o.cmnd_flag, cx.scene, cx.out, who, true);
                cx.out.push(Out::TrapRemoved { pos: cx.scene.chars[who].pos });
                cx.out.push(Out::Se2d(104));
                return true;
            }
            _ => {}
        }
    }
    fw2lw(cx, who);
    match o.act_num {
        1 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            if c == 0 {
                if trapped(o.gim_id) {
                    invoke_trap(cx, who, o);
                } else {
                    break_object(cx, who, o);
                }
                delete_cmnd(&mut o.cmnd_flag, cx.scene, cx.out, who, true);
                use_entry(cx, o);
                o.fade_flag = 2;
                o.fade_cnt = 5;
            }
            if o.act_cnt >= 6 {
                o.act_num = 2;
                o.act_cnt = 0;
            }
        }
        2 if o.act_cnt == 0 => return true,
        _ => {}
    }
    o.hit.pos = cx.scene.chars[who].pos;
    false
}

/// `ccGimBox::virusMain()` (gcmn 0x00454410): the virus crystal.
fn virus_act(cx: &mut Cx, who: usize, o: &mut EntryObj) -> bool {
    if o.affect_flag {
        o.affect_flag = false;
        if cx.scene.chars[who].affect.ty == 11 {
            o.act_num = 1;
            o.act_cnt = 0;
        }
    }
    fw2lw(cx, who);
    match o.act_num {
        1 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            if c == 0 {
                delete_cmnd(&mut o.cmnd_flag, cx.scene, cx.out, who, true);
                use_entry(cx, o);
            }
            if o.act_cnt >= 29 {
                o.act_num = 2;
                o.act_cnt = 0;
            }
        }
        2 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            if c == 0 {
                o.fade_flag = 2;
                o.fade_cnt = 30;
                cx.out.push(Out::Se2d(75));
                let mut p = cx.scene.chars[who].pos;
                p[2] = add(p[2], geom::k(100.0));
                cx.out.push(Out::VirusCrystal { pos: p });
            }
            if o.act_cnt >= 31 {
                return true;
            }
        }
        _ => {}
    }
    o.hit.pos = cx.scene.chars[who].pos;
    false
}

/// `pos` raised by half the character's height.
fn mid(ch: &Char) -> V4 {
    let mut p = ch.pos;
    p[2] = add(p[2], div(ch.base().height, geom::k(2.0)));
    p
}

/// `ccGimBox::breakObject()` (gcmn 0x00453ac0): the debris by row
/// (barrels and wooden boxes 0, pots 1, bodies 2, eggs 3) with its sound
/// (86-89), from half the height up.
fn break_object(cx: &mut Cx, who: usize, o: &EntryObj) {
    let (what, se) = match o.gim_id {
        2 | 4 | 7 | 8 => (0, 86),
        9 | 10 => (1, 87),
        11 | 12 => (2, 88),
        13 | 14 => (3, 89),
        _ => return,
    };
    let ch = &cx.scene.chars[who];
    let p = mid(ch);
    cx.out.push(Out::Se { se, pos: ch.pos });
    cx.out.push(Out::Crush { what, pos: p });
}

/// `ccGimBox::invokeTrap()` (gcmn 0x00453be0): the party's message, the
/// trap on `affectPerson` (trap 0 the hit, 1 and 2 the skills: the
/// world's [`Call::TrapDamage`], [`Call::TrapSkill`]), sound 39,
/// `effOpenTrapBox(pos + height / 2, kind, trap)` (kind 0 the treasure
/// box, 1 the wooden box and barrel; trap 0, 3 or 4).
fn invoke_trap(cx: &mut Cx, who: usize, o: &EntryObj) {
    cx.out.push(Out::SpcMessage(MSG_OPEN_TRAP_BOX));
    let target = cx.scene.chars[who].affect.person;
    let sid = i32::from(cx.scene.chars[who].skill_id);
    let trap = match o.ent.param[2] {
        0 => {
            call(cx, who, Call::TrapDamage { target, sid });
            0
        }
        1 => {
            call(cx, who, Call::TrapSkill { target, sid });
            3
        }
        2 => {
            call(cx, who, Call::TrapSkill { target, sid });
            4
        }
        // Never for a trapped row; the game's register is then stale.
        _ => 0,
    };
    let ch = &cx.scene.chars[who];
    let p = mid(ch);
    cx.out.push(Out::Se { se: 39, pos: ch.pos });
    match o.gim_id {
        1 => cx.out.push(Out::OpenTrapBox { pos: p, kind: 0, trap }),
        3 | 5 => cx.out.push(Out::OpenTrapBox { pos: p, kind: 1, trap }),
        _ => {}
    }
}

/// A world call for object `who` with the tables, the scene and both
/// generators at hand.
fn call(cx: &mut Cx, who: usize, c: Call) {
    let mut none = crate::rand::Rand::new(0);
    let mut at = At { t: cx.t, scene: cx.scene, rand: &mut none, cc: cx.cc };
    cx.world.call(who, c, &mut at);
}

/// (0, 800, -530, 1): where the statue's glow and dust ring sit from it.
const STATUE_OFS: V4 = [0, 0x4448_0000, 0xc404_8000, ONE];

/// `ccGimIdol::main()` (gcmn 0x00459920).
fn idol_main(cx: &mut Cx, who: usize, o: &mut EntryObj) -> bool {
    if o.freeze_flag {
        return false;
    }
    let Class::Idol(idol) = &mut o.class else { return false };
    rays(cx, who, &mut idol.rad);
    if o.init_flag {
        if idol.effsw == 1 {
            let pos = cx.scene.chars[who].pos;
            let m = crate::geom::rot_matrix(&crate::geom::unit_matrix(), o.dirc);
            let v = crate::geom::apply_matrix(&m, STATUE_OFS);
            let at = crate::geom::vadd(v, pos);
            cx.out.push(Out::StatueOfGod { who, pos: at });
        }
        o.init_flag = false;
    }
    fw2lw(cx, who);
    let pos = cx.scene.chars[who].pos;
    match o.act_num {
        0 => {
            o.act_cnt += 1;
            if o.affect_flag {
                o.affect_flag = false;
                if cx.scene.chars[who].affect.ty == 11 {
                    o.act_num = 1;
                    o.act_cnt = 0;
                }
            }
        }
        1 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            if c == 0 {
                if idol.idol_id == 6 {
                    cx.out.push(Out::Se { se: 164, pos });
                    cx.out.push(Out::TrapRemoved { pos });
                } else {
                    cx.out.push(Out::Se { se: 73, pos });
                    cx.out.push(Out::OpenBox { pos });
                    idol.rad.rad_flag = true;
                }
                delete_cmnd(&mut o.cmnd_flag, cx.scene, cx.out, who, true);
                delete_event_entry(cx.save, cx.game.field, &o.ent);
                o.ent.param[2] = 0;
                if let Some(a) = idol_anim(idol.idol_id, 1) {
                    cx.world.anim_set(who, AnmSlot::Main, a);
                }
                o.anm_flag = 0;
            } else {
                if o.act_cnt == 100 {
                    cx.out.push(Out::Se { se: 56, pos });
                    idol.effsw = 0;
                }
                if o.act_cnt >= 101 && o.act_cnt & 1 == 0 {
                    cx.out.push(Out::DustRing { who, ofs: STATUE_OFS });
                }
                if o.anm_flag != 0 {
                    o.act_num = 2;
                    o.act_cnt = 0;
                }
            }
        }
        2 => {
            let c = o.act_cnt;
            o.act_cnt += 1;
            if c == 0
                && let Some(a) = idol_anim(idol.idol_id, 2)
            {
                cx.world.anim_set(who, AnmSlot::Main, a);
            }
        }
        _ => {}
    }
    o.anm_flag = i32::from(cx.world.anim_forward(who, AnmSlot::Main, 256));
    cx.out.push(Out::IdolDraw { who, dirc: o.dirc });
    false
}

/// `ccGimSymbol::main()` (gcmn 0x0045aa40): nothing while frozen; `posP` and
/// back; row 17's `symMain` (0x0045ab00), row 18's `objMain` ([`obj_main`]).
/// `symMain`: a used symbol starts spent; act 0 lights a fire every other
/// frame until affect 11; act 1 casts (frame 0: off the command lists, sound
/// 35, `effUseSymbol`, `ccItemSkillRequest(this, opener, trapNum, 0)`), a fire
/// and a spark each frame, and at frame 30 is spent with its light out.
fn symbol_main(cx: &mut Cx, who: usize, o: &mut EntryObj) -> bool {
    if o.freeze_flag {
        return false;
    }
    fw2lw(cx, who);
    if o.gim_id == 18 {
        let Class::Symbol(mut sym) = std::mem::replace(&mut o.class, Class::None) else { return false };
        let r = obj_main(cx, who, o, &mut sym);
        o.class = Class::Symbol(sym);
        return r;
    }
    if o.gim_id != 17 {
        return false;
    }
    let Class::Symbol(sym) = &mut o.class else { return false };
    if o.init_flag {
        if sym.act != 0 {
            sym.fires.iter_mut().for_each(|f| f.state = 0);
            sym.act = 2;
            sym.cnt = 0;
        }
        o.init_flag = false;
    }
    if sym.act != 2 {
        let intensity = add(ONE, crate::enemy_ai::rand_f(cx.cc, 0x3e4c_cccd));
        let q = cx.world.w2p(sym.light_pos);
        sym.light_pos = cx.world.p2w(q);
        cx.out.push(Out::SymbolLight { who, pos: Some(sym.light_pos), intensity });
        sym.light_on = true;
    }
    match sym.act {
        0 => {
            if sym.cnt & 1 != 0 {
                set_fire(cx, sym, 1, 1);
            }
            sym.cnt += 1;
            if o.affect_flag {
                o.affect_flag = false;
                if cx.scene.chars[who].affect.ty == 11 {
                    sym.act = 1;
                    sym.cnt = 0;
                }
            }
        }
        1 => {
            let c = sym.cnt;
            sym.cnt += 1;
            if c == 0 {
                delete_cmnd(&mut o.cmnd_flag, cx.scene, cx.out, who, true);
                o.ent.param[2] = 0;
                cx.out.push(Out::Se { se: 35, pos: cx.scene.chars[who].pos });
                cx.out.push(Out::UseSymbol { pos: sym.light_pos });
                let target = cx.scene.chars[who].affect.person;
                let sid = i32::from(cx.scene.chars[who].skill_id);
                call(cx, who, Call::TrapSkill { target, sid });
            } else if c == 30 {
                sym.act = 2;
                sym.cnt = 0;
                if sym.light_on {
                    cx.out.push(Out::SymbolLight { who, pos: None, intensity: 0 });
                    sym.light_on = false;
                }
            }
            // Also on the frame that spent it.
            set_fire(cx, sym, 1, 1);
            invoke_eff(cx, sym);
        }
        _ => {}
    }
    if o.disp_sw {
        let pos = cx.scene.chars[who].pos;
        o.disp_sw = cx.world.camera_deg(pos, 12288);
        if o.disp_sw {
            o.anm_flag = i32::from(cx.world.anim_forward(who, AnmSlot::Main, 256));
            cx.out.push(Out::IdolDraw { who, dirc: o.dirc });
            let mut out = std::mem::take(cx.out);
            for k in 0..sym.fires.len() {
                if sym.fires[k].state != 0 {
                    fire_main(cx, &mut sym.fires[k], sym.pat_num, Some(&mut out));
                }
            }
            *cx.out = out;
        }
    }
    false
}

/// `ccGimSymbol::objMain()` (gcmn 0x0045aed0), row 18's (the lakes' symbol,
/// no body): the light each frame; it casts as `symMain` does, and act 2
/// removes it with its light. On an even count it puffs `effSmoke` from 10 out
/// along a heading of three `ccRand()` turns, flying out along it and 2.5 up.
fn obj_main(cx: &mut Cx, who: usize, o: &mut EntryObj, sym: &mut Symbol) -> bool {
    let intensity = add(ONE, crate::enemy_ai::rand_f(cx.cc, 0x3e4c_cccd));
    let q = cx.world.w2p(sym.light_pos);
    sym.light_pos = cx.world.p2w(q);
    cx.out.push(Out::SymbolLight { who, pos: Some(sym.light_pos), intensity });
    sym.light_on = true;
    match sym.act {
        0 => {
            sym.cnt += 1;
            if o.affect_flag {
                o.affect_flag = false;
                if cx.scene.chars[who].affect.ty == 11 {
                    sym.act = 1;
                    sym.cnt = 0;
                }
            }
        }
        1 => {
            let c = sym.cnt;
            sym.cnt += 1;
            if c == 30 {
                sym.act = 2;
                sym.cnt = 0;
                if sym.light_on {
                    cx.out.push(Out::SymbolLight { who, pos: None, intensity: 0 });
                    sym.light_on = false;
                }
            } else if c == 0 {
                delete_cmnd(&mut o.cmnd_flag, cx.scene, cx.out, who, true);
                cx.out.push(Out::Se { se: 35, pos: cx.scene.chars[who].pos });
                cx.out.push(Out::UseSymbol { pos: sym.light_pos });
                let target = cx.scene.chars[who].affect.person;
                let sid = i32::from(cx.scene.chars[who].skill_id);
                call(cx, who, Call::TrapSkill { target, sid });
            }
        }
        2 => {
            // ccDestGimSymbol takes the light out as the object goes.
            cx.out.push(Out::SymbolLight { who, pos: None, intensity: 0 });
            sym.light_on = false;
            return true;
        }
        _ => {}
    }
    if sym.cnt & 1 == 0 {
        let (pos, v) = scatter(cx, sym, geom::k(2.5));
        cx.out.push(Out::SymbolSmoke { pos, v });
    }
    false
}

/// A heading of three `ccRand()` turns (X, Y, Z): the light's place 10 out
/// along it, and a velocity along it with `up` added to z.
fn scatter(cx: &mut Cx, sym: &Symbol, up: F) -> (V4, V4) {
    let mut m = geom::unit_matrix();
    m = geom::rot_matrix_x(&m, geom::deg2rad(cx.cc.rand() as u16 as i16));
    m = geom::rot_matrix_y(&m, geom::deg2rad(cx.cc.rand() as u16 as i16));
    m = geom::rot_matrix_z(&m, geom::deg2rad(cx.cc.rand() as u16 as i16));
    let d = geom::apply_matrix(&m, [geom::k(10.0), 0, 0, ONE]);
    let mut pos = sym.light_pos;
    for k in 0..3 {
        pos[k] = add(pos[k], d[k]);
    }
    let mut v = geom::apply_matrix(&m, [ONE, 0, 0, ONE]);
    v[2] = add(v[2], up);
    (pos, v)
}

/// `ccGimSymbol::invokeEff()` (gcmn 0x0045a860): a spark from the light's
/// place, 10 out along a heading of three `ccRand()` turns (X, Y, Z),
/// flying out along it and 2 up: `ccParticleExplode(pos, v, 1.5, 1)`.
fn invoke_eff(cx: &mut Cx, sym: &Symbol) {
    let (pos, v) = scatter(cx, sym, geom::k(2.0));
    cx.out.push(Out::SymbolSpark { pos, v });
}

// the setters -------------------------------------------------------------------------

/// `ccRotate(r, a)` (main 0x00102190): `r + a` back into -pi..pi (by 2 pi
/// the way `a` turned).
pub fn cc_rotate(r: F, a: F) -> F {
    let x = add(r, a);
    if !le(a, 0) {
        if !le(x, PI) { sub(x, TWO_PI) } else { x }
    } else if lt(x, geom::NEG_PI) {
        add(x, TWO_PI)
    } else {
        x
    }
}

/// `ccGetItemEditCode(code)` (main 0x00179100): an item named the edit
/// data's way (a letter for the category, `'1'`-`'6'` the weapons, `'A'`
/// `'B'` `'G'` `'H'` the armour, `'R'` `'D'` `'U'` `'X'` `'T'` `'E'` the
/// items, then three digits) as `category << 16 | index`: the row of that
/// category whose code is `code`; with no such row the index is the
/// code's low 24 bits; another letter gives 0.
pub fn item_edit_code(t: &Tables, code: i32) -> i32 {
    let letter = code >> 24;
    let low = code & 0x00ff_ffff;
    let (cat, codes): (i32, Vec<i32>) = match letter {
        82 => (10, t.items[0].iter().map(|i| i.code).collect()),
        68 => (11, t.items[1].iter().map(|i| i.code).collect()),
        85 => (12, t.items[2].iter().map(|i| i.code).collect()),
        88 => (13, t.items[3].iter().map(|i| i.code).collect()),
        84 => (14, t.items[4].iter().map(|i| i.code).collect()),
        69 => (15, t.items[5].iter().map(|i| i.code).collect()),
        71 => (8, t.armor[2].iter().map(|e| e.code).collect()),
        66 => (9, t.armor[3].iter().map(|e| e.code).collect()),
        72 => (6, t.armor[0].iter().map(|e| e.code).collect()),
        65 => (7, t.armor[1].iter().map(|e| e.code).collect()),
        49..=54 => {
            let k = (letter - 49) as usize;
            (k as i32, t.weapons[k].iter().map(|e| e.code).collect())
        }
        _ => return 0,
    };
    let idx = codes.iter().position(|&c| c == code).map_or(low, |i| i as i32);
    (cat << 16) | idx
}

/// The edit data's item word as `SetItemBox` spells it: the category
/// letter (the top half) and the low byte as three digits (hundreds, the
/// rest over ten, units).
fn box_item_code(flag: i32) -> i32 {
    let hi = flag >> 16;
    let v = flag & 0xff;
    let h = (v - v % 100) / 100 + 48;
    let t = (v - v % 10) / 10 + 48;
    let u = v % 10 + 48;
    (hi << 24) | (h << 16) | (t << 8) | u
}

/// The room row's item word as `SetIDOL` spells it: the top half the
/// letter, the low half as three digits.
fn idol_item_code(item: i32) -> i32 {
    let hi = item >> 16;
    let lo = item - (hi << 16);
    let h = lo / 100 + 48;
    let r = lo % 100;
    let t = r / 10 + 48;
    let u = r % 10 + 48;
    (hi << 24) | (h << 16) | (t << 8) | u
}

/// `SetItemBox`'s `kind` to gimmick row for the type-3 rows (gcmn
/// 0x006965f0, 53 words), as the executable holds it; word 5 is replaced
/// by `GetFood()` when the function copies it.
pub const KIND_GIMMICK: [i32; 53] = {
    let mut t = [0; 53];
    t[0] = 6;
    t[1] = 1;
    t[4] = 17;
    t[6] = 19;
    let mut k = 27;
    while k < 53 {
        t[k] = 6;
        k += 1;
    }
    t
};

/// `CheckBossEffect()` (gcmn 0x005b63a0): whether story area `field`'s
/// boss room still has its warning: not in areas 23, 25, 48, 71, 77 and
/// 101; in the areas below until bit 62 of their word of the save is set
/// (the boss beaten); in any other area always.
pub fn check_boss_effect(save: &piney_data::save::SaveData, field: i32) -> bool {
    let word = match field {
        23 | 25 | 48 | 71 | 77 | 101 => return false,
        17 => 0x5568,
        18 => 0x5580,
        19 => 0x5588,
        21 => 0x55a0,
        22 => 0x55b8,
        44 => 0x5838,
        45 => 0x5840,
        50 => 0x5880,
        72 => 0x5b68,
        74 => 0x5b80,
        75 => 0x5ba0,
        76 => 0x5ba8,
        100 => 0x5e68,
        103..=106 => 0x5eb0,
        _ => return true,
    };
    save.i32(word + 4) & 0x4000_0000 == 0
}

/// A special room (`ROOMDATA.type` 16-24 or 34): `SetItemBox` builds no
/// room for its rows.
fn special_room(d: &DungeonGims, floor: i32, block: i32) -> bool {
    let ty = d.rooms.iter().find(|r| r.ty >= 16 && r.floor == floor && r.block == block).map_or(0, |r| r.ty);
    matches!(ty, 16..=24 | 34)
}

/// `DUNGEON::SetItemBox()` (gcmn 0x005beb30): each kind-0 `gimPos` slot a
/// treasure box, a golden egg (`fieldrand(100)` under 10; a lake's food) or
/// the area's food (under 30), `entRoot` -1. In a story dungeon each
/// `GIMMICKDATA` row too, `entRoot` 0 with the next event entry number: type 0
/// a box at (x, y, 250), type 3 the row `KIND_GIMMICK` gives (kinds 7-26
/// skipped; the warning only with no banned room and while
/// [`check_boss_effect`]). The rooms `SetRoom` builds for the land checks are
/// the runtime's.
pub fn set_item_box(
    ctrl: &mut EntryCtrl,
    cx: &mut Cx,
    seam: &mut dyn Seam,
    d: &mut DungeonGims,
    fieldrand: &mut dyn FnMut(u32) -> u32,
) {
    while let Some(s) = crate::entry::take_slot(d, 0) {
        let mut ep = entry_param_clear();
        ep.pos = s.pos;
        ep.dirc = s.dirc;
        ep.ty = 1;
        let r = fieldrand(100) as i32;
        ep.id = 0;
        if r < 10 {
            ep.id = if d.dtype == 8 || d.dtype == 9 { d.food } else { 22 };
        } else if r < 30 {
            ep.id = d.food;
        }
        ep.area = 2;
        ep.floor = i32::from(s.floor);
        ep.block = i32::from(s.block);
        ep.ent_root = -1;
        ep.area_num = cx.game.dungeon;
        ctrl.entry_object(cx, seam, &mut ep);
    }
    if !d.story {
        return;
    }
    let Some(edit) = d.edit.clone() else { return };
    // The GetFood() the function asks first is stored over the table's
    // sixth word (its local copy at sp+240, the result at sp+260): kind 5
    // is the area's food.
    let mut table = KIND_GIMMICK;
    table[5] = d.food;
    for (row, g) in edit.iter().enumerate() {
        let mut ep = entry_param_clear();
        ep.pos = [from_int(g.x), from_int(g.y), 0, 0];
        ep.ty = 1;
        ep.area = 2;
        ep.floor = g.floor;
        ep.block = g.block;
        ep.ent_root = 0;
        ep.area_num = cx.game.dungeon;
        let dz = match g.direc {
            3 => geom::neg(HALF_PI),
            2 => HALF_PI,
            1 => PI,
            _ => 0,
        };
        ep.dirc = [0, 0, dz, 0];
        let _ = special_room(d, g.floor, g.block);
        match g.ty {
            0 => {
                ep.pos = [from_int(g.x), from_int(g.y), geom::k(250.0), 0];
                ep.id = match g.kind {
                    1 => 2,
                    2 => 4,
                    _ => 0,
                };
                if g.flag != 0 {
                    ep.param[1] = item_edit_code(cx.t, box_item_code(g.flag));
                }
                ep.param[0] = d.counter;
                d.counter = d.counter.wrapping_add(1);
                ctrl.entry_object(cx, seam, &mut ep);
            }
            3 => {
                if (7..27).contains(&g.kind) {
                    continue;
                }
                let Some(&id) = usize::try_from(g.kind).ok().and_then(|k| table.get(k)) else { continue };
                ep.id = id;
                if id == 19 {
                    // A banned room somewhere: no warning, and no number
                    // taken. Else at the nearest gate or door of the
                    // room, the next event entry number, made while the
                    // area's boss stands (CheckBossEffect).
                    if d.ban_room {
                        continue;
                    }
                    if let Some(Some(p)) = d.warn_pos.get(row) {
                        ep.pos = *p;
                    }
                    ep.param[0] = d.counter;
                    d.counter = d.counter.wrapping_add(1);
                    if check_boss_effect(cx.save, cx.game.field) {
                        ctrl.entry_object(cx, seam, &mut ep);
                    }
                    continue;
                }
                if g.kind >= 27 {
                    ep.param[1] = (g.kind - 27) | 0xf_0000;
                }
                if g.kind == 0 {
                    ep.param[1] = 0;
                }
                ep.param[0] = d.counter;
                d.counter = d.counter.wrapping_add(1);
                ctrl.entry_object(cx, seam, &mut ep);
            }
            _ => {}
        }
    }
}

/// `DUNGEON::SetIDOL()` (gcmn 0x005be750): every `gimPos` slot of kind 2
/// in turn an idol (`WORLD_MAN.timeSym` 1 outside the lake types: the Zeit
/// statue, row 44; a lake dungeon (`lakeFlag`): the spring, 20; else by
/// `GetFieldAttrb()` 0-5: 42, 38, 39, 40, 41, 43), `entRoot` -1, `land` 0,
/// facing the room's turn + 3.14 (`ccRotate`); in a story dungeon
/// `entRoot` 0, the next event entry number, and the item the room's row
/// holds.
pub fn set_idol(ctrl: &mut EntryCtrl, cx: &mut Cx, seam: &mut dyn Seam, d: &mut DungeonGims) {
    while let Some(s) = crate::entry::take_slot(d, 2) {
        let mut ep = entry_param_clear();
        ep.pos = s.pos;
        ep.ty = 1;
        if d.time_sym == 1 && d.dtype != 8 && d.dtype != 9 {
            ep.id = 44;
        } else if d.lake {
            ep.id = 20;
        } else {
            match d.field_attr {
                0 => ep.id = 42,
                1 => ep.id = 38,
                2 => ep.id = 39,
                3 => ep.id = 40,
                4 => ep.id = 41,
                5 => ep.id = 43,
                _ => {}
            }
        }
        ep.area = 2;
        ep.floor = i32::from(s.floor);
        ep.block = i32::from(s.block);
        ep.land = 0;
        let turn = d
            .rotate
            .get(usize::try_from(ep.floor).unwrap_or(usize::MAX))
            .and_then(|f| f.get(usize::try_from(ep.block).unwrap_or(usize::MAX)))
            .copied()
            .unwrap_or(0);
        ep.dirc[2] = cc_rotate(turn, 0x4048_f5c3);
        ep.ent_root = -1;
        ep.area_num = cx.game.dungeon;
        if d.story {
            ep.ent_root = 0;
            ep.param[0] = d.counter;
            let item = d.rooms.iter().find(|r| r.floor == ep.floor && r.block == ep.block).map_or(0, |r| r.item);
            if item != 0 {
                ep.param[1] = item_edit_code(cx.t, idol_item_code(item));
            }
            d.counter = d.counter.wrapping_add(1);
        }
        ctrl.entry_object(cx, seam, &mut ep);
    }
}

/// `DUNGEON::EntryBreakObject()` (gcmn 0x005bff10)'s guard: nothing in the
/// lake types, in the tutorial's dungeon (`game.field` 14), or in a story
/// room with an event; otherwise its breakables are placed from the room's
/// `OBJ_0pr2*`-`OBJ_0pr7*` dummies (`EntryBreakObjectMain`: piney-world's
/// `DungeonArea::breakables_here` and `Combat::start_entries`).
pub fn break_objects_placed(d: &DungeonGims, field: i32, floor: i32, block: i32) -> bool {
    if d.dtype == 8 || d.dtype == 9 || field == 14 {
        return false;
    }
    !d.rooms.iter().any(|r| r.event != 0 && r.floor == floor && r.block == block)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_words_spell_three_digits() {
        // D0001's rows: 'R' 005 and 'D' 056 (the flag's low byte 0x38).
        assert_eq!(box_item_code(5373957), 0x5230_3035);
        assert_eq!(box_item_code(4456504), 0x4430_3536);
        assert_eq!(idol_item_code(3211265), 0x3130_3031);
    }

    #[test]
    fn kind_table_matches_the_game() {
        assert_eq!(&KIND_GIMMICK[..8], &[6, 1, 0, 0, 17, 0, 19, 0]);
        assert!(KIND_GIMMICK[8..27].iter().all(|&x| x == 0));
        assert!(KIND_GIMMICK[27..].iter().all(|&x| x == 6));
    }

    #[test]
    fn rotate_wraps_the_way_it_turns() {
        let r = cc_rotate(PI, 0x4048_f5c3);
        assert!(lt(r, 0));
        assert_eq!(cc_rotate(0, 0x4048_f5c3), 0x4048_f5c3);
    }
}
