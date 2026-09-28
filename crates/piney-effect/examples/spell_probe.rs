//! Answers `tools/test_effect_spell_rs.py` and `tools/test_effect_skill_rs.py`:
//! runs the port's spells and the skills' other effects on the requests it is
//! sent and prints each answer as one JSON line, so the tests can run the same
//! frames through the game's own code in eemu (`spell_probe ISO < requests`).
//! Numbers are hex; floats travel as their bit patterns. The requests (`reset`,
//! `player`, `camera`, `char`, `skill`, `frame`, `posp`, `affect`, the
//! starters, `spawn`, `set`, `endflag`) are the tests'. `ccSkill::Main` is
//! the battle crate's; the probe does what it does for a spell around
//! [`Effects::spell_system`].

use std::collections::HashMap;
use std::io::BufRead;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_effect::draw::{Camera, DrawRec};
use piney_effect::drawelm::{AnmObj, DrawElm, DrawKind};
use piney_effect::eff::Eff;
use piney_effect::effect::{Effect, Obj};
use piney_effect::element::ElmPtr;
use piney_effect::element::{Animate, Base, Element};
use piney_effect::files::ObjRef;
use piney_effect::particle::Generator;
use piney_effect::spell::Spell;
use piney_effect::summoned::{SummonKind, Summoned};
use piney_effect::thunder::{Effect2, ThunderBolt, ThunderData};
use piney_effect::upheaval::{MngrKind, PartKind};
use piney_effect::{CharRef, Effects, Event, Host, IntRef, ONE, V4, VecRef, convergence, summons};
use piney_world::Rand;
use piney_world::mt::Mt;

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list(v: &[u32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

#[derive(Clone, Copy)]
struct Char {
    pos: V4,
    dirc: V4,
    height: u32,
    width: u32,
    listed: bool,
    dead: i16,
    ty: i32,
    size: i32,
    skill: [i16; 2],
    pos_p: Option<V4>,
    affect: Option<CharRef>,
    /// +0x1e0, an idol's `effsw`.
    effsw: i32,
}

struct Probe {
    rand: Rand,
    mt: Mt,
    draws: u64,
    player: V4,
    camera: Camera,
    area: (i32, i32, i32),
    ground: u32,
    chars: HashMap<CharRef, Char>,
}

impl Probe {
    fn new(seed: u64, gseed: u32, area: (i32, i32, i32)) -> Probe {
        Probe {
            rand: Rand(seed),
            mt: Mt::seeded(gseed),
            draws: 0,
            player: [0, 0, 0, ONE],
            camera: Camera::default(),
            area,
            ground: 0,
            chars: HashMap::new(),
        }
    }

    fn ch(&self, c: CharRef) -> Option<&Char> {
        self.chars.get(&c)
    }
}

impl Host for Probe {
    fn rand(&mut self) -> i32 {
        self.rand.rand()
    }
    fn player_pos(&self) -> V4 {
        self.player
    }
    fn camera(&self) -> Camera {
        self.camera
    }
    fn char_pos(&self, c: CharRef) -> V4 {
        self.ch(c).map_or([0, 0, 0, ONE], |c| c.pos)
    }
    fn char_dirc(&self, c: CharRef) -> V4 {
        self.ch(c).map_or([0; 4], |c| c.dirc)
    }
    fn char_height(&self, c: CharRef) -> u32 {
        self.ch(c).map_or(0, |c| c.height)
    }
    fn char_width(&self, c: CharRef) -> u32 {
        self.ch(c).map_or(0, |c| c.width)
    }
    fn check_target(&self, c: CharRef) -> bool {
        self.ch(c).is_some_and(|c| c.listed)
    }
    fn char_dead(&self, c: CharRef) -> i16 {
        self.ch(c).map_or(0, |c| c.dead)
    }
    fn object_size(&self, c: CharRef) -> i32 {
        self.ch(c).map_or(0, |c| c.size)
    }
    fn char_type(&self, c: CharRef) -> i32 {
        self.ch(c).map_or(0, |c| c.ty)
    }
    fn genrand(&mut self) -> u32 {
        self.draws += 1;
        self.mt.genrand()
    }
    fn area(&self) -> (i32, i32, i32) {
        self.area
    }
    fn char_pos_p(&self, c: CharRef) -> V4 {
        self.ch(c).and_then(|c| c.pos_p).unwrap_or([0; 4])
    }
    fn affect_person(&self, c: CharRef) -> Option<CharRef> {
        self.ch(c).and_then(|c| c.affect)
    }
    /// The ccChar's word at +0x1e0 (an idol's `effsw`), else on.
    fn char_int(&self, c: CharRef, off: u16) -> i32 {
        match off {
            piney_effect::gimmick::IDOL_EFFSW => self.ch(c).map_or(0, |c| c.effsw),
            _ => 1,
        }
    }
    /// The test's flat land at `ground`.
    fn land_hit_check2(&mut self, pos: V4, offset_z: u32, _mask: u32) -> u32 {
        let end = piney_effect::ee::add(pos[2], offset_z);
        let (a, b) = (f32::from_bits(pos[2]), f32::from_bits(end));
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let g = f32::from_bits(self.ground);
        if lo <= g && g <= hi { self.ground } else { piney_effect::NO_HIT }
    }
    /// The test's flat land at `ground`, met on the segment from 105 above
    /// `pos` to 1000 below; else `pos.z`.
    fn land_hit_check(&mut self, pos: V4, _mask: u32) -> u32 {
        let top = f32::from_bits(piney_effect::ee::add(pos[2], 0x42d2_0000));
        let bottom = f32::from_bits(piney_effect::ee::sub(pos[2], 0x447a_0000));
        let g = f32::from_bits(self.ground);
        if bottom <= g && g <= top { self.ground } else { pos[2] }
    }
}

fn who(r: Option<CharRef>) -> i64 {
    r.map_or(-1, i64::from)
}

fn vref(r: Option<VecRef>) -> String {
    match r {
        None => "null".into(),
        Some(VecRef::CharPos(c)) => format!("[\"pos\", {c}]"),
        Some(VecRef::CharDirc(c)) => format!("[\"dirc\", {c}]"),
        Some(VecRef::EffectPos(k)) => format!("[\"effpos\", {k}]"),
        Some(VecRef::EffectRot(k)) => format!("[\"effrot\", {k}]"),
        Some(VecRef::EffectPosT(k)) => format!("[\"effposT\", {k}]"),
        Some(VecRef::CharAt(c, off)) => format!("[\"at\", {c}, {off}]"),
        Some(VecRef::Anchor(k)) => format!("[\"anchor\", {k}]"),
    }
}

fn gref(r: Option<VecRef>) -> String {
    match r {
        Some(VecRef::EffectPos(k)) => format!("[\"eff\", {k}, 0]"),
        Some(VecRef::EffectRot(k)) => format!("[\"eff\", {k}, 32]"),
        r => vref(r),
    }
}

fn slot(fx: &Effects, i: usize, e: &Effect) -> String {
    let bits = u32::from(e.dist_sw)
        | u32::from(e.disp_sw) << 1
        | u32::from(e.pause_sw) << 2
        | u32::from(e.end_flag) << 3
        | u32::from(e.zyx_flag) << 4
        | (u32::from(e.level as u8) & 15) << 5;
    let obj = match &e.obj {
        Obj::None => "null".into(),
        Obj::Clump(o) => format!("[\"clump\", \"{}\"]", fx.assets.name(*o)),
        Obj::Anm { obj, play, .. } => format!("[\"anm\", \"{}\", {}]", fx.assets.name(*obj), play.time),
        Obj::Eff(f) => format!(
            "[\"eff\", {}, {}, {}, {}, {}, {}, {}, {}, {}]",
            f.scale_x,
            f.scale_y,
            f.rotate,
            f.color,
            f.transparency,
            f.flag,
            list(&f.pos),
            f.prim,
            f.clut.map_or("null".into(), |c| format!("\"{}\"", fx.assets.name(c)))
        ),
    };
    // The convergence's pieces keep a pointer to their controller's
    // temp[0]: its slot.
    let temp = if (convergence::PIECE_FIRST..=convergence::PIECE_LAST).contains(&e.id) {
        format!("[[\"eff\", {}], {}, {}, {}]", e.temp[0], e.temp[1], e.temp[2], e.temp[3])
    } else if e.id == summons::DRILL {
        // Its two ccAnm (temp[0], temp[1]): the port keeps them aside.
        let a = |k: usize| {
            fx.spells.drills.get(&i).and_then(|d| d.anm[k].as_ref()).map_or("null".into(), |o| match o {
                Obj::Anm { obj, play, .. } => format!("[\"anm\", \"{}\", {}]", fx.assets.name(*obj), play.time),
                _ => "null".into(),
            })
        };
        format!("[{}, {}, {}, {}]", a(0), a(1), e.temp[2], e.temp[3])
    } else if e.id == summons::LOCKON && e.temp[0] != 0 {
        // The lock-on's pointer to its count: slot + 1 in the port.
        format!("[[\"eff\", {}], {}, {}, {}]", e.temp[0] - 1, e.temp[1], e.temp[2], e.temp[3])
    } else {
        list(&e.temp)
    };
    format!(
        concat!(
            "{{\"i\": {}, \"id\": {}, \"status\": {}, \"life\": {}, \"age\": {}, \"cnt\": {}, \"pat\": {}, ",
            "\"bits\": {}, \"param\": {}, \"flags\": {}, \"pos\": {}, \"offset\": {}, \"rot\": {}, ",
            "\"speed\": {}, \"scale\": {}, \"posT\": {}, \"rotSpeed\": {}, \"velocity\": {}, ",
            "\"transparency\": {}, \"target\": {}, \"posPtr\": {}, \"rotPtr\": {}, \"link\": {}, ",
            "\"temp\": {}, \"sn\": {}, \"obj\": {}}}"
        ),
        i,
        e.id,
        e.status,
        e.life_time,
        e.age,
        e.cnt,
        e.tex_anm_pat,
        bits,
        e.param,
        e.flags,
        list(&e.pos),
        list(&e.offset),
        list(&e.rot),
        list(&e.speed),
        list(&e.scale),
        list(&e.pos_t),
        list(&e.rot_speed.map(u32::from)),
        e.velocity,
        e.transparency,
        who(e.target),
        vref(e.pos_ptr),
        vref(e.rot_ptr),
        e.link.map_or(-1, |l| l as i64),
        temp,
        e.sn,
        obj
    )
}

fn draw(fx: &Effects, d: &DrawRec) -> String {
    match d {
        DrawRec::Clump { obj, matrix, alpha, layer, clut } => {
            let mut s =
                format!("[\"clump\", \"{}\", {}, {}, {}", fx.assets.name(*obj), list(&matrix.concat()), alpha, layer);
            if clut.is_some() {
                s += &format!(", {}", clut_names(fx, *obj, *clut));
            }
            s + "]"
        }
        DrawRec::Anm { obj, play, matrix, alpha, layer } => format!(
            "[\"anm\", \"{}\", {}, {}, {}, {}]",
            fx.assets.name(*obj),
            play.time,
            list(&matrix.concat()),
            alpha,
            layer
        ),
        DrawRec::Eff { eff, pat, layer } => format!(
            "[\"eff\", {}, {}, {}, {}, {}, {}, {}, {}, {}]",
            eff.chunk,
            pat,
            list(&eff.pos),
            eff.scale_x,
            eff.scale_y,
            eff.rotate,
            eff.color,
            eff.transparency,
            layer
        ),
    }
}

fn event(e: &Event) -> Option<String> {
    Some(match e {
        Event::Sound3d { se, pos } => format!("[\"se3d\", {se}, {}]", list(pos)),
        Event::Sound3dNote { se, pos, note } => format!("[\"se3dnote\", {se}, {}, {note}]", list(pos)),
        Event::CameraShake([power, cycle, time, dirc]) => format!("[\"shake\", {power}, {cycle}, {time}, {dirc}]"),
        Event::Noise { bs } => format!("[\"noise\", {bs}]"),
        Event::SoundNote { se, note } => format!("[\"senote\", {se}, {note}]"),
        Event::SkillDamage { spell, attacker, target, sid } => {
            format!("[\"damage\", {}, {}, {spell}, {sid}]", who(*attacker), who(*target))
        }
        Event::SkillDamageAt { attacker, pos, ttype, sid } => {
            format!("[\"damage_at\", {}, {}, {ttype}, {sid}]", who(*attacker), list(pos))
        }
        Event::SkillDamage2 { spell, attacker, target, pos, ttype, sid } => {
            format!("[\"damage2\", {}, {}, {}, {ttype}, {spell}, {sid}]", who(*attacker), who(*target), list(pos))
        }
        Event::SkillRelease { .. } => return None,
        Event::Flash { time, color, rect } => format!("[\"flash\", {time}, {color}, {}]", list(rect)),
    })
}

/// A JSON object built member by member, named as DWARF names the game's.
struct J(Vec<String>);

impl J {
    fn new() -> J {
        J(Vec::new())
    }
    fn raw(&mut self, k: &str, v: String) -> &mut J {
        self.0.push(format!("\"{k}\": {v}"));
        self
    }
    fn i(&mut self, k: &str, v: impl Into<i64>) -> &mut J {
        self.raw(k, v.into().to_string())
    }
    fn u(&mut self, k: &str, v: u32) -> &mut J {
        self.raw(k, v.to_string())
    }
    fn v(&mut self, k: &str, v: &[u32]) -> &mut J {
        self.raw(k, list(v))
    }
    fn s(&self) -> String {
        format!("{{{}}}", self.0.join(", "))
    }
}

fn opt<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map_or("null".into(), |v| v.to_string())
}

fn m4(m: &[[u32; 4]; 4]) -> String {
    format!("[{}]", m.iter().map(|c| list(c)).collect::<Vec<_>>().join(", "))
}

/// `ccAnimateObject`'s members.
fn anim_fields(j: &mut J, a: &Animate) {
    j.u("m_startFade", a.start_fade).u("m_endFade", a.end_fade).u("m_fadeSpd", a.fade_spd);
    j.u("m_fadeAccel", a.fade_accel).i("m_fadeFlag", a.fade_flag);
    j.v("m_startScale", &a.start_scale).v("m_endScale", &a.end_scale).v("m_scaleSpd", &a.scale_spd);
    j.v("m_scaleAccel", &a.scale_accel).i("m_scaleFlag", a.scale_flag).v("m_accel", &a.accel);
    j.v("m_startSpeed", &a.start_speed).v("m_endSpeed", &a.end_speed).i("m_posFlag", a.pos_flag);
    j.i("m_accelFlag", a.accel_flag).v("m_pos", &a.pos).v("m_dirc", &a.dirc).v("m_offset", &a.offset);
    j.v("m_speed", &a.speed).v("m_sp", &a.sp).v("m_ep", &a.ep).v("m_scale", &a.scale);
    j.u("m_transparency", a.transparency);
}

/// An embedded `ccAnimateObject`.
fn animate(a: &Animate) -> String {
    let mut j = J::new();
    anim_fields(&mut j, a);
    j.s()
}

/// `ccAnimateObject` and `ccEffectElement`'s members.
fn base(j: &mut J, b: &Base) {
    anim_fields(j, &b.anim);
    j.raw("m_skillPtr", opt(b.skill)).raw("m_target", opt(b.target)).i("m_level", b.level);
    j.i("m_endFlag", b.end_flag).i("m_delFlag", b.del_flag).i("m_life", b.life).i("m_attr", b.attr);
    j.i("m_count", b.count).i("m_proccess", b.proccess).raw("m_status", format!("{:?}", b.status));
    j.i("m_interval", b.interval).v("m_accel@170", &b.accel);
    j.raw("m_syncPos", vref(b.sync_pos)).raw("m_syncDirc", vref(b.sync_dirc));
}

fn thunder_data(d: &ThunderData) -> String {
    let mut j = J::new();
    j.u("DefaultAngle", d.default_angle).u("RandAngle", d.rand_angle).u("DefaultScale", d.default_scale);
    j.u("RandScale", d.rand_scale).i("Mode", d.mode).i("EFF_SW", d.eff_sw).v("RndPoint", &d.rnd_point);
    j.v("OffSet", &d.off_set);
    j.s()
}

fn bolt(fx: &Effects, t: &ThunderBolt) -> String {
    let mut j = J::new();
    base(&mut j, &t.base);
    let tbl: Vec<String> = t.thunder_tbl.iter().map(|p| format!("[{}, {}]", list(&p.spot), p.break_point)).collect();
    j.raw("m_thunderTbl", format!("[{}]", tbl.join(", "))).v("CenterPos", &t.center_pos).v("TopPos", &t.top_pos);
    j.raw("Pos", "null".into()).raw("Scale", "null".into()).u("TopRange", t.top_range);
    j.u("BottomRange", t.bottom_range).i("Thunder_Mode", t.thunder_mode).i("Num", t.num).i("flg", t.flg);
    j.i("flg2", t.flg2).i("hoge", t.hoge).raw("DefaultData", thunder_data(&t.default_data));
    j.u("DefaultAngle", t.default_angle).u("RandAngle", t.rand_angle).i("DefaultScale", t.default_scale);
    j.i("RandScale", t.rand_scale).v("RndPoint", &t.rnd_point).raw("SpMat", m4(&t.sp_mat));
    j.raw("Effccsc", "\"particle\"".into());
    j.raw("EffCmp", clump(fx, t.eff_cmp, t.clut)).i("Time", t.time).i("EndFlg", t.end_flg);
    format!("[\"ccThunderBoltElement\", {}]", j.s())
}

/// A CLUT swap (from, to) of `obj`'s file as the harness records
/// `ChangeClut(to, from)`: `[[to, from]]` by name.
fn clut_names(fx: &Effects, obj: ObjRef, clut: Option<(u32, u32)>) -> String {
    let n = |o: u32| format!("\"{}\"", fx.assets.name(ObjRef { file: obj.file, object: o }));
    clut.map_or("null".into(), |(f, t)| format!("[[{}, {}]]", n(t), n(f)))
}

/// A `ccClump *`: its object and CLUT swap.
fn clump(fx: &Effects, obj: Option<ObjRef>, clut: Option<(u32, u32)>) -> String {
    let name = |o: ObjRef| format!("\"{}\"", fx.assets.name(o));
    match obj {
        None => "null".into(),
        Some(o) => {
            let c = clut_names(fx, o, clut);
            format!("[{}, {}]", name(o), c)
        }
    }
}

fn cstr(s: Option<&str>) -> String {
    s.map_or("null".into(), |s| format!("\"{s}\""))
}

fn element(fx: &Effects, e: &Element) -> String {
    let mut j = J::new();
    let cls = match e {
        Element::Busy => "busy",
        Element::Ring(r) => {
            base(&mut j, &r.base);
            j.v("Pos", &r.pos).v("Rot", &r.rot).v("Scale", &r.scale).v("Spoint", &r.spoint);
            j.raw("CMPSTR", cstr(r.cmp_str)).raw("CLUTSTR", cstr(r.clut_str)).raw("CLUT", cstr(r.clut_name));
            j.i("EnyFlg", r.eny_flg).i("Flg", r.flg).raw("FlgP", "null".into()).u("Transparency", r.transparency);
            j.u("Tpoint", r.tpoint).raw("Effccsc", "\"particle\"".into()).raw("EffCmp", clump(fx, r.eff_cmp, r.clut));
            "ccRingElement"
        }
        Element::Tornade(t) => {
            base(&mut j, &t.base);
            j.i("m_skillPtr@190", t.skill).raw("m_offsets", m4(&t.offsets)).i("m_index", t.index);
            j.v("m_tPos", &t.t_pos).i("m_skillType", t.skill_type);
            "ccTornadeElement"
        }
        Element::Fall(f) => return fall_elm(fx, f),
        Element::ThunderFall(f) => {
            base(&mut j, &f.base);
            j.raw("m_drawElmTbl", format!("[{}]", vec!["null"; 256].join(", "))).i("m_elmNum", f.elm_num);
            let el: Vec<String> = f
                .elements
                .iter()
                .map(|s| {
                    let mut k = J::new();
                    k.raw("elm", draw_elm(fx, &s.elm)).i("interval", s.interval).i("status", s.status);
                    k.s()
                })
                .collect();
            j.raw("m_elements", format!("[{}]", el.join(", ")));
            j.i("m_skillPtr@5a4", f.skill).v("m_tPos", &f.t_pos);
            "ccThunderFallElement"
        }
        Element::Convergence(c) => {
            base(&mut j, &c.base);
            j.raw("m_drawElmTbl", format!("[{}]", vec!["null"; 256].join(", "))).i("m_elmNum", c.elm_num);
            let el: Vec<String> = c
                .elements
                .iter()
                .map(|s| {
                    let mut k = J::new();
                    k.raw("elm", draw_elm(fx, &s.elm)).i("interval", s.interval);
                    k.raw("cp", format!("[{}, {}]", list(&s.cp[0]), list(&s.cp[1]))).u("time", s.time);
                    k.i("status", s.status).i("life", s.life);
                    k.s()
                })
                .collect();
            j.raw("m_elements", format!("[{}]", el.join(", "))).i("m_skillPtr@5a4", c.skill);
            j.i("m_generateSEOne", c.generate_se_one).i("m_moveSEOne", c.move_se_one);
            j.i("m_shockSEOne", c.shock_se_one).v("m_tPos", &c.t_pos);
            "ccConvergenceElement"
        }
        Element::Draw(d) => return draw_elm(fx, d),
        Element::Upheaval(u) => {
            base(&mut j, &u.base);
            j.raw("m_drawElmTbl", format!("[{}]", vec!["null"; 256].join(", "))).i("m_elmNum", 0);
            j.i("m_skillPtr@5a0", u.skill);
            match &u.kind {
                MngrKind::Soil | MngrKind::Dark => {}
                MngrKind::Ice(p) => {
                    let el: Vec<String> = p
                        .elements
                        .iter()
                        .map(|s| {
                            let e = s.elm.as_ref().map_or("null".into(), |e| draw_elm(fx, e));
                            format!("{{\"elm\": {e}, \"status\": {}}}", s.status)
                        })
                        .collect();
                    j.raw("m_elements", format!("[{}]", el.join(", ")));
                    j.raw("m_bigIce", p.big_ice.as_ref().map_or("null".into(), |e| draw_elm(fx, e)));
                    j.raw("m_animateBigIce", animate(&p.animate_big_ice));
                }
                MngrKind::Tree(p) => {
                    j.raw("m_bigTree", clump(fx, p.big_tree, None)).v("m_treeOffset", &p.tree_offset);
                    j.raw("m_anmRoots", anm(fx, &p.anm_roots)).raw("m_treeAnimate", animate(&p.tree_animate));
                    j.raw("m_gpLeaf", if p.gp_leaf.is_some() { "\"ptr\"".into() } else { "null".into() });
                }
            }
            j.v("m_tPos", &u.t_pos);
            u.class()
        }
        Element::UpheavalPart(u) => {
            base(&mut j, &u.base);
            match &u.kind {
                PartKind::Rock { rock } => {
                    j.raw("m_rock", clump(fx, *rock, None));
                }
                PartKind::Tree { tree } => {
                    j.raw("m_tree", clump(fx, *tree, None));
                }
                PartKind::Hand { hand, offset } => {
                    j.raw("m_hand", anm(fx, hand)).v("m_offset@1a0", offset);
                }
            }
            u.class()
        }
        Element::Summons(u) => {
            base(&mut j, &u.base);
            j.raw("m_drawElmTbl", format!("[{}]", vec!["null"; 256].join(", "))).i("m_elmNum", 0);
            j.raw("m_effCircle", elm_ptr(fx, u.circle)).raw("m_effSummon", summoned(fx, u.summoned()));
            j.i("m_skillPtr@5a8", u.skill);
            "ccSummonsSystemElement"
        }
        Element::Shield(u) => {
            base(&mut j, &u.base);
            let a = u
                .anm
                .as_ref()
                .map_or("null".into(), |(o, p)| format!("[\"anm\", \"{}\", {}]", fx.assets.name(*o), p.time));
            j.raw("m_anmShield", a).i("m_targetChar", u.target);
            "ccResistantShieldElement"
        }
        Element::EnergyGrow(u) => {
            base(&mut j, &u.base);
            j.raw("m_energy", eff_state(fx, u.energy.as_deref()));
            let t: Vec<String> = u
                .tbl
                .iter()
                .map(|p| {
                    let mut k = J::new();
                    k.u("transparency", p.transparency).v("pos", &p.pos).i("status", p.status);
                    k.i("anmIndex", p.anm_index).u("velocity", p.velocity).u("fadeIn", p.fade_in);
                    k.u("fadeOut", p.fade_out).u("noFade", p.no_fade);
                    k.s()
                })
                .collect();
            j.raw("m_energyTbl", format!("[{}]", t.join(", "))).i("m_tblNum", u.tbl.len() as i64);
            j.i("m_genV", u.gen_v).i("m_genA", u.gen_a).u("m_genRnd", u.gen_rnd);
            j.u("m_movV", u.mov[0]).u("m_movA", u.mov[1]).u("m_movMax", u.mov[2]).u("m_movRnd", u.mov[3]);
            j.u("m_radius", u.rad[0]).u("m_radV", u.rad[1]).u("m_radMax", u.rad[2]).u("m_radRnd", u.rad[3]);
            j.u("m_fadeIn", u.fades[0]).u("m_noFade", u.fades[1]).u("m_fadeOut", u.fades[2]);
            j.u("m_fadeRnd", u.fades[3]);
            "ccEnergyGrowElement"
        }
    };
    format!("[\"{cls}\", {}]", j.s())
}

/// A pointer to an element in the manager: its slot, "gone" once it is
/// not there.
fn elm_ptr(fx: &Effects, p: Option<ElmPtr>) -> String {
    match p {
        None => "null".into(),
        Some(p) => match fx.spells.elements.get_ptr(p) {
            Some(_) => p.slot.to_string(),
            None => "\"gone\"".into(),
        },
    }
}

/// A `ccEff` as the checks compare it: its chunk, position, scale, turn,
/// colour, transparency, PRIM and CLUT.
fn eff_state(fx: &Effects, e: Option<&Eff>) -> String {
    let Some(e) = e else { return "null".into() };
    let clut = e.clut.map_or("null".into(), |c| format!("\"{}\"", fx.assets.name(c)));
    format!(
        "[{}, {}, {}, {}, {}, {}, {}, {}, {clut}]",
        e.chunk,
        list(&e.pos),
        e.scale_x,
        e.scale_y,
        e.rotate,
        e.color,
        e.transparency,
        e.prim
    )
}

fn gp(g: Option<u32>) -> String {
    if g.is_some() { "\"ptr\"".into() } else { "null".into() }
}

fn v2(v: &[V4; 2]) -> String {
    format!("[{}, {}]", list(&v[0]), list(&v[1]))
}

/// A summoned element (`m_effSummon`).
fn summoned(fx: &Effects, u: &Summoned) -> String {
    let mut j = J::new();
    base(&mut j, &u.base);
    j.raw("m_drawElmTbl", format!("[{}]", vec!["null"; 256].join(", "))).i("m_elmNum", u.elm_num);
    let arr = |v: Vec<String>| format!("[{}]", v.join(", "));
    match &u.kind {
        SummonKind::Fire { elements } => {
            let el =
                elements.iter().map(|s| format!("{{\"elm\": {}, \"interval\": {}}}", draw_elm(fx, &s.elm), s.interval));
            j.raw("m_elements", arr(el.collect()));
        }
        SummonKind::Water { elements, ice_fall } => {
            let el = elements.as_ref().map_or("null".into(), |els| {
                arr(els
                    .iter()
                    .map(|s| format!("{{\"elm\": {}, \"interval\": {}}}", draw_members(fx, &s.elm), s.interval))
                    .collect())
            });
            j.raw("m_elements", el).raw("m_gp", "null".into());
            j.raw("m_iceFall", ice_fall.as_ref().map_or("null".into(), |f| fall_elm(fx, f)));
        }
        SummonKind::Thunder { bolts, small, base_pos, shock_se_one } => {
            let b =
                bolts.iter().map(|b| format!("{{\"elm\": {}, \"status\": {}}}", draw_members(fx, &b.elm), b.status));
            j.raw("m_thunderTbl", arr(b.collect()));
            let s =
                small.iter().map(|s| format!("{{\"thunder\": {}, \"status\": {}}}", elm_ptr(fx, s.thunder), s.status));
            j.raw("m_smallThunderTbl", arr(s.collect())).v("m_basePos", base_pos).i("m_shockSEOne", *shock_se_one);
        }
        SummonKind::Dark { bats, bat_generate_se, bat_shock_se, smoke_generate_se_one, balls, dark_ball } => {
            let el = bats.as_ref().map_or("null".into(), |v| {
                arr(v
                    .iter()
                    .map(|b| {
                        let mut k = J::new();
                        k.raw("elm", draw_members(fx, &b.elm)).i("status", b.status).v("spP", &b.sp).v("epP", &b.ep);
                        k.raw("cpP", v2(&b.cp)).u("splineTime", b.t).i("timeMode", b.time_mode);
                        k.s()
                    })
                    .collect())
            });
            j.raw("m_elements", el).i("m_batGenerateSE", *bat_generate_se).i("m_batShockSE", *bat_shock_se);
            j.i("m_smokeGenerateSEOne", *smoke_generate_se_one);
            let bl = balls.as_ref().map_or("null".into(), |v| {
                arr(v
                    .iter()
                    .map(|b| {
                        let mut k = J::new();
                        k.raw("elm", draw_members(fx, &b.elm)).v("spP", &b.sp).v("epP", &b.ep).v("cpP", &b.cp[0]);
                        k.i("status", b.status)
                            .u("splineTime", b.t)
                            .u("addTime", b.add_time)
                            .i("timeMode", b.time_mode);
                        k.s()
                    })
                    .collect())
            });
            j.raw("m_ballTbl", bl).raw("m_darkBall", draw_elm(fx, dark_ball));
        }
        SummonKind::Soil { elements } => {
            let el = elements.iter().map(|s| {
                format!(
                    "{{\"elm\": {}, \"status\": {}, \"interval\": {}}}",
                    draw_members(fx, &s.elm),
                    s.status,
                    s.interval
                )
            });
            j.raw("m_elements", arr(el.collect()));
        }
        SummonKind::Tree { needles, roots, gp_leaf, cmp_tree, anm_roots, tree_alpha } => {
            let el = needles.as_ref().map_or("null".into(), |v| {
                arr(v
                    .iter()
                    .map(|n| {
                        let mut k = J::new();
                        k.raw("elm", draw_members(fx, &n.elm)).i("interval", n.interval).i("status", n.status);
                        k.raw("cp", v2(&n.cp)).u("splineTime", n.t);
                        k.s()
                    })
                    .collect())
            });
            j.raw("m_needleTbl", el);
            j.raw("m_rootsTbl", arr(roots.iter().map(|p| format!("{{\"pos\": {}}}", list(p))).collect()));
            j.raw("m_gpLeaf", gp(*gp_leaf)).raw("m_cmpTree", clump(fx, *cmp_tree, None));
            j.raw("m_anmRoots", anm(fx, anm_roots)).u("m_treeAlpha", *tree_alpha);
        }
        SummonKind::Goblin { gen_pos, gen_pos_p, elements, stars } => {
            j.v("m_genPos", gen_pos).v("m_genPosP", gen_pos_p);
            let el = elements.as_ref().map_or("null".into(), |v| {
                arr(v
                    .iter()
                    .map(|e| {
                        let mut k = J::new();
                        k.raw("gp", gp(e.gp)).raw("star", draw_members(fx, &e.star)).i("status", e.status);
                        k.v("pos", &e.pos).u("splineTime", e.t).u("splineSpd", e.spd).raw("cp", v2(&e.cp));
                        k.raw("cpP", v2(&e.cp_p)).v("ep", &e.ep).v("epP", &e.ep_p);
                        k.s()
                    })
                    .collect())
            });
            j.raw("m_elements", el);
            let st = stars.as_ref().map_or("null".into(), |v| {
                arr(v
                    .iter()
                    .map(|s| {
                        format!(
                            "{{\"elm\": {}, \"interval\": {}, \"status\": {}}}",
                            draw_members(fx, &s.elm),
                            s.interval,
                            s.status
                        )
                    })
                    .collect())
            });
            j.raw("m_starTbl", st);
        }
    }
    j.v("m_tPos", &u.t_pos);
    format!("[\"{}\", {}]", u.class(), j.s())
}

fn fall_elm(fx: &Effects, f: &piney_effect::fall::FallElement) -> String {
    let mut j = J::new();
    base(&mut j, &f.base);
    j.raw("m_drawElmTbl", format!("[{}]", vec!["null"; 256].join(", "))).i("m_elmNum", f.elm_num);
    j.u("SPIRAL_RADIUS", f.spiral_radius).u("SPIRAL_HEIGHT", f.spiral_height).i("m_hitCheck", f.hit_check);
    j.u("m_velocity", f.velocity).i("m_skillPtr@5b0", f.skill).i("m_iceShockSEOne", f.ice_shock_se_one);
    let el: Vec<String> = f
        .elements
        .iter()
        .map(|s| {
            let mut k = J::new();
            k.raw("elm", draw_elm(fx, &s.elm)).u("height", s.height).u("radius", s.radius).u("rad", s.rad);
            k.i("interval", s.interval).i("status", s.status).i("seOn", s.se_on);
            k.s()
        })
        .collect();
    j.raw("m_elements", format!("[{}]", el.join(", ")));

    format!("[\"ccFallElement\", {}]", j.s())
}

fn anm(fx: &Effects, a: &Option<AnmObj>) -> String {
    a.as_ref().map_or("null".into(), |a| format!("[\"anm\", \"{}\", {}]", fx.assets.name(a.obj), a.play.time))
}

/// A drawn element: `[class, {members}]`.
fn draw_elm(fx: &Effects, d: &DrawElm) -> String {
    format!("[\"{}\", {}]", d.class(), draw_members(fx, d))
}

/// A drawn element's members (as one held in a table is dumped).
fn draw_members(fx: &Effects, d: &DrawElm) -> String {
    let mut j = J::new();
    base(&mut j, &d.base);
    match &d.kind {
        DrawKind::Fire { anm: a, kasa, kasa_rot, sync_sw, count } => {
            j.raw("m_anmFire", anm(fx, a)).raw("m_cmpKasa", clump(fx, *kasa, None)).u("m_kasaRot", *kasa_rot);
            j.i("m_syncSW", *sync_sw).i("m_count@1a0", *count);
        }
        DrawKind::Rock { cmp } => {
            j.raw("m_cmpRock", clump(fx, *cmp, None));
        }
        DrawKind::Dark { cmp } => {
            j.raw("m_cmpDark", clump(fx, *cmp, None));
        }
        DrawKind::Ice { cmp } => {
            j.raw("m_cmpIce", clump(fx, *cmp, None));
        }
        DrawKind::Thunder { bow, connection, anm_index, b_shock } => {
            let c: Vec<String> =
                connection.iter().map(|c| format!("{{\"pos\": {}, \"posP\": {}}}", list(&c[0]), list(&c[1]))).collect();
            j.raw("m_bow", clump(fx, *bow, None)).raw("m_connection", format!("[{}]", c.join(", ")));
            j.i("m_anmIndex", *anm_index).i("m_bShock", *b_shock);
        }
        DrawKind::FireBall { anm: a } => {
            j.raw("m_anmBall", anm(fx, a));
        }
        DrawKind::PlasmaBall { anm: a, org_scale } => {
            j.raw("m_anmBall", anm(fx, a)).u("m_orgScale", *org_scale);
        }
        DrawKind::DarkBat { anm: a } => {
            j.raw("m_anmBat", anm(fx, a));
        }
        DrawKind::Blantch { cmp } => {
            j.raw("m_cmpTree", clump(fx, *cmp, None));
        }
        DrawKind::Explode { anm: a, subst } => {
            j.raw("m_anm", anm(fx, a));
            let s: Vec<String> = subst.iter().map(|(o, c)| format!("[\"{o}\", \"{c}\"]")).collect();
            j.raw("subst", format!("[{}]", s.join(", ")));
        }
        DrawKind::MagicCircle { cmp, clut, scale_spd, scale_limit, fade_spd, fade_limit } => {
            j.raw("m_cmpCircle", clump(fx, *cmp, *clut))
                .u("m_scaleSpd@194", *scale_spd)
                .u("m_scaleLimit", *scale_limit);
            j.u("m_fadeSpd@19c", *fade_spd).u("m_fadeLimit", *fade_limit);
        }
        DrawKind::Bubble { eff, pat } => {
            j.raw("m_bubble", eff_state(fx, eff.as_deref())).u("m_pat", *pat);
        }
        DrawKind::Needle { cmp } => {
            j.raw("m_needle", clump(fx, *cmp, None));
        }
        DrawKind::Star { eff, gp_sm, gp_tail, gp_tail2 } => {
            j.raw("m_star", eff_state(fx, eff.as_deref())).raw("m_gpSm", gp(*gp_sm)).raw("m_gpTail", gp(*gp_tail));
            j.raw("m_gpTail2", gp(*gp_tail2));
        }
        DrawKind::DarkBall { gp_star, gp_conv, eff } => {
            j.raw("m_gpStar", gp(*gp_star))
                .raw("m_gpConv", gp(*gp_conv))
                .raw("m_effDark", eff_state(fx, eff.as_deref()));
        }
        DrawKind::LightBall { gp_ball, cmp_ball } => {
            j.raw("m_gpBall", gp(*gp_ball)).raw("m_cmpBall", clump(fx, *cmp_ball, None));
        }
    }
    j.s()
}

fn effect2(fx: &Effects, i: usize, e: &Effect2) -> String {
    format!(
        "{{\"i\": {i}, \"end\": {}, \"id\": {}, \"status\": {}, \"life\": {}, \"age\": {}, \"cnt\": {}, \"target\": {}, \"eff\": {}}}",
        u8::from(e.end_flag),
        e.id,
        e.status,
        e.life_time,
        e.age,
        e.cnt,
        who(e.target),
        e.bolt.as_ref().map_or("null".into(), |b| bolt(fx, b))
    )
}

/// The volume's `particleForceFieldTbl` (INF main 0x00343850, 4736 bytes).
static FF_TBL: std::sync::OnceLock<std::ops::Range<u32>> = std::sync::OnceLock::new();

fn generator(g: &Generator) -> String {
    // A force field passed to the constructor (outside
    // particleForceFieldTbl), as the harness names it: its address; the
    // row's own: null.
    let tbl = FF_TBL.get().cloned().unwrap_or(0..0);
    let ff: Vec<String> =
        g.ff.iter()
            .map(|f| match f {
                Some(f) if !tbl.contains(&f.va) => f.va.to_string(),
                _ => "null".into(),
            })
            .collect();
    format!(
        concat!(
            "{{\"row\": {}, \"ff\": [{}], \"sync\": [{}, {}, {}], \"syncSW\": {}, \"types\": {}, \"pTexMod\": {}, ",
            "\"rot\": {}, \"pos\": {}, \"pos2\": {}, \"offset\": {}, \"offset2\": {}, \"velocity\": {}}}"
        ),
        if g.row == usize::MAX { format!("[\"va\", {}]", g.param.map_or(0, |p| p.va)) } else { g.row.to_string() },
        ff.join(", "),
        gref(g.sync_rot),
        gref(g.sync_pos),
        gref(g.sync_pos2),
        match g.sync_sw {
            None => "null".into(),
            Some(IntRef::EffectFlags(k)) => format!("[\"eff\", {k}, 104]"),
            Some(IntRef::EffectTemp(k, t)) => format!("[\"eff\", {k}, {}]", 0xa0 + 4 * t),
            Some(IntRef::CharAt(c, off)) => format!("[\"at\", {c}, {off}]"),
        },
        u32::from(g.sync_pos_type) | u32::from(g.sync_pos_type2) << 1,
        g.p_tex_mod,
        list(&g.rot),
        list(&g.pos),
        list(&g.pos2),
        list(&g.offset),
        list(&g.offset2),
        list(&g.velocity)
    )
}

fn skill(fx: &Effects, s: &Spell) -> String {
    let flags = (s.stype as u32 & 15) | ((s.status as u32) & 3) << 4 | u32::from(s.hold) << 8;
    let eff: Vec<String> = s.eff_ptr.iter().map(|e| e.map_or(-1, |k| k as i64).to_string()).collect();
    format!(
        concat!(
            "{{\"key\": {}, \"live\": {}, \"id\": {}, \"flags\": {}, \"level\": {}, \"count\": {}, \"trigger\": {}, ",
            "\"step\": {}, \"effNum\": {}, \"atkCnt\": {}, \"tempCnt\": {}, \"skillType\": {}, \"tType\": {}, ",
            "\"cPos\": {}, \"cDirc\": {}, \"cHeight\": {}, \"tPos\": {}, \"creator\": {}, \"target\": {}, ",
            "\"effPtr\": [{}], \"effElm\": {}}}"
        ),
        s.key,
        s.live,
        s.id,
        flags,
        s.level,
        s.count,
        s.trigger,
        s.step,
        s.eff_num,
        s.atk_cnt,
        s.temp_cnt,
        s.skill_type,
        s.t_type,
        list(&s.c_pos),
        list(&s.c_dirc),
        s.c_height,
        list(&s.t_pos),
        who(s.creator),
        who(s.target),
        eff.join(", "),
        // An element deleted since is a dangling pointer in the game.
        s.elm.map_or("-1".into(), |e| if fx.spells.elements.get(e).is_some() {
            e.to_string()
        } else {
            "\"gone\"".into()
        })
    )
}

/// `ccSkill::Main` (gcmn 0x005731d0) for an attack spell with no
/// animation, around the port's system: the battle crate's
/// `SkillRun::main` does the same. Returns the endFlag.
fn skill_main(fx: &mut Effects, host: &mut Probe, key: u32) -> i8 {
    let Some(s) = fx.spells.get(key).cloned() else { return 1 };
    if s.status != 0 {
        return s.status;
    }
    let mut s = s;
    if let Some(c) = s.creator
        && (!host.check_target(c) || !matches!(host.char_dead(c), 0 | 1))
    {
        s.creator = None;
    }
    if let Some(c) = s.creator
        && host.check_target(c)
    {
        s.c_pos = host.char_pos(c);
        s.c_dirc = host.char_dirc(c);
    }
    let goes_on = s.skill_type & 0x4000 != 0;
    let release = |s: &Spell, host: &mut Probe| {
        if let Some(c) = s.creator
            && (s.stype == 0 || s.stype == 2)
            && let Some(ch) = host.chars.get_mut(&c)
        {
            ch.skill = [0, 0];
        }
    };
    if s.stype == 0 || s.stype == 2 {
        let gone = s.creator.is_none_or(|c| !host.check_target(c) || host.char_dead(c) != 0);
        if gone && s.skill_type & 1 != 0 {
            s.status = -1;
            *fx.spells.get_mut(key).unwrap() = s;
            return -1;
        }
    }
    if let Some(t) = s.target {
        if host.check_target(t) {
            if host.char_dead(t) == 0 || goes_on {
                s.t_pos = host.char_pos(t);
            } else {
                s.status = 1;
                release(&s, host);
                *fx.spells.get_mut(key).unwrap() = s;
                return 1;
            }
        } else if goes_on {
            s.target = None;
        } else {
            s.status = 1;
            release(&s, host);
            *fx.spells.get_mut(key).unwrap() = s;
            return 1;
        }
    }
    let count = s.count;
    *fx.spells.get_mut(key).unwrap() = s;
    if fx.spells.get(key).unwrap().skill_type & 2 != 0 {
        fx.spell_system(host, key);
    } else if fx.spells.get(key).unwrap().id != 0 {
        fx.spells.get_mut(key).unwrap().status = 1;
    }
    // ccSkillHold when holdFlag is set: the battle crate's.
    let s = fx.spells.get_mut(key).unwrap();
    s.count = count.wrapping_add(1);
    s.status
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap();
    let volume = iso.volume().unwrap();
    let t = piney_data::tables::effect::of(volume);
    FF_TBL.set(t.force_fields_va()..t.force_fields_va() + 0x20 * t.force_fields().len() as u32).unwrap();
    let mut fx = Effects::new(&archive, volume).unwrap();
    let fresh = fx.spells.clone();
    let mut host = Probe::new(0, 4352, (1, 0, 0));
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        if w.is_empty() {
            continue;
        }
        let n = |i: usize| hex(w[i]);
        let v3 = |i: usize| [hex(w[i]), hex(w[i + 1]), hex(w[i + 2]), ONE];
        match w[0] {
            "reset" => {
                fx.ctrl = piney_effect::effect::EffectCtrl::new();
                fx.ctrl.town = n(3) == 0;
                fx.particles = Default::default();
                fx.spells = fresh.clone();
                fx.take_events();
                host = Probe::new(u64::from(n(1)), n(2), (n(3) as i32, n(4) as i32, n(5) as i32));
                println!("{{}}");
            }
            "player" => {
                host.player = v3(1);
                println!("{{}}");
            }
            "ground" => {
                host.ground = n(1);
                println!("{{}}");
            }
            "camera" => {
                host.camera.eye = v3(1);
                host.camera.cam_pos = v3(4);
                host.camera.cam_view = v3(7);
                println!("{{}}");
            }
            "char" => {
                let (old, pos_p, affect, effsw) =
                    host.chars.get(&n(1)).map_or(([0, 0], None, None, 0), |c| (c.skill, c.pos_p, c.affect, c.effsw));
                host.chars.insert(
                    n(1),
                    Char {
                        pos: v3(2),
                        height: n(5),
                        width: n(6),
                        dirc: [n(7), n(8), n(9), 0],
                        listed: n(10) != 0,
                        dead: n(11) as i16,
                        ty: n(12) as i32,
                        size: n(13) as i32,
                        skill: old,
                        pos_p,
                        affect,
                        effsw,
                    },
                );
                println!("{{}}");
            }
            "skill" => {
                let opt = |v: u32| if v == u32::MAX { None } else { Some(v) };
                let (key, sid, st, cp, tp) = (n(1), n(2) as i32, n(3) as i8, opt(n(4)), opt(n(5)));
                fx.spell_request(&host, key, sid, st, cp, tp);
                if let Some(c) = cp
                    && let Some(ch) = host.chars.get_mut(&c)
                {
                    ch.skill = [sid as i16, 1];
                }
                println!("{{}}");
            }
            "frame" => {
                fx.step(&mut host);
                let keys: Vec<u32> = fx.spells.runs.iter().filter(|s| s.live).map(|s| s.key).collect();
                for k in keys {
                    if skill_main(&mut fx, &mut host, k) != 0 {
                        fx.spell_remove(k);
                    }
                }
                let events = fx.take_events();
                for e in &events {
                    if let Event::SkillRelease { ch } = e
                        && let Some(c) = host.chars.get_mut(ch)
                    {
                        c.skill = [0, 0];
                    }
                }
                let slots: Vec<String> = fx
                    .ctrl
                    .effects
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.status != 0)
                    .map(|(i, e)| slot(&fx, i, e))
                    .collect();
                let draws: Vec<String> = fx.draws().iter().map(|d| draw(&fx, d)).collect();
                let events: Vec<String> = events.iter().filter_map(event).collect();
                let gens: Vec<String> = std::mem::take(&mut fx.particles.started).iter().map(generator).collect();
                let mut skills: Vec<&Spell> = fx.spells.runs.iter().collect();
                skills.sort_by_key(|s| s.key);
                let skills: Vec<String> = skills.iter().map(|s| skill(&fx, s)).collect();
                let elements: Vec<String> = fx
                    .spells
                    .elements
                    .slots
                    .iter()
                    .enumerate()
                    .filter_map(|(i, e)| {
                        e.as_ref().map(|e| format!("[{i}, {}]", element(&fx, e)[1..].trim_end_matches(']')))
                    })
                    .collect();
                let eff2: Vec<String> = fx
                    .spells
                    .effect2
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.status != 0)
                    .map(|(i, e)| effect2(&fx, i, e))
                    .collect();
                let mut ids: Vec<&CharRef> = host.chars.keys().collect();
                ids.sort();
                let chars: Vec<String> = ids
                    .iter()
                    .map(|c| format!("\"{}\": [{}, {}]", c, host.chars[c].skill[0], host.chars[c].skill[1]))
                    .collect();
                println!(
                    concat!(
                        "{{\"slots\": [{}], \"draws\": [{}], \"events\": [{}], \"gens\": [{}], \"skills\": [{}], ",
                        "\"chars\": {{{}}}, \"elements\": [{}], \"eff2\": [{}], \"rand\": {}, \"genrand\": {}}}"
                    ),
                    slots.join(", "),
                    draws.join(", "),
                    events.join(", "),
                    gens.join(", "),
                    skills.join(", "),
                    chars.join(", "),
                    elements.join(", "),
                    eff2.join(", "),
                    host.rand.0,
                    host.draws
                );
            }
            "posp" => {
                if let Some(c) = host.chars.get_mut(&n(1)) {
                    c.pos_p = Some([n(2), n(3), n(4), n(5)]);
                }
                println!("{{}}");
            }
            "affect" => {
                let by = if n(2) == u32::MAX { None } else { Some(n(2)) };
                if let Some(c) = host.chars.get_mut(&n(1)) {
                    c.affect = by;
                }
                println!("{{}}");
            }
            "effsw" => {
                if let Some(c) = host.chars.get_mut(&n(1)) {
                    c.effsw = n(2) as i32;
                }
                println!("{{}}");
            }
            "endflag" => {
                fx.ctrl.effects[n(1) as usize].end_flag = true;
                println!("{{}}");
            }
            "set" => {
                let e = &mut fx.ctrl.effects[n(1) as usize];
                let v4 = [
                    n(3),
                    w.get(4).map_or(0, |s| hex(s)),
                    w.get(5).map_or(0, |s| hex(s)),
                    w.get(6).map_or(0, |s| hex(s)),
                ];
                match w[2] {
                    "pos" => e.pos = v4,
                    "offset" => e.offset = v4,
                    "rot" => e.rot = v4,
                    "speed" => e.speed = v4,
                    "scale" => e.scale = v4,
                    "posT" => e.pos_t = v4,
                    "temp" => e.temp = v4,
                    "life" => e.life_time = n(3) as i16,
                    "transparency" => e.transparency = n(3),
                    "param" => e.param = n(3) as i32,
                    "flags" => e.flags = n(3) as i32,
                    "target" => e.target = (n(3) != u32::MAX).then_some(n(3)),
                    other => panic!("unknown field {other}"),
                }
                println!("{{}}");
            }
            name => {
                let i = |k: usize| n(k) as i32;
                let ret = match name {
                    "skillstart" => fx.skill_start(&mut host, n(1), i(2), i(3), i(4)),
                    "skillstartp" => {
                        let ty = fx.spells.data.skill_type(i(2));
                        fx.skill_start_param(&mut host, n(1), ty, i(2) == 1, i(3), i(4))
                    }
                    "skillstarteffect" => fx.skill_start_effect(&mut host, n(1), i(2), i(3)),
                    "shock" => fx.shock_wave(&mut host, v3(1), i(4)),
                    "heal" => fx.heal(&mut host, n(1), i(2)),
                    "healskill" => fx.heal_skill(&mut host, n(1), i(2)),
                    "cure" => fx.cure(&mut host, n(1)),
                    "sanity" => fx.sanity(&mut host, n(1)),
                    "resurrect" => fx.resurrect(&mut host, n(1)),
                    "openbox" => {
                        fx.open_box(&mut host, v3(1));
                        Some(1)
                    }
                    "removetrap" => fx.remove_trap(&mut host, v3(1), i(4), i(5)),
                    "virus" => fx.virus_crystal(&mut host, v3(1)),
                    "crushbarrel" | "crushegg" | "crushpot" | "crushcorpse" => {
                        use piney_effect::gimmick::Fragment;
                        let kind = match name {
                            "crushbarrel" => Fragment::Wood,
                            "crushegg" => Fragment::Eggshell,
                            "crushpot" => Fragment::Pot,
                            _ => Fragment::Bone,
                        };
                        fx.crush(&mut host, kind, v3(1), i(4));
                        Some(1)
                    }
                    "opentrap" => fx.open_trap_box(&mut host, v3(1), i(4), i(5)),
                    "statue" => {
                        fx.statue_of_god(&mut host, v3(1), n(4));
                        Some(1)
                    }
                    "abilityup" => fx.ability_up(&mut host, n(1), i(2)),
                    "abilitydown" => fx.ability_down(&mut host, n(1), i(2)),
                    "shield" => fx.resistant_shield(&mut host, n(1), i(2), i(3)),
                    "icerock" => {
                        let (ctrl, mut cx) = fx.split(&mut host);
                        piney_effect::debris::eff_ice_rock(ctrl, &mut cx, v3(1), v3(4), n(7), n(8), i(9))
                    }
                    "spawn" => {
                        let (ctrl, cx) = fx.split(&mut host);
                        ctrl.new_effect(&cx, n(1) as i16)
                    }
                    other => panic!("unknown request {other}"),
                };
                let events: Vec<String> = fx.take_events().iter().filter_map(event).collect();
                let gens: Vec<String> = std::mem::take(&mut fx.particles.started).iter().map(generator).collect();
                println!(
                    "{{\"ret\": {}, \"events\": [{}], \"gens\": [{}], \"rand\": {}, \"genrand\": {}}}",
                    ret.map_or(-1, |r| r as i64),
                    events.join(", "),
                    gens.join(", "),
                    host.rand.0,
                    host.draws
                );
            }
        }
    }
}
