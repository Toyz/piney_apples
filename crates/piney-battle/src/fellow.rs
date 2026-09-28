//! A party member's frame: `ccFellow` (`fellow.cpp`, gcmn 0x0041b5f0-0x0041e46c)
//! and the `ccSpcChar` pieces it calls (`SetActNum`/`SetActNumOld`
//! 0x0059d630/0x0059d640, `HitCheck` 0x0059ee20). [`Frame::main`] is
//! `ccFellow::Main`; the order, the acts and the affects in the frame are in
//! docs/engine/battle.md ("A party member's frame"). The flags the affects and
//! the decisions share live twice, in [`Char`] and [`Spc`]: [`sync_in`] copies
//! the first into the second as a frame starts, [`sync_out`] back as it ends.
//! The world is [`FellowWorld`], the rest of the game [`Runtime`].

use piney_data::save::SaveData;

use crate::affect::{self, AffectCtx};
use crate::chara::{self, Char, Env, cmod, spc_flag};
use crate::event::{Event, Events, Who};
use crate::exp::Party;
use crate::follow;
use crate::geom::{F, ONE, V4, add, atan2f, cosf, div, fptoui, from_int, le, lt, mul, neg, sinf, sqrtf, sub};
use crate::param::{AiParam, F_ONE, cond};
use crate::party_ai::{Call, Crew, Ctx, Game, Runtime, Spc, skill_check_type};
use crate::rand::Rng;
use crate::scene::Scene;
use crate::tables::Tables;
use crate::world::{AnmSlot, Note, World};

/// The acts (`actNum`), as `fellowAnimTbl` names their clips.
pub mod act {
    /// Standing (`nut0`), in battle; 1 its fidget.
    pub const STAND: i16 = 0;
    pub const FIDGET: i16 = 1;
    /// Standing at ease (`nut2`); 3 and 4 its fidget.
    pub const EASE: i16 = 2;
    pub const EASE_FIDGET: i16 = 3;
    pub const EASE_FIDGET_END: i16 = 4;
    pub const RUN: i16 = 5;
    pub const WALK: i16 = 6;
    /// Hurt (7, 8), down (9), lying down (10).
    pub const HURT: i16 = 7;
    pub const DOWN: i16 = 9;
    pub const LYING: i16 = 10;
    /// Transferring out of the area (12), away (14), in (13).
    pub const TRANSFER_OUT: i16 = 12;
    pub const TRANSFER_IN: i16 = 13;
    pub const AWAY: i16 = 14;
    /// The normal attack's swings.
    pub const ATTACK: i16 = 15;
    pub const ATTACK2: i16 = 16;
    /// Spells and arts by their skill type bit (0x100 ... 0x1000).
    pub const SKILL: i16 = 17;
}

/// What a member's frame reads from the executable and the overlay: the
/// clip names of its acts and the following's constants.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MotionTables {
    /// `fellowAnimTbl` of `charTbl` rows 1-17 (index row - 1).
    pub anim: Vec<Vec<String>>,
    /// `fpAngleOffset[-1..=3]`: the halfword before the array (the high
    /// half of `epitaphStrM3[1]`), then its four.
    pub fp_angle_offset: [u16; 5],
    /// `fpOkRange` (120.0).
    pub ok_range: F,
    /// `CFOFP` (2.5).
    pub cfofp: F,
}

impl MotionTables {
    /// The volume's (`tables::combat`): `fellowAnimTbl` (INF gcmn
    /// 0x005d4050, one per `fellowNN.cpp`, 0x60 apart), `fpAngleOffset[4]`
    /// (INF main 0x003782a0, read from the halfword before it:
    /// `checkPartyMenberNum` -1 indexes it), `fpOkRange` and `CFOFP` (INF
    /// main 0x003782a8, 0x003782ac).
    pub fn of(volume: piney_data::volume::Volume) -> MotionTables {
        let t = piney_data::tables::combat::of(volume);
        let o = t.fp_angle_offset();
        MotionTables {
            anim: t.fellow_anims().iter().map(|r| r.iter().map(|s| s.to_string()).collect()).collect(),
            fp_angle_offset: [t.fp_angle_before(), o[0], o[1], o[2], o[3]],
            ok_range: t.fp_ok_range().to_bits(),
            cfofp: t.cfofp().to_bits(),
        }
    }

    /// `motionTbl[act]` of a member whose `motionTbl` is row `row`'s
    /// table; empty outside the tables (never reached: acts stay 0-21).
    pub fn clip(&self, row: i16, act: i16) -> &str {
        usize::try_from(row - 1)
            .ok()
            .and_then(|r| self.anim.get(r))
            .and_then(|t| usize::try_from(act).ok().and_then(|a| t.get(a)))
            .map_or("", |s| s.as_str())
    }
}

/// The world queries of a member's frame beyond [`World`].
pub trait FellowWorld: World {
    /// `WORLD_MAN::GetTransMode()` (main 0x001a3ad0): the party is carried
    /// (the member keeps its place on the carrier, `diskOffset`).
    fn trans_mode(&mut self) -> bool;
    /// `WORLD_MAN::GetTransCenter(out)` (main 0x001a3b10): the carrier's
    /// centre.
    fn trans_center(&mut self) -> V4;
    /// `WORLD_MAN::CheckEventArea()` (main 0x001a2180): the field's event
    /// area (the collision runs whatever the distance).
    fn event_area(&mut self) -> bool;
    /// `ccChar::Draw()` (gcmn 0x0056b1c0) on `who`: whether it was drawn
    /// (the arms effect runs only then).
    fn draw(&mut self, who: usize) -> bool;
}

/// The field's globals a frame reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Field {
    /// `ghoFlag` (main 0x00378cc0, `ccCheckGtHackAnm`): no AI runs.
    pub gho_flag: bool,
    /// `WORLD_MAN.warpFlag` (+0x168): the transfer in is a warp's.
    pub warp_flag: bool,
}

/// What a frame hands to presentation, in call order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Out {
    /// An event of the rules, characters named by scene index: each
    /// `EntryAffect` made (already applied), what the affect showed (the
    /// numbers, hit marks, the panel, `ccEnemy::selectTarget` of an enemy
    /// hit ...), `CalcBattleDamage`'s effects, `ChatMessageAttack`,
    /// `DispConditionEffect`.
    Rule(Event),
    /// `ccSpcChar::EquipWeapon()` (0x0059d650), `DeleteWeaponCCS()`
    /// (0x0059dda0): the weapon model swapped (`weaponChangeSW` 1, -1).
    EquipWeapon,
    DeleteWeaponCcs,
    /// `effLevelUp(this)`.
    LevelUp,
    /// `effOpenBox(&pos)`: the body lying down fades (made twice).
    OpenBox {
        pos: V4,
    },
    /// `ccDeleteCmnd(this)` (0x00519700) took it off the lists: the menu's
    /// target moves on if it was this (`ccChangeCmndTarget(0)`).
    DeleteCmnd,
    /// `ccCoord::SetMatrix_PosRotZYX(anm, pos, dirc)`.
    SetMatrix {
        pos: V4,
        dirc: V4,
    },
    /// `ccSpcChar::ArmsEffect()` (0x0059ddd0), `ClearArmsEffect()`
    /// (0x0059e3b0).
    ArmsEffect,
    ClearArmsEffect,
    /// `ccSpcChar::SetArmsEffectColor(sid)` (0x0059e460),
    /// `StartArmsEffect(sid)` (0x0059e530).
    ArmsEffectColor(i16),
    StartArmsEffect(i16),
    /// `effSkillStart(this, sid, flag, 0)`: `flag` is `skillStatus` bit 3
    /// (an item's).
    SkillStart {
        sid: i16,
        flag: i32,
    },
    /// `effTransfer(this)`, `effWarpTransfer(this)`.
    Transfer,
    WarpTransfer,
    /// `ccSeSetParamSPC(param, this)`: a footstep's sound (notes 1, 2).
    Se {
        param: u32,
    },
    /// `ccEffPawSmoke(this, speed)`: running dust.
    PawSmoke {
        speed: F,
    },
}

const F_3: F = 0x4040_0000;
const F_8: F = 0x4100_0000;
const HALF: F = 0x3f00_0000;
const F_20: F = 0x41a0_0000;
const F_30: F = 0x41f0_0000;
const F_50: F = 0x4248_0000;
const F_78: F = 0x429c_0000;
const F_256: F = 0x4380_0000;
/// 1.375f: the run clip's speed over `speedRate`.
const F_1_375: F = 0x3fb0_0000;
const F_7000: F = 0x45da_c000;
/// `ccLandHitCheck`'s mask for the ground, `ccHitCheckLM2`'s for walls.
pub const LAND_MASK: u32 = 0x2000_0001;
pub const WALL_MASK: u32 = 0x4000_0001;

/// Bits of the word at +0xe0 ([`crate::chara::SpcChar::flags`]) beyond
/// [`spc_flag`].
pub mod flag {
    pub const DISP: u32 = 1 << 1;
    pub const RUN: u32 = 1 << 5;
    /// `weaponChangeSW`, a signed 2-bit field.
    pub const WEAPON_SHIFT: u32 = 8;
    pub const EXIT: u32 = 1 << 10;
    pub const RECALL: u32 = 1 << 12;
}

/// The [`Char`] copies of the fields [`Spc`] also holds, into the
/// [`Spc`]: `actNum`, `targetChar`, `moveFlag`, `stopFlag`, `runFlag`,
/// `ghostFlag`.
pub fn sync_in(scene: &Scene, crew: &mut Crew, me: usize) {
    let c = &scene.chars[me];
    let f = c.spc_char.flags;
    let s = crew.spc.entry(me).or_default();
    s.act_num = c.spc_char.act_num;
    s.target_char = c.target_char;
    s.move_flag = f & spc_flag::MOVE != 0;
    s.stop_flag = f & spc_flag::STOP != 0;
    s.run_flag = f & flag::RUN != 0;
    s.ghost = f & spc_flag::GHOST != 0;
}

/// The [`Spc`]'s back into the [`Char`], and `hitSW` into `hit_enabled`.
pub fn sync_out(scene: &mut Scene, crew: &Crew, me: usize) {
    let Some(s) = crew.spc.get(&me) else { return };
    let c = &mut scene.chars[me];
    c.spc_char.act_num = s.act_num;
    c.target_char = s.target_char;
    let mut f = c.spc_char.flags & !(spc_flag::MOVE | spc_flag::STOP | flag::RUN | spc_flag::GHOST);
    for (on, b) in [(s.move_flag, spc_flag::MOVE), (s.stop_flag, spc_flag::STOP), (s.run_flag, flag::RUN)] {
        if on {
            f |= b;
        }
    }
    if s.ghost {
        f |= spc_flag::GHOST;
    }
    c.spc_char.flags = f;
    c.spc_char.hit_enabled = s.body_hit.sw;
    c.spc_char.sys_msg_id = crew.sys_msg_id(me);
}

/// `ccEntryCmnd(ch)` (gcmn 0x00519630): onto the end of the command list
/// of its kind (the party's for type bits 0x7, the foes' for 0xe0, else
/// the objects'), unless on one already.
pub fn entry_cmnd(scene: &mut Scene, ch: usize) {
    if scene.listed(ch) {
        return;
    }
    let ty = scene.chars[ch].ty();
    if ty & 7 != 0 {
        scene.pc_list.push(ch);
    } else if ty & 0xe0 != 0 {
        scene.ene_list.push(ch);
    } else {
        scene.obj_list.push(ch);
    }
}

/// `ccDeleteCmnd(ch)` (gcmn 0x00519700): off the command list of its
/// kind. Returns whether it was on the lists.
pub fn delete_cmnd(scene: &mut Scene, ch: usize) -> bool {
    if !scene.listed(ch) {
        return false;
    }
    let ty = scene.chars[ch].ty();
    let l = if ty & 7 != 0 {
        &mut scene.pc_list
    } else if ty & 0xe0 != 0 {
        &mut scene.ene_list
    } else {
        &mut scene.obj_list
    };
    if let Some(i) = l.iter().position(|&c| c == ch) {
        l.remove(i);
    }
    true
}

/// `ccSpcChar.partyFlag`, the signed 3-bit field.
fn party_flag(c: &Char) -> i32 {
    ((c.party_flag & 7) << 29) >> 29
}

/// Everything a member's frame reads and writes, borrowed for the frame.
pub struct Frame<'a> {
    pub t: &'a Tables,
    pub mt: &'a MotionTables,
    pub scene: &'a mut Scene,
    pub party: &'a Party,
    pub save: &'a mut SaveData,
    pub crew: &'a mut Crew,
    pub game: &'a Game,
    pub field: &'a Field,
    /// `g_entCtrl`'s enemies, in order (for the decisions).
    pub ents: &'a [usize],
    /// The rules' environment (`CalcReal`'s timers, the normal attack's
    /// hit); `sp_regene_speed` is worked out here for the member.
    pub env: &'a Env,
    pub rng: &'a mut dyn Rng,
    /// The world; with [`crate::party_motion::Movement`] for [`Frame::rt`],
    /// a [`crate::party_motion::Share`] of the cell the movement holds:
    ///
    /// ```text
    /// let cell = RefCell::new(&mut world);
    /// let mut rt = Movement { world: &cell, keep: &mut keep, inner: &mut own, .. };
    /// Frame { world: &mut Share::new(&cell), rt: &mut rt, .. }.main(me);
    /// ```
    pub world: &'a mut dyn FellowWorld,
    /// The rest of the game: skill requests and everything the decisions
    /// call, the following and the navigation included
    /// ([`crate::party_motion::Movement`]).
    pub rt: &'a mut dyn Runtime,
    /// What the affects need beyond the characters ([`AffectCtx`]): `ccMenu`
    /// exists (the party panel can shake), and `ccSkillCheck(ch)` (gcmn
    /// 0x005723e0, [`crate::flow::Skills::check`]): the id of a character's
    /// running skill a hit interrupts.
    pub menu: bool,
    pub skill_check: &'a dyn Fn(usize) -> i32,
    /// What the frame hands to presentation, in order.
    pub out: Vec<Out>,
}

impl Frame<'_> {
    fn spc(&mut self, me: usize) -> &mut Spc {
        self.crew.spc.entry(me).or_default()
    }

    fn sp(&self, me: usize) -> Spc {
        self.crew.spc.get(&me).copied().unwrap_or_default()
    }

    fn flags(&mut self, me: usize) -> &mut u32 {
        &mut self.scene.chars[me].spc_char.flags
    }

    fn set_flag(&mut self, me: usize, bit: u32, on: bool) {
        let f = self.flags(me);
        if on {
            *f |= bit;
        } else {
            *f &= !bit;
        }
    }

    fn has_ai(&self, me: usize) -> bool {
        self.crew.ais.contains_key(&me)
    }

    /// `ai->manualSW` (the code reads it without testing for an AI).
    fn manual(&self, me: usize) -> bool {
        self.crew.ais.get(&me).is_some_and(|a| a.manual_sw)
    }

    fn ai_param(&self, me: usize) -> AiParam {
        self.crew.ais.get(&me).and_then(|a| self.t.ai_params.get(a.param)).copied().unwrap_or_default()
    }

    /// A rule's events at the point the rule makes them: each
    /// `EntryAffect` among them applied at once ([`Frame::entry_affect`]),
    /// the rest handed on ([`Frame::effect`]).
    fn rules(&mut self, me: usize, ev: Events) {
        for mut e in ev {
            e.map_who(|w| if w == Who::Me { Who::Char(me) } else { w });
            match e {
                Event::Affect { on: Who::Char(on), by, kind, p } => {
                    let by = if let Who::Char(i) = by { Some(i) } else { None };
                    self.entry_affect(me, on, by, kind, p);
                }
                e => self.effect(e),
            }
        }
    }

    /// `on->EntryAffect(by, kind, p0, p1, p2)` (gcmn 0x0056b020) made now,
    /// as the game makes it inside the rule: the affect and the
    /// character's `affectFunc` ([`crate::affect::entry_affect`];
    /// `ccFellow::Influence` 0x0041bdb0, `Influence` 0x0059ac50,
    /// `ccEnemyInfluence` 0x00432840), then what they call, in order
    /// ([`Frame::effect`]). The member's two copies of its fields are made
    /// one around it ([`sync_out`], [`sync_in`]), and the character's AI id
    /// (`CheckSysMsgID`) is the bus's.
    fn entry_affect(&mut self, me: usize, on: usize, by: Option<usize>, kind: i16, p: [i16; 3]) {
        let by_w = by.map_or(Who::Nobody, Who::Char);
        self.out.push(Out::Rule(Event::Affect { on: Who::Char(on), by: by_w, kind, p }));
        sync_out(self.scene, self.crew, me);
        let id = self.crew.sys_msg_id(on);
        self.scene.chars[on].spc_char.sys_msg_id = id;
        let ctx = AffectCtx {
            party: self.party,
            menu: self.menu,
            skill_check: self.skill_check,
            boss: None,
            volume: self.t.volume,
        };
        let mut ev = Events::new();
        affect::entry_affect(self.t, self.scene, &ctx, on, by, kind, p, self.rng, &mut ev);
        sync_in(self.scene, self.crew, me);
        if on != me && self.scene.chars[on].ty() & 7 != 0 && self.crew.spc.contains_key(&on) {
            // Another party member's decisions' copies follow the affect.
            sync_in(self.scene, self.crew, on);
        }
        for e in ev {
            self.effect(e);
        }
    }

    /// An event of a rule or an affect: what is logic in the game is done
    /// here - `ccSkillRequest(ch, 0, 0)` (the runtime's), the body out of
    /// or into the collision (`ccCharHit::HitDisable`/`HitEnable`, the
    /// world's), the party's "down" (`ccAISysMsgSendP(0x1000c, -1, id,
    /// 0xffff, 0, 30, -1, ch)`) and "up" (`ccAISysMsgDeleteDelay(0x1000d,
    /// id, id)`) on the bus, the AI's talk ended (`talkFlag` 0); the rest
    /// goes to presentation.
    fn effect(&mut self, e: Event) {
        match e {
            Event::CancelAttack(Who::Char(i)) => self.cancel(i),
            Event::HitDisable(Who::Char(i)) | Event::HitEnable(Who::Char(i)) => {
                let on = matches!(e, Event::HitEnable(_));
                let mut hit = self.sp(i).body_hit;
                self.world.hit_switch(i, &mut hit, on);
                self.spc(i).body_hit = hit;
            }
            Event::SysMsgDown { on: Who::Char(i), id } => {
                self.crew.send(0x1000c, id as u16, 0xffff, 0, 30, -1, Some(i));
            }
            Event::SysMsgUp { id, .. } => {
                self.crew.sys.delete_delay(0x1000d, id as u16, id as u16);
            }
            Event::TalkOff(Who::Char(i)) => {
                if let Some(a) = self.crew.ais.get_mut(&i) {
                    a.talk_flag = false;
                }
            }
            e => self.out.push(Out::Rule(e)),
        }
    }

    /// `ccChar::CalcReal(flag)` (gcmn 0x0056ba30) on the member, its
    /// timers' affects made where `ConditionTimeCount` makes them: the
    /// stats and maxima ([`crate::chara::calc_real`] without the timers),
    /// then with `flag` 0 the timers ([`Frame::condition_time_count`]),
    /// then the equipment's condition effects and the buffs as the timers
    /// left them (the same again), and with `flag` 0 or below outside the
    /// Root Town the condition's effect shown.
    pub fn calc_real(&mut self, me: usize, flag: i32, env: &Env) {
        let mut ev = Events::new();
        chara::calc_real(self.t, &mut self.scene.chars[me], 1, env, &mut ev);
        if flag == 0 {
            self.condition_time_count(me, env);
        }
        chara::calc_real(self.t, &mut self.scene.chars[me], 1, env, &mut ev);
        self.rules(me, ev);
        if flag <= 0 && env.area != 0 {
            self.out.push(Out::Rule(Event::DispCondition(Who::Char(me))));
        }
    }

    /// `ccChar::ConditionTimeCount` (gcmn 0x0056bfd0) on the member, as
    /// [`crate::chara::condition_time_count`] but with each tick's
    /// `EntryAffect` (9 HP, 3 poison, 10 SP, 4 curse, on itself) made at
    /// once, so that what follows sees it (a member poisoned down loses its
    /// conditions before the SP ticks); `CheckSpRegeneSpeed` is asked where
    /// the game asks it.
    fn condition_time_count(&mut self, me: usize, env: &Env) {
        const TIMED: [usize; 12] = [0, 1, 2, 4, 5, 6, 8, 9, 10, 11, 12, 13];
        let at_least_1 = |v: i32| if v == 0 { 1 } else { v } as i16;
        let ch = &mut self.scene.chars[me];
        if let Some((temp, time)) = ch.temp_time_mut() {
            for i in TIMED {
                let t = time[i];
                if t > 0 {
                    time[i] = t - 1;
                    if t - 1 == 0 {
                        temp[i] = 0;
                    }
                } else if t < 0 {
                    time[i] = 0;
                    temp[i] = 0;
                }
            }
        }
        let is_pc = ch.is_pc();
        let c = &mut ch.cond;
        for i in [cond::SLEEP, cond::CONFUSION, cond::CHARM, cond::PARALYSIS] {
            if c[i] > 0 {
                c[i] -= 1;
            } else if c[i] < 0 {
                c[i] = 0;
            }
        }
        if c[cond::SPEED] > 0 {
            c[cond::SPEED] -= 1;
            if c[cond::SPEED] == 0 {
                c.speed_value = F_ONE;
            }
        } else if c[cond::SPEED] < 0 {
            c[cond::SPEED] = 0;
            c.speed_value = F_ONE;
        }
        if c[cond::REGENE_HP] != 0 {
            c[cond::REGENE_HP] = c[cond::REGENE_HP].wrapping_sub(1);
            if cmod(i32::from(c[cond::REGENE_HP]), 60) == 0 {
                let v = at_least_1(i32::from(ch.max_hp) / 50);
                self.entry_affect(me, me, Some(me), 9, [v, 0, 0]);
            }
        }
        let ch = &mut self.scene.chars[me];
        if ch.cond[cond::POISON] != 0 && env.menu_type == -1 {
            ch.cond[cond::POISON] = ch.cond[cond::POISON].wrapping_sub(1);
            if cmod(i32::from(ch.cond[cond::POISON]), 90) == 0 && !(is_pc && ch.no_death) {
                let v = at_least_1(i32::from(ch.max_hp) / 100);
                self.entry_affect(me, me, Some(me), 3, [v, 0, 0]);
            }
        }
        if self.scene.chars[me].cond[cond::CURSE] == 0 {
            let regene = !self.sp(me).move_flag && self.scene.chars[me].skill_id == 0;
            let ch = &mut self.scene.chars[me];
            let tick = env.count.is_multiple_of(60);
            let max_sp = i32::from(ch.max_sp);
            if is_pc {
                let div = match (regene, env.in_battle != 0) {
                    (true, true) => 50,
                    (true, false) => 20,
                    (false, true) => 100,
                    (false, false) => 33,
                };
                if tick {
                    ch.sp = ch.sp.wrapping_add(at_least_1(max_sp / div));
                }
                if ch.sp > ch.max_sp {
                    ch.sp = ch.max_sp;
                }
            } else if tick {
                ch.sp = ch.sp.wrapping_add(at_least_1(max_sp / 50));
                if ch.sp > ch.max_sp {
                    ch.sp = ch.max_sp;
                }
            }
        }
        let ch = &mut self.scene.chars[me];
        if ch.cond[cond::REGENE_SP] != 0 {
            ch.cond[cond::REGENE_SP] = ch.cond[cond::REGENE_SP].wrapping_sub(1);
            if cmod(i32::from(ch.cond[cond::REGENE_SP]), 60) == 0 {
                let v = at_least_1(i32::from(ch.max_sp) / 50);
                self.entry_affect(me, me, Some(me), 10, [v, 0, 0]);
            }
        }
        let ch = &mut self.scene.chars[me];
        if ch.cond[cond::CURSE] != 0 {
            ch.cond[cond::CURSE] = ch.cond[cond::CURSE].wrapping_sub(1);
            if cmod(i32::from(ch.cond[cond::CURSE]), 90) == 0 {
                let v = at_least_1(i32::from(ch.max_sp) / 100);
                self.entry_affect(me, me, Some(me), 4, [v, 0, 0]);
            }
        }
    }

    /// `ccSkillRequest(this, 0, 0)`: the running normal attack is called
    /// off.
    fn cancel(&mut self, me: usize) {
        let c = Call::SkillRequest { me, target: None, sid: 0 };
        self.rt.call(c, self.scene, self.crew, self.rng);
    }

    /// `ccSkillDamageValue(this, tp, sid)` (0x00594e90), 0 with no target.
    fn value(&self, me: usize, tp: Option<usize>, sid: i32) -> i32 {
        let Some(tp) = tp else { return 0 };
        let (a, b) = (self.scene.listed(me), self.scene.listed(tp));
        crate::damage::skill_damage_value(
            self.t,
            &self.scene.chars[me],
            &self.scene.chars[tp],
            sid,
            a,
            b,
            &Env::default(),
        )
        .dmg
    }

    /// `ccAISysMsgSendP(0x10008, -1, id, id, 0, 0, estimate, tp)`: the
    /// damage about to be dealt, to the party at once.
    fn announce(&mut self, me: usize, sid: i32) {
        let tp = self.sp(me).target_char;
        let id = self.crew.sys_msg_id(me) as u16;
        let v = self.value(me, tp, sid);
        self.crew.send(0x10008, id, id, 0, 0, v, tp);
    }

    /// `targetChar` on the lists and not down (`dead` 0).
    fn target_up(&self, me: usize) -> bool {
        self.sp(me).target_char.is_some_and(|t| self.scene.listed(t) && self.scene.chars[t].cond[cond::DEAD] == 0)
    }

    /// The normal attack's reach: `ccSpcChar::DistanceToTarget` within the
    /// AI's `armsRange`, the AI's `distTg` standing in for a distance of
    /// exactly -1.0.
    fn in_arms(&self, me: usize) -> bool {
        let Some(tg) = self.sp(me).target_char else { return false };
        let d = crate::flow::spc_distance_to_target(self.scene, me, tg);
        let arms = self.ai_param(me).arms_range;
        let d = if crate::geom::eq(crate::geom::MINUS_ONE, d) {
            self.crew.ais.get(&me).map_or(0, |a| a.dist_tg)
        } else {
            d
        };
        le(d, arms)
    }

    /// Face `targetChar` (on the lists, not itself): `dirc.z =
    /// atan2f(dx, -dy)` of their `posP`; under manual control also
    /// `ccAI::SetDircZ(RAD2DEG(dirc.z))`.
    fn face_target(&mut self, me: usize) {
        let Some(tg) = self.sp(me).target_char.filter(|&t| self.scene.listed(t) && t != me) else { return };
        let (t, p) = (self.scene.chars[tg].pos_p, self.scene.chars[me].pos_p);
        let h = atan2f(sub(t[0], p[0]), neg(sub(t[1], p[1])));
        self.spc(me).dirc[2] = h;
        if self.manual(me) {
            follow::set_dirc_z(self.crew, me, crate::geom::rad2deg(h) as u16);
        }
    }

    /// An attack swing starts: act `a`, the counters and `anmFlag` 0,
    /// `restraintSW` and `trajectorySW` set.
    fn swing(&mut self, me: usize, a: i16) {
        self.spc(me).act_num = a;
        let c = &mut self.scene.chars[me];
        c.anm_flag = 0;
        c.spc_char.flags |= spc_flag::RESTRAINT | spc_flag::TRAJECTORY;
    }

    /// `ccFellow::Main()` (gcmn 0x0041b5f0): the member's frame (see the
    /// module's table).
    pub fn main(&mut self, me: usize) {
        sync_in(self.scene, self.crew, me);
        let w = ((self.scene.chars[me].spc_char.flags >> flag::WEAPON_SHIFT) as i32) << 30 >> 30;
        if w == -1 {
            self.out.push(Out::DeleteWeaponCcs);
            *self.flags(me) &= !(3 << flag::WEAPON_SHIFT);
        } else if w == 1 {
            self.out.push(Out::EquipWeapon);
            *self.flags(me) |= 3 << flag::WEAPON_SHIFT;
        }
        let manual = self.manual(me);
        let c = &self.scene.chars[me];
        let fl = if c.id() == 8 && self.game.event_lock == 1 && manual && c.no_death {
            -1
        } else {
            i32::from(c.cond[cond::DEAD])
        };
        let env = *self.env;
        self.calc_real(me, fl, &env);
        if chara::check_level_up(self.t, &mut self.scene.chars[me]) != 0 && self.sp(me).act_num != act::AWAY {
            self.out.push(Out::LevelUp);
        }
        self.frame_pos(me);
        self.spc(me).move_pos = crate::geom::VF0;
        if !manual && self.world.trans_mode() {
            let dc = self.world.trans_center();
            let dc = self.world.w2p(dc);
            let disk = self.sp(me).disk_offset;
            let c = &mut self.scene.chars[me];
            c.pos_p = crate::geom::vadd(dc, disk);
            c.pos_p[3] = ONE;
            c.pos = self.world.p2w(c.pos_p);
        }
        if self.scene.chars[me].cond[cond::DEAD] == 2 {
            let c = &mut self.scene.chars[me];
            let n = c.spc_char.cnt;
            c.spc_char.cnt = n.wrapping_sub(1);
            if n < 0 {
                c.spc_char.cnt = 0;
                c.anm_flag = 1;
                c.spc_char.act_num_old = act::LYING;
                c.cond[cond::DEAD] = 3;
                let pos = c.pos;
                self.spc(me).act_num = act::LYING;
                self.out.push(Out::OpenBox { pos });
                self.out.push(Out::OpenBox { pos });
            }
        }
        let c = &self.scene.chars[me];
        if c.spc_char.flags & flag::RECALL != 0 && party_flag(c) < 0 && self.sp(me).act_num == act::AWAY {
            let c = &mut self.scene.chars[me];
            c.spc_char.flags &= !flag::RECALL;
            c.party_flag = 1;
            self.spc(me).act_num = act::TRANSFER_IN;
            if let Some(a) = self.crew.ais.get_mut(&me) {
                a.invite_flag = true;
                a.act_type = 95;
                a.arrival_chat_cnt = -1;
            }
        }
        if party_flag(&self.scene.chars[me]) == -2 && self.sp(me).act_num == act::AWAY {
            let id = self.crew.ais.get(&me).map_or(-1, |a| a.sys_msg.id);
            self.crew.withdraw(id);
            if delete_cmnd(self.scene, me) {
                self.out.push(Out::DeleteCmnd);
            }
            if self.sp(me).body_hit.sw {
                let mut hit = self.sp(me).body_hit;
                self.world.hit_switch(me, &mut hit, false);
                self.spc(me).body_hit = hit;
            }
            *self.flags(me) |= flag::EXIT;
            sync_out(self.scene, self.crew, me);
            return;
        }
        if self.has_ai(me) && !self.field.gho_flag {
            self.brains(me);
        }
        let c = &self.scene.chars[me].cond;
        if c[cond::HOLD] != 0 || c[cond::SLEEP] != 0 || c[cond::PARALYSIS] != 0 {
            let s = self.spc(me);
            s.move_flag = false;
            s.now_speed = 0;
        } else {
            self.step(me);
        }
        self.hit_check(me);
        let mv = self.sp(me).move_pos;
        let c = &mut self.scene.chars[me];
        c.pos[0] = add(c.pos[0], mv[0]);
        c.pos[1] = add(c.pos[1], mv[1]);
        let pos = c.pos;
        let z = self.world.land(pos, LAND_MASK);
        self.scene.chars[me].pos[2] = z;
        let attr = self.world.hit_attribute();
        self.spc(me).hit_attribute = attr;
        self.frame_pos(me);
        self.action(me);
        for note in self.world.anim_notes(me, AnmSlot::Main) {
            self.check_note(me, note);
        }
        let (pos, dirc) = (self.scene.chars[me].pos, self.sp(me).dirc);
        self.out.push(Out::SetMatrix { pos, dirc });
        let cloak = self.scene.chars[me].spc_char.cloak;
        let s = self.spc(me);
        s.transparency = cloak;
        s.set_transparency = cloak;
        if self.scene.chars[me].spc_char.flags & flag::DISP != 0 {
            let arms = if self.world.draw(me) {
                let dead = self.scene.chars[me].cond[cond::DEAD];
                let s = self.sp(me);
                (dead == 0 || (s.ghost && dead == 4)) && !matches!(s.act_num, 12..=14)
            } else {
                false
            };
            self.out.push(if arms { Out::ArmsEffect } else { Out::ClearArmsEffect });
        }
        let s = self.spc(me);
        if s.disp_wait != 0 {
            s.disp_wait -= 1;
            if s.disp_wait <= 0 {
                s.disp_wait = 0;
                *self.flags(me) |= flag::DISP;
                entry_cmnd(self.scene, me);
            }
        }
        if !self.manual(me) && self.world.trans_mode() {
            let dc = self.world.trans_center();
            let mut d = crate::geom::vsub(self.scene.chars[me].pos, dc);
            d[3] = ONE;
            self.spc(me).disk_offset = d;
        }
        self.scene.chars[me].cond[cond::HOLD] = 0;
        let s = self.spc(me);
        s.stop_cnt = if s.move_flag { 0 } else { s.stop_cnt.saturating_add(1) };
        s.cycle = s.cycle.wrapping_add(1);
        sync_out(self.scene, self.crew, me);
    }

    /// `posP = W2P(pos); pos = P2W(posP)`.
    fn frame_pos(&mut self, me: usize) {
        let p = self.world.w2p(self.scene.chars[me].pos);
        self.scene.chars[me].pos_p = p;
        self.scene.chars[me].pos = self.world.p2w(p);
    }

    /// `ccAI::Brains()` (gcmn 0x0057ca00) for the member, its calls made
    /// on the frame's runtime.
    fn brains(&mut self, me: usize) -> i32 {
        let mut ctx = Ctx {
            t: self.t,
            scene: &mut *self.scene,
            party: self.party,
            save: &mut *self.save,
            crew: &mut *self.crew,
            game: self.game,
            ents: self.ents,
            rng: &mut *self.rng,
            rt: &mut *self.rt,
        };
        ctx.brains(me)
    }

    /// `ccFellow::Move()` (gcmn 0x0041bc50): the step this frame
    /// (`movePos`, `nowSpeed`) along the heading `dirc.z`: 3.0, or `speed`
    /// running, times `speedValue` (a condition's slow or haste) and
    /// `speedRate`; none unless `moveFlag`. A member walking with an
    /// attack running (`skillStatus` bit 1) calls the attack off. Nothing
    /// at `dead` 2 or 3 (after `posP` is refreshed).
    pub fn step(&mut self, me: usize) {
        let p = self.world.w2p(self.scene.chars[me].pos);
        self.scene.chars[me].pos_p = p;
        let dead = self.scene.chars[me].cond[cond::DEAD];
        if dead == 2 || dead == 3 {
            return;
        }
        let s = self.sp(me);
        let go = if s.move_flag {
            if self.scene.chars[me].skill_status & 2 != 0 {
                self.cancel(me);
            }
            ONE
        } else {
            0
        };
        let s = self.sp(me);
        let h = s.dirc[2];
        let spd = if s.run_flag { s.speed } else { F_3 };
        let sv = self.scene.chars[me].cond.speed_value;
        let rate = mul(go, s.speed_rate);
        let x = mul(rate, mul(mul(sv, spd), sinf(h)));
        let y = mul(rate, mul(mul(sv, neg(spd)), cosf(h)));
        let s = self.spc(me);
        s.move_pos[0] = x;
        s.move_pos[1] = y;
        s.now_speed = mul(mul(sv, spd), rate);
    }

    /// `ccSpcChar::HitCheck(movePos)` (gcmn 0x0059ee20): the move checked
    /// against the other bodies and the walls (the rules are in
    /// docs/engine/battle.md, "Collision"). Returns 1 when a party member's
    /// body (kind 2) was touched, plus 2 when a push left less than 1 (8
    /// running) of move on the ground.
    pub fn hit_check(&mut self, me: usize) -> i32 {
        let mut res = 0;
        let mv = self.sp(me).move_pos;
        let pos = self.scene.chars[me].pos;
        let land = |w: &mut dyn FellowWorld, mut v: V4| {
            v[2] = w.land(v, LAND_MASK);
            v
        };
        let mut hp1 = mv;
        for k in 0..3 {
            hp1[k] = add(hp1[k], pos[k]);
        }
        hp1[3] = ONE;
        hp1 = land(self.world, hp1);
        let width = self.scene.chars[me].base().width;
        let s = self.spc(me);
        s.body_hit.pos = hp1;
        s.body_hit.radius = add(width, s.now_speed);
        let c = &self.scene.chars[me];
        if c.ty() & 1 == 0 && self.game.area == 1 && !self.world.event_area() {
            let mut d = self.scene.chars[me].pos_p;
            d[2] = 0;
            d[3] = ONE;
            if !le(sqrtf(crate::geom::dot(d, d)), F_7000) {
                return 0;
            }
        }
        let c = &self.scene.chars[me];
        if self.sp(me).act_num == act::AWAY || c.cond[cond::DEAD] == 4 {
            let mut hit = self.sp(me).body_hit;
            self.world.hit_switch(me, &mut hit, false);
            self.spc(me).body_hit = hit;
        }
        let manual = self.manual(me);
        let saved = self.sp(me).body_hit.mask;
        if manual {
            self.spc(me).body_hit.mask = 0xffff_fff8;
        }
        let height = self.sp(me).body_hit.height;
        let raised = |mut v: V4| {
            v[2] = add(v[2], height);
            v
        };
        let mut hit = self.sp(me).body_hit;
        if self.world.collide(me, &mut hit) != 0 {
            if self.world.hit_char_type() & 2 != 0 {
                res |= 1;
            }
            hp1 = crate::geom::vadd(mv, hit.offset);
            for k in 0..3 {
                hp1[k] = add(hp1[k], pos[k]);
            }
            hp1[3] = ONE;
            hp1 = land(self.world, hp1);
            hit.pos = hp1;
            if self.world.collide(me, &mut hit) != 0 {
                if self.world.hit_char_type() & 2 != 0 {
                    res |= 1;
                }
                let mut hp2 = crate::geom::vadd(hp1, hit.offset);
                hp2 = land(self.world, hp2);
                hp2[3] = ONE;
                hp1 = crate::geom::vscale(crate::geom::vadd(hp1, hp2), HALF);
                hp1[3] = ONE;
                hp1 = land(self.world, hp1);
            }
            self.spc(me).body_hit = hit;
            let mut m = if !clear(self.world.line(raised(pos), raised(hp1), WALL_MASK, 1)) {
                crate::geom::VF0
            } else {
                crate::geom::vsub(hp1, pos)
            };
            let mut b = raised(crate::geom::vadd(pos, m));
            b[3] = ONE;
            if !clear(self.world.line(raised(pos), b, WALL_MASK, 1)) {
                m = crate::geom::VF0;
            }
            self.spc(me).move_pos = m;
            let spd = if self.sp(me).run_flag { F_8 } else { ONE };
            let n = neg(spd);
            if lt(m[0], spd) && !le(m[0], n) && lt(m[1], spd) && !le(m[1], n) {
                res |= 2;
            }
        } else {
            self.spc(me).body_hit = hit;
            if !clear(self.world.line(raised(pos), raised(hp1), WALL_MASK, 1)) {
                self.spc(me).move_pos = crate::geom::VF0;
            }
        }
        if manual {
            self.spc(me).body_hit.mask = saved;
        }
        res
    }

    /// `ccFellow::CheckNote(note)` (gcmn 0x0041e1e0), through
    /// `ccFellowCheckNote` (0x0041e440): a note the member's animation
    /// passed. 1 and 2 are footsteps (the sound while shown, dust while
    /// running); 0x8005 lands the normal attack
    /// ([`crate::flow::fellow_attack_note`], the AI's `armsRange` and
    /// `distTg`); the rest do nothing.
    pub fn check_note(&mut self, me: usize, note: Note) {
        match note.event {
            1 | 2 => {
                if self.scene.chars[me].spc_char.flags & flag::DISP != 0 {
                    self.out.push(Out::Se { param: note.param });
                }
                if self.sp(me).act_num == act::RUN {
                    let speed = self.sp(me).speed;
                    self.out.push(Out::PawSmoke { speed });
                }
            }
            0x8005 => {
                self.scene.chars[me].target_char = self.sp(me).target_char;
                let arms = self.ai_param(me).arms_range;
                let dist_tg = self.crew.ais.get(&me).map_or(0, |a| a.dist_tg);
                let mut ev = Events::new();
                crate::flow::fellow_attack_note(self.t, self.scene, me, arms, dist_tg, self.rng, self.env, &mut ev);
                self.rules(me, ev);
            }
            _ => {}
        }
    }

    /// `ccFellow::Action()` (gcmn 0x0041c670): the act state machine (see
    /// the module doc).
    pub fn action(&mut self, me: usize) {
        self.skill_state(me);
        let c = &self.scene.chars[me];
        if c.skill_status & 2 != 0 && c.skill_id == 1 {
            let s = self.spc(me);
            s.atk_anm_cnt = s.atk_anm_cnt.wrapping_add(1);
        }
        self.fidget(me);
        if self.scene.chars[me].anm_flag != 0 {
            self.act_end(me);
        }
        self.fade(me);
        self.walk(me);
        let s = self.sp(me);
        if s.move_flag {
            self.spc(me).walk_run_cnt = s.walk_run_cnt.wrapping_add(1);
        } else {
            self.spc(me).walk_run_cnt = 0;
        }
        if self.has_ai(me) && self.manual(me) && self.sp(me).act_num == act::STAND {
            self.spc(me).act_num = act::FIDGET;
        }
        let a = self.sp(me).act_num;
        if self.scene.chars[me].spc_char.act_num_old != a {
            let name = self.mt.clip(self.sp(me).motion, a).to_string();
            self.world.anim_set(me, AnmSlot::Main, &name);
        }
        let s = self.sp(me);
        let sv = self.scene.chars[me].cond.speed_value;
        let step = match a {
            act::RUN => fptoui(mul(F_256, mul(mul(F_1_375, s.speed_rate), sv))) as u16,
            act::WALK => fptoui(mul(F_256, mul(s.speed_rate, sv))) as u16,
            _ => 256,
        };
        let done = self.world.anim_forward(me, AnmSlot::Main, step);
        self.scene.chars[me].anm_flag = done;
        self.transfer(me);
        let a = self.sp(me).act_num;
        self.scene.chars[me].spc_char.act_num_old = a;
        let s = self.spc(me);
        if s.transfer_lag > 0 {
            s.transfer_lag -= 1;
        } else {
            s.act_cnt = s.act_cnt.wrapping_add(1);
        }
        if s.atk_dellay > 0 {
            s.atk_dellay -= 1;
        }
    }

    /// Action's first part: the normal attack against what it should not
    /// hit, a requested skill started, the combo run, a skill's end.
    fn skill_state(&mut self, me: usize) {
        let c = &self.scene.chars[me];
        if c.skill_id == 1
            && let Some(tg) = self.sp(me).target_char.filter(|&t| self.scene.listed(t))
        {
            let ty = self.scene.chars[tg].ty();
            let cc = &self.scene.chars[me].cond;
            if cc[cond::CONFUSION] == 0 && cc[cond::CHARM] == 0 && ty & 0xe0 == 0 {
                self.cancel(me);
            }
            let ty = self.sp(me).target_char.map_or(0, |t| self.scene.chars[t].ty());
            if self.scene.chars[me].cond[cond::CHARM] != 0 && ty as u32 & 0x0700_000f == 0 {
                self.cancel(me);
            }
            let ty = self.sp(me).target_char.map_or(0, |t| self.scene.chars[t].ty());
            if self.scene.chars[me].cond[cond::CONFUSION] != 0 && ty as u32 & 0x0700_00ef == 0 {
                self.cancel(me);
            }
        }
        let st = self.scene.chars[me].skill_status;
        if st & 1 != 0 {
            let sid = self.scene.chars[me].skill_id;
            if sid != 180 && !self.target_up(me) {
                self.cancel(me);
                self.scene.chars[me].spc_char.attack = 0;
                return;
            }
            if sid == 1 {
                if self.sp(me).act_num >= 7 {
                    return;
                }
                let c = &mut self.scene.chars[me];
                c.skill_status = (c.skill_status ^ 1) | 2;
                self.swing(me, act::ATTACK);
                let c = &mut self.scene.chars[me];
                c.spc_char.attack = 1;
                self.hold_still(me);
                self.face_target(me);
                self.out.push(Out::ArmsEffectColor(sid));
                self.announce(me, 1);
                return;
            }
            let ty = self.t.skill(i32::from(sid)).map_or(0, |k| k.ty);
            let a = if ty & 0x100 != 0 {
                17
            } else if ty & 0x200 != 0 {
                18
            } else if ty & 0x400 != 0 {
                19
            } else if ty & 0x800 != 0 {
                20
            } else if ty & 0x1000 != 0 {
                21
            } else {
                return;
            };
            let c = &mut self.scene.chars[me];
            c.skill_status = (c.skill_status ^ 1) | 2;
            c.spc_char.act_num_old = -1;
            self.swing(me, a);
            self.hold_still(me);
            self.face_target(me);
            let c = &self.scene.chars[me];
            let flag = i32::from(c.skill_status & 8 != 0);
            self.out.push(Out::SkillStart { sid: c.skill_id, flag });
            self.out.push(Out::ArmsEffectColor(c.skill_id));
            self.out.push(Out::StartArmsEffect(c.skill_id));
            if self.manual(me) {
                return;
            }
            let cc = &self.scene.chars[me].cond;
            if cc[cond::CHARM] != 0 || cc[cond::CONFUSION] != 0 {
                return;
            }
            let sid = i32::from(self.scene.chars[me].skill_id);
            let k = skill_check_type(self.t, sid);
            if k == 0 || k == 1 {
                self.announce(me, sid);
            }
            return;
        }
        if st & 2 != 0 {
            let c = &self.scene.chars[me];
            if c.skill_id != 1 || c.cond[cond::SLEEP] != 0 || c.cond[cond::PARALYSIS] != 0 {
                return;
            }
            match c.spc_char.attack {
                1 | 2 => {
                    let a = self.sp(me).act_num;
                    if a == act::ATTACK && c.anm_flag != 0 {
                        if !self.combo_reach(me) {
                            return;
                        }
                        self.swing(me, act::ATTACK2);
                        let c = &mut self.scene.chars[me];
                        c.spc_char.act_num_old = -1;
                        c.spc_char.attack = 3;
                        self.spc(me).atk_anm_cnt = 0;
                        self.face_target(me);
                        self.spc(me).move_flag = false;
                        self.announce(me, 1);
                    } else if a < 7 {
                        self.scene.chars[me].spc_char.attack = 3;
                    }
                }
                3 => {
                    let s = self.sp(me);
                    if s.act_num < 5 && i32::from(s.atk_anm_cnt) >= s.consecutive_cnt {
                        self.scene.chars[me].spc_char.attack = 4;
                    }
                }
                4 => {
                    if self.sp(me).act_num >= 7 || !self.combo_reach(me) {
                        return;
                    }
                    self.swing(me, act::ATTACK);
                    self.spc(me).atk_anm_cnt = 0;
                    self.scene.chars[me].spc_char.attack = 2;
                    self.face_target(me);
                    self.spc(me).move_flag = false;
                    self.announce(me, 1);
                }
                _ => {}
            }
            return;
        }
        if st != 0 {
            return;
        }
        let c = &mut self.scene.chars[me];
        if c.skill_id == 1 {
            c.skill_id = 0;
            c.spc_char.flags &= !(spc_flag::RESTRAINT | spc_flag::TRAJECTORY);
        }
        c.spc_char.attack = 0;
    }

    /// A combo's next swing needs the target up and in reach; otherwise
    /// the attack is called off (`attack` 0).
    fn combo_reach(&mut self, me: usize) -> bool {
        if !self.target_up(me) || !self.in_arms(me) {
            self.cancel(me);
            self.scene.chars[me].spc_char.attack = 0;
            return false;
        }
        true
    }

    /// A skill starting roots the member: `atkAnmCnt` 0, `moveFlag` 0,
    /// `stopFlag` 1, `runFlag` 0.
    fn hold_still(&mut self, me: usize) {
        let s = self.spc(me);
        s.atk_anm_cnt = 0;
        s.move_flag = false;
        s.stop_flag = true;
        s.run_flag = false;
    }

    /// The idle fidget: free and standing out of battle, every 451 frames
    /// or so (`reactCnt` restarts at `rand() % 60`) act 0 turns to 1 and 2
    /// to 3, and a member of the party away from the Root Town not a
    /// ghost queues 0x10015 to itself in 0, 11 or 22 frames.
    fn fidget(&mut self, me: usize) {
        let c = &self.scene.chars[me];
        let idle = !self.manual(me)
            && c.cond[cond::HOLD] == 0
            && c.cond[cond::SLEEP] == 0
            && c.cond[cond::PARALYSIS] == 0
            && c.cond[cond::DEAD] == 0
            && self.sp(me).act_num < 4
            && self.game.in_battle != 1;
        if !idle {
            self.spc(me).react_cnt = 0;
            return;
        }
        let s = self.spc(me);
        s.react_cnt = s.react_cnt.wrapping_add(1);
        if s.react_cnt < 451 {
            return;
        }
        let r = self.rng.rand();
        self.spc(me).react_cnt = chara::cmod(r, 60) as i16;
        let a = match self.sp(me).act_num {
            act::EASE => act::EASE_FIDGET,
            act::STAND => act::FIDGET,
            _ => return,
        };
        self.spc(me).act_num = a;
        if self.has_ai(me) && self.game.area != 0 && self.game.in_battle != 1 && !self.sp(me).ghost {
            let id = self.crew.sys_msg_id(me) as u16;
            self.crew.sys.delete_delay(0x10015, id, id);
            let q = chara::cmod(self.rng.rand() >> 3, 3) * 11;
            self.crew.send(0x10015, id, id, 0, q as u16, -1, Some(me));
        }
    }

    /// An act's animation ended (`anmFlag`): the attack and skill acts go
    /// back to walking, standing or as they were, with the next attack's
    /// delay; the fidgets end; the transfers move on.
    fn act_end(&mut self, me: usize) {
        let a = self.sp(me).act_num;
        match a {
            15..=21 => {
                if self.sp(me).move_flag {
                    self.spc(me).act_num = act::RUN;
                }
                if self.sp(me).stop_flag {
                    self.spc(me).act_num = act::STAND;
                }
                let c = &mut self.scene.chars[me];
                if a >= act::SKILL {
                    c.spc_char.arms_effect_sw = 0;
                }
                c.spc_char.flags &= !(spc_flag::RESTRAINT | spc_flag::TRAJECTORY | spc_flag::PAUSE);
                let r = self.rng.rand();
                let s = self.spc(me);
                s.atk_dellay = ((r >> 3) & 31) + 70;
                s.atk_anm_cnt = 0;
            }
            act::FIDGET => self.spc(me).act_num = act::EASE,
            act::EASE_FIDGET => self.spc(me).act_num = act::EASE_FIDGET_END,
            act::TRANSFER_OUT => {
                self.spc(me).act_num = act::AWAY;
                self.set_flag(me, spc_flag::RESTRAINT, true);
                let s = self.spc(me);
                s.move_flag = false;
                s.act_cnt = 0;
                self.set_flag(me, spc_flag::TRAJECTORY, false);
            }
            act::TRANSFER_IN => {
                self.spc(me).act_num = act::EASE;
                self.scene.chars[me].spc_char.act_num_old = act::EASE;
                self.set_flag(me, spc_flag::RESTRAINT, false);
                self.spc(me).act_cnt = 0;
                if !self.manual(me) {
                    entry_cmnd(self.scene, me);
                }
                if self.scene.chars[me].cond[cond::DEAD] == 0 {
                    let mut hit = self.sp(me).body_hit;
                    self.world.hit_switch(me, &mut hit, true);
                    self.spc(me).body_hit = hit;
                }
            }
            0 | 2 | 5 | 9 | 10 | 11 => {}
            _ => {
                self.spc(me).act_num = act::STAND;
                self.set_flag(me, spc_flag::PAUSE, false);
            }
        }
    }

    /// Going down and up: lying (act 10) at `dead` 3 or 4 fades out over 50
    /// frames into a ghost (`dead` 4, act 2; the whole party down hides it,
    /// `dispSW` 0); a ghost standing fades in to half over 30 frames;
    /// `dead` 5 gets up over 78 frames (`dead` 0).
    fn fade(&mut self, me: usize) {
        let dead = self.scene.chars[me].cond[cond::DEAD];
        if dead == 3 || dead == 4 {
            let a = self.sp(me).act_num;
            if matches!(a, 6 | 5 | 2 | 0) {
                let ghost = self.sp(me).ghost;
                let c = &mut self.scene.chars[me];
                if dead != 4 || ghost {
                    c.spc_char.cloak = HALF;
                } else {
                    let n = c.spc_char.cnt;
                    let f = if n < 30 {
                        c.spc_char.cnt = n + 1;
                        mul(HALF, div(from_int(n), F_30))
                    } else {
                        c.spc_char.flags &= !spc_flag::RESTRAINT;
                        c.spc_char.cnt = 0;
                        self.spc(me).ghost = true;
                        self.out.push(Out::ClearArmsEffect);
                        HALF
                    };
                    self.scene.chars[me].spc_char.cloak = f;
                }
            } else if a == act::LYING {
                let c = &mut self.scene.chars[me];
                let n = c.spc_char.cnt;
                if n < 50 {
                    c.spc_char.cloak = div(from_int(50 - n), F_50);
                    c.spc_char.cnt = n + 1;
                } else {
                    c.anm_flag = 0;
                    c.spc_char.act_num_old = -1;
                    c.spc_char.flags |= spc_flag::RESTRAINT;
                    c.spc_char.flags &= !spc_flag::TRAJECTORY;
                    c.cond[cond::DEAD] = 4;
                    c.spc_char.cnt = 0;
                    let s = self.spc(me);
                    s.act_num = act::EASE;
                    s.move_flag = false;
                    if self.party.annihilated(self.scene) {
                        self.set_flag(me, flag::DISP, false);
                    }
                    self.scene.chars[me].spc_char.cloak = 0;
                }
            }
        }
        if self.scene.chars[me].cond[cond::DEAD] == 5 {
            let c = &mut self.scene.chars[me];
            let n = c.spc_char.cnt;
            let f = if n < 18 {
                c.spc_char.cnt = n + 1;
                0
            } else if n < 78 {
                c.spc_char.cnt = n + 1;
                div(from_int(n), F_78)
            } else {
                c.spc_char.flags &= !spc_flag::RESTRAINT;
                c.spc_char.cnt = 0;
                c.cond[cond::DEAD] = 0;
                self.spc(me).ghost = false;
                self.out.push(Out::ClearArmsEffect);
                ONE
            };
            self.scene.chars[me].spc_char.cloak = f;
        }
    }

    /// Walking, running and standing acts from `moveFlag`, `stopFlag` and
    /// `runFlag`: starting to move from a standing act walks (6); stopping
    /// from 5 or 6 stands (0, or 2 in the Root Town, under manual control
    /// or as a ghost); moving, `runFlag` picks 5 or 6.
    fn walk(&mut self, me: usize) {
        let s = self.sp(me);
        if s.stop_flag && s.move_flag {
            self.spc(me).stop_flag = false;
            if s.act_num < 5 {
                self.spc(me).act_num = act::WALK;
            }
        } else if !s.stop_flag && !s.move_flag {
            self.spc(me).stop_flag = true;
            if s.act_num == act::RUN || s.act_num == act::WALK {
                let two = self.game.area == 0
                    || (self.has_ai(me) && self.manual(me))
                    || self.scene.chars[me].cond[cond::DEAD] == 4;
                self.spc(me).act_num = if two { act::EASE } else { act::STAND };
            }
        }
        let s = self.sp(me);
        if !s.stop_flag && s.move_flag {
            if s.run_flag && s.act_num == act::WALK {
                self.spc(me).act_num = act::RUN;
            } else if !s.run_flag && s.act_num == act::RUN {
                self.spc(me).act_num = act::WALK;
            }
        }
    }

    /// The transfer acts' fades: out (12: the effect and off the collision
    /// at frame 1, faded by frame 41, done at 141), in (13: the effect at
    /// frame 30, or 0 for a warp; faded in over 50-70, 20-40 for a warp).
    fn transfer(&mut self, me: usize) {
        let a = self.sp(me).act_num;
        let ghost = self.sp(me).ghost;
        let n = self.sp(me).act_cnt;
        if a == act::TRANSFER_IN {
            let warp = self.field.warp_flag;
            let (at, from, to): (i16, i32, i32) = if warp { (0, 20, 40) } else { (30, 50, 70) };
            let mut f = 0;
            if n == at {
                self.out.push(if warp { Out::WarpTransfer } else { Out::Transfer });
            } else if to < i32::from(n) {
                self.scene.chars[me].anm_flag = 1;
                f = if ghost { HALF } else { ONE };
            } else if from < i32::from(n) {
                f = div(from_int(i32::from(n) - from), from_int(to - from));
                if !le(f, ONE) {
                    f = ONE;
                }
                if ghost {
                    f = mul(f, HALF);
                }
            }
            self.scene.chars[me].spc_char.cloak = if i32::from(self.sp(me).act_cnt) < from { 0 } else { f };
        } else if a == act::TRANSFER_OUT {
            let mut f = 0;
            if n == 1 {
                self.out.push(Out::Transfer);
                let mut hit = self.sp(me).body_hit;
                self.world.hit_switch(me, &mut hit, false);
                self.spc(me).body_hit = hit;
            } else if n >= 141 {
                self.scene.chars[me].anm_flag = 1;
            } else if n >= 22 {
                f = div(from_int(41 - i32::from(n)), F_20);
                if lt(f, 0) {
                    f = 0;
                }
            }
            let n = self.sp(me).act_cnt;
            self.scene.chars[me].spc_char.cloak = if n < 22 {
                if ghost { HALF } else { ONE }
            } else if ghost {
                mul(f, HALF)
            } else {
                f
            };
        }
    }
}

/// `c.eq.s` against -1.0: nothing in the way.
fn clear(r: F) -> bool {
    crate::geom::eq(crate::geom::MINUS_ONE, r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Scene;

    #[test]
    fn command_lists_by_kind() {
        let mut s = Scene::default();
        let mut pc = Char::other(crate::param::Base { ty: 4, ..Default::default() }, [0; 16]);
        pc.party_flag = 1;
        let foe = Char::other(crate::param::Base { ty: 0x20, ..Default::default() }, [0; 16]);
        let a = s.add(pc, 9);
        let b = s.add(foe, 9);
        entry_cmnd(&mut s, a);
        entry_cmnd(&mut s, b);
        entry_cmnd(&mut s, a);
        assert_eq!((s.pc_list.clone(), s.ene_list.clone()), (vec![a], vec![b]));
        assert!(delete_cmnd(&mut s, a));
        assert!(!delete_cmnd(&mut s, a));
        assert!(s.pc_list.is_empty());
    }

    #[test]
    fn flags_copy_both_ways() {
        let mut s = Scene::default();
        let mut c = Char::other(Default::default(), [0; 16]);
        c.spc_char.flags = spc_flag::MOVE | flag::RUN | flag::DISP;
        c.spc_char.act_num = 6;
        let me = s.add(c, 0);
        let mut crew = Crew::default();
        sync_in(&s, &mut crew, me);
        let sp = crew.spc[&me];
        assert!(sp.move_flag && sp.run_flag && !sp.stop_flag && !sp.ghost);
        assert_eq!(sp.act_num, 6);
        crew.spc.get_mut(&me).unwrap().run_flag = false;
        crew.spc.get_mut(&me).unwrap().stop_flag = true;
        sync_out(&mut s, &crew, me);
        assert_eq!(s.chars[me].spc_char.flags, spc_flag::MOVE | spc_flag::STOP | flag::DISP);
    }
}
