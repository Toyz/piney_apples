//! GCMN.PRG's drawn elements (effect2.cpp): `ccDrawElement` (`Main`
//! 0x004ea670) and the models the spell elements move about - the fire ball
//! `ccFireElement`, the rock `ccRockElement`, the dark ball `ccDarkElement`,
//! the ice `ccIceElement`, the lightning `ccThunderElement` and the rest -
//! and the explosion `ccExplodeElement` that `effExplode3` puts in the
//! element manager. Their constructors and draws are in
//! docs/engine/effects.md ("The element manager and the elements").

use piney_data::volume::Volume;
use piney_world::pose::Play;

use crate::draw::DrawRec;
use crate::ee::{self, F, ONE, V4, VF0};
use crate::eff::Eff;
use crate::effect::EFFECT_LAYER;
use crate::element::{Base, Element};
use crate::files::ObjRef;
use crate::particle::GenRef;
use crate::spell::attr;
use crate::{Cx, pfx, space, thunder, vu};

const HALF_PI: F = 0x3fc9_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
pub(crate) const PI: F = 0x4049_0fdb;
pub(crate) const TWO_PI: F = 0x40c9_0fdb;

/// The drawn elements' tables (gcmn), names read from the executable.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawTables {
    /// `ccRockElement`'s clumps (0x005ed6b0), `ccIceElement`'s
    /// (0x005ed6c0).
    pub rock: [String; 4],
    pub ice: [String; 9],
    /// `ccBlantchElement`'s branches (0x005ed710).
    pub blantch: [String; 4],
    /// `ccExplodeElement`'s objects (0x005ed720), their palettes
    /// (0x005ed730), the palettes' suffixes by element (0x006ac540, 8
    /// apart: fire, water, thunder, wind, dark, soil).
    pub subst: [String; 4],
    pub clut: [String; 4],
    pub suffix: [String; 6],
    /// `effExplode3`'s kinds (0x005ed7c0): the element of each.
    pub kinds: [i32; 6],
}

impl DrawTables {
    /// The volume's (`tables::effect`).
    pub fn read(volume: Volume) -> DrawTables {
        let t = piney_data::tables::effect::of(volume);
        use crate::spell::{first, strings};
        DrawTables {
            rock: strings(t.draw_rock()),
            ice: strings(t.draw_ice()),
            blantch: strings(t.draw_blantch()),
            subst: strings(t.draw_subst()),
            clut: strings(t.draw_clut()),
            suffix: strings(t.draw_suffix()),
            kinds: first(t.draw_kinds()),
        }
    }
}

/// A `ccAnm` an element owns: its object and playback.
#[derive(Clone, Debug, PartialEq)]
pub struct AnmObj {
    pub obj: ObjRef,
    pub play: Play,
}

impl AnmObj {
    /// `new ccAnm`, `SetAnm(GetCCSAdrs(file), name, 0)`.
    pub fn new(cx: &Cx, file: &str, name: &str) -> Option<AnmObj> {
        let obj = cx.assets.find(file, name)?;
        let play = Play::new(&cx.assets.files[obj.file], name)?;
        Some(AnmObj { obj, play })
    }

    /// `_AnimateForward(frameSpd)` when it plays: true at its end.
    pub(crate) fn forward(&mut self, cx: &Cx) -> bool {
        self.play.forward(&cx.assets.files[self.obj.file])
    }

    /// `SetAnm` of the same animation again: from its start.
    pub(crate) fn restart(&mut self) {
        self.play.time = 0;
        self.play.posed = 0;
        self.play.frame_spd = 256;
    }

    pub(crate) fn draw(&self, cx: &mut Cx, matrix: vu::M4, alpha: F) {
        cx.draws.push(DrawRec::Anm { obj: self.obj, play: self.play.clone(), matrix, alpha, layer: EFFECT_LAYER });
    }
}

/// The clump `name` of particle.ccs (`GetChunkAdrsF`, `ccClump::Init`).
pub fn particle_clump(cx: &Cx, name: &str) -> Option<ObjRef> {
    cx.assets.find("particle", name)
}

pub(crate) fn draw_clump(cx: &mut Cx, obj: Option<ObjRef>, matrix: vu::M4, alpha: F) {
    draw_clump_clut(cx, obj, matrix, alpha, None);
}

/// A clump with its palette swapped (from, to).
pub(crate) fn draw_clump_clut(cx: &mut Cx, obj: Option<ObjRef>, matrix: vu::M4, alpha: F, clut: Option<(u32, u32)>) {
    if let Some(obj) = obj {
        cx.draws.push(DrawRec::Clump { obj, matrix, alpha, layer: EFFECT_LAYER, clut });
    }
}

/// `new ccEff`, `Init(GetChunkAdrsF("particle", name), 1)`, then PRIM's
/// fog bit (0x20) off, as the summons' sprites set theirs up.
pub(crate) fn particle_eff(cx: &Cx, name: &str) -> Option<Box<Eff>> {
    let o = cx.assets.find("particle", name)?;
    let (j, c) = cx.assets.eff_chunk(o)?;
    let mut e = Eff::init(o.file, j, c, true, &cx.assets.alpha_blend);
    e.prim &= !0x20;
    Some(Box::new(e))
}

/// `ccEff::Draw(pos, pat)` (main 0x0013bcb0's caller): `pos` into the
/// sprite, then drawn.
pub(crate) fn draw_eff_at(cx: &mut Cx, eff: &mut Eff, pos: V4, pat: u16) {
    eff.pos[0] = pos[0];
    eff.pos[1] = pos[1];
    eff.pos[2] = pos[2];
    cx.draws.push(DrawRec::Eff { eff: Box::new(eff.clone()), pat, layer: EFFECT_LAYER });
}

/// The unit matrix with `s` on its diagonal.
fn scale_matrix(s: V4) -> vu::M4 {
    let mut m = vu::UNIT;
    m[0][0] = s[0];
    m[1][1] = s[1];
    m[2][2] = s[2];
    m
}

/// An angle on by `d`, wrapped to (-pi, pi] once past pi.
fn turn(a: F, d: F) -> F {
    let a = ee::add(a, d);
    if ee::le(a, PI) { a } else { ee::sub(a, TWO_PI) }
}

/// What a drawn element draws.
#[derive(Clone, Debug, PartialEq)]
pub enum DrawKind {
    /// +0x190 `m_anmFire`, +0x194 `m_cmpKasa`, +0x198 `m_kasaRot`, +0x19c
    /// `m_syncSW` (1), +0x1a0 `m_count`.
    Fire {
        anm: Option<AnmObj>,
        kasa: Option<ObjRef>,
        kasa_rot: F,
        sync_sw: i32,
        count: i32,
    },
    /// +0x190 the clump.
    Rock {
        cmp: Option<ObjRef>,
    },
    Dark {
        cmp: Option<ObjRef>,
    },
    Ice {
        cmp: Option<ObjRef>,
    },
    /// +0x190 `m_bow`, +0x1a0 `m_connection[20]` (each the point and it
    /// in the player's frame), +0x420 `m_anmIndex`, +0x424 `m_bShock`.
    Thunder {
        bow: Option<ObjRef>,
        connection: Box<[[V4; 2]; 20]>,
        anm_index: i32,
        b_shock: i32,
    },
    /// +0x190 `m_anm`, and its objects' palettes (object, CLUT).
    Explode {
        anm: Option<AnmObj>,
        subst: Vec<(String, String)>,
    },
    /// +0x190 `m_anmBall` (ANM_x300, looping).
    FireBall {
        anm: Option<AnmObj>,
    },
    /// +0x190 `m_anmBall` (ANM_x502, once), +0x194 `m_orgScale` (never
    /// set).
    PlasmaBall {
        anm: Option<AnmObj>,
        org_scale: F,
    },
    /// +0x190 `m_anmBat` (ANM_x603, looping).
    DarkBat {
        anm: Option<AnmObj>,
    },
    /// +0x190 `m_cmpTree`.
    Blantch {
        cmp: Option<ObjRef>,
    },
    /// `ccMagicCircleElement`: +0x190 `m_cmpCircle` (CMP_x031, its palette
    /// swapped by element: (from, to)), +0x194 `m_scaleSpd`, +0x198
    /// `m_scaleLimit`, +0x19c `m_fadeSpd`, +0x1a0 `m_fadeLimit`.
    MagicCircle {
        cmp: Option<ObjRef>,
        clut: Option<(u32, u32)>,
        scale_spd: F,
        scale_limit: F,
        fade_spd: F,
        fade_limit: F,
    },
    /// `ccBubbleElement`: +0x190 `m_bubble` (EFF_x006), +0x194 `m_pat`.
    Bubble {
        eff: Option<Box<Eff>>,
        pat: u32,
    },
    /// `ccNeedleElement`: +0x190 `m_needle`.
    Needle {
        cmp: Option<ObjRef>,
    },
    /// `ccStarElement`: +0x190 `m_star` (EFF_x018), +0x200 `m_gpSm`
    /// (never set), +0x204 `m_gpTail`, +0x208 `m_gpTail2` (never set).
    Star {
        eff: Option<Box<Eff>>,
        gp_sm: Option<GenRef>,
        gp_tail: Option<GenRef>,
        gp_tail2: Option<GenRef>,
    },
    /// `ccDarkBallElement`: +0x190 `m_gpStar`, +0x194 `m_gpConv`, +0x198
    /// `m_effDark` (EFF_x000).
    DarkBall {
        gp_star: Option<GenRef>,
        gp_conv: Option<GenRef>,
        eff: Option<Box<Eff>>,
    },
    /// `ccLightBallElement`: +0x190 `m_gpBall`, +0x194 `m_cmpBall` (never
    /// set).
    LightBall {
        gp_ball: Option<GenRef>,
        cmp_ball: Option<ObjRef>,
    },
}

/// A `ccDrawElement` (0x190 bytes and its class's own).
#[derive(Clone, Debug, PartialEq)]
pub struct DrawElm {
    pub base: Base,
    pub kind: DrawKind,
}

impl DrawElm {
    /// `new ccFireElement` (gcmn 0x004f96f0).
    pub fn fire(cx: &Cx) -> DrawElm {
        let kind = DrawKind::Fire {
            anm: AnmObj::new(cx, "particle", "ANM_x300"),
            kasa: particle_clump(cx, "CMP_x100"),
            kasa_rot: 0,
            sync_sw: 1,
            count: 0,
        };
        DrawElm { base: Base::default(), kind }
    }

    /// `new ccRockElement` (0x004fa2c0): one ccRand() for the clump.
    pub fn rock(cx: &mut Cx) -> DrawElm {
        let r = cx.host.genrand() as i32 & 3;
        let name = cx.spells.data.draw.rock[r as usize].clone();
        DrawElm { base: Base::default(), kind: DrawKind::Rock { cmp: particle_clump(cx, &name) } }
    }

    /// `new ccDarkElement` (0x004fa5b0): its heading three ccRandF(pi).
    pub fn dark(cx: &mut Cx) -> DrawElm {
        let mut base = Base::default();
        for k in 0..3 {
            base.anim.dirc[k] = thunder::rand_f(cx, PI);
        }
        DrawElm { base, kind: DrawKind::Dark { cmp: particle_clump(cx, "CMP_x604") } }
    }

    /// `new ccIceElement(n)` (0x004faf90).
    pub fn ice(cx: &mut Cx, n: i32) -> DrawElm {
        let n = if (0..9).contains(&n) { n } else { (cx.host.genrand() as i32).wrapping_abs() & 7 };
        let name = cx.spells.data.draw.ice[n as usize].clone();
        DrawElm { base: Base::default(), kind: DrawKind::Ice { cmp: particle_clump(cx, &name) } }
    }

    /// `new ccThunderElement` (0x004fa940).
    pub fn thunder(cx: &Cx) -> DrawElm {
        let kind = DrawKind::Thunder {
            bow: particle_clump(cx, "CMP_x012"),
            connection: Box::new([[[0; 4]; 2]; 20]),
            anm_index: 0,
            b_shock: 0,
        };
        DrawElm { base: Base::default(), kind }
    }

    /// `new ccExplodeElement(attr)` (0x004fd8d0): scale 2, ANM_x305 with
    /// its objects' palettes by element.
    pub fn explode(cx: &Cx, atr: i32) -> DrawElm {
        let t = &cx.spells.data.draw;
        let suffix = match atr {
            attr::FIRE => Some(&t.suffix[0]),
            attr::WATER => Some(&t.suffix[1]),
            attr::THUNDER => Some(&t.suffix[2]),
            attr::WIND => Some(&t.suffix[3]),
            attr::DARK => Some(&t.suffix[4]),
            attr::SOIL => Some(&t.suffix[5]),
            // The suffix is a register the callers never leave unset.
            _ => None,
        };
        let subst =
            (0..4).map(|k| (t.subst[k].clone(), format!("{}{}", t.clut[k], suffix.map_or("", |s| s)))).collect();
        let mut base = Base { attr: atr, ..Base::default() };
        base.anim.scale = [0x4000_0000, 0x4000_0000, 0x4000_0000, ONE];
        DrawElm { base, kind: DrawKind::Explode { anm: AnmObj::new(cx, "particle", "ANM_x305"), subst } }
    }

    /// `new ccFireBallElement` (gcmn 0x004fbfd0).
    pub fn fire_ball(cx: &Cx) -> DrawElm {
        DrawElm { base: Base::default(), kind: DrawKind::FireBall { anm: AnmObj::new(cx, "particle", "ANM_x300") } }
    }

    /// `new ccPlasmaBallElement` (gcmn 0x004fc2d0).
    pub fn plasma_ball(cx: &Cx) -> DrawElm {
        let kind = DrawKind::PlasmaBall { anm: AnmObj::new(cx, "particle", "ANM_x502"), org_scale: 0 };
        DrawElm { base: Base::default(), kind }
    }

    /// `new ccDarkBatElement` (gcmn 0x004fd5e0).
    pub fn dark_bat(cx: &Cx) -> DrawElm {
        DrawElm { base: Base::default(), kind: DrawKind::DarkBat { anm: AnmObj::new(cx, "particle", "ANM_x603") } }
    }

    /// `new ccBlantchElement` (gcmn 0x004fd2e0): one of four branches
    /// (abs(ccRand() & 3)).
    pub fn blantch(cx: &mut Cx) -> DrawElm {
        let r = (cx.host.genrand() as i32 & 3).wrapping_abs();
        let name = cx.spells.data.draw.blantch[r as usize].clone();
        DrawElm { base: Base::default(), kind: DrawKind::Blantch { cmp: particle_clump(cx, &name) } }
    }

    /// `new ccMagicCircleElement(attr)` (gcmn 0x004f8400): CMP_x031, its
    /// palette CLT_x031 made the element's (fire c1, thunder c2, wind c3,
    /// dark c4, soil c6, water none, any other c5).
    pub fn magic_circle(cx: &Cx, atr: i32) -> DrawElm {
        let atr = atr & 0xfc;
        let t = &cx.spells.data.summoned.circle_clut;
        let name = match atr {
            attr::FIRE => Some(&t[0]),
            attr::THUNDER => Some(&t[1]),
            attr::WIND => Some(&t[2]),
            attr::DARK => Some(&t[3]),
            attr::SOIL => Some(&t[5]),
            attr::WATER => None,
            _ => Some(&t[4]),
        };
        let clut = name.and_then(|n| {
            let to = cx.assets.find("particle", n)?;
            let from = cx.assets.find("particle", &t[6])?;
            Some((from.object, to.object))
        });
        let base = Base { attr: atr, ..Base::default() };
        let kind = DrawKind::MagicCircle {
            cmp: particle_clump(cx, "CMP_x031"),
            clut,
            scale_spd: 0,
            scale_limit: ONE,
            fade_spd: 0,
            fade_limit: ONE,
        };
        DrawElm { base, kind }
    }

    /// `new ccBubbleElement` (gcmn 0x004fe960): fading in over 8 frames,
    /// growing to 10 over 8.
    pub fn bubble(cx: &Cx) -> DrawElm {
        let mut base = Base::default();
        let a = &mut base.anim;
        a.start_fade = 0;
        a.end_fade = ONE;
        a.fade_spd = 0x3e00_0000;
        a.transparency = 0;
        a.fade_flag = 1;
        let es = [0x4120_0000, 0x4120_0000, 0x4120_0000, ONE];
        a.start_scale = VF0;
        a.end_scale = es;
        for k in 0..3 {
            a.scale_spd[k] = ee::div(ee::sub(es[k], VF0[k]), 0x4100_0000);
        }
        a.scale = VF0;
        a.scale_flag = 1;
        a.scale_accel = VF0;
        DrawElm { base, kind: DrawKind::Bubble { eff: particle_eff(cx, "EFF_x006"), pat: 0 } }
    }

    /// `new ccNeedleElement` (gcmn 0x004f9be0): one of four (abs(ccRand() &
    /// 3)).
    pub fn needle(cx: &mut Cx) -> DrawElm {
        let r = (cx.host.genrand() as i32 & 3).wrapping_abs();
        let name = cx.spells.data.summoned.needles[r as usize].clone();
        DrawElm { base: Base::default(), kind: DrawKind::Needle { cmp: particle_clump(cx, &name) } }
    }

    /// `new ccStarElement` (gcmn 0x004f9ee0): EFF_x018 and its tail's
    /// generator at it, paused.
    pub fn star(cx: &mut Cx) -> DrawElm {
        let base = Base::default();
        let pos = base.anim.pos;
        let (param, ff) = (cx.spells.data.summoned.star_gen, cx.spells.data.summoned.star_ff);
        let g = pfx::start_param(cx, param, [Some(ff), None, None, None], |g| {
            g.pos = pos;
            g.sync_pos_type = false;
        });
        if let Some(g) = cx.particles.get_mut(g) {
            g.pause = true;
        }
        let kind = DrawKind::Star { eff: particle_eff(cx, "EFF_x018"), gp_sm: None, gp_tail: Some(g), gp_tail2: None };
        DrawElm { base, kind }
    }

    /// `new ccDarkBallElement` (gcmn 0x004f9160): EFF_x000 (blend 2,
    /// CLT_x000c7) and two generators following it (the gathering, then the
    /// stars), paused.
    pub fn dark_ball(cx: &mut Cx) -> DrawElm {
        let base = Base::default();
        let pos = base.anim.pos;
        let mut eff = particle_eff(cx, "EFF_x000");
        if let Some(e) = eff.as_mut() {
            e.set_blend_type(2, &cx.assets.alpha_blend);
            e.clut = cx.assets.find("particle", "CLT_x000c7");
        }
        let t = cx.spells.data.summoned.clone();
        let follow = |g: &mut crate::particle::Generator| {
            g.pos = pos;
            g.dist_sw = false;
            g.g_sync = true;
            g.sync_pos_type = false;
        };
        let conv = pfx::start_param(cx, t.conv_gen, [Some(t.conv_ff), Some(t.conv_ff), None, None], follow);
        if let Some(g) = cx.particles.get_mut(conv) {
            g.pause = true;
        }
        let star = pfx::start_param(cx, t.dark_star_gen, [None; 4], |g| {
            g.pos = pos;
            g.dist_sw = false;
            g.p_tex_mod = t.dark_star_tex;
            g.g_sync = true;
            g.sync_pos_type = false;
        });
        if let Some(g) = cx.particles.get_mut(star) {
            g.pause = true;
        }
        DrawElm { base, kind: DrawKind::DarkBall { gp_star: Some(star), gp_conv: Some(conv), eff } }
    }

    /// A `BALL_T`'s `ccLightBallElement`, made (0x004f3ab0) with no
    /// generator.
    pub fn light_ball() -> DrawElm {
        DrawElm { base: Base::default(), kind: DrawKind::LightBall { gp_ball: None, cmp_ball: None } }
    }

    /// `ccLightBallElement::Init(pos, n)` (gcmn 0x004f8a20): at `pos`, its
    /// generator started there (texture 109 + n, 31 from n 8).
    pub fn light_ball_init(&mut self, cx: &mut Cx, pos: V4, n: i16) {
        self.base.anim.pos = pos;
        self.base.anim.sp = pos;
        let tex = if n >= 8 { 31 } else { n.max(0) + 109 };
        let (param, ff) = (cx.spells.data.summoned.ball_gen, cx.spells.data.summoned.ball_ff);
        let g = pfx::start_param(cx, param, [Some(ff), None, None, None], |g| {
            g.sync_pos_type = false;
            g.dist_sw = false;
            g.p_tex_mod = tex;
        });
        if let DrawKind::LightBall { gp_ball, .. } = &mut self.kind {
            *gp_ball = Some(g);
        }
    }

    /// The class `ccEffectElementManager` and the checks know it by.
    pub fn class(&self) -> &'static str {
        match self.kind {
            DrawKind::Fire { .. } => "ccFireElement",
            DrawKind::Rock { .. } => "ccRockElement",
            DrawKind::Dark { .. } => "ccDarkElement",
            DrawKind::Ice { .. } => "ccIceElement",
            DrawKind::Thunder { .. } => "ccThunderElement",
            DrawKind::Explode { .. } => "ccExplodeElement",
            DrawKind::FireBall { .. } => "ccFireBallElement",
            DrawKind::PlasmaBall { .. } => "ccPlasmaBallElement",
            DrawKind::DarkBat { .. } => "ccDarkBatElement",
            DrawKind::Blantch { .. } => "ccBlantchElement",
            DrawKind::MagicCircle { .. } => "ccMagicCircleElement",
            DrawKind::Bubble { .. } => "ccBubbleElement",
            DrawKind::Needle { .. } => "ccNeedleElement",
            DrawKind::Star { .. } => "ccStarElement",
            DrawKind::DarkBall { .. } => "ccDarkBallElement",
            DrawKind::LightBall { .. } => "ccLightBallElement",
        }
    }

    /// `ccDrawElement::Main` (gcmn 0x004ea670), then the class's `Draw`.
    pub fn main(&mut self, cx: &mut Cx) {
        let life = self.base.life;
        if life >= 0 {
            self.base.life = life - 1;
            if life == 0 {
                self.base.del_flag = 1;
            }
        }
        self.draw(cx);
    }

    fn draw(&mut self, cx: &mut Cx) {
        let a = self.base.anim.clone();
        match &mut self.kind {
            DrawKind::Fire { anm, kasa, kasa_rot, .. } => {
                let mut m = scale_matrix(a.scale);
                m = vu::rot_y(&m, *kasa_rot);
                m = vu::rot_z(&m, HALF_PI);
                m = vu::rot_x(&m, a.dirc[0]);
                m = vu::rot_y(&m, a.dirc[1]);
                m = vu::rot_z(&m, a.dirc[2]);
                m = vu::rot_z(&m, NEG_HALF_PI);
                m = vu::trans(&m, a.pos);
                if let Some(anm) = anm {
                    let ended = anm.forward(cx);
                    anm.draw(cx, m, a.transparency);
                    if ended {
                        anm.restart();
                    }
                }
                draw_clump(cx, *kasa, m, a.transparency);
                *kasa_rot = turn(*kasa_rot, 0x3dd6_7750);
            }
            DrawKind::Rock { cmp } => {
                draw_clump(cx, *cmp, vu::pos_rot_zyx_scale(a.pos, a.dirc, a.scale), a.transparency);
            }
            DrawKind::Dark { cmp } => {
                draw_clump(cx, *cmp, vu::pos_rot_zyx_scale(a.pos, a.dirc, a.scale), a.transparency);
                for k in 1..3 {
                    self.base.anim.dirc[k] = turn(self.base.anim.dirc[k], 0x3e56_7750);
                }
            }
            DrawKind::Ice { cmp } => {
                let mut m = scale_matrix(a.scale);
                m = vu::rot_x(&m, a.dirc[0]);
                m = vu::rot_y(&m, a.dirc[1]);
                m = vu::rot_z(&m, a.dirc[2]);
                m = vu::rot_z(&m, NEG_HALF_PI);
                // m_pos into the matrix's translation row, w and all.
                m[3] = a.pos;
                draw_clump(cx, *cmp, m, a.transparency);
            }
            DrawKind::Thunder { bow, connection, anm_index, b_shock } => {
                let (bow, idx) = (*bow, *anm_index);
                thunder_draw(cx, &a, bow, connection, idx);
                let mut idx = idx;
                if idx != 19 {
                    idx += 1;
                    if idx == 19 {
                        *b_shock = 1;
                    }
                }
                if *b_shock == 1 {
                    self.base.end_flag = 1;
                    self.base.status[0] = 1;
                    *b_shock = 2;
                }
                if *b_shock == 2 {
                    let mut t = ee::sub(self.base.anim.transparency, 0x3d88_8889);
                    if ee::lt(t, 0) {
                        t = 0;
                        self.base.del_flag = 1;
                    }
                    self.base.anim.transparency = t;
                }
                *anm_index = idx;
            }
            DrawKind::FireBall { anm } | DrawKind::DarkBat { anm } => {
                if let Some(anm) = anm {
                    let ended = anm.forward(cx);
                    anm.draw(cx, vu::pos_rot_zyx_scale(a.pos, a.dirc, a.scale), a.transparency);
                    if ended {
                        anm.restart();
                    }
                }
            }
            DrawKind::PlasmaBall { anm, .. } => {
                if let Some(anm) = anm {
                    anm.forward(cx);
                    anm.draw(cx, vu::pos_rot_zyx_scale(a.pos, a.dirc, a.scale), a.transparency);
                }
            }
            DrawKind::Blantch { cmp } | DrawKind::Needle { cmp } => {
                draw_clump(cx, *cmp, vu::pos_rot_zyx_scale(a.pos, a.dirc, a.scale), a.transparency);
            }
            DrawKind::MagicCircle { cmp, clut, scale_spd, scale_limit, fade_spd, fade_limit } => {
                draw_clump_clut(cx, *cmp, vu::pos_rot_zyx_scale(a.pos, a.dirc, a.scale), a.transparency, *clut);
                let b = &mut self.base;
                if !ee::eq(0, *scale_spd) {
                    let mut v = ee::add(b.anim.scale[0], *scale_spd);
                    let there = if ee::le(*scale_spd, 0) { ee::le(v, *scale_limit) } else { !ee::lt(v, *scale_limit) };
                    if there {
                        v = *scale_limit;
                        *scale_spd = 0;
                        b.status[1] += 1;
                    }
                    for k in 0..3 {
                        b.anim.scale[k] = v;
                    }
                }
                if !ee::eq(0, *fade_spd) {
                    let mut t = ee::add(b.anim.transparency, *fade_spd);
                    let there = if ee::le(*fade_spd, 0) { ee::le(t, *fade_limit) } else { !ee::lt(t, *fade_limit) };
                    if there {
                        t = *fade_limit;
                        *fade_spd = 0;
                        b.status[0] += 1;
                    }
                    b.anim.transparency = t;
                }
            }
            DrawKind::Bubble { eff, pat } => {
                if let Some(e) = eff.as_mut() {
                    e.transparency = a.transparency;
                    e.scale_x = a.scale[0];
                    e.scale_y = a.scale[1];
                    draw_eff_at(cx, e, a.pos, *pat as u16);
                    *pat += 1;
                    if *pat >= u32::from(e.pat_num) {
                        *pat = 0;
                    }
                }
                let an = &mut self.base.anim;
                let faded = an.animate_fade();
                let scaled = an.animate_scale();
                match self.base.proccess {
                    0 if faded && scaled => {
                        let an = &mut self.base.anim;
                        an.start_fade = ONE;
                        an.end_fade = 0;
                        an.fade_spd = 0xbe00_0000;
                        an.transparency = ONE;
                        an.fade_flag = 1;
                        let s = an.scale[0];
                        let ss = [s, s, s, ONE];
                        an.start_scale = ss;
                        an.end_scale = VF0;
                        for k in 0..3 {
                            an.scale_spd[k] = ee::div(ee::sub(VF0[k], ss[k]), 0x4100_0000);
                        }
                        an.scale = ss;
                        an.scale_flag = 1;
                        an.scale_accel = VF0;
                        self.base.proccess += 1;
                    }
                    1 if faded && scaled => self.base.del_flag = 1,
                    _ => {}
                }
            }
            DrawKind::Star { eff, .. } => {
                if let Some(e) = eff.as_mut() {
                    e.scale_x = a.scale[0];
                    e.scale_y = a.scale[0];
                    e.transparency = a.transparency;
                    e.rotate = a.dirc[1];
                    draw_eff_at(cx, e, a.pos, 0);
                }
                self.base.anim.dirc[1] = turn(self.base.anim.dirc[1], 0x3e56_7750);
            }
            DrawKind::DarkBall { gp_star, eff, .. } => {
                if let Some(e) = eff.as_mut() {
                    e.scale_y = a.scale[0];
                    e.scale_x = a.scale[0];
                    draw_eff_at(cx, e, a.pos, 0);
                }
                let c = self.base.proccess;
                self.base.proccess = c + 1;
                if c == 0
                    && let Some(g) = gp_star.and_then(|g| cx.particles.get_mut(g))
                {
                    g.pause = false;
                }
            }
            // Never drawn: its owner (the dark summons' BALL_T) never runs
            // it.
            DrawKind::LightBall { .. } => {}
            DrawKind::Explode { anm, .. } => {
                let mut ended = false;
                if let Some(anm) = anm {
                    ended = anm.forward(cx);
                    anm.draw(cx, vu::pos_rot_zyx_scale(a.pos, a.dirc, a.scale), a.transparency);
                }
                self.base.anim.animate_fade();
                self.base.anim.animate_scale();
                let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
                let an = &mut self.base.anim;
                an.pos = space::p2w(ee::vadd(space::w2p(an.pos, player, bounds), an.speed), player, bounds);
                if ended {
                    self.base.del_flag = 1;
                }
            }
        }
    }
}

/// `ccThunderElement::Draw`'s chain: the points between redrawn at random,
/// then `idx` links drawn.
fn thunder_draw(cx: &mut Cx, a: &crate::element::Animate, bow: Option<ObjRef>, c: &mut [[V4; 2]; 20], idx: i32) {
    let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
    c[0][0] = a.sp;
    c[0][1] = space::w2p(c[0][0], player, bounds);
    c[19][0] = a.ep;
    c[19][1] = space::w2p(c[19][0], player, bounds);
    let sp = space::w2p(a.sp, player, bounds);
    for i in 0..18 {
        let x = thunder::rand_f(cx, 0x42c8_0000);
        let z = ee::sub(ee::mul(0x42c8_0000, ee::from_int(-i)), 0x42c8_0000);
        let r = thunder::rand_f(cx, PI);
        let v = ee::apply(&vu::rot_z(&vu::UNIT, r), [x, 0, z, ONE]);
        let k = (i + 1) as usize;
        c[k][1] = ee::vadd(v, sp);
        c[k][0] = space::p2w(c[k][1], player, bounds);
    }
    for i in 0..idx.max(0) as usize {
        let (p, q) = (c[i][1], c[i + 1][1]);
        let dirc = get_dirc(p, q);
        let d = get_dist(cx.assets.volume, p, q);
        let d3 = get_dist3d(cx.assets.volume, p, q);
        let tilt = ee::atan2f(ee::sub(c[i][0][2], c[i + 1][0][2]), d);
        let mut m = vu::UNIT;
        m[0][0] = 0x4040_0000;
        m[1][1] = ee::mul(0x3a91_a2b4, d3);
        m[2][2] = 0x4040_0000;
        m = vu::rot_z(&m, NEG_HALF_PI);
        m = vu::rot_y(&m, tilt);
        m = vu::rot_z(&m, dirc);
        m = vu::rot_z(&m, NEG_HALF_PI);
        m = vu::trans(&m, c[i][0]);
        draw_clump(cx, bow, m, a.transparency);
    }
}

/// `ccGetDist(a, b)` (main 0x001d9dd0): the distance on the ground, by
/// the volume's `sqrtf` (OUT main 0x001e6e70 has `sqrt.s`).
pub fn get_dist(volume: Volume, a: V4, b: V4) -> F {
    let mut d = ee::vsub(b, a);
    d[2] = 0;
    d[3] = ONE;
    ee::sqrtf_on(volume, ee::dot(d, d))
}

/// `ccGetDist3D(a, b)` (main 0x001d9e40), by the volume's `sqrtf`.
pub fn get_dist3d(volume: Volume, a: V4, b: V4) -> F {
    let d = ee::vsub(b, a);
    ee::sqrtf_on(volume, ee::dot(d, d))
}

/// `ccGetDirc(a, b)` (main 0x001d9ce0): the heading from `a` to `b`
/// (atan2 + pi/2, wrapped to [-pi, pi]).
pub fn get_dirc(a: V4, b: V4) -> F {
    let dx = ee::sub(b[0], a[0]);
    let dy = ee::sub(b[1], a[1]);
    let mut r = ee::add(HALF_PI, ee::atan2f(dy, dx));
    if !ee::le(r, PI) {
        r = ee::sub(r, TWO_PI);
    }
    if ee::lt(r, 0xc049_0fdb) {
        r = ee::add(r, TWO_PI);
    }
    r
}

/// `effExplode3(pos, spd, scale, n)` (gcmn 0x005000b0): an explosion of
/// kind n (0-5, else a random one: abs(ccRand() % 6)) at `pos` moving by
/// `spd`, in the first empty slot of the manager (none: deleted, None).
pub fn eff_explode3(cx: &mut Cx, pos: V4, spd: V4, scale: F, n: i32) -> Option<usize> {
    let n = if (0..6).contains(&n) { n } else { (cx.host.genrand() as i32 % 6).wrapping_abs() };
    let atr = cx.spells.data.draw.kinds[n as usize] & 0xfc;
    let mut e = DrawElm::explode(cx, atr);
    e.base.anim.pos = pos;
    e.base.anim.speed = spd;
    e.base.anim.scale[0] = scale;
    e.base.anim.scale[1] = scale;
    e.base.anim.scale[2] = scale;
    cx.spells.elements.add(Element::Draw(Box::new(e)))
}
