//! Equipment (PERSONAL, menu 63): `EquipmentMenu` (gcmn 0x00536f70) with
//! `EquipmentMenuDisp` (0x00538130); the preview `ccEquipChangeMenuSub`
//! (0x00538de0) works out, and the save's side of a change,
//! `ccSaveData::ChangeEquipment` (main 0x00177010, `ccChangeEquipment`
//! 0x00177070 with `ccCheckSkillCount` 0x00176ce0, `ccAddSkill` 0x00177590
//! and `ccDelSkill` 0x001776e0). The page is on member `equipSpcNum`, one
//! slot a page; jobs 0-5 wear weapon category 0-5 up to level 2, 3, 3, 3,
//! 2 and 1. The steps are in docs/engine/field-ui.md (Equipment).

use piney_battle::chara::add_elm;
use piney_battle::param::{Elm, SpcParam};

use crate::Request;
use crate::ctrl::{Ctx, Flow, MenuCtrl, SE_BACK, SE_MOVE, SE_OK};
use crate::items;
use crate::menus::personal::check;
use crate::menus::status::{
    SPC_EQUIPMENT, SPC_JOB, SPC_PARAM, SPC_SIZE, beff_disp, param_disp, skill_disp, skill_list, spc_i16, spc_real,
};
use crate::menus::system::{extract_menu, str_cat};
use crate::spr::{font_type, set_clm};
use crate::window::{disp_button, disp_line_h, disp_line_v, disp_scroll_bar, disp_square, set_type};
use piney_desktop::eef::from_int;

/// An equipment row's numbers, what the equipment page reads of it.
#[derive(Clone, Copy, Debug, Default)]
struct EquipRow {
    /// `elm` (+0x0a), `beff` (+0x2a), `level` (+0x38), `skillID[3]` (+0x3a),
    /// `price` (+0x40).
    elm: Elm,
    beff: [i16; 5],
    level: i16,
    skills: [i16; 3],
    price: i32,
}

/// `ccGetEquipParam(category, id)`'s rows by category (0-5 the jobs'
/// weapons, 6-9 head, body, arm, leg), which it indexes without a check:
/// an empty slot (-1) reads the bytes before the table, kept as a row
/// (`piney_data::tables::battle`'s `*_before`); a row past a table's end
/// reads as none.
#[derive(Clone, Debug, Default)]
pub struct EquipRaw {
    rows: Vec<Vec<EquipRow>>,
    before: Vec<EquipRow>,
}

impl EquipRaw {
    pub fn of(volume: piney_data::volume::Volume, t: &piney_battle::Tables) -> EquipRaw {
        let row = |e: &piney_battle::param::EquipParam| EquipRow {
            elm: e.elm,
            beff: e.beff,
            level: e.level,
            skills: e.skill_id,
            price: e.price,
        };
        let mut rows: Vec<Vec<EquipRow>> = t.weapons.iter().map(|w| w.iter().map(row).collect()).collect();
        rows.extend(t.armor.iter().map(|a| a.iter().map(row).collect()));
        let b = piney_data::tables::battle::of(volume);
        let before = [
            b.weapon1_before(),
            b.weapon2_before(),
            b.weapon3_before(),
            b.weapon4_before(),
            b.weapon5_before(),
            b.weapon6_before(),
            b.head_before(),
            b.body_before(),
            b.arm_before(),
            b.leg_before(),
        ]
        .iter()
        .map(|r| {
            let f = &r.beff;
            EquipRow {
                elm: piney_battle::param::elm_of(&r.elm),
                beff: [f.drain_hp, f.drain_sp, f.critical, f.dying, f.invincible],
                level: r.level,
                skills: r.skill_id,
                price: r.price,
            }
        })
        .collect();
        EquipRaw { rows, before }
    }

    fn row(&self, cat: i32, id: i32) -> Option<EquipRow> {
        let c = usize::try_from(cat).ok()?;
        if id == -1 {
            return self.before.get(c).copied();
        }
        self.rows.get(c)?.get(usize::try_from(id).ok()?).copied()
    }

    /// `level` (+0x38).
    pub fn level(&self, cat: i32, id: i32) -> i16 {
        self.row(cat, id).map_or(0, |r| r.level)
    }

    /// `skillID[3]` (+0x3a).
    pub fn skills(&self, cat: i32, id: i32) -> [i16; 3] {
        self.row(cat, id).map_or([-1; 3], |r| r.skills)
    }

    /// `price` (+0x40).
    pub fn price(&self, cat: i32, id: i32) -> i32 {
        self.row(cat, id).map_or(0, |r| r.price)
    }

    /// `beff` (+0x2a): drainHP, drainSP, critical, dying, invincible.
    pub fn beff(&self, cat: i32, id: i32) -> [i16; 5] {
        self.row(cat, id).map_or([0; 5], |r| r.beff)
    }

    /// `elm` (+0x0a, a `ccCharParamElement`).
    pub fn elm(&self, cat: i32, id: i32) -> Elm {
        self.row(cat, id).map_or([0; 16], |r| r.elm)
    }
}

/// The equipment slot of a category (`ccGetEquipmentNum`'s table): the
/// weapons in slot 4, head, body, arm, leg in 0-3.
pub fn slot(cat: i32) -> Option<usize> {
    match cat {
        0..=5 => Some(4),
        6..=9 => Some(cat as usize - 6),
        _ => None,
    }
}

/// The job's weapon category and the highest level it wears.
pub fn job_weapon(job: i32) -> (i32, i16) {
    match job {
        0 => (0, 2),
        1 => (1, 3),
        2 => (2, 3),
        3 => (3, 3),
        4 => (4, 2),
        5 => (5, 1),
        // The game leaves the category as the caller's register had it.
        _ => (0, 0),
    }
}

/// `ccCheckSkillCount(equipment, skill, spc)`: how many of the pieces worn
/// carry the skill (the weapon's table by the job; none outside 0-5).
fn check_skill_count(raw: &EquipRaw, eq: &[i16; 6], skill: i16, job: i16) -> i32 {
    if skill == -1 {
        return 0;
    }
    let mut n = 0;
    let mut count =
        |cat: i32, id: i16| n += raw.skills(cat, i32::from(id)).iter().filter(|&&s| s == skill).count() as i32;
    for (cat, k) in [(6, 0), (7, 1), (8, 2), (9, 3)] {
        count(cat, eq[k]);
    }
    if (0..6).contains(&job) {
        count(i32::from(job), eq[4]);
    }
    n
}

/// `ccAddSkill(list, skill)`: the skill added (Data Drain's levels 2-5 are
/// one, 2) unless there already, in the first free place; then the list
/// sorted, lowest first, the free places last.
pub fn add_skill(list: &mut [i16; 20], skill: i16) {
    let skill = if (2..6).contains(&skill) { 2 } else { skill };
    let mut copy = [0i16; 20];
    let mut free = None;
    for (k, c) in copy.iter_mut().enumerate() {
        let v = list[k];
        if v == skill {
            return;
        }
        if free.is_none() && v == -1 {
            free = Some(k);
        }
        *c = v;
    }
    if let Some(k) = free {
        list[k] = skill;
        copy[k] = skill;
    }
    for slot in list.iter_mut() {
        let mut low = 304;
        let mut at = None;
        for (k, &v) in copy.iter().enumerate() {
            if v >= 0 && v < low {
                low = v;
                at = Some(k);
            }
        }
        match at {
            Some(k) if low != 304 => {
                copy[k] = -1;
                *slot = low;
            }
            _ => *slot = -1,
        }
    }
}

/// `ccDelSkill(list, skill)`: every place holding it emptied.
pub fn del_skill(list: &mut [i16; 20], skill: i16) {
    for v in list.iter_mut() {
        if *v == skill {
            *v = -1;
        }
    }
}

/// `ccChangeEquipment(equipment, skills, category, id, spc)`: the old
/// piece's skills no other piece worn carries dropped (each the first place
/// it is in), the piece put on (-1: none changed), its skills added; the
/// sets' skills: all four armour pieces 60, 61, 62 or 63 give 291-294; head
/// 68, body 67, arm 66, leg 67 and weapon 8 give 289.
pub fn change_equipment(raw: &EquipRaw, eq: &mut [i16; 6], skills: &mut [i16; 20], cat: i32, id: i32, job: i16) {
    let mut id = id;
    if id != -1 {
        let old = slot(cat).map_or(-1, |s| i32::from(eq[s]));
        let gone = raw.skills(cat, old).map(|s| if check_skill_count(raw, eq, s, job) < 2 { s } else { -1 });
        for s in gone {
            if s != -1
                && let Some(p) = skills.iter_mut().find(|v| **v == s)
            {
                *p = -1;
            }
        }
        if let Some(s) = slot(cat) {
            eq[s] = id as i16;
        }
    } else {
        id = slot(cat).map_or(-1, |s| i32::from(eq[s]));
    }
    for s in raw.skills(cat, id) {
        if s != -1 {
            add_skill(skills, s);
        }
    }
    if let Some(k) = (60..64).position(|v| eq[..4].iter().all(|&e| e == v)) {
        add_skill(skills, 291 + k as i16);
    } else {
        for s in 291..295 {
            del_skill(skills, s);
        }
    }
    if eq[..5] == [68, 67, 66, 67, 8] {
        add_skill(skills, 289);
    } else {
        del_skill(skills, 289);
    }
}

/// What the page shows for a piece: `ccEquipChangeMenuSub(id, cond, elm,
/// sklist, category, item)` on copies of the member's equipment and
/// skills: the added effects the pieces would give, the parameters
/// (`elm` + the pieces + `temp`), and the skills as (skill, colour) pairs:
/// the ones had now first (7 kept, 23 lost), then the ones gained (20).
pub struct Preview {
    pub cond: [i16; 5],
    pub elm: Elm,
    pub skills: Vec<(i16, usize)>,
}

pub fn preview(x: &Ctx, id: i32, cat: i32, item: i32) -> Preview {
    let raw = &x.texts.pers.equip_raw;
    let spc = SpcParam::from_save(&x.save.save, id.clamp(0, 17) as usize);
    let mut eq = spc.equipment;
    let had = skill_list(x, id);
    let mut sk = had;
    change_equipment(raw, &mut eq, &mut sk, cat, item, spc.job);
    let mut skills = vec![(-1i16, 0usize); 20];
    let mut n = 0;
    'had: for &v in &had {
        if v <= 0 {
            continue;
        }
        skills[n] = (v, if sk.contains(&v) { 7 } else { 23 });
        n += 1;
        if n >= 20 {
            break 'had;
        }
    }
    if n < 20 {
        for &v in &sk {
            if v <= 0 || had.contains(&v) {
                continue;
            }
            skills[n] = (v, 20);
            n += 1;
            if n >= 20 {
                break;
            }
        }
    }
    let job = i32::from(spc.job);
    let weapon = (0..6).contains(&job);
    // The added effects: the largest of each; a job outside 0-5 takes the
    // leg piece's again.
    let mut cond = [0i16; 5];
    let mut take = |b: [i16; 5]| {
        for (c, v) in cond.iter_mut().zip(b) {
            if *c < v {
                *c = v;
            }
        }
    };
    for (cat, k) in [(6, 0), (7, 1), (8, 2), (9, 3)] {
        take(raw.beff(cat, i32::from(eq[k])));
    }
    take(if weapon { raw.beff(job, i32::from(eq[4])) } else { raw.beff(9, i32::from(eq[3])) });
    // The parameters: head + body + leg + arm + weapon (the arm again for
    // a job outside 0-5), then elm + that + temp.
    let mut tune = raw.elm(6, i32::from(eq[0]));
    add_elm(&mut tune, &raw.elm(7, i32::from(eq[1])), 999, -999);
    add_elm(&mut tune, &raw.elm(9, i32::from(eq[3])), 999, -999);
    add_elm(&mut tune, &raw.elm(8, i32::from(eq[2])), 999, -999);
    let w = if weapon { raw.elm(job, i32::from(eq[4])) } else { raw.elm(8, i32::from(eq[2])) };
    add_elm(&mut tune, &w, 999, -999);
    let mut elm = spc.elm;
    add_elm(&mut elm, &tune, 999, -999);
    add_elm(&mut elm, &spc.temp, 999, -999);
    Preview { cond, elm, skills }
}

/// `ccChar::CalcReal(1)` on a party member, as far as the menus see it:
/// `real` and `tune` into the member's record in the save; the character's
/// added effects (condition 2-6) and its maximum HP and SP (the record's).
pub fn calc_real(t: &piney_battle::Tables, save: &mut crate::SaveState, c: &mut crate::world::CharInfo) {
    let id = i32::from(c.id);
    if !(0..18).contains(&id) {
        return;
    }
    let mut p = SpcParam::from_save(&save.save, id as usize);
    // The type is the character's (`base`), not the record's.
    p.base.ty = c.types as i32;
    let mut ch = piney_battle::Char::pc(p);
    let mut ev = Vec::new();
    piney_battle::chara::calc_real(t, &mut ch, 1, &piney_battle::Env::default(), &mut ev);
    let Some(p) = ch.spc() else { return };
    if c.types as i32 & piney_battle::param::ty::PC == 0 {
        return;
    }
    let at = SPC_PARAM + SPC_SIZE * id as usize;
    for k in 0..16 {
        save.save.set_i16(at + 0x48 + 2 * k, p.real[k]);
        save.save.set_i16(at + 0x68 + 2 * k, p.tune[k]);
    }
    for k in 2..7 {
        c.condition[k] = ch.cond[k];
    }
    c.max_hp = ch.max_hp;
    c.max_sp = ch.max_sp;
}

fn idx(m: &MenuCtrl) -> usize {
    m.list_at(m.menu)
}

/// `ccSaveData::GetEquipmentNum(id, category)`.
pub fn equipment_num(x: &Ctx, id: i32, cat: i32) -> i32 {
    slot(cat).map_or(-1, |s| i32::from(spc_i16(x, id, SPC_EQUIPMENT + 2 * s)))
}

/// The page's category, the piece worn and the candidates.
struct Page {
    cat: i32,
    worn: i32,
    list: Vec<i32>,
}

fn page(x: &Ctx, id: i32, page: i16) -> Page {
    let job = i32::from(spc_i16(x, id, SPC_JOB));
    let (wcat, max) = job_weapon(job);
    let cat = if page == 0 { wcat } else { i32::from(page) + 5 };
    let worn = equipment_num(x, id, cat);
    let raw = &x.texts.pers.equip_raw;
    let pc = id.clamp(0, 17) as usize;
    let list = (0..items::ITEMS)
        .map(|k| items::save_item(x.save, pc, k))
        .filter(|it| i32::from(it.cat) == cat && i32::from(it.id) != worn && raw.level(cat, i32::from(it.id)) <= max)
        .map(|it| i32::from(it.id))
        .collect();
    Page { cat, worn, list }
}

fn cand(p: &Page, k: i16) -> i32 {
    usize::try_from(k).ok().and_then(|k| p.list.get(k)).copied().unwrap_or(-1)
}

/// Back to the piece worn (select -1) or the candidates' last row.
fn out_of_grid(l: &mut crate::tables::MenuList) {
    if l.my <= 0 || l.select < 0 {
        l.index = 0;
        l.select = -1;
    } else {
        l.index = 1;
    }
}

/// The moves of proccess 1 (0x005373c0 - 0x0053799c): whether the help
/// changes.
fn cursor(m: &mut MenuCtrl, x: &mut Ctx) -> i32 {
    let i = idx(m);
    let r = x.pad.repeat.bits();
    let (right, left, up, down) = (r & 0x2000 != 0, r & 0x8000 != 0, r & 0x1000 != 0, r & 0x4000 != 0);
    let mut moved = 0;
    let mut se = false;
    let l = &mut m.lists[i];
    match l.index {
        0 => {
            if right {
                (l.index, l.sx, l.sy) = (2, 1, 0);
                se = true;
                moved += 1;
            } else if down {
                if l.my > 0 {
                    (l.index, l.select) = (1, 0);
                } else {
                    (l.index, l.sx, l.sy) = (2, 0, 11);
                }
                se = true;
                moved += 1;
            }
        }
        1 => {
            if right {
                (l.index, l.sx, l.sy) = (2, 1, 0);
                se = true;
                moved += 1;
            } else if l.select == 0 && up {
                (l.index, l.select) = (0, -1);
                se = true;
                moved += 1;
            } else if l.select >= l.my - 1 && down {
                (l.index, l.sx, l.sy) = (2, 0, 11);
                se = true;
                moved += 1;
            } else {
                m.select_scr(0, 2, 0, x);
                return 0;
            }
        }
        2 => {
            if l.sx > 0 && l.sy < 8 {
                if right {
                    l.sx += 1;
                    if l.sx >= 3 {
                        l.sx = 2;
                    } else {
                        se = true;
                        moved += 1;
                    }
                } else if left {
                    l.sx -= 1;
                    se = true;
                    if l.sx <= 0 {
                        out_of_grid(l);
                    }
                    moved += 1;
                } else if up {
                    l.sy -= 1;
                    if l.sy < 0 {
                        l.sy = 0;
                    } else {
                        se = true;
                        moved += 1;
                    }
                } else if down {
                    se = true;
                    l.sy += 1;
                    moved += 1;
                }
            } else if l.sx > 0 && l.sy < 11 {
                if right {
                    l.sx += 1;
                    if l.sx >= 3 {
                        l.sx = 2;
                    } else {
                        se = true;
                        moved += 1;
                    }
                } else if left {
                    l.sx -= 1;
                    se = true;
                    if l.sx <= 0 {
                        out_of_grid(l);
                    }
                    moved += 1;
                } else if up {
                    se = true;
                    l.sy -= 1;
                    moved += 1;
                } else if down {
                    se = true;
                    l.sy += 1;
                    if l.sy >= 11 {
                        l.sx += 1;
                        l.sy = 11;
                    }
                    moved += 1;
                }
            } else if l.sy >= 11 {
                if right {
                    l.sx += 1;
                    if l.sx >= 4 {
                        l.sx = 3;
                    } else {
                        se = true;
                        moved += 1;
                    }
                } else if left {
                    l.sx -= 1;
                    if l.sx < 0 {
                        l.sx = 0;
                    } else {
                        se = true;
                        moved += 1;
                    }
                } else if up {
                    se = true;
                    l.sy -= 1;
                    if l.sy < 11 {
                        if l.sx < 2 {
                            if l.my > 0 {
                                (l.index, l.select) = (1, l.my - 1);
                            } else {
                                (l.index, l.select) = (0, -1);
                            }
                        } else {
                            l.sx -= 1;
                            l.sy = 10;
                        }
                    }
                    moved += 1;
                } else if down {
                    l.sy += 1;
                    if l.sy >= 15 {
                        l.sy = 14;
                    } else {
                        se = true;
                        moved += 1;
                    }
                }
            }
        }
        _ => {}
    }
    if se {
        x.se(SE_MOVE);
    }
    moved
}

/// The rows shown (at most 8) and every row back to the top.
fn to_top(l: &mut crate::tables::MenuList) {
    l.select = -1;
    l.index = 0;
    l.sx = 0;
    l.sy = 0;
    l.dy = 0;
}

/// `EquipmentMenu` (menu 63).
pub fn equipment_menu(m: &mut MenuCtrl, x: &mut Ctx) -> Flow {
    let i = idx(m);
    let id = i32::from(m.equip_spc_num());
    let mut p = page(x, id, m.lists[i].page);
    m.lists[i].my = p.list.len() as i16;
    let mut moved = 0;
    let step = m.proccess;
    match step {
        0 | 1 => {
            if m.proccess == 0 {
                m.exception_disp = 1;
                m.lists[i].y = m.lists[i].my.min(8);
                if m.first_time != 0 {
                    to_top(&mut m.lists[i]);
                    m.first_time = 0;
                }
                m.proccess += 1;
                moved = 1;
            }
            let old = m.lists[i].page;
            let push = x.pad.push.bits();
            if push & 0x8 != 0 {
                x.se(SE_MOVE);
                let l = &mut m.lists[i];
                l.page += 1;
                if l.page >= 5 {
                    l.page = 0;
                }
                moved += 1;
            } else if push & 0x4 != 0 {
                x.se(SE_MOVE);
                let l = &mut m.lists[i];
                l.page -= 1;
                if l.page < 0 {
                    l.page = 4;
                }
                moved += 1;
            }
            if m.lists[i].page != old {
                to_top(&mut m.lists[i]);
                p = page(x, id, m.lists[i].page);
                let l = &mut m.lists[i];
                l.my = p.list.len() as i16;
                l.y = l.my.min(8);
            } else {
                p = page(x, id, m.lists[i].page);
            }
            moved += cursor(m, x);
            let push = x.pad.push.bits();
            if x.pushed_cancel() {
                x.se(SE_BACK);
                m.msg.close();
                let prev = m.lists[i].prev;
                m.back_to_prev(prev);
            } else if x.pushed_ok() {
                let l = m.lists[i].clone();
                let new = cand(&p, l.select);
                if l.index == 1 && new >= 0 {
                    x.se(SE_OK);
                    let pc = id;
                    let s = &x.save.save;
                    let worn_n = piney_battle::item::get_item_num(s, pc, p.cat, p.worn);
                    if worn_n >= 99 {
                        (m.menu_status, m.wait_count, m.proccess) = (3, 1, 10);
                    } else if piney_battle::item::get_item_slot(s, pc) < 0
                        && worn_n <= 0
                        && piney_battle::item::get_item_num(s, pc, p.cat, new) >= 2
                    {
                        (m.menu_status, m.wait_count, m.proccess) = (3, 0, 10);
                    } else {
                        change(x, id, p.cat, p.worn, new);
                        let l = &mut m.lists[i];
                        l.index = 0;
                        l.select = -1;
                        l.dy = 0;
                        p.worn = equipment_num(x, id, p.cat);
                        moved += 1;
                        m.proccess += 1;
                    }
                }
            } else if push & 0x10 != 0 {
                let l = m.lists[i].clone();
                let item = match l.index {
                    0 => Some(p.worn),
                    1 => Some(cand(&p, l.select)),
                    _ => None,
                };
                if let Some(item) = item {
                    m.item_num = (p.cat << 16) | item;
                    m.msg.close();
                    x.se(SE_OK);
                    m.change_menu_to(64);
                }
            }
        }
        2 => {
            // The model task (ccThEquipMenu) is taken as done a frame on.
            m.proccess -= 1;
            warning(m, x);
        }
        10 => warning(m, x),
        11 if check(m, x) => {
            m.msg.close();
            m.menu_status = 1;
            m.exception_disp = 1;
            m.proccess = 0;
        }
        _ => {}
    }
    if moved != 0 {
        let l = m.lists[i].clone();
        let item = if l.select >= 0 { cand(&p, l.select) } else { p.worn };
        help(m, x, id, p.cat, item);
    }
    Flow::Done
}

/// Proccess 10's body: with the window closed, the warning.
fn warning(m: &mut MenuCtrl, x: &mut Ctx) {
    if m.menu_status != 0 {
        return;
    }
    m.exception_disp = 0;
    let w = x.texts.pers.equip_warn.get(m.wait_count.max(0) as usize).cloned().unwrap_or_default();
    let names = x.save.names();
    m.msg.open_info([Some(&w[0]), Some(&w[1]), Some(&w[2]), None], &names);
    m.proccess += 1;
}

/// OK on a candidate: the new piece from the list, the old into it, worn;
/// the stats again; the model.
fn change(x: &mut Ctx, id: i32, cat: i32, worn: i32, new: i32) {
    let order = x.texts.pers.category_order;
    piney_battle::item::del_item(&mut x.save.save, id, cat, new, 1);
    piney_battle::item::add_item(&mut x.save.save, &order, id, cat, worn, 1);
    let at = SPC_PARAM + SPC_SIZE * id.clamp(0, 17) as usize;
    let mut eq: [i16; 6] = std::array::from_fn(|k| x.save.save.i16(at + SPC_EQUIPMENT + 2 * k));
    let mut sk = skill_list(x, id);
    let job = x.save.save.i16(at + SPC_JOB);
    change_equipment(&x.texts.pers.equip_raw, &mut eq, &mut sk, cat, new, job);
    for (k, v) in eq.iter().enumerate() {
        x.save.save.set_i16(at + SPC_EQUIPMENT + 2 * k, *v);
    }
    for (k, v) in sk.iter().enumerate() {
        x.save.save.set_i16(items::SKILL_LIST + 40 * id.clamp(0, 17) as usize + 2 * k, *v);
    }
    let t = &x.texts.pers.battle;
    if let Some(c) = x.world.party.iter_mut().flatten().find(|c| i32::from(c.id) == id) {
        calc_real(t, x.save, c);
        let target = c.handle;
        x.req.push(Request::CalcReal { target });
        x.req.push(Request::ChangeEquip { target, cat, item: new });
    }
}

/// The help of the cursor's place (0x00537e24 on).
fn help(m: &mut MenuCtrl, x: &Ctx, id: i32, cat: i32, item: i32) {
    let pv = preview(x, id, cat, item);
    let l = m.lists[idx(m)].clone();
    let (sx, sy) = (i32::from(l.sx), i32::from(l.sy));
    let t = &x.texts.pers;
    let mut text = t.status_help.get((sx * 15 + sy).max(0) as usize).cloned().unwrap_or_default();
    let grid = sx > 0 && sy < 11;
    if grid {
        if sy < 8 {
            let s = pv.skills.get(((sx - 1) * 8 + sy) as usize).map_or(-1, |v| v.0);
            if s >= 0
                && let Some(p) = x.texts.items.skill(i32::from(s))
            {
                text = p.help.clone();
            }
        } else {
            let want = (sy - 8) + (sx - 1) * 3;
            let had = member(x, id).map(|c| c.condition).unwrap_or_default();
            let mut n = 0;
            for k in 0..5 {
                if pv.cond[k] != 0 || had[2 + k] != 0 {
                    if n == want {
                        text = t.beff_help[k].clone();
                    }
                    n += 1;
                }
            }
        }
    } else if l.index < 2 {
        text = t.status_help.get(60 + l.index.max(0) as usize).cloned().unwrap_or_default();
    }
    let names = x.save.names();
    let third = if grid { Some(&text[2][..]) } else { None };
    m.msg.change(0x100, None, [Some(&text[0]), Some(&text[1]), third], &names);
    m.msg.set_pos(39.0, 334.0);
}

/// The member's character (`ccSpcManager`'s by `CheckSpc`).
fn member<'a>(x: &'a Ctx, id: i32) -> Option<&'a crate::world::CharInfo> {
    x.world.party.iter().flatten().find(|c| i32::from(c.id) == id)
}

/// `EquipmentMenuDisp`.
pub fn equipment_menu_disp(m: &mut MenuCtrl, x: &mut Ctx) {
    let i = idx(m);
    let l = m.lists[i].clone();
    let id = i32::from(m.equip_spc_num());
    let p = page(x, id, l.page);
    let a = m.alpha;
    let fr = x.frame_rate;
    m.win.set_colour(7);
    m.win.set_alpha(a);
    for (dx, dy, f) in [(39.0, 16.0, 0), (214.0, 32.0, 1), (46.0, 260.0, 2), (46.0, 68.0, 3), (228.0, 182.0, 4)] {
        m.win.dx = dx;
        m.win.dy = dy;
        match f {
            0 => disp_square(&mut m.win, 29, 16, None),
            1 => disp_line_v(&mut m.win, 10),
            2 => disp_line_h(&mut m.win, 28),
            3 => disp_line_h(&mut m.win, 10),
            _ => disp_line_h(&mut m.win, 15),
        }
    }
    m.item_icon.set_colour(7);
    m.item_icon.set_alpha(a);
    // The page's labels (colour 17), then the piece worn.
    let bt = &x.texts.pers.battle;
    let name = |item: i32| bt.equip(p.cat, item).map(|e| e.name.clone()).unwrap_or_default();
    let mut buf = x.texts.pers.equip_change.get(l.page.max(0) as usize).cloned().unwrap_or_default();
    let k6 = &mut m.setting[6];
    k6.set_colour(17);
    k6.set_alpha(a);
    set_clm(k6, 16, 0, 0, 1);
    for (n, dx, dy) in [(0, 50.0, 30.0), (1, 116.0, 30.0), (2, 50.0, 86.0), (3, 116.0, 86.0)] {
        k6.dx = dx;
        k6.dy = dy;
        k6.make_packet(n);
    }
    k6.set_colour(7);
    k6.set_alpha(a);
    str_cat(&mut buf, &name(p.worn), 16);
    if l.index == 0 {
        m.cursor.disp(&mut m.win, 40.0, 50.0, 10, 0, a, 3, fr);
    }
    let k6 = &mut m.setting[6];
    k6.dx = 66.0;
    k6.dy = 50.0;
    k6.make_packet(4);
    let ic = &mut m.item_icon;
    ic.set_grid(14, 16, 14.0, 16.0, 0, 0x1200, 9);
    ic.dx = 50.0;
    ic.dy = 48.0;
    ic.make_packet(x.texts.items.item_icon(p.cat));
    m.setting_text[6] = buf;
    if l.y < l.my {
        set_type(&mut m.win, 1);
        m.win.dx = 50.0;
        m.win.dy = 90.0;
        disp_scroll_bar(&mut m.win, 10, i32::from(l.y), i32::from(l.dy), i32::from(l.my));
    }
    // The candidates.
    m.kanji.set_colour(7);
    m.kanji.set_alpha(a);
    set_clm(&mut m.kanji, 16, 0, 0, 1);
    m.kanji.set_alpha(a);
    let mut buf = Vec::new();
    let mut n = 0i32;
    for (k, &item) in p.list.iter().enumerate() {
        if (k as i32) < i32::from(l.dy) {
            continue;
        }
        str_cat(&mut buf, &name(item), 16);
        let y = n * 19;
        if l.index == 1 && n == i32::from(l.select) - i32::from(l.dy) {
            m.cursor.disp(&mut m.win, 40.0, from_int(y + 106), 10, i32::from(l.select) + 100, a, 3, fr);
        }
        m.kanji.dx = 66.0;
        m.kanji.dy = from_int(y + 106);
        m.kanji.make_packet(n);
        m.item_icon.set_alpha(a);
        m.item_icon.dx = 50.0;
        m.item_icon.dy = from_int(y + 104);
        m.item_icon.make_packet(x.texts.items.item_icon(p.cat));
        n += 1;
        if n >= i32::from(l.y) {
            break;
        }
    }
    extract_menu(m, buf);
    // What the piece under the cursor would make of the member.
    let item = if l.select >= 0 { cand(&p, l.select) } else { p.worn };
    let pv = preview(x, id, p.cat, item);
    font_type(&mut m.font, 1);
    m.font.set_colour(7);
    m.font.set_alpha(a);
    m.font.shadow = true;
    set_type(&mut m.win, 1);
    skill_disp(m, x, 220.0, 30.0, &pv.skills);
    let had: [i16; 5] = member(x, id).map_or([0; 5], |c| std::array::from_fn(|k| c.condition[2 + k]));
    beff_disp(m, x, 220.0, 196.0, Some(&had), Some(&pv.cond));
    let real = spc_real(x, id);
    param_disp(m, x, 40.0, 270.0, &pv.elm, &real);
    if l.index == 2 {
        let (sx, sy) = (i32::from(l.sx), i32::from(l.sy));
        let (cx, cy, w) = if sx > 0 && sy < 8 {
            ((sx - 1) * 126 + 221, sy * 17 + 47, 7)
        } else if sx > 0 && sy < 11 {
            ((sx - 1) * 126 + 221, (sy - 8) * 17 + 213, 7)
        } else if sy >= 11 {
            if sx >= 2 {
                ((sx - 2) * 90 + 292, (sy - 11) * 17 + 287, 4)
            } else {
                (sx * 126 + 40, (sy - 11) * 17 + 287, 6)
            }
        } else {
            (220, 196, 40)
        };
        m.cursor.disp(&mut m.win, from_int(cx), from_int(cy), w, sy + 200 + sx * 10, a, 3, fr);
    }
    for (dx, dy, b) in [(24.0, 200.0, 4), (24.0, 224.0, 13), (488.0, 200.0, 6), (488.0, 224.0, 12)] {
        m.win.dx = dx;
        m.win.dy = dy;
        disp_button(&mut m.win, b, x.count, x.frame_rate);
    }
}
