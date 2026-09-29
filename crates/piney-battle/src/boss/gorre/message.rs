//! `ccBoss05::SendMessage` (OUT gcmn 0x004adbb0): what a brother reports
//! to Gorre ([`Msg`]) and what Gorre does of it, the three's one HP among
//! it.

use piney_data::field::ee;

use super::brother::Msg;
use super::{
    CAM_Y, Gorre, ROW, act, change_action, change_next_pattern, cinema_off, cursor, entry_brothers, exec_pattern_index,
    on_brother, target_ok,
};
use crate::boss::{Boss, Cx, Out, Tbl};
use crate::item;

/// `@2074`: the spells a brother casts on Gorre's target 120 frames into
/// the Tornade, one by `ccRand() & 3`.
fn tornade_skill(cx: &mut Cx) -> i32 {
    let k = (cx.cc.rand() & 3).unsigned_abs() as usize;
    cx.data.gorre.tornade_skills[k]
}

/// `SendMessage(br, msg)`: brother `from`'s report, `cx.me` Gorre's own
/// character. The Wave's end, the Tornade's steps and a death wait on both
/// brothers' flags (and clear them, but the Tornade's end: the next
/// Tornade's first report ends it at once); an affect and a hold at once.
pub(super) fn send(b: &mut Boss, x: &mut Gorre, from: usize, msg: Msg, cx: &mut Cx) {
    let n = msg as usize;
    let both = x.brothers.iter().all(|p| p.x.sent[n]);
    match msg {
        Msg::Affect => return on_report_affect(b, x, from, cx),
        // `EntryAffect(other, br, 5, 0, 0, 0)`: the other held too.
        Msg::Held => return cx.affect(x.brothers[1 - from].me, 5, 0),
        _ if !both => return,
        Msg::WaveDone => {
            b.unlock_player();
            entry_brothers(x, cx);
        }
        Msg::Dead => b.exit = 1,
        Msg::TornadeHidden => {
            let me = cx.me;
            cx.scene.chars[me].pos = b.target_pos;
            let mut vp = b.target_pos;
            let mut eye = b.calc_effect_camera_pos(cx, vp, CAM_Y);
            eye[2] = ee::add(eye[2], 0x42c8_0000);
            vp[2] = ee::add(vp[2], 0x4348_0000);
            cx.out(Out::CameraPos { cam: 3, pos: eye });
            cx.out(Out::CameraView { cam: 3, view: vp });
        }
        Msg::TornadeDone => {
            cx.out(Out::CameraChange(2));
            b.unlock_player();
            entry_brothers(x, cx);
            cinema_off(cx);
            return change_next_pattern(b, x, cx);
        }
        Msg::TornadeSpell => {
            let me = cx.me;
            if let Some(t) = cx.scene.chars[me].target_char
                && target_ok(cx, Some(t))
            {
                let sid = tornade_skill(cx);
                let br1 = x.brothers[1].me;
                if let Some(k) = item::item_skill_request(cx.t, cx.scene, br1, t, sid, 0, false, cx.rand) {
                    cx.out(Out::SkillFrom { user: br1, target: t, skill: k });
                }
            }
        }
    }
    for p in &mut x.brothers {
        p.x.sent[n] = false;
    }
    if msg == Msg::WaveDone {
        change_next_pattern(b, x, cx);
    }
}

/// `SendMessage`'s msg 0: the brother's affect, by type. A hit (1, 3)
/// takes the three's one HP (a tenth, never under half, while `cheatHP`,
/// the broken protect shared), down to the death; a heal (7, 9) adds to
/// it; a drain (13) the Drain; 21 all three to row 4's stats at 6000.
fn on_report_affect(b: &mut Boss, x: &mut Gorre, from: usize, cx: &mut Cx) {
    let me = cx.me;
    let bm = x.brothers[from].me;
    let a = &cx.scene.chars[bm].affect;
    let (ty, p0) = (a.ty, a.param[0]);
    let (hp, mhp) = (cx.scene.chars[bm].hp, cx.scene.chars[bm].max_hp);
    match ty {
        21 => {
            for k in 0..2 {
                crate::boss::kyvia::set_base_param(cx, x.brothers[k].me, ROW);
            }
            for c in [me, x.brothers[0].me, x.brothers[1].me] {
                let ch = &mut cx.scene.chars[c];
                ch.hp = 6000;
                ch.max_hp = 6000;
                if let Some(f) = ch.foe_state_mut() {
                    f.pp = -1;
                    f.pp_count = 0;
                }
            }
        }
        13 => {
            change_action(b, x, cx, act::DRAIN, 2, true);
            cx.scene.chars[bm].affect.ty = 0;
            b.cheat_hp = 0;
        }
        7 | 9 => {
            for _ in 0..2 {
                cx.out(Out::FlyFont { kind: 20, n: i32::from(p0) });
            }
            share_hp(x, me, cx, hp.wrapping_add(p0).min(mhp));
        }
        1 | 3 => {
            if cx.annihilated() || cx.game_over {
                return;
            }
            let mut h = if b.cheat_hp != 0 {
                share_protect(x, me, from, cx);
                hp.wrapping_sub(p0 / 10).max(mhp / 2)
            } else {
                hp.wrapping_sub(p0)
            };
            if h <= 0 {
                h = 0;
                fall(b, x, cx);
            } else if cx.scene.chars[me].affect.ty == 1 {
                // Gorre's own last affect, not the brother's.
                let n = if cx.env.count & 1 != 0 { act::DMG0 } else { act::DMG1 };
                change_action(b, x, cx, n, 0, true);
            }
            share_hp(x, me, cx, h);
            // `GetPP() >= GetMaxPP() / 2`: the Super table, once.
            let gp = cx.scene.chars[me].foe_state().map_or(0, |f| f.pp);
            let max = cx.t.bosses.get(ROW).map_or(0, |r| r.max_pp);
            if b.pat_mode == 0 && gp >= max >> 1 {
                b.set_pattern_tbl(Tbl::Super);
                b.pat_mode = 1;
                let tbl = cx.data.gorre.words(Tbl::Super).to_vec();
                b.pat_index = exec_pattern_index(b, x, cx, &tbl, 0);
            }
        }
        _ => {}
    }
}

/// The three's one HP.
fn share_hp(x: &Gorre, me: usize, cx: &mut Cx, hp: i16) {
    for c in [me, x.brothers[0].me, x.brothers[1].me] {
        cx.scene.chars[c].hp = hp;
    }
}

/// A hit under `cheatHP`: the struck brother's broken-protect count is
/// Gorre's; at a fresh break (300) the other's protect breaks with it; its
/// gauge is Gorre's and the other's.
fn share_protect(x: &Gorre, me: usize, from: usize, cx: &mut Cx) {
    let (bm, other) = (x.brothers[from].me, x.brothers[1 - from].me);
    let Some((pp, count)) = cx.scene.chars[bm].foe_state().map(|f| (f.pp, f.pp_count)) else { return };
    let chars = &mut cx.scene.chars;
    if count >= 0
        && let Some(f) = chars[me].foe_state_mut()
    {
        f.pp_count = count;
    }
    if count == 300
        && let Some(f) = chars[other].foe_state_mut()
    {
        f.pp = 0;
        f.pp_count = 300;
    }
    for c in [me, other] {
        if let Some(f) = chars[c].foe_state_mut() {
            f.pp = pp;
        }
    }
}

/// The three down (HP 0): the boss camera off, the party's conditions
/// cleared and the party held, the brothers untargetable, the music out,
/// Gorre's Dead (forbid 2).
fn fall(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    if std::mem::take(&mut x.cam_sw) {
        cx.out(Out::BossCamOff);
    }
    cx.out(Out::ClearSpcCondition);
    b.lock_player(cx, false);
    for k in 0..2 {
        on_brother(x, k, cx, |bb, cx| bb.erase_cmnd_target(cx));
    }
    cursor(cx, true);
    cx.out(Out::MusicFade { t: 30 });
    change_action(b, x, cx, act::DEAD, 2, true);
}
