//! The thunder of the thunder tornado: `effSkillTornadeThunderPos` (main
//! 0x001d5b20), its controller effect -17 (second chain 0x001cae60, bolts at
//! the counts of `tornadeThunderTbl`), `ccEffect2` (main effect.cpp, the 100
//! small slots `ccEffectCtrl::Main` runs after the 500) and
//! `ccThunderBoltElement` (gcmn 0x00500dc0, Draw 0x00501470), the bolt a
//! `ccEffect2` draws: strokes of random segments of `CMP_x012`, redrawn at
//! random each frame. The rules are in docs/engine/effects.md
//! ("TornadoSystem").

use piney_data::volume::Volume;

use crate::ee::{self, F, ONE, V4, VF0};
use crate::effect::{EFFECT_LAYER, EffectCtrl};
use crate::element::Base;
use crate::files::ObjRef;
use crate::{Cx, Event, space, vu};

/// The thunder's controller.
pub const THUNDER_POS: i16 = -17;
/// `effWork2`: the `ccEffect2` slots.
pub const SLOTS2: usize = 100;

/// `ccEffThunderData` (0x40 bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ThunderData {
    pub default_angle: F,
    pub rand_angle: F,
    pub default_scale: F,
    pub rand_scale: F,
    pub mode: i32,
    pub eff_sw: u8,
    pub rnd_point: V4,
    pub off_set: V4,
}

impl ThunderData {
    pub fn of(d: &piney_data::tables::types::EffThunderData) -> ThunderData {
        ThunderData {
            default_angle: d.default_angle.to_bits(),
            rand_angle: d.rand_angle.to_bits(),
            default_scale: d.default_scale.to_bits(),
            rand_scale: d.rand_scale.to_bits(),
            mode: d.mode,
            eff_sw: d.eff_sw,
            rnd_point: d.rnd_point.map(f32::to_bits),
            off_set: d.off_set.map(f32::to_bits),
        }
    }
}

/// The thunder's tables.
#[derive(Clone, Debug, PartialEq)]
pub struct ThunderTables {
    pub skill_thunder_eff2: ThunderData,
    /// `tornadeThunderTbl`: each entry's offset into `lists`.
    pub tbl: [u32; 16],
    /// The bytes the entries point into (and what follows them).
    pub lists: Vec<i8>,
}

impl ThunderTables {
    /// The volume's (`tables::effect`).
    pub fn read(volume: Volume) -> ThunderTables {
        let t = piney_data::tables::effect::of(volume);
        ThunderTables {
            skill_thunder_eff2: ThunderData::of(&t.thunder_eff2()),
            tbl: std::array::from_fn(|i| t.thunder_offsets()[i] as u32),
            lists: t.thunder_times().iter().flat_map(|p| [p.time, p.param]).collect(),
        }
    }

    /// `tornadeThunderTbl[param][k]`.
    fn at(&self, param: i32, k: i32) -> i32 {
        let base = self.tbl[(param & 15) as usize] as i64;
        i32::from(self.lists.get((base + i64::from(k)) as usize).copied().unwrap_or(0))
    }
}

/// `THUNDER_T` (0x20 bytes): a stroke's point and its segments.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ThunderT {
    pub spot: V4,
    pub break_point: i32,
}

/// `ccThunderBoltElement` (gcmn effect2.cpp, 0x290 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct ThunderBolt {
    pub base: Base,
    /// +0x190 `m_thunderTbl`: `new THUNDER_T[Num + 1]`.
    pub thunder_tbl: Vec<ThunderT>,
    /// +0x1a0 `CenterPos`, +0x1b0 `TopPos` (never set).
    pub center_pos: V4,
    pub top_pos: V4,
    /// +0x1c8 `TopRange` (never set), +0x1cc `BottomRange`.
    pub top_range: F,
    pub bottom_range: F,
    /// +0x1d0 `Thunder_Mode`, +0x1d4 `Num`, +0x1d8 `flg`, +0x1d9 `flg2`,
    /// +0x1dc `hoge` (frames drawn).
    pub thunder_mode: i32,
    pub num: i32,
    pub flg: u8,
    pub flg2: u8,
    pub hoge: i32,
    /// +0x1e0 `DefaultData` (set only with no data).
    pub default_data: ThunderData,
    /// +0x220 `DefaultAngle`, +0x224 `RandAngle`, +0x228 `DefaultScale`,
    /// +0x22c `RandScale`, +0x230 `RndPoint`, +0x240 `SpMat` (never set).
    pub default_angle: F,
    pub rand_angle: F,
    pub default_scale: i32,
    pub rand_scale: i32,
    pub rnd_point: V4,
    pub sp_mat: vu::M4,
    /// +0x284 `EffCmp`: CMP_x012, duplicated, its CLUT swapped.
    pub eff_cmp: Option<ObjRef>,
    pub clut: Option<(u32, u32)>,
    /// +0x288 `Time`, +0x28c `EndFlg`.
    pub time: i32,
    pub end_flg: u8,
}

/// libgcc's `fptosi`: a float to an int, towards zero.
pub fn fptosi(v: F) -> i32 {
    ee::to_int(v)
}

/// `ccRandF(x)` (main 0x001d9a90): `x * (int)genrand() / 2^31` in double
/// precision.
pub fn rand_f(cx: &mut Cx, x: F) -> F {
    let r = f64::from(cx.host.genrand() as i32);
    ((f64::from(f32::from_bits(x)) * (r / 2_147_483_648.0)) as f32).to_bits()
}

/// `(float)((double)a + fabs((double)b))`.
pub(crate) fn add_abs(a: f64, b: F) -> F {
    ((a + f64::from(f32::from_bits(b)).abs()) as f32).to_bits()
}

impl ThunderBolt {
    /// `new ccThunderBoltElement(vec, time, ran, num, dat)` (gcmn
    /// 0x00500dc0).
    pub fn new(cx: &mut Cx, vec: V4, time: i32, ran: F, num: i32, dat: Option<ThunderData>) -> ThunderBolt {
        let mut t = ThunderBolt {
            base: Base::default(),
            thunder_tbl: vec![ThunderT::default(); (num.max(0) + 1) as usize],
            center_pos: vec,
            top_pos: [0; 4],
            top_range: 0,
            bottom_range: ran,
            thunder_mode: 0,
            num,
            flg: 0,
            flg2: 0,
            hoge: 0,
            default_data: ThunderData::default(),
            default_angle: 0,
            rand_angle: 0,
            default_scale: 0,
            rand_scale: 0,
            rnd_point: [0; 4],
            sp_mat: [[0; 4]; 4],
            eff_cmp: None,
            clut: None,
            time,
            end_flg: 0,
        };
        match dat {
            Some(d) => {
                t.default_scale = fptosi(d.default_scale);
                t.rand_scale = fptosi(d.rand_scale);
                t.default_angle = d.default_angle;
                t.rand_angle = d.rand_angle;
                t.thunder_mode = d.mode;
                t.flg = d.eff_sw;
                t.flg2 = d.eff_sw;
                t.rnd_point = d.rnd_point;
            }
            None => {
                t.flg = 1;
                t.flg2 = 1;
                t.default_scale = 1;
                t.rand_scale = 10;
                t.default_angle = 0;
                t.rand_angle = ee::PI;
                t.thunder_mode = 1;
                t.rnd_point = [0x42c8_0000, 0x42c8_0000, 0x42c8_0000, ONE];
                t.default_data = ThunderData {
                    default_angle: t.default_angle,
                    rand_angle: t.rand_angle,
                    default_scale: ee::from_int(t.default_scale),
                    rand_scale: ee::from_int(t.rand_scale),
                    mode: t.thunder_mode,
                    eff_sw: t.flg,
                    rnd_point: t.rnd_point,
                    off_set: VF0,
                };
            }
        }
        t.eff_cmp = cx.assets.find("particle", "CMP_x012");
        let new = if t.thunder_mode == 1 { "CLT_x012c1" } else { "CLT_x012" };
        t.clut = cx
            .assets
            .find("particle", "CLT_x012")
            .zip(cx.assets.find("particle", new))
            .map(|(f, t)| (f.object, t.object));
        t.set_break_point(cx);
        t.unit_point(cx);
        t.hoge = 0;
        t
    }

    /// `SetBreakPoint()` (gcmn 0x005013f0).
    fn set_break_point(&mut self, cx: &mut Cx) {
        for i in 0..self.num.max(0) as usize {
            self.thunder_tbl[i].break_point = (cx.host.genrand() as i32 & 7).abs() + 5;
        }
    }

    /// `UnitPoint()` (gcmn 0x005012f0).
    fn unit_point(&mut self, cx: &mut Cx) {
        let r = ee::div(self.bottom_range, 0x4120_0000);
        for i in 0..self.num.max(0) as usize {
            let mut spot = VF0;
            let m = vu::rot_z(&vu::UNIT, rand_f(cx, ee::PI));
            spot[1] = ee::sub(self.bottom_range, rand_f(cx, r));
            let spot = ee::apply(&m, spot);
            self.thunder_tbl[i].spot = ee::vadd(spot, self.center_pos);
        }
    }

    /// `Draw()` (gcmn 0x00501470), onto `layer`.
    pub fn draw(&mut self, cx: &mut Cx, layer: i16) {
        if self.flg != 0 {
            // MissileSmokeGenerator's smoke: only with EFF_SW, which the
            // tornado's data leaves clear.
            self.center_pos = ee::vadd(self.center_pos, self.default_data.off_set);
            self.flg = 0;
        }
        const MODEL_RANGE: F = 0x4348_0000;
        let range = ee::mul(0x3a91_a2b4, MODEL_RANGE);
        for i in 0..self.num.max(0) as usize {
            let scale = add_abs(f64::from(self.default_scale), rand_f(cx, ee::from_int(self.rand_scale)));
            let mut spot_new = self.thunder_tbl[i].spot;
            for _ in 0..self.thunder_tbl[i].break_point {
                let mut mat = vu::UNIT;
                let test = vu::UNIT;
                let new_pos = [0, MODEL_RANGE, 0, ONE];
                mat[0][0] = scale;
                mat[1][1] = range;
                mat[2][2] = scale;
                let rot = rand_f(cx, ee::PI);
                let x_rot = add_abs(f64::from(f32::from_bits(self.default_angle)), rand_f(cx, self.rand_angle));
                let mat = vu::rot_z(&vu::rot_x(&mat, x_rot), rot);
                let test = vu::rot_z(&vu::rot_x(&test, x_rot), rot);
                let spot = spot_new;
                let model = vu::trans(&mat, spot);
                spot_new = ee::vadd(ee::apply(&test, new_pos), spot);
                if let Some(obj) = self.eff_cmp {
                    cx.draws.push(crate::draw::DrawRec::Clump {
                        obj,
                        matrix: model,
                        alpha: ONE,
                        layer,
                        clut: self.clut,
                    });
                }
            }
        }
        let old = self.hoge;
        self.hoge = old.wrapping_add(1);
        if old == self.time {
            self.base.del_flag = 1;
            self.end_flg = 1;
            // BurstSmokeGenerator's smoke with flg2: see above.
        }
    }
}

/// `ccEffect2` (main effect.cpp, 0x14 bytes).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Effect2 {
    /// +0x00 `endFlag`, +0x02 `id`, +0x04 `status`, +0x06 `lifeTime`, +0x08
    /// `age`, +0x0a `cnt`.
    pub end_flag: bool,
    pub id: i16,
    pub status: i16,
    pub life_time: i16,
    pub age: i16,
    pub cnt: i16,
    /// +0x0c `targetPtr`.
    pub target: Option<crate::CharRef>,
    /// +0x10 `effPtr`: ids 0 and 1 hold a bolt.
    pub bolt: Option<Box<ThunderBolt>>,
}

/// `ccEffect2::InitEffect(id)` (main 0x001cc240) on the first free slot.
pub fn new_effect2(slots: &mut [Effect2], id: i16) -> Option<usize> {
    let i = slots.iter().position(|e| e.status == 0)?;
    slots[i] = Effect2 { end_flag: false, id, status: 1, life_time: -1, age: 0, cnt: 0, target: None, bolt: None };
    Some(i)
}

/// The rest of `ccEffectCtrl::Main` (main 0x001c38a0): each live
/// `ccEffect2`'s `Main` (0x001cc290), on the effect layer.
pub fn effect2_main(cx: &mut Cx) {
    for i in 0..SLOTS2 {
        if cx.spells.effect2[i].status == 0 {
            continue;
        }
        let mut e = std::mem::take(&mut cx.spells.effect2[i]);
        if e.end_flag {
            e.status = 0;
            e.bolt = None;
        } else {
            if e.id == 0 || e.id == 1 {
                let ended = e.bolt.as_ref().is_some_and(|b| b.end_flg != 0);
                if ended {
                    e.end_flag = true;
                    e.bolt = None;
                } else if let Some(b) = e.bolt.as_mut() {
                    b.draw(cx, EFFECT_LAYER);
                }
            }
            e.cnt = e.cnt.wrapping_add(1);
            if e.life_time != -1 {
                let old = e.age;
                e.age = old.wrapping_add(1);
                if e.life_time < old {
                    e.end_flag = true;
                }
            }
        }
        cx.spells.effect2[i] = e;
    }
}

/// `effSkillTornadeThunderPos(pos, n)` (main 0x001d5b20; with an offset
/// 0x001d5c50).
pub fn eff_skill_tornade_thunder_pos(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    pos: V4,
    offset: Option<V4>,
    n: i32,
) -> Option<usize> {
    let i = ctrl.new_effect(cx, THUNDER_POS)?;
    let e = &mut ctrl.effects[i];
    e.life_time = 30;
    e.level = ((n << 28) >> 28) as i8;
    e.pos_t = pos;
    let r = cx.host.rand();
    ctrl.effects[i].param = (n - 1) * 4 + r % 4;
    if let Some(o) = offset {
        ctrl.effects[i].offset = o;
    }
    cx.raise(Event::Sound3d { se: 68, pos });
    Some(i)
}

/// Effect -17's case of the second chain (main 0x001cae60).
pub fn thunder_pos_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let (param, flags, cnt, pos_t, offset) = {
        let e = &ctrl.effects[i];
        (e.param, e.flags, e.cnt, e.pos_t, e.offset)
    };
    let t = &cx.spells.data.thunder;
    if i32::from(cnt) != t.at(param, flags * 2) {
        return;
    }
    let mut n = t.at(param, flags * 2 + 1);
    let player = cx.host.player_pos();
    let bounds = cx.host.bounds();
    let mut p = ee::vadd(space::w2p(pos_t, player, bounds), offset);
    p[3] = ONE;
    let p = space::p2w(p, player, bounds);
    let dat = cx.spells.data.thunder.skill_thunder_eff2;
    while n != 0 {
        let k = new_effect2(&mut cx.spells.effect2, 1);
        let num = (cx.host.genrand() as i32 & 7).abs() + 1;
        let bolt = ThunderBolt::new(cx, p, 1, 0x437a_0000, num, Some(dat));
        if let Some(k) = k {
            cx.spells.effect2[k].bolt = Some(Box::new(bolt));
        }
        n -= 1;
    }
    ctrl.effects[i].flags = flags + 1;
}
