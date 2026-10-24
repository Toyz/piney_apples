//! Bosses: `ccBoss` (boss.cpp, gcmn 0x0045bcf0-0x0045fbd0) and Skeith
//! (`ccBoss01`, 0x0047b300-0x0047ee68), entry type 7 code 0; Innis (1)
//! [`innis`], Magus (2) [`magus`], Fidchell (3) [`fidchell`], Kyvia (12, 13)
//! [`kyvia`]. `ccBossEntryStart(code)` (0x0045b2a0) starts the effect
//! manager ([`Effects`]) and `bossFunc[code]`, which makes the boss and runs
//! [`Boss::main`] each frame. The tables come from the build ([`BossData`]);
//! sounds, the camera and the pictures are [`Out`]s. docs/engine/boss.md
//! and boss-innis.md, boss-magus.md, boss-kyvia.md, boss-fidchell.md.

pub mod blur;
pub mod corbenik;
pub mod fidchell;
pub mod gorre;
pub mod innis;
pub mod kyvia;
pub mod macha;
pub mod magus;
pub mod tarvos;

use piney_data::field::ee;
use piney_data::libm;

use crate::affect::{self, AffectCtx};
use crate::chara::{self, Env};
use crate::damage::{self, Roll};
use crate::enemy_ai::{World, get_dirc, rand_f, search_near_person};
use crate::event::{Event, Events, Who};
use crate::exp::Party;
use crate::geom::{self, V4};
use crate::item::{self, ItemSkill};
use crate::param::{SkillParam, cond};
use crate::rand::Rng;
use crate::scene::Scene;
use crate::tables::Tables;
use crate::world::CharHit;

type F = u32;

const PI: F = 0x4049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
const ONE: F = 0x3f80_0000;
const HALF: F = 0x3f00_0000;
const VF0: V4 = [0, 0, 0, ONE];
/// `SelectTarget`'s radius everywhere Skeith looks: 1e6.
const FAR: F = 0x4974_23f0;

/// `BossSkillTbl` rows Skeith's attacks use: the cross (0), the wave (1),
/// the magic (2).
pub const SKILL_CROSS: usize = 0;
pub const SKILL_WAVE: usize = 1;
pub const SKILL_MAGIC: usize = 2;
/// The magic's animation and the wave's (in `xeffect`).
pub const ANM_MAGIC: &str = "ANM_ex11mag0";
pub const ANM_WAVE: &str = "ANM_xx11wave";

/// The acts (`actNum`) Skeith's `Think` and `Action` switch on.
pub mod act {
    pub const NEUTRAL: i16 = 0;
    pub const DAMAGE0: i16 = 1;
    pub const DAMAGE1: i16 = 2;
    pub const CROSS: i16 = 3;
    pub const WAVE: i16 = 4;
    pub const DRAIN: i16 = 5;
    pub const MAGIC: i16 = 6;
    pub const DASH: i16 = 7;
    pub const WANDER: i16 = 8;
    /// Drained by Kite: to the Epitaph's patterns.
    pub const EPITAPH: i16 = 11;
    pub const EPITAPH_NEUTRAL: i16 = 12;
    pub const EPITAPH_WAVE: i16 = 13;
    pub const DEAD: i16 = 14;
    pub const ESCAPE: i16 = 16;
    pub const RETURN: i16 = 17;
    pub const CHASE: i16 = 18;
    pub const RAND_DRIVE: i16 = 21;
}

/// Skeith's tables (`tables::combat`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkeithData {
    /// `boss01NormalActTbl`, `boss01SuperActTbl`, `boss01EpitaphActTbl`
    /// (INF gcmn 0x005eb430, 0x005eb490, 0x005eb530, listed by @1035 at
    /// 0x005eb598): the words to their `-1` and one past it.
    pub act_tbls: [Vec<i32>; 3],
    /// @1378 (INF 0x005eb608): pattern 10's skills, one by `ccRand() % 3`.
    pub rand_skills: [i32; 3],
    /// `Boss01AnmTbl` (INF 0x005eb5b0): the act's animation (22 names,
    /// None for none).
    pub anm_tbl: Vec<Option<String>>,
}

impl SkeithData {
    /// The volume's.
    pub fn of(volume: piney_data::volume::Volume) -> SkeithData {
        let t = piney_data::tables::combat::of(volume);
        let acts = t.skeith_acts();
        let r = t.skeith_rand_skills();
        SkeithData {
            act_tbls: [acts[0].to_vec(), acts[1].to_vec(), acts[2].to_vec()],
            rand_skills: [r[0], r[1], r[2]],
            anm_tbl: t.skeith_anims().iter().map(|a| a.map(str::to_string)).collect(),
        }
    }
}

/// Every boss's tables, for the volume ([`SkeithData`], [`innis::InnisData`],
/// [`magus::MagusData`], [`kyvia::KyviaData`], [`fidchell::FidchellData`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BossData {
    pub skeith: SkeithData,
    pub innis: innis::InnisData,
    pub magus: magus::MagusData,
    pub kyvia: kyvia::KyviaData,
    pub fidchell: fidchell::FidchellData,
    pub gorre: gorre::GorreData,
    pub macha: macha::MachaData,
    pub tarvos: tarvos::TarvosData,
    pub corbenik: corbenik::CorbenikData,
}

impl BossData {
    /// The volume's.
    pub fn of(volume: piney_data::volume::Volume) -> BossData {
        BossData {
            skeith: SkeithData::of(volume),
            innis: innis::InnisData::of(volume),
            magus: magus::MagusData::of(volume),
            kyvia: kyvia::KyviaData::of(volume),
            fidchell: fidchell::FidchellData::of(volume),
            gorre: gorre::GorreData::of(volume),
            macha: macha::MachaData::of(volume),
            tarvos: tarvos::TarvosData::of(volume),
            corbenik: corbenik::CorbenikData::of(volume),
        }
    }
}

/// Which class a [`Boss`] is (`ccBoss`'s vtable): what its `Main` and
/// `Affect` run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Class {
    /// `ccBoss01`.
    #[default]
    Skeith,
    /// `ccBoss02` and its three slaves.
    Innis(Box<innis::Innis>),
    /// `ccBoss03`: the body; its leaves are bosses of their own characters.
    Magus(Box<magus::Magus>),
    /// `ccBoss03Leaf`.
    MagusLeaf(Box<magus::leaf::Leaf>),
    /// `ccBossKyvia01` or `ccBossKyvia02`: the body; its core and gomoras
    /// are bosses of their own characters.
    Kyvia(Box<kyvia::Kyvia>),
    /// `kyviaCore`.
    KyviaCore(Box<kyvia::core::Core>),
    /// `kyviaGomora`.
    Gomora(Box<kyvia::gomora::Gomora>),
    /// `ccBoss04`.
    Fidchell(Box<fidchell::Fidchell>),
    /// `ccBoss05`: the body; its two brothers are bosses of their own
    /// characters ([`gorre::brother::Brother`]).
    Gorre(Box<gorre::Gorre>),
    /// `ccBoss05Brother`.
    GorreBrother(Box<gorre::brother::Brother>),
    /// `ccBoss06`.
    Macha(Box<macha::Macha>),
    Tarvos(Box<tarvos::Tarvos>),
    /// `ccBoss08`, the first form: its seeds are bosses of their own
    /// characters.
    Corbenik(Box<corbenik::first::First>),
    /// `ccBoss08Seed`.
    CorbenikSeed(Box<corbenik::seed::Seed>),
    /// `ccBoss08_b`, the second form: its roots are bosses of their own
    /// characters.
    CorbenikB(Box<corbenik::second::Second>),
    /// `ccBoss08Root`.
    CorbenikRoot(Box<corbenik::second::Root>),
    /// `ccBoss08_c`: Corbenik's third form; its eyes and killer eyes are
    /// bosses of their own characters.
    CorbenikC(Box<corbenik::third::Third>),
    /// `ccBoss08Eye`, `ccBoss08KillerEye`.
    CorbenikEye(Box<corbenik::eye::Eye>),
    CorbenikKiller(Box<corbenik::eye::Killer>),
    /// A bare `ccBoss` (a slave's base).
    Plain,
}

/// Which table `patTbl` points at.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tbl {
    /// Before the constructor's `SetPatternTbl` (a null table).
    #[default]
    None,
    Normal,
    Super,
    Epitaph,
}

impl Tbl {
    fn words(self, d: &SkeithData) -> &[i32] {
        match self {
            Tbl::None => &[],
            Tbl::Normal => &d.act_tbls[0],
            Tbl::Super => &d.act_tbls[1],
            Tbl::Epitaph => &d.act_tbls[2],
        }
    }
}

/// `ENTRYAFTERIMAGE_T` (0x50 bytes): an after-image's colour blend
/// (`SetFogBlend(colorAlpha, color)`, none for colour -1; `colorAlpha`
/// moves by `colorSpd` a draw, held to 0-100), its transparency (dead once
/// `transSpd` takes it to 0 or 1), `maxFrame` (counted down, never read)
/// and its scale from `ss` to `es` over `scaleTime` draws (1 for none).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AfterImageParams {
    pub color: u32,
    pub color_alpha: F,
    pub color_spd: F,
    pub max_frame: F,
    pub transparency: F,
    pub trans_spd: F,
    pub ss: V4,
    pub es: V4,
    pub scale_time: F,
}

impl AfterImageParams {
    /// `ccBossAfterImageManager::EntryAfterImageAnm`'s own, for a null
    /// param (OUT gcmn 0x0046c7b8): colour 0x00505000 at 100, transparency
    /// 1 by -1/80, `maxFrame` 40, `ss` and `es` (0, 0, 0, 1), no scale.
    pub const ENTRY: AfterImageParams = AfterImageParams {
        color: 0x0050_5000,
        color_alpha: 0x42c8_0000,
        color_spd: 0,
        max_frame: 0x4220_0000,
        transparency: 0x3f80_0000,
        trans_spd: 0xbc4c_cccd,
        ss: [0, 0, 0, 0x3f80_0000],
        es: [0, 0, 0, 0x3f80_0000],
        scale_time: 0,
    };
}

/// A `ccAnm` as the rules read it: the clip, `frameNow * 256 + frameCnt`,
/// `frameSpd` (+0x9c), the clip's length and whether it loops.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Anm {
    pub clip: Option<String>,
    pub time: u32,
    pub speed: u16,
    pub frames: u32,
    pub looping: bool,
    /// The time the pose was last evaluated at (`_AnimateForward`'s, before
    /// a looping clip wraps): what the picture poses the body at.
    pub posed: u32,
}

impl Anm {
    /// `ccAnm::ccAnm`: no clip, `frameSpd` 256.
    pub fn new() -> Anm {
        Anm { clip: None, time: 0, speed: 256, frames: 0, looping: false, posed: 0 }
    }

    /// `SetAnm(chunk, 0)`: the clip from frame 0.
    pub fn set(&mut self, clip: &str, clips: &dyn Fn(&str) -> Option<(u32, bool)>) {
        let (frames, looping) = clips(clip).unwrap_or((0, false));
        self.clip = Some(clip.to_string());
        self.time = 0;
        self.posed = 0;
        self.frames = frames;
        self.looping = looping;
    }

    /// `frameNow` (+0x98).
    pub fn frame(&self) -> i32 {
        (self.time >> 8) as i32
    }

    /// `_AnimateForward(frameSpd)` when a clip is set (+0xac), else 0.
    pub fn forward(&mut self) -> bool {
        if self.clip.is_none() {
            return false;
        }
        let f = piney_data::anim::forward_clip(self.frames, self.looping, self.time, u32::from(self.speed));
        self.time = f.time;
        if let Some(t) = f.pose_at {
            self.posed = t;
        }
        f.ended
    }
}

/// `ccAnimateObject`'s fade (`SetFade` gcmn 0x004e8f70, `AnimateFade`
/// 0x004e8fc0): the transparency the dashes and chases blink to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fade {
    pub start: F,
    pub end: F,
    pub spd: F,
    pub accel: F,
    pub flag: i32,
    pub transparency: F,
}

impl Fade {
    fn set(&mut self, start: F, end: F, time: F) {
        if ee::le(time, 0) {
            self.transparency = end;
            return;
        }
        self.start = start;
        self.end = end;
        self.spd = ee::div(ee::sub(end, start), time);
        self.transparency = start;
        self.flag = 1;
    }

    fn animate(&mut self) -> bool {
        if self.flag == 0 {
            return true;
        }
        self.transparency = ee::add(self.transparency, self.spd);
        if ee::lt(self.end, self.start) {
            if ee::lt(self.transparency, self.end) {
                self.transparency = self.end;
                self.flag = 0;
                return true;
            }
        } else if !ee::le(self.transparency, self.end) {
            self.transparency = self.end;
            self.flag = 0;
            return true;
        }
        self.spd = ee::add(self.spd, self.accel);
        false
    }
}

/// The boss effects whose `m_bEnabled` a boss waits on, as the manager
/// (`ccBossEffManager::Draw`, gcmn 0x00461750) runs them: once a frame
/// before the boss, each enabled effect's `Draw` and each disabled one
/// deleted. Only their lifetimes (and Innis's missiles' flight) are kept
/// here; their pictures are the runtime's ([`Out::Effect`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Effects {
    pub slots: Vec<Option<Eff>>,
}

/// One effect's life: what it is and the frames its `Draw` has run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Eff {
    pub kind: EffKind,
    pub enabled: bool,
    pub count: i32,
    pub proc: i32,
    /// `ccEffSamonRing.Transparency` (+0x6c) as it bursts.
    pub fade: F,
    /// A missile's flight ([`innis::Missile`]).
    pub missile: Option<Box<innis::Missile>>,
    /// The meteors' flights ([`kyvia::Meteorite`]).
    pub meteorite: Option<Box<kyvia::Meteorite>>,
    /// Magus's needles ([`magus::Needle`]).
    pub needle: Option<Box<magus::Needle>>,
    /// Fidchell's spells ([`fidchell::eff::Fx`]).
    pub fidchell: Option<Box<fidchell::eff::Fx>>,
    /// A thunderbolt of Kyvia's ([`kyvia::thunder::Bolt`]).
    pub bolt: Option<Box<kyvia::thunder::Bolt>>,
    /// Kyvia's magical ball ([`kyvia::magic_ball::MagicalBall`]).
    pub ball: Option<Box<kyvia::magic_ball::MagicalBall>>,
    /// Kyvia's hell's express ([`kyvia::hell::Hell`]).
    pub hell: Option<Box<kyvia::hell::Hell>>,
    /// Macha's lasers and blast ([`macha::eff::Fx`]).
    pub macha: Option<Box<macha::eff::Fx>>,
    /// Tarvos's spike rain and phantom ([`tarvos::eff::Fx`]).
    pub tarvos: Option<Box<tarvos::eff::Fx>>,
    /// Corbenik's wave and flush ([`corbenik::eff::Fx`]).
    pub corbenik: Option<Box<corbenik::eff::Fx>>,
}

/// The effects the bosses make (`ccBossEff*Create`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffKind {
    /// `ccBossEffWaveShockCreate(pos, dirc, scale)` (0x00478820) at 1.0.
    WaveShock,
    /// The same at another scale (Macha's Epitaph's 2.0).
    WaveShockScaled { scale: F },
    /// `ccBossEffMagicSquareCreate(pos, n)` (0x00478b10).
    MagicSquare { n: i32 },
    /// `ccBossEffForceGeneratorCreate(p, rot, speed, r0, r1, num, life,
    /// clt)` (0x004797e0): a `ccBossEffBrightMagicSquare` of `num` photons
    /// living `life` frames.
    ForceGenerator { num: i32, life: i32, speed: F, r0: F, r1: F, clt: i32 },
    /// `ccBossEffAutoSamonRingCreate(pos, rot, param, n)` (0x00479900).
    AutoSamonRing { n: i32 },
    /// The same with another `param` (Macha's blast's (0, 2/3, 10)).
    AutoSamonRingOf { n: i32, param: [F; 4] },
    /// `ccBossEffIceBreakCreate(pos, scale)` (0x00479240).
    IceBreak,
    /// `ccBossEffDeadCreate(pos, pos, 0)` (0x004795d0).
    Dead,
    /// `ccBossEffSamonRingCreate(pos, rot, scale, model)` (MUT gcmn
    /// 0x0048d670) and the burst the caller sets: `BurstScale(spoint)`,
    /// `BurstTransparency(tpoint)`, `EnyFlg` 1.
    SamonRing { model: i32, scale: V4, spoint: V4, tpoint: F },
    /// `ccBossEff{Ice,Lightning,Blaze}MissileCreate(vec, NO, cb, cam)`
    /// (MUT 0x0048d750, 0x0048d830, 0x0048d910): a spline through `ctrl`.
    Missile { element: innis::Element, no: i32, ctrl: [V4; 4] },
    /// `ccBossEffMeteoriteMissileCreate(pos, range, IN, OUT, cb, cam)` (MUT
    /// 0x0048d9f0): `IN` meteors falling round `pos` (Kyvia's MegidFlame).
    Meteorite { count: i32 },
    /// `ccBossEffNeedleCreate(pos, 0, 50, 0.3, n, 3, 15, 15)` (MUT
    /// 0x0048e430): `n` needles out of the ground (Magus's).
    Needle { n: i32 },
    /// `ccBossEffAutoSamonRingCreate(pos, rot, (0.5, 1, 0, 40), 195)`: a
    /// leaf of Magus's dies.
    LeafRing,
    /// `ccBossEffMeteoSwormCreate(sp, ep, n, 500, 50, &camView)` (OUT gcmn
    /// 0x00489de0): Fidchell's meteors ([`fidchell::eff::MeteoSworm`]).
    MeteoSworm { n: i32 },
    /// `ccBossEffThunderStormCreate(pos, 200, n)` (OUT 0x00489fa0).
    ThunderStorm { n: i32 },
    /// `ccBossEffRockTowerCreate(pos, n)` (OUT 0x0048a3a0).
    RockTower { n: i32 },
    /// `ccBossEffFinalPhotonFlashCreate(pos)` (OUT gcmn, Gorre's
    /// `OnThinkKerse`): a finishing flash at the target. No picture yet
    /// (crates/piney-game/src/fx.rs); its life (72 frames) is measured.
    FinalPhotonFlash,
    /// `ccBossEffThunderboltCreate(pos, time, range, num, dat)` (OUT gcmn
    /// 0x00489540): `num` bolts of Kyvia's ([`kyvia::thunder::Bolt`]).
    Thunderbolt { num: i32 },
    /// `ccBossEffMagicalBallCreate(ran, pos, blur, cb)` (QUA gcmn
    /// 0x0037ca70): Kyvia's third fight's ball.
    MagicalBall,
    /// `HellSpecialExpressCreate(pos, dataSet)` (QUA gcmn 0x0037bf40):
    /// Kyvia's fourth fight's hell's express.
    HellExpress,
    /// `ccBossEffLaserShotCreate(from, to, speed)` (QUA gcmn 0x0037d250):
    /// a laser of Macha's eyes ([`macha::eff::Laser`]).
    LaserShot,
    /// `ccBossEffLightBallShockCreate(pos)` (QUA gcmn 0x0037d320): Macha's
    /// light ball's blast ([`macha::eff::Shock`]).
    LightBallShock,
    /// `ccBossEffSpikeRainCreate(target, owner, cb)` (QUA gcmn 0x0037d180):
    /// Tarvos's 32 spikes ([`tarvos::eff::SpikeRain`]).
    SpikeRain,
    /// `ccBossEffPhantomCreate(pos, dirc)` (QUA gcmn 0x0037dca0): Tarvos's
    /// phantom ([`tarvos::eff::Phantom`]).
    Phantom,
    /// `ccBossEffEruptionCreate(pos, n, 100, 10)` (QUA gcmn 0x0037b7b0):
    /// made off, its `Draw` empty; the next pass frees its slot.
    Eruption,
    /// `ccBossEffWaveCreate(pos, cb)` (QUA gcmn 0x0037dd60): Corbenik's
    /// wave ([`corbenik::eff::Wave`]).
    CorbenikWave,
    /// `ccBossEffFlushCreate(pos, start, end)` (QUA gcmn 0x0037dec0): a
    /// seed's flush ([`corbenik::eff::Flush`]).
    Flush,
    /// `ccBossEffStarDustFlashCreate(pos, target)` (QUA gcmn 0x0037dbd0):
    /// the second form's star dust ([`corbenik::eff::StarDust`]).
    StarDustFlash,
    /// `ccBossEffTransformCreate(pos)` (QUA gcmn 0x0037e270): its barrier's
    /// transform ([`corbenik::eff::Transform`]).
    Transform,
    /// `ccBossEffThunder3Create(sp, ep, w, life, follow, clut)` (QUA gcmn
    /// 0x0037dc40): a bolt of the star dust's
    /// ([`corbenik::stardust::Thunder`]).
    Thunder3,
    /// `ccBossEffRootDrainCreate(pos)` (QUA gcmn 0x0037e140), a
    /// `ccBossEffDrain`: the root drain ([`corbenik::eff::Drain`]).
    RootDrain,
    /// `ccBossEffTeraLaserCreate(pos, rot, scale, target, cb)` (QUA gcmn
    /// 0x0037e060): the third form's tera laser ([`corbenik::laser::Tera`]).
    TeraLaser,
    /// `ccBossEffRefrectionLaserCreate(sp, ep, eyes, n, cb)` (QUA gcmn
    /// 0x0037da10): a laser bounced off the eyes
    /// ([`corbenik::laser::Refrect`]).
    RefrectionLaser,
}

impl Effects {
    const SLOTS: usize = 1024;

    /// The manager's `Create`: the first free slot, or -1 (none: the
    /// effect is made and lost).
    fn create(&mut self, kind: EffKind) -> i32 {
        if self.slots.is_empty() {
            self.slots = vec![None; Self::SLOTS];
        }
        let e = Eff {
            kind,
            enabled: true,
            count: 0,
            proc: 0,
            fade: ONE,
            missile: None,
            meteorite: None,
            needle: None,
            fidchell: None,
            bolt: None,
            ball: None,
            hell: None,
            macha: None,
            tarvos: None,
            corbenik: None,
        };
        match self.slots.iter().position(Option::is_none) {
            Some(k) => {
                self.slots[k] = Some(e);
                k as i32
            }
            None => -1,
        }
    }

    /// `m_bEnabled` of the effect `ccGetBossEffAdrs(id)` returned; a slot
    /// freed since reads as disabled (the game reads the deleted object,
    /// whose flag was 0 when it went).
    pub fn enabled(&self, id: Option<i32>) -> bool {
        id.and_then(|k| usize::try_from(k).ok())
            .and_then(|k| self.slots.get(k))
            .and_then(|s| s.as_ref())
            .is_some_and(|e| e.enabled)
    }

    /// `_g_bossEffManager->IsValidAdrs(ccGetBossEffAdrs(id))`: still in
    /// the manager (a disabled one until the next pass frees it).
    pub fn valid(&self, id: Option<i32>) -> bool {
        id.and_then(|k| usize::try_from(k).ok()).and_then(|k| self.slots.get(k)).is_some_and(Option::is_some)
    }

    /// One pass of the manager.
    pub fn tick(&mut self) {
        for s in &mut self.slots {
            match s {
                Some(e) if !e.enabled => *s = None,
                Some(e) => e.draw(),
                None => {}
            }
        }
    }
}

impl Eff {
    /// The effect's `Draw` as far as its life goes: the `Draw`s until it
    /// clears `m_bEnabled`, as tools/test_battle_boss_rs.py and
    /// test_battle_innis_rs.py measure them by running the game's `Create`
    /// and `Draw` (docs/engine/boss.md, boss-innis.md). A missile's `Draw`
    /// is [`innis::Missile::draw`].
    pub(crate) fn draw(&mut self) {
        self.count += 1;
        let life = match self.kind {
            // ccBossEffWaveShock::Draw (0x0046b3f0): its ccAnm to the end
            // of ANM_ex31lhit (46 frames).
            EffKind::WaveShock | EffKind::WaveShockScaled { .. } => 45,
            // ccBossEffMagicSquareCreate(pos, n) makes a ccBossEffLight
            // (pos, n, 80, 10), for n 1 (pos, 1, 60, 10): 1 + life + wait.
            EffKind::MagicSquare { n: 1 } => 71,
            EffKind::MagicSquare { .. } => 91,
            // ccBossEffBrightMagicSquare (0x0046dd90): photon k waits
            // (k / 2) * 10 frames, then lives `life`.
            EffKind::ForceGenerator { num, life, .. } => life + (num - 1) / 2 * 10 + 3,
            // ccBossEffAutoSamonRing::Draw (0x0046ed70): its
            // ccEffSamonRing fades out in 20 frames, whatever the model.
            EffKind::AutoSamonRing { .. } | EffKind::AutoSamonRingOf { .. } | EffKind::LeafRing => 20,
            // ccBossEffIceBreak::Draw (0x0046cc10): the flash (1 frame),
            // 61 frames, 61 more; the last one clears it.
            EffKind::IceBreak => 1 + 61 + 61,
            EffKind::Dead => 120,
            // ccBossEffFinalPhotonFlashCreate::Draw (0x0046f460): the life
            // (1000.0, -100.0 a Draw) goes under 0 on the 11th call, which
            // rings and starts a 61-call countdown (old count reaching 60)
            // before m_bEnabled clears: 11 + 61.
            EffKind::FinalPhotonFlash => 72,
            // ccEffSamonRing::Draw (MUT 0x004786b0), bursting: the
            // transparency falls by Tpoint a Draw; gone once below 0.
            EffKind::SamonRing { tpoint, .. } => {
                self.fade = ee::sub(self.fade, tpoint);
                if ee::lt(self.fade, 0) {
                    self.enabled = false;
                }
                return;
            }
            EffKind::Missile { .. }
            | EffKind::Meteorite { .. }
            | EffKind::Needle { .. }
            | EffKind::MeteoSworm { .. }
            | EffKind::ThunderStorm { .. }
            | EffKind::RockTower { .. }
            | EffKind::Thunderbolt { .. }
            | EffKind::MagicalBall
            | EffKind::HellExpress
            | EffKind::LaserShot
            | EffKind::LightBallShock
            | EffKind::SpikeRain
            | EffKind::Phantom
            | EffKind::Eruption
            | EffKind::CorbenikWave
            | EffKind::Flush
            | EffKind::StarDustFlash
            | EffKind::Transform
            | EffKind::Thunder3
            | EffKind::RootDrain
            | EffKind::TeraLaser
            | EffKind::RefrectionLaser => return,
        };
        if self.count >= life {
            self.enabled = false;
        }
    }
}

/// What a boss's fight does to a particle generator it keeps (the
/// member that holds its pointer).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenOp {
    /// `ParticleKill()`: the pointer kept.
    Kill,
    /// +5 bit 0, `pauseFlag`: the generator held still while set.
    Pause(bool),
    /// The pointer cleared: the generator runs out on its own.
    Forget,
}

/// What the rules ask of the rest of the game, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Out {
    /// `EntryAfterImage(&m_eai)`: an after-image of the boss's pose.
    AfterImage,
    /// `m_anmw.SetMatrix_PosRotZYX(pos, dirc)` and `DrawParts(m_anmw, 0)`:
    /// the wave (`xeffect`'s `ANM_xx11wave`, [`Boss::anm_wave`]) drawn at
    /// the boss this frame, on objLayer.
    DrawWave,
    /// `ccSeOn3D(se, pos)`.
    Se3d {
        se: i32,
        pos: V4,
    },
    /// `ccSeOn(se)`.
    Se {
        se: i32,
    },
    /// `ccSeOn3DNote(se, pos, note)`.
    Se3dNote {
        se: i32,
        pos: V4,
        note: u8,
    },
    /// `scFadeDef->EntryFlash(t, colour, 0, 0, 512, 384)`.
    Flash {
        t: i32,
        colour: u32,
    },
    /// A boss effect made (`ccBossEff*Create`), in [`Boss::effects`]'s
    /// slot `id` (-1 none), with what its picture needs.
    Effect {
        id: i32,
        kind: EffKind,
        pos: V4,
        dirc: V4,
    },
    /// `bossCam->QuakeCam(q)` (with a boss camera): the vector of three
    /// `ccRandF(q[k])` it drew.
    Quake([F; 3]),
    /// `ccMenu +0x26 cursolOff`.
    CursorOff(bool),
    /// `ccMenu->forbid` (+0xfe) and `forbidChatExcept` (+0x100).
    MenuForbid {
        on: bool,
        chat_except: bool,
    },
    /// A stream shown over the fight: `ccMenu +0xf2` the stream, `+0xf0`
    /// `mask` (a drained member's `1 << slot`, else 0), `openReqNum`
    /// 0x104a (StreamMenu, 74, no dim), `mode` 0, `firstTime` 1.
    StreamMenu {
        stream: i32,
        mask: i32,
    },
    /// `_g_bossEffManager->OnCinemaMode(n)` / `OffCinemaMode()`.
    Cinema(Option<i32>),
    /// `worldman->eventArea->SwitchLayer()`.
    SwitchLayer,
    /// `BeginStageEffect(rgba, t0, t1, t2)` / `EndStageEffect(id, rgba,
    /// t)` on the stage fader, when the stage has one.
    StageBegin {
        rgba: u32,
        t: [i32; 3],
    },
    StageEnd {
        rgba: u32,
        t: i32,
    },
    /// `ccBufferReverce` drawn on its own layer (the magic's last part).
    Reverse,
    /// `BeginDeadEffect`'s camera: the boss camera handed to camera 1 and
    /// camera 3 set at `eye` looking at the boss (`view`, its place then);
    /// `ccSqFade(0, 0, 30, 3)` when `music_fade`.
    DeadCamera {
        eye: V4,
        view: V4,
        music_fade: bool,
    },
    /// `ccItemSkillRequest(this, target, sid, flag)`: the target's scene
    /// index and what the request made.
    Skill(usize, ItemSkill),
    /// `ccDeleteCmnd(this)`: off the command lists (done on the scene).
    DeleteCmnd,
    /// `ccHitMarkDisp(this, affectPerson)` and `ccEntryFlyFontNew(kind, n,
    /// pos, this, 1, 1)`: a hit's mark and number.
    HitMark {
        by: Option<usize>,
    },
    FlyFont {
        kind: i32,
        n: i32,
    },
    /// `ccClearSpcCondition()`: the party's conditions cleared (the boss
    /// is down).
    ClearSpcCondition,
    /// `bossCam->SetMode(mode, range, hi, who)` (MUT gcmn 0x00475ae0).
    CamMode {
        mode: i32,
    },
    /// `bossCam->SetFreeCamPosView(pos, view)` (MUT 0x00475c90): the eye
    /// and the point looked at while the camera is in mode 5.
    FreeCam {
        pos: V4,
        view: V4,
    },
    /// `bossCam` +0xe0 (Mutation's pitch) set to `v`, or with `add` raised
    /// by it.
    CamPitch {
        add: bool,
        v: F,
    },
    /// A write to the effect manager's blur through `ccBoss` +0x148.
    Blur(blur::Set),
    /// A particle generator started (`startParticleGenerator`), by its
    /// rows, at `pos`.
    Particles {
        which: innis::Gen,
        pos: V4,
    },
    /// `effResistantShield(this, magic, -1)`: a blow of the kind the boss
    /// resists (Innis's spells, 1; Gorre's brothers' by row). `stretched`:
    /// a brother's, its element reshaped ([`gorre::brother`]).
    Shield {
        magic: i32,
        stretched: bool,
    },
    /// A particle generator of Kyvia's fight started: which, where, and
    /// the member that keeps it.
    KyviaParticles {
        which: kyvia::Gen,
        at: kyvia::Place,
        held: Option<kyvia::Held>,
    },
    /// What the fight does to a generator it keeps.
    KyviaGen {
        held: kyvia::Held,
        op: kyvia::GenOp,
    },
    /// `bossCam->SetRotXLimit(v)` (MUT gcmn 0x00475a40).
    CamRotXLimit(F),
    /// `bossCam` +0x139 `ZrotControlFlg` (Kyvia's): the right stick, or
    /// L1 and R1, turns the eye about the player.
    CamSway(bool),
    /// `EVENTAREAB8::Move()` (MUT gcmn 0x0041e750): the disc on to its next
    /// stage.
    DiscNextStage,
    /// `ccSqFade(0, 0, t, 3)`: the music out.
    MusicFade {
        t: i32,
    },
    /// `ccItemSkillRequest(user, target, sid, flag)` or
    /// `ccSkillRequestParam(user, target, sid, param)` from one of a boss's
    /// parts: the request made.
    SkillFrom {
        user: usize,
        target: usize,
        skill: ItemSkill,
    },
    /// `effSkillStart(who, sid, 0, 0)`.
    SkillStart {
        who: usize,
        sid: i32,
    },
    /// `effSmokeRock(pos, rot, 0, 20, -1, 75)`: a meteor's landing.
    SmokeRock {
        pos: V4,
        rot: V4,
    },
    /// `bossCam->SetMode(mode, range, 0, 0)` (MUT gcmn 0x00475ae0) for modes
    /// 3 and 4: the eye eased to `range` behind (3), then back (4).
    CamModeRange {
        mode: i32,
        range: F,
    },
    /// `bossCam` +0x01 `InitLock` set: the next turn goes straight to the
    /// boss.
    CamInitLock,
    /// Kyvia's arm (`anmw`, `ANM_ex0batc0`) drawn this frame at its pose
    /// time `time`, at `pos` turned by `dirc` (`DrawParts(anmw, 0)`).
    DrawArm {
        time: u32,
        pos: V4,
        dirc: V4,
    },
    /// `EntryAfterImageAnm(anm, pos, dirc, params)`: an after-image of a
    /// `ccAnm` of the boss's own (Kyvia's arm) at its frame `frame`
    /// (`ccBossAfterImageEx::Init`: at frame 0 one step on).
    AfterImageAnm {
        frame: i32,
        pos: V4,
        dirc: V4,
        params: AfterImageParams,
    },
    /// `changeCamera(n)` (main): the active camera (1 the field's, 3 the
    /// event's).
    CameraChange(i32),
    /// `cameraSetPos(pos, cam)`, `cameraSetView(view, cam)`.
    CameraPos {
        cam: i32,
        pos: V4,
    },
    CameraView {
        cam: i32,
        view: V4,
    },
    /// `if (checkCameraShakeRange(pos)) cameraShake(s[0], s[1], s[2],
    /// s[3])`: the runtime checks the range and shakes (its `rand()`).
    CameraShake {
        pos: V4,
        s: [i32; 4],
    },
    /// `scFadeDef->EntryFlash2(t_in, t_out, colour)` (no hold) or
    /// `EntryFlash3(t_in, t_out, hold, colour)`: in, held, out.
    FlashFade {
        t_in: i32,
        t_out: i32,
        hold: Option<i32>,
        colour: u32,
    },
    /// `scFadeDef->EntryFade(t, mode, colour, 0, 0, 512, 384)`.
    EntryFade {
        t: i32,
        mode: i32,
        colour: u32,
    },
    /// `ccSeOnNote(se, note)`.
    SeNote {
        se: i32,
        note: u8,
    },
    /// `ccSeOn3DLoop(se, pos)` or `ccSeOffLoop(se, id)` on the id the
    /// boss keeps ([`LoopOp`]).
    SeLoop {
        se: i32,
        op: LoopOp,
    },
    /// What Magus shows beside ([`magus::Pic`]).
    Magus(magus::Pic),
    /// What Fidchell shows or asks beside ([`fidchell::Pic`]).
    Fidchell(fidchell::Pic),
    /// What Gorre's brothers show beside their bodies ([`gorre::Pic`]).
    Gorre(gorre::Pic),
    /// `OnCinemaMode` with a skill's name (OUT gcmn 0x004733b0): the bars
    /// and the name `sid`'s row gives (OUT 0x00472550).
    CinemaSkill(i32),
    /// `bossCam->MaxRenge` (+0xd8) set.
    CamMaxRange(F),
    /// `OffBossCamera()` (OUT gcmn 0x004700a0) while `useBossCam`: camera 1
    /// takes the boss camera's eye, view and turn, `changeCamera(1)`, the
    /// boss camera off.
    BossCamOff,
    /// `bossCam->CamPos` (+0x20) written straight (Kyvia's fourth fight's
    /// scene).
    CamEye(V4),
    /// `EVENTAREAB8`'s `body[n]` hit off (`HitDisable`) or on, on field 12
    /// (QUA gcmn 0x0030c620, 0x0030c690).
    FieldBodyHit {
        n: usize,
        on: bool,
    },
    /// `ccMenu` +0x238 `dummyTarget` (Quarantine's): the target whose HP
    /// the gauge shows.
    DummyTarget(Option<usize>),
    /// `EntryAfterImage(&t)`: an after-image of the boss's pose in its own
    /// params (Macha's white ones).
    AfterImageWith(AfterImageParams),
    /// What Macha shows beside its body ([`macha::Pic`]).
    Macha(macha::Pic),
    /// What Tarvos and its effects show ([`tarvos::Pic`]).
    Tarvos(tarvos::Pic),
    /// `ccMenu->noiz->SetNoizBs(bs)`: the screen broken up for `bs`
    /// frames (Tarvos's magic, a leftover register's 20).
    Noise(i32),
    /// What Corbenik, its seeds and its effects show ([`corbenik::Pic`]).
    Corbenik(corbenik::Pic),
    /// `activeCamPtr->type` (+0x5c) set.
    CameraType(i32),
    /// `bossCam->lock` (+0x00) set.
    CamLock(bool),
    /// `bossCam->CamView` (+0x10) written straight (Corbenik's star dust
    /// while the camera is free).
    CamLook(V4),
    /// `cameraSetRot(rot, cam)`.
    CameraRot {
        cam: i32,
        rot: V4,
    },
    /// `ccMenu->panelStatus` (+0x0c) set.
    PanelStatus(i16),
    /// `g_sinRot` (`ccBoss06BlurParamFuncCB`'s phase) set.
    BlurSinRot(F),
}

/// `ccBoss`, with `ccBoss01`'s members (Skeith's); another class's own
/// state is in [`Boss::class`]. Floats are bits; positions `[x, y, z, w]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Boss {
    // ccBoss
    /// `actAnmTbl` (+0x1a8): the act's clip, by act (none for some).
    pub anm_tbl: Vec<Option<String>>,
    pub class: Class,
    pub target_pos: V4,
    pub target_pos_p: V4,
    pub target_dist: F,
    pub target_dirc: F,
    pub move_spd: F,
    pub move_dirc: F,
    pub move_vector: V4,
    pub stop_count: i16,
    pub act_num: i16,
    pub act_proccess: i16,
    pub act_count: i16,
    /// `ccChar.dirc` (+0x60): the heading, z the yaw.
    pub dirc: V4,
    pub act_forbid: i8,
    pub anm_status: i8,
    pub stop: i8,
    pub cheat_hp: i8,
    pub exit: i8,
    pub draw_sw: i8,
    pub body_hit_sw: i8,
    pub lock_player: i8,
    pub epitaph: i8,
    pub menu_forbid: i32,
    pub center_pos: V4,
    pub center_pos_p: V4,
    pub center_dist: F,
    pub center_dirc: F,
    pub pat_tbl: Tbl,
    pub pat_index: i32,
    pub pat_num: i32,
    pub wait_pat_num: i32,
    pub dash_pos: V4,
    pub dash_prev_pos: V4,
    pub erase_target: i32,
    pub body_hit_on_erase: i32,
    pub stop_time: i32,
    pub stop_counter: i32,
    pub reserve_forbid_menu: i32,
    pub reserve_forbid_chat_except: i32,
    /// `useStageEff`: the stage has a fader (`InitStageEffect`).
    pub use_stage_eff: bool,
    // ccBoss01
    pub eff_force: Option<i32>,
    pub eff_ice_break: Option<i32>,
    pub eff_dead: Option<i32>,
    pub animate: Fade,
    pub animate_dead: Fade,
    pub stage_eff_id: i32,
    pub pat_mode: i32,
    pub m_act_dirc: V4,
    /// `transparency` (+0x88) and `setTransparency` (+0x8c) of the
    /// `ccChar`: what the draw blends to.
    pub transparency: F,
    pub set_transparency: F,
    pub anm: Anm,
    pub anm_wave: Anm,
    pub effects: Effects,
    /// What an affect asked for between frames ([`entry`]), for the
    /// runtime to take ([`Boss::take_pending`]).
    pub pending: Vec<Out>,
    /// Affects that reached the boss from a frame with no [`BossEnv`]
    /// (Kite's or a member's): type, params and who, in order, for
    /// [`apply_queued`] at the start of the boss's own task.
    pub queued: Vec<(i16, [i16; 3], Option<usize>)>,
}

/// The rest of the game the rules read and write: the scene (the boss is
/// `scene.chars[me]`), the party (`ccPartyManager.memberChar`), the
/// player's frame, both generators and what a frame knows.
pub struct Cx<'a> {
    pub t: &'a Tables,
    pub data: &'a BossData,
    pub scene: &'a mut Scene,
    pub party: &'a Party,
    pub world: World<'a>,
    pub env: &'a Env,
    pub actx: &'a AffectCtx<'a>,
    /// newlib's `rand()`.
    pub rand: &'a mut dyn Rng,
    /// `ccRand()`.
    pub cc: &'a mut dyn Rng,
    /// A clip's (frames, looping) in the boss's files (`x11`, `xeffect`).
    pub clips: &'a dyn Fn(&str) -> Option<(u32, bool)>,
    /// `ccCharHit::CollisionDetection` of the body hit at `pos`: the push
    /// out of whatever it overlaps.
    pub collide: &'a mut dyn FnMut(V4) -> Option<V4>,
    /// The collision list as the boss's own parts reach it ([`PartHits`]).
    pub part_hits: &'a mut dyn PartHits,
    /// `compulsionGameOver`.
    pub game_over: bool,
    /// `bossCam` is set: `InitBossCamera` made the camera, so `QuakeCam`
    /// runs and draws from `ccRand`.
    pub boss_cam: bool,
    /// What the cameras show this frame ([`CamView`]).
    pub cam: CamView,
    /// `ccLandHitCheck(pos, 0x20000000)`: the ground's height under `pos`.
    pub land: &'a mut dyn FnMut(V4) -> F,
    /// `EVENTAREAB8`'s disc as Kyvia reads it ([`DiscView`]).
    pub disc: DiscView,
    /// `ccCheckParticleGenerator(g)` of a generator the rules made and
    /// keyed ([`Out::Corbenik`]'s `Gen`): still on the particle system's
    /// list (its life and its particles' run out on their own).
    pub gen_alive: &'a dyn Fn(u32) -> bool,
    pub me: usize,
    pub out: Vec<Out>,
    pub ev: Events,
}

/// A boss's looping sound effect ([`Out::SeLoop`]): started at a place,
/// its id kept; or ended, if the kept id is not -1, and with `forget` the
/// id set -1 after.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopOp {
    On(V4),
    Off { forget: bool },
}

/// The collision list (`ccCharHitTop`, main 0x0037890c) as a boss's own
/// parts reach it, each `bodyHit` by its part's character (Kyvia's
/// gomoras, `kyviaGomora::Move`).
pub trait PartHits {
    /// `ccCharHit::HitEnable` (`on`) or `HitDisable`: `hit` into the list
    /// at its tail, or out of it; `hit.sw` follows.
    fn switch(&mut self, who: usize, hit: &mut CharHit, on: bool);
    /// `ccCharHit::CollisionDetection` of the bodies (`mask2` 0): the
    /// list's copy made `hit`, then `hit.offset` the push out of the
    /// others ([`crate::world::char_push`]); true when one was touched.
    fn collide(&mut self, who: usize, hit: &mut CharHit) -> bool;
    /// The list's copy of `who`'s body made `hit` (its owner moved it).
    fn sync(&mut self, who: usize, hit: &CharHit);
    /// `ccHitCheckLM(sp, ep, mask)` on the land: where the segment first
    /// meets it (Macha's lasers). None without land.
    fn line(&mut self, _sp: V4, _ep: V4, _mask: u32) -> Option<V4> {
        None
    }
}

/// No list: a switch only sets `sw`, and nothing is ever touched.
pub struct NoHits;

impl PartHits for NoHits {
    fn switch(&mut self, _: usize, hit: &mut CharHit, on: bool) {
        hit.sw = on;
    }

    fn collide(&mut self, _: usize, hit: &mut CharHit) -> bool {
        hit.offset = [0; 4];
        false
    }

    fn sync(&mut self, _: usize, _: &CharHit) {}
}

/// The cameras as a boss's rules read them: `cameraGetRot(camID)` (the
/// active camera's turn), `cameraGetPos(2)`, `cameraGetView(2)` (the boss
/// camera's eye and view), `bossCam->CheckMoveCamera()`, the boss
/// camera's `ResetFlg` (+0x04: 0 once a mode 3 or 4 move is done),
/// `checkCameraShakeRange` of the boss's place as the frame starts, and
/// `cameraGetPos(camID)`, `cameraGetView(camID)` (Kyvia's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CamView {
    pub rot: V4,
    pub pos: V4,
    pub view: V4,
    pub moving: bool,
    pub reset: i32,
    pub shake: bool,
    /// The boss camera's own eye (`CamPos`, +0x20), which Kyvia's fourth
    /// fight reads and moves.
    pub eye: V4,
    /// The active camera's eye and view: in Kyvia's fights always the boss
    /// camera's, as nothing reachable there changes `camID` from 2
    /// (docs/engine/boss-kyvia-second.md "Which camera").
    pub active_pos: V4,
    pub active_view: V4,
    /// The boss camera's own view (`CamView`, +0x10), which Corbenik's
    /// second form reads and moves.
    pub boss_view: V4,
    /// What `ccBoss::Draw`'s camera fade reads, None for no fade.
    pub fade: Option<CamFade>,
}

/// `ccBoss::Draw`'s camera fade (`ccGetCameraTransparency`): the active
/// camera's eye (`activeCamPtr->pos`) and pitch (`->deg[1]`), whether it is
/// the eye view (`checkCameraType() == 1`: no fade), and the far end and
/// fade `game.area` picks (a town 4000 and 400, else 7000 and 600).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CamFade {
    pub eye: V4,
    pub deg1: i16,
    pub eye_view: bool,
    pub far: F,
    pub fade: F,
}

/// `WORLD_MAN.eventmap` as an `EVENTAREAB8` for Kyvia's rules: `IsMove()`,
/// `discPrevPos` (+0xca0), `DMY_marker01`'s place in its file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiscView {
    pub moving: bool,
    pub prev_pos: V4,
    pub marker: V4,
    /// The effect manager's blur colour (`ccBossBlur` +0x1c) as the boss's
    /// task starts, and `scFadeDef`'s four elements (`CheckFade`, the free
    /// one `EntryFlash3` takes): the fourth fight reads both.
    pub blur_abgr: u32,
    pub blur_scale: F,
    /// The blur's `m_exit` (+0x28) as the boss's task starts (Macha's
    /// `Transfer` waits on it).
    pub blur_exit: u32,
    pub fades: [FadeView; 4],
}

/// A `ccScFadeElement`'s `status`, `cnt` and `tcnt` (main fade.cpp).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FadeView {
    pub status: i16,
    pub cnt: i16,
    pub tcnt: i16,
}

impl Cx<'_> {
    fn out(&mut self, o: Out) {
        self.out.push(o);
    }

    /// `if (bossCam) bossCam->QuakeCam((q, q, q, 1))` (0x0045ff80): three
    /// `ccRandF(q)` for x, y and z.
    fn quake_cam(&mut self, q: F) {
        if self.boss_cam {
            let v = std::array::from_fn(|_| rand_f(self.cc, q));
            self.out(Out::Quake(v));
        }
    }

    /// `bossCam->QuakeCam(q)` (0x0045ff80) where the caller has no
    /// `if (bossCam)`: Innis's fight always has the camera.
    fn quake_vec(&mut self, q: [F; 3]) {
        let v = std::array::from_fn(|k| rand_f(self.cc, q[k]));
        self.out(Out::Quake(v));
    }

    fn pos(&self) -> V4 {
        self.scene.chars[self.me].pos
    }

    fn se3d(&mut self, se: i32) {
        let pos = self.pos();
        self.out(Out::Se3d { se, pos });
    }

    /// `ccCheckTarget(c)`: on a command list.
    fn valid(&self, c: Option<usize>) -> bool {
        c.is_some_and(|c| self.scene.listed(c))
    }

    fn dead(&self, c: usize) -> bool {
        self.scene.chars[c].cond[cond::DEAD] != 0
    }

    fn annihilated(&self) -> bool {
        self.party.annihilated(self.scene)
    }

    /// `c->EntryAffect(this, kind, p0, 0, 0)`.
    fn affect(&mut self, on: usize, kind: i16, p0: i16) {
        let me = self.me;
        let mut ev = Events::new();
        affect::entry_affect(self.t, self.scene, self.actx, on, Some(me), kind, [p0, 0, 0], self.rand, &mut ev);
        self.ev.extend(ev);
    }
}

fn w2p(cx: &Cx, v: V4) -> V4 {
    cx.world.frame.w2p(v)
}

fn p2w(cx: &Cx, v: V4) -> V4 {
    cx.world.frame.p2w(v)
}

/// The square root `ccGetDist` and `_ccBossSkillDamage` take: newlib's
/// `sqrtf` on Infection and Mutation, the FPU's `sqrt.s` (truncating)
/// inline on Outbreak and Quarantine (OUT main 0x001e6ec0).
fn sqrt_of(cx: &Cx, v: F) -> F {
    geom::sqrt_on(cx.actx.volume, v)
}

/// `ccGetDist(a, b)` as the volume's main has it: its square root is
/// [`sqrt_of`]'s.
fn dist(cx: &Cx, a: V4, b: V4) -> F {
    crate::enemy_ai::get_dist_on(cx.actx.volume, a, b)
}

impl Boss {
    /// The constructors (`ccBoss::ccBoss` 0x0045bcf0, `ccBoss01::ccBoss01`
    /// 0x0047b410) as far as the rules go: Skeith stands 500 behind Kite
    /// (`plw.pos` less 500 on y, Kite's heading), its break-proof HP on,
    /// drawn and hit, in act 0 (its animation set), on the command lists,
    /// the Normal patterns set (the first run's index discarded: the first
    /// `ChangeNextPattern` runs pattern 0 again). `centerPos` is the event
    /// area's `DMY_center01` (`center`, world).
    pub fn new(cx: &mut Cx, kite_pos: V4, kite_dirc: V4, center: V4) -> Boss {
        let mut b = Boss {
            anm_tbl: cx.data.skeith.anm_tbl.clone(),
            wait_pat_num: -2,
            pat_num: 13,
            stage_eff_id: -1,
            // InitStageEffect: the stage fader and its layer made.
            use_stage_eff: true,
            transparency: ONE,
            set_transparency: ONE,
            anm: Anm::new(),
            anm_wave: Anm::new(),
            // the ccAnimateObjects start opaque
            animate: Fade { transparency: ONE, ..Fade::default() },
            animate_dead: Fade { transparency: ONE, ..Fade::default() },
            ..Boss::default()
        };
        // ccBoss01's On/Off: exit off, drawn, body hit on, cheat HP on.
        b.exit = 0;
        b.draw_sw = 1;
        b.body_hit_sw = 1;
        b.cheat_hp = 1;
        let me = cx.me;
        let ch = &mut cx.scene.chars[me];
        let mut pos = kite_pos;
        pos[1] = ee::sub(pos[1], 0x43fa_0000);
        ch.pos = pos;
        b.dirc = kite_dirc;
        b.change_action(cx, act::NEUTRAL, 0, true);
        crate::fellow::entry_cmnd(cx.scene, me);
        b.stop_time = 0;
        b.stop_counter = 0;
        b.set_pattern_tbl(Tbl::Normal);
        let _ = b.exec_pattern_index(cx, Tbl::Normal.words(&cx.data.skeith).to_vec().as_slice(), 0);
        b.pat_mode = 0;
        b.center_pos = [center[0], center[1], center[2], ONE];
        b.center_pos_p = w2p(cx, b.center_pos);
        // The blur on under `ccBoss03BlurParamCB` (INF gcmn 0x0047bc40).
        cx.out(Out::Blur(blur::Set::Enable));
        cx.out(Out::Blur(blur::Set::Func(blur::BlurFunc::Boss03)));
        b
    }

    // --- the frame ----------------------------------------------------------------------

    /// The boss task's frame: the effect manager's pass (its task runs
    /// first in the frame), then the class's `Main`.
    /// `ccBossEffManager::Draw`'s pass over the effects whose `Draw` the
    /// class's rules keep (Kyvia's meteors, bolts and ball), before the
    /// blur's `Main` and the boss's task. The other classes' effects run
    /// in their `main`.
    pub fn effects_pass(&mut self, cx: &mut Cx) {
        if let Class::Kyvia(_) = self.class {
            kyvia::effects_pass(self, cx);
        }
    }

    pub fn main(&mut self, cx: &mut Cx) {
        match self.class {
            Class::Innis(_) => innis::main(self, cx),
            Class::Magus(_) => magus::main(self, cx),
            Class::Kyvia(_) => kyvia::main(self, cx),
            Class::Fidchell(_) => fidchell::main(self, cx),
            Class::Macha(_) => macha::main(self, cx),
            Class::Tarvos(_) => tarvos::main(self, cx),
            Class::Corbenik(_) => corbenik::first::main(self, cx),
            Class::CorbenikB(_) => corbenik::second::main(self, cx),
            Class::CorbenikC(_) => corbenik::third::main(self, cx),
            Class::Gorre(_) => gorre::main(self, cx),
            _ => self.skeith_main(cx),
        }
    }

    /// `ccBoss01::Main` (gcmn 0x0047bd50), with the manager's pass before.
    fn skeith_main(&mut self, cx: &mut Cx) {
        self.effects.tick();
        self.base_main(cx, &mut |b, cx| {
            b.think(cx);
            b.action(cx);
        });
        let me = cx.me;
        cx.scene.chars[me].cond[cond::HOLD] = 0;
        // DrawCross (0x0047c390): in the cross it finds the sword's dummies
        // once (proccess 0 to 1) and draws its trail (the runtime's).
        if self.act_num == act::CROSS && self.act_proccess == 0 {
            self.act_proccess = 1;
        }
        if (self.lock_player != 0 || self.erase_target != 0)
            && self.epitaph == 0
            && let Some(p) = cx.scene.param_mut(me)
            && p.pp_count > 0
        {
            p.pp_count += 1;
        }
    }

    /// `ccChar::CalcReal(0)` (gcmn 0x0056ba30) on the boss: what it raises
    /// on itself (the protect's return and its picture, the condition
    /// effect) named by its index. The ticks it enters on itself
    /// (`EntryAffect(this, this, 9 | 3 | 10 | 4)`) are queued for its
    /// `Affect`, which runs them at the start of its next task.
    pub(crate) fn calc_real(&mut self, cx: &mut Cx) {
        let me = cx.me;
        let mut ev = Events::new();
        // On the block its `param` points at (Gorre's, for a brother after
        // the drain's 21).
        let (t, env) = (cx.t, cx.env);
        cx.scene.lend(me, |ch| chara::calc_real(t, ch, 0, env, &mut ev));
        for mut e in ev {
            e.map_who(|w| if w == Who::Me { Who::Char(me) } else { w });
            match e {
                Event::Affect { on: Who::Char(c), kind, p, .. } if c == me => self.queued.push((kind, p, Some(me))),
                e => cx.ev.push(e),
            }
        }
    }

    /// `ccBoss::Main` (0x0045c270), the class's `Think` and `Action` in
    /// `class`.
    pub(crate) fn base_main(&mut self, cx: &mut Cx, class: &mut dyn FnMut(&mut Boss, &mut Cx)) {
        if self.exit != 0 {
            return;
        }
        let me = cx.me;
        self.calc_real(cx);
        self.calc_target_info(cx);
        self.center_pos_p = w2p(cx, self.center_pos);
        let pp = cx.scene.chars[me].pos_p;
        self.center_dist = dist(cx, pp, self.center_pos_p);
        self.center_dirc = get_dirc(pp, self.center_pos_p);
        if self.stop != 0 {
            self.stop_count += 1;
            if self.stop_count >= 61 {
                self.stop_count = 0;
                self.stop = 0;
            }
            self.move_spd = 0;
        } else {
            self.stop_count = 0;
        }
        class(self, cx);
        if self.lock_player != 0 {
            for m in cx.party.members.into_iter().flatten() {
                if cx.valid(Some(m)) {
                    cx.affect(m, 5, 32767);
                }
            }
            if self.reserve_forbid_menu == 1 && cx.env.menu_forbid == 0 && cx.env.menu_type == -1 {
                cx.out(Out::MenuForbid { on: true, chat_except: self.reserve_forbid_chat_except != 0 });
                self.reserve_forbid_menu = 0;
                self.reserve_forbid_chat_except = 0;
            }
        } else if self.reserve_forbid_menu == -1 && cx.env.menu_type == -1 {
            cx.out(Out::MenuForbid { on: false, chat_except: false });
            self.reserve_forbid_menu = 0;
            self.reserve_forbid_chat_except = 0;
        }
        self.move_(cx);
        if self.draw_sw != 0 {
            self.pre_draw_anm(cx);
        }
    }

    /// `ccBoss::Move` (0x0045c600).
    fn move_(&mut self, cx: &mut Cx) {
        let me = cx.me;
        let mut pp = w2p(cx, cx.scene.chars[me].pos);
        if !geom::eq(0, self.move_spd) {
            let s = ee::mul(self.move_spd, libm::sinf(self.move_dirc));
            pp[0] = ee::add(pp[0], s);
            let c = ee::mul(self.move_spd, libm::cosf(self.move_dirc));
            pp[1] = ee::sub(pp[1], c);
        }
        pp = geom::vadd(pp, self.move_vector);
        let mut pos = p2w(cx, pp);
        if self.body_hit_sw != 0
            && let Some(mut off) = (cx.collide)(pos)
        {
            off[2] = 0;
            pos = geom::vadd(pos, off);
            pp = geom::vadd(pp, off);
        }
        let ch = &mut cx.scene.chars[me];
        ch.pos_p = pp;
        ch.pos = pos;
    }

    /// `ccBoss01::PreDrawAnm` (0x0047c340) and `ccBoss::PreDrawAnm`
    /// (0x0045d440): the wave's speed 512 in its acts; the boss's
    /// animation forward, its end in `anmStatus`.
    fn pre_draw_anm(&mut self, _cx: &mut Cx) {
        if self.class == Class::Skeith && (self.act_num == act::WAVE || self.act_num == act::EPITAPH_WAVE) {
            self.anm_wave.speed = 512;
        }
        self.anm_status = i8::from(self.anm.forward());
    }

    // --- actions --------------------------------------------------------------------------

    /// `ccBoss::ChangeAction(n, forbid, af)` (0x0045d310): `af` nonzero
    /// sets the act's animation.
    fn change_action(&mut self, cx: &mut Cx, n: i16, forbid: i16, af: bool) {
        let ok = match forbid {
            0 => self.act_forbid == 0,
            1 => {
                self.act_forbid = 0;
                true
            }
            2 | 3 => {
                self.act_forbid = forbid as i8;
                true
            }
            _ => false,
        };
        if !ok {
            return;
        }
        self.act_num = n;
        self.act_proccess = 0;
        self.act_count = 0;
        if af && let Some(Some(clip)) = usize::try_from(n).ok().and_then(|k| self.anm_tbl.get(k)) {
            let clip = clip.clone();
            self.anm.set(&clip, cx.clips);
        }
        if self.epitaph == 0 && self.act_num == act::EPITAPH_NEUTRAL {
            self.epitaph = 1;
        }
    }

    fn set_pattern_tbl(&mut self, t: Tbl) {
        self.pat_tbl = t;
        self.pat_index = 0;
    }

    /// `ccBoss::ChangeNextPattern` (0x0045d210).
    fn change_next_pattern(&mut self, cx: &mut Cx) {
        let tbl = self.pat_tbl.words(&cx.data.skeith).to_vec();
        if tbl.get(self.pat_index as usize) == Some(&-1) {
            self.pat_index = 0;
        }
        self.move_spd = 0;
        self.move_vector = [0; 4];
        if self.epitaph != 0 {
            self.change_action(cx, act::EPITAPH_NEUTRAL, 3, true);
        } else {
            self.change_action(cx, act::NEUTRAL, 1, true);
        }
        self.pat_index = self.exec_pattern_index(cx, &tbl, self.pat_index);
    }

    /// `ccBoss::ExecPattern(pat, p0, p1, p2)` (0x0045d1a0).
    fn exec_pattern(&mut self, cx: &mut Cx, pat: i32, p0: i32) {
        let tbl = [pat, p0, 0, 0, -1];
        self.exec_pattern_index(cx, &tbl, 0);
    }

    fn neutral_act(&mut self, cx: &mut Cx, epitaph_forbid: i16, forbid: i16) {
        if self.epitaph != 0 {
            self.change_action(cx, act::EPITAPH_NEUTRAL, epitaph_forbid, true);
        } else {
            self.change_action(cx, act::NEUTRAL, forbid, true);
        }
    }

    /// `ccBoss01::ExecPatternIndex(tbl, i)` (0x0047c500), falling back on
    /// `ccBoss::ExecPatternIndex` (0x0045cad0): the next index.
    fn exec_pattern_index(&mut self, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
        let word = |k: i32| usize::try_from(k).ok().and_then(|k| tbl.get(k)).copied().unwrap_or(0);
        let mut orig = i;
        let mut i = i;
        let mut pat = word(i);
        i += 1;
        if pat == -1 {
            i = 1;
            orig = 1;
            pat = word(0);
        }
        self.pat_num = pat;
        match pat {
            2 => {
                let t = word(i);
                i += 1;
                self.stop_time = t;
                self.stop_counter = 0;
                if t == -1 && !cx.annihilated() {
                    self.stop_time = 30;
                    self.stop_counter = 0;
                }
                self.neutral_act(cx, 3, 1);
            }
            1 => {
                if self.epitaph != 0 {
                    self.change_action(cx, act::EPITAPH_WAVE, 3, true);
                } else {
                    self.change_action(cx, act::WAVE, 3, true);
                }
            }
            14 => self.change_action(cx, act::CROSS, 3, true),
            15 => self.change_action(cx, act::DRAIN, 3, true),
            16 => self.change_action(cx, act::MAGIC, 3, true),
            9..=11 => {
                let me = cx.me;
                let mut target = cx.scene.chars[me].target_char;
                if pat != 11 {
                    let ty = word(i);
                    i += 1;
                    target = self.select_target(cx, ty, FAR);
                    cx.scene.chars[me].target_char = target;
                }
                if !cx.valid(target) || target.is_some_and(|t| cx.dead(t)) {
                    target = self.select_target(cx, 3, FAR);
                    cx.scene.chars[me].target_char = target;
                    if !cx.valid(target) || target.is_some_and(|t| cx.dead(t)) {
                        self.exec_pattern(cx, 2, 30);
                        return orig;
                    }
                }
                self.calc_target_info(cx);
                let (on, sid) = if pat == 10 {
                    let k = (cx.cc.rand() % 3).unsigned_abs() as usize;
                    (target, cx.data.skeith.rand_skills[k])
                } else {
                    let sid = word(i);
                    i += 1;
                    (if pat == 9 { target } else { Some(me) }, sid)
                };
                if let Some(tp) = on
                    && let Some(s) = item::item_skill_request(cx.t, cx.scene, me, tp, sid, 0, false, cx.rand)
                {
                    cx.out(Out::Skill(tp, s));
                }
                self.exec_pattern(cx, 2, -2);
            }
            _ => return self.base_exec_pattern_index(cx, tbl, orig),
        }
        i
    }

    /// `ccBoss::ExecPatternIndex` (0x0045cad0).
    fn base_exec_pattern_index(&mut self, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
        let word = |k: i32| usize::try_from(k).ok().and_then(|k| tbl.get(k)).copied().unwrap_or(0);
        let orig = i;
        let mut i = i;
        let mut pat = word(i);
        i += 1;
        self.move_spd = 0;
        self.move_vector = [0; 4];
        if pat == -1 {
            pat = word(0);
            i = 1;
        }
        self.pat_num = pat;
        let me = cx.me;
        match pat {
            2 => {
                let mut t = word(i);
                i += 1;
                if t == -1 && !cx.annihilated() {
                    t = 30;
                }
                self.stop_time = t;
                self.stop_counter = 0;
                self.neutral_act(cx, 3, 3);
            }
            3 => self.change_action(cx, act::ESCAPE, 3, true),
            4 => self.change_action(cx, act::CHASE, 3, true),
            5 => self.change_action(cx, act::RETURN, 3, true),
            7 => self.change_action(cx, act::WANDER, 3, true),
            6 => {
                let d = ee::from_int(word(i));
                i += 1;
                let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), self.center_dirc), NEG_HALF_PI);
                let v = geom::apply_matrix(&m, [d, 0, 0, 0]);
                let p = geom::vadd(v, cx.scene.chars[me].pos_p);
                self.dash_pos = p2w(cx, p);
                self.dash_prev_pos = cx.scene.chars[me].pos;
                self.change_action(cx, act::DASH, 3, true);
            }
            8 => self.neutral_act(cx, 3, 3),
            9 | 11 => {
                let ty = word(i);
                i += 1;
                let sid = word(i);
                i += 1;
                if pat == 9 {
                    let t = self.select_target(cx, ty, FAR);
                    cx.scene.chars[me].target_char = t;
                    if !cx.valid(t) || t.is_some_and(|t| cx.dead(t)) {
                        self.exec_pattern(cx, 2, 30);
                        return orig;
                    }
                    self.calc_target_info(cx);
                }
                let on = if pat == 9 { cx.scene.chars[me].target_char } else { Some(me) };
                if let Some(tp) = on
                    && let Some(s) = item::item_skill_request(cx.t, cx.scene, me, tp, sid, 0, false, cx.rand)
                {
                    cx.out(Out::Skill(tp, s));
                }
                self.exec_pattern(cx, 2, 30);
            }
            12 => {
                let sid = word(i);
                i += 1;
                let members = cx.party.members;
                let mut last = None;
                for m in members.into_iter().flatten() {
                    if cx.valid(Some(m)) && !cx.dead(m) {
                        last = Some(m);
                    }
                }
                for m in members.into_iter().flatten() {
                    if Some(m) != last
                        && cx.valid(Some(m))
                        && !cx.dead(m)
                        && let Some(s) = item::item_skill_request(cx.t, cx.scene, me, m, sid, 0, false, cx.rand)
                    {
                        cx.out(Out::Skill(m, s));
                    }
                }
                if let Some(l) = last
                    && cx.valid(Some(l))
                    && !cx.dead(l)
                    && let Some(s) = item::item_skill_request(cx.t, cx.scene, me, l, sid, 1, false, cx.rand)
                {
                    cx.out(Out::Skill(l, s));
                }
                self.exec_pattern(cx, 2, -2);
            }
            _ => self.neutral_act(cx, 3, 1),
        }
        i
    }

    // --- targets ----------------------------------------------------------------------------

    /// `ccBoss::SelectTarget(type, radius)` (0x0045e070).
    fn select_target(&mut self, cx: &mut Cx, ty: i32, radius: F) -> Option<usize> {
        let me = cx.me;
        let pcs = cx.scene.pc_list.clone();
        if pcs.len() == 1 && cx.dead(pcs[0]) {
            self.target_dirc = self.dirc[2];
            self.move_dirc = self.dirc[2];
            return None;
        }
        match ty {
            0 => return search_near_person(cx.scene, &cx.world, cx.actx.volume, me, 3, 0, radius),
            1 => {
                let mut best = 0x0080_0000;
                let mut pick = None;
                let pp = cx.scene.chars[me].pos_p;
                for &c in &pcs {
                    if cx.dead(c) {
                        continue;
                    }
                    let d = dist(cx, pp, cx.scene.chars[c].pos_p);
                    if ee::lt(best, d) {
                        best = d;
                        pick = Some(c);
                    }
                }
                return pick;
            }
            _ => {}
        }
        // Types 2-14 compare the members alive (jump table @2063, INF
        // gcmn 0x006aa3d0): `a0` the most, `v1` the least of HP (2, 3),
        // `real`'s physical attack (4, 5) and defence (6, 7), magic attack
        // (8, 9) and defence (10, 11); 12 the last with no condition, 13
        // the most conditions; 14 counts them as 13 does but against the
        // least's 32767, and never picks.
        let (mut hi, mut lo) = (i32::from(i16::MIN), i32::from(i16::MAX));
        let (mut a0, mut v1) = (None, None);
        for &c in &pcs {
            let ch = &cx.scene.chars[c];
            if ch.hp <= 0 {
                continue;
            }
            let real = cx.scene.real(c);
            let field = |k: usize| i32::from(real[k]);
            let conds = || {
                let v = &ch.cond.v;
                let slow = v[15] != 0 && ee::lt(ch.cond.speed_value, ONE);
                [v[8], v[9], v[10], v[11], v[13], v[14]].iter().filter(|&&x| x != 0).count() as i32 + i32::from(slow)
            };
            let (most, v) = match ty {
                2 => (true, i32::from(ch.hp)),
                3 => (false, i32::from(ch.hp)),
                4 => (true, field(crate::param::elm::P_ATK)),
                5 => (false, field(crate::param::elm::P_ATK)),
                6 => (true, field(crate::param::elm::P_DEF)),
                7 => (false, field(crate::param::elm::P_DEF)),
                8 => (true, field(crate::param::elm::M_ATK)),
                9 => (false, field(crate::param::elm::M_ATK)),
                10 => (true, field(crate::param::elm::M_DEF)),
                11 => (false, field(crate::param::elm::M_DEF)),
                12 => {
                    let v = &ch.cond.v;
                    if [v[8], v[9], v[10], v[11], v[13], v[14], v[15]].iter().all(|&x| x == 0) {
                        v1 = Some(c);
                    }
                    continue;
                }
                13 => (true, conds()),
                _ => continue,
            };
            if most && hi < v {
                hi = v;
                a0 = Some(c);
            } else if !most && v < lo {
                lo = v;
                v1 = Some(c);
            }
        }
        let pick = v1.or(a0)?;
        if !geom::eq(0, radius) {
            let p = w2p(cx, cx.scene.chars[me].pos);
            if !ee::le(dist(cx, p, cx.scene.chars[pick].pos_p), radius) {
                return None;
            }
        }
        Some(pick)
    }

    /// `ccBoss::CalcTargetInfo` (0x0045ec60).
    fn calc_target_info(&mut self, cx: &mut Cx) {
        let me = cx.me;
        let Some(t) = cx.scene.chars[me].target_char.filter(|&t| cx.scene.listed(t)) else { return };
        self.target_pos_p = cx.scene.chars[t].pos_p;
        self.target_pos = cx.scene.chars[t].pos;
        let pp = cx.scene.chars[me].pos_p;
        self.target_dirc = get_dirc(pp, self.target_pos_p);
        self.target_dist = dist(cx, pp, self.target_pos_p);
    }

    /// `ccBoss::Draw`'s transparency (QUA gcmn 0x00361040): `setTransparency`
    /// times the camera's fade (`transDist` is set on every boss), or as it
    /// is in the eye view; whether the clip is drawn (0.05 or more, or the
    /// camera inside it).
    pub(crate) fn draw_fade(&mut self, cx: &Cx, pos: V4) -> bool {
        let me = cx.me;
        let (w, h) = (cx.scene.chars[me].base().width, cx.scene.chars[me].base().height);
        let (t, hide) = match cx.cam.fade {
            Some(f) if !f.eye_view => {
                let mut hide = false;
                let k = geom::camera_transparency(
                    cx.actx.volume,
                    w2p(cx, pos),
                    w2p(cx, f.eye),
                    f.deg1,
                    w,
                    h,
                    f.far,
                    f.fade,
                    &mut hide,
                );
                (ee::mul(self.set_transparency, k), hide)
            }
            _ => (self.set_transparency, false),
        };
        self.transparency = t;
        hide || !ee::lt(t, 0x3d4c_cccd)
    }

    /// `ccBoss::EraseCmndTarget` (0x0045df40): the boss "dead" to the
    /// party's targeting while it runs off or dashes.
    fn erase_cmnd_target(&mut self, cx: &mut Cx) {
        let me = cx.me;
        if cx.scene.chars[me].cond[cond::DEAD] == 0 && cx.scene.listed(me) {
            cx.scene.chars[me].cond[cond::DEAD] = 1;
            self.body_hit_on_erase = i32::from(self.body_hit_sw);
            self.body_hit_sw = 0;
            self.erase_target = 1;
        }
    }

    /// `ccBoss::EntryCmndTarget` (0x0045dfc0).
    fn entry_cmnd_target(&mut self, cx: &mut Cx) {
        let me = cx.me;
        if self.exit == 0 && cx.scene.listed(me) {
            cx.scene.chars[me].cond[cond::DEAD] = 0;
            if self.body_hit_on_erase != 0 {
                self.body_hit_sw = 1;
                self.body_hit_on_erase = 0;
            }
            self.erase_target = 0;
        }
    }

    /// `ccBoss::BeginStageEffect(rgba, t0, t1, t2)` (0x0045e8d0): the
    /// stage fader's element (the first, 0), or -1 without a fader.
    fn begin_stage_effect(&mut self, cx: &mut Cx, rgba: u32, t: [i32; 3]) -> i32 {
        if !self.use_stage_eff {
            return -1;
        }
        cx.out(Out::StageBegin { rgba, t });
        0
    }

    /// `ccBoss::EndStageEffect(id, rgba, t)` (0x0045e9b0), for an element
    /// 0-3 (the runtime checks it still runs).
    fn end_stage_effect(&mut self, cx: &mut Cx, id: i32, rgba: u32, t: i32) {
        if self.use_stage_eff && (0..4).contains(&id) {
            cx.out(Out::StageEnd { rgba, t });
        }
    }

    fn lock_player(&mut self, cx: &Cx, chat_except: bool) {
        self.menu_forbid = i32::from(cx.env.menu_forbid);
        self.lock_player = 1;
        self.reserve_forbid_menu = 1;
        self.reserve_forbid_chat_except = i32::from(chat_except);
    }

    fn unlock_player(&mut self) {
        self.lock_player = 0;
        self.reserve_forbid_menu = -1;
    }

    fn spd_up(&mut self, ms: F, rate: F) {
        if ee::lt(self.move_spd, ms) {
            self.move_spd = ee::add(self.move_spd, rate);
            if !ee::lt(self.move_spd, ms) {
                self.move_spd = ms;
            }
        }
    }

    fn spd_down(&mut self, ms: F, rate: F) {
        if !ee::le(self.move_spd, ms) {
            self.move_spd = ee::sub(self.move_spd, rate);
            if ee::le(self.move_spd, ms) {
                self.move_spd = ms;
            }
        }
    }

    fn set_dirc(&mut self, to: F) {
        self.dirc[2] = geom::set_dirc(self.dirc[2], to, 256);
    }

    /// `IsValidArea(p)` (0x0047ca00): within 1500 of the centre.
    fn valid_area(&self, cx: &Cx, p: V4) -> bool {
        ee::lt(dist(cx, p, self.center_pos_p), 0x44bb_8000)
    }

    /// Skeith's odd wrap: above pi a turn off, below pi a turn on.
    fn wrap(v: F) -> F {
        if !ee::le(v, PI) {
            ee::sub(v, TWO_PI)
        } else if ee::lt(v, PI) {
            ee::add(v, TWO_PI)
        } else {
            v
        }
    }

    fn cursor_off(cx: &mut Cx, on: bool) {
        cx.out(Out::CursorOff(on));
    }

    // --- the damage ---------------------------------------------------------------------------

    fn skill(cx: &Cx, i: usize) -> SkillParam {
        cx.t.boss_skills.get(i).cloned().unwrap_or_default()
    }

    /// `ccBossSkillDamage(this, target | pos, i)` (0x0045f8b0,
    /// 0x0045f9f0): targetable while it hits.
    fn skill_damage(&mut self, cx: &mut Cx, target: Option<usize>, pos: Option<V4>, i: usize) -> i32 {
        let erased = self.erase_target;
        self.entry_cmnd_target(cx);
        let sk = Self::skill(cx, i);
        let n = match pos {
            Some(p) => self.damage_at(cx, p, &sk),
            None => self.damage_on(cx, target, &sk),
        };
        if erased != 0 {
            self.erase_cmnd_target(cx);
        }
        n
    }

    /// `ccBossSkillDamage(this, pos, &sk)` (MUT gcmn 0x00475140): as
    /// [`Boss::skill_damage`] at a place with a skill of the boss's own.
    fn skill_damage_with(&mut self, cx: &mut Cx, pos: V4, sk: &SkillParam) -> i32 {
        let erased = self.erase_target;
        self.entry_cmnd_target(cx);
        let n = self.damage_at(cx, pos, sk);
        if erased != 0 {
            self.erase_cmnd_target(cx);
        }
        n
    }

    fn hit(&mut self, cx: &mut Cx, m: usize, sk: &SkillParam, half: bool) {
        let me = cx.me;
        let att = cx.scene.lend(me, |c| c.clone());
        let listed = true;
        let (t, env) = (cx.t, cx.env);
        let (rand, ev) = (&mut *cx.rand, &mut cx.ev);
        let d = cx
            .scene
            .lend(m, |tgt| damage::calc_battle_damage(t, &att, tgt, sk, ONE, Roll::Draw, listed, rand, env, ev));
        let mut dmg = d.dmg as i16;
        if half {
            dmg = ee::to_int(ee::mul(ee::from_int(i32::from(dmg)), HALF)) as i16;
        }
        cx.affect(m, 1, dmg);
    }

    fn in_range(cx: &Cx, m: usize, c: V4, range: F) -> bool {
        let p = cx.scene.chars[m].pos_p;
        let v = [ee::sub(p[0], c[0]), ee::sub(p[1], c[1]), 0, ONE];
        let dd = geom::dot(v, v);
        let d = ee::sub(sqrt_of(cx, dd), cx.scene.chars[m].base().width);
        ee::le(d, range)
    }

    /// `_ccBossSkillDamage(this, target, sk)` (0x0045f0e0).
    fn damage_on(&mut self, cx: &mut Cx, target: Option<usize>, sk: &SkillParam) -> i32 {
        let me = cx.me;
        if !cx.scene.listed(me) || cx.scene.chars[me].cond[cond::DEAD] != 0 {
            return 0;
        }
        let half = sk.ty & 0x8000 != 0;
        let range = sk.target_range;
        if !ee::lt(range, 0) && (!cx.valid(target) || target.is_some_and(|t| cx.dead(t))) {
            return 0;
        }
        let mut n = 0;
        if !ee::le(range, 0) {
            let c = cx.scene.chars[target.unwrap()].pos_p;
            for m in cx.party.members.into_iter().flatten() {
                if cx.valid(Some(m)) && !cx.dead(m) && Self::in_range(cx, m, c, range) {
                    self.hit(cx, m, sk, half && Some(m) != target);
                    n += 1;
                }
            }
        } else if ee::lt(range, 0) {
            for m in cx.party.members.into_iter().flatten() {
                if cx.valid(Some(m)) && !cx.dead(m) {
                    self.hit(cx, m, sk, false);
                    n += 1;
                }
            }
        } else if let Some(t) = target {
            self.hit(cx, t, sk, false);
            n = 1;
        }
        n
    }

    /// `_ccBossSkillDamage(this, pos, sk)` (0x0045f470).
    fn damage_at(&mut self, cx: &mut Cx, pos: V4, sk: &SkillParam) -> i32 {
        let me = cx.me;
        if !cx.scene.listed(me) {
            return 0;
        }
        let half = sk.ty & 0x8000 != 0;
        let range = sk.target_range;
        let c = if sk.ty & 0x2000 != 0 { cx.scene.chars[me].pos_p } else { w2p(cx, pos) };
        let mut n = 0;
        if !ee::le(range, 0) {
            for m in cx.party.members.into_iter().flatten() {
                if cx.valid(Some(m)) && !cx.dead(m) && Self::in_range(cx, m, c, range) {
                    self.hit(cx, m, sk, half);
                    n += 1;
                }
            }
        } else if ee::lt(range, 0) {
            for m in cx.party.members.into_iter().flatten() {
                if cx.valid(Some(m)) && !cx.dead(m) {
                    self.hit(cx, m, sk, false);
                    n += 1;
                }
            }
        }
        n
    }

    // --- Think ------------------------------------------------------------------------------

    /// `ccBoss01::Think` (0x0047c1e0).
    fn think(&mut self, cx: &mut Cx) {
        match self.act_num {
            act::NEUTRAL => self.on_neutral(cx),
            act::DAMAGE0 | act::DAMAGE1 => self.on_damage(cx),
            act::CROSS => self.on_cross(cx),
            act::WAVE => self.on_wave(cx),
            act::DRAIN => self.on_drain(cx),
            act::MAGIC => self.on_magic(cx),
            act::DASH => self.on_dash(cx),
            act::WANDER => self.on_wander(cx),
            act::EPITAPH_NEUTRAL => self.on_epitaph_neutral(cx),
            act::EPITAPH_WAVE => self.on_epitaph_wave(cx),
            act::ESCAPE => self.on_escape(cx),
            act::RETURN => self.on_return(cx),
            act::CHASE => self.on_chase(cx),
            act::RAND_DRIVE => {}
            act::EPITAPH | act::DEAD => {}
            _ => self.change_action(cx, act::NEUTRAL, 0, true),
        }
    }

    /// The wait of `OnThinkNeutral` and `OnThinkEpitaphNeutral`: true when
    /// it is over.
    fn wait_over(&mut self, cx: &Cx) -> bool {
        if self.stop_time > 0 {
            let c = self.stop_counter;
            self.stop_counter = c + 1;
            if self.stop_time < c {
                self.stop_time = 0;
                return true;
            }
            return false;
        }
        match self.stop_time {
            -1 => false,
            -2 => {
                if cx.scene.chars[cx.me].skill_id != 0 {
                    return false;
                }
                self.stop_time = 0;
                true
            }
            _ => true,
        }
    }

    /// `OnThinkNeutral` (0x0047ca50).
    fn on_neutral(&mut self, cx: &mut Cx) {
        self.move_spd = 0;
        self.move_vector = [0; 4];
        self.set_dirc(self.target_dirc);
        if !self.wait_over(cx) {
            return;
        }
        let t = self.select_target(cx, 0, FAR);
        cx.scene.chars[cx.me].target_char = t;
        if cx.valid(t) {
            self.calc_target_info(cx);
            self.change_next_pattern(cx);
        }
    }

    /// `OnThinkDamage` (0x0047cba0).
    fn on_damage(&mut self, cx: &mut Cx) {
        self.on_neutral(cx);
        if self.anm.frame() == 0 {
            cx.se3d(195);
            self.change_action(cx, act::NEUTRAL, 1, false);
        }
    }

    /// `OnThinkEpitaphNeutral` (0x0047cc20).
    fn on_epitaph_neutral(&mut self, cx: &mut Cx) {
        let c = self.act_count;
        self.act_count += 1;
        self.move_spd = 0;
        self.move_vector = [0; 4];
        self.set_dirc(self.move_dirc);
        if !self.wait_over(cx) {
            return;
        }
        let t = self.select_target(cx, 0, FAR);
        cx.scene.chars[cx.me].target_char = t;
        if !cx.valid(t) {
            return;
        }
        self.calc_target_info(cx);
        if self.pat_num == 8 && c < 300 && !ee::le(self.target_dist, 0x4461_0000) {
            return;
        }
        self.change_next_pattern(cx);
    }

    /// `OnThinkEpitaphWave` (0x0047cdc0).
    fn on_epitaph_wave(&mut self, cx: &mut Cx) {
        match self.act_proccess {
            0 => {
                self.anm_wave.set(ANM_WAVE, cx.clips);
                self.act_proccess += 1;
                cx.se3d(223);
                return;
            }
            1 => {
                let f = self.anm.frame();
                if f == 20 {
                    cx.se3d(224);
                } else if f >= 29 {
                    if f == 29 {
                        cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
                        self.effect(cx, EffKind::WaveShock);
                    }
                    let done = self.anm_wave.forward();
                    self.anm_wave.speed = 512;
                    cx.out(Out::DrawWave);
                    cx.quake_cam(0x420c_0000);
                    if self.anm_status != 0 && done {
                        self.change_action(cx, act::EPITAPH_NEUTRAL, 3, true);
                    }
                }
            }
            _ => return,
        }
        let c = self.act_count;
        self.act_count += 1;
        if c == 30 {
            let pos = cx.pos();
            self.skill_damage(cx, None, Some(pos), SKILL_WAVE);
        }
    }

    /// `OnThinkChase` (0x0047d000).
    fn on_chase(&mut self, cx: &mut Cx) {
        let c = self.act_count;
        self.act_count += 1;
        match self.act_proccess {
            0 => {
                self.spd_up(0x4316_0000, 0x4316_0000);
                self.erase_cmnd_target(cx);
                Self::cursor_off(cx, true);
                self.animate.set(self.transparency, HALF, 0x4170_0000);
                cx.se3d(229);
                let t = self.select_target(cx, 0, FAR);
                cx.scene.chars[cx.me].target_char = t;
                self.calc_target_info(cx);
                self.act_proccess += 1;
            }
            1 => {
                if c & 3 == 3 {
                    cx.out(Out::AfterImage);
                    if c == 3 {
                        cx.se3d(57);
                    }
                }
                let me = cx.me;
                let t = cx.scene.chars[me].target_char;
                if cx.valid(t) {
                    self.move_dirc = self.target_dirc;
                    if ee::lt(self.target_dist, 0x43c8_0000) {
                        self.change_next_pattern(cx);
                        self.chase_end(cx);
                        return;
                    }
                } else {
                    let t = self.select_target(cx, 0, FAR);
                    cx.scene.chars[me].target_char = t;
                    if !cx.valid(t) {
                        self.exec_pattern(cx, 2, -1);
                        self.chase_end(cx);
                        return;
                    }
                    self.calc_target_info(cx);
                }
                if c >= 90 {
                    self.act_proccess += 1;
                }
                self.dirc[2] = self.move_dirc;
                self.animate.animate();
                self.set_transparency = self.animate.transparency;
            }
            2 => self.chase_end(cx),
            _ => {}
        }
    }

    fn chase_end(&mut self, cx: &mut Cx) {
        self.set_transparency = ONE;
        self.entry_cmnd_target(cx);
        Self::cursor_off(cx, false);
    }

    /// `OnThinkEscape` (0x0047d2f0).
    fn on_escape(&mut self, cx: &mut Cx) {
        let c = self.act_count;
        self.act_count += 1;
        if c >= 91 {
            self.entry_cmnd_target(cx);
            Self::cursor_off(cx, false);
            self.change_next_pattern(cx);
            return;
        }
        if self.stop != 0 || cx.scene.chars[cx.me].cond[cond::HOLD] != 0 {
            self.move_spd = 0;
            return;
        }
        if c == 0 {
            self.move_dirc = self.center_dirc;
            self.m_act_dirc[2] = self.center_dirc;
        }
        self.move_dirc = if self.act_proccess != 0 {
            ee::add(0x3f06_0a92, self.m_act_dirc[2])
        } else {
            ee::sub(self.m_act_dirc[2], 0x3f06_0a92)
        };
        if c & 31 == 0 {
            self.act_proccess ^= 1;
        }
        self.move_dirc = Self::wrap(self.move_dirc);
        self.move_spd = 0x420c_0000;
        if !self.valid_area(cx, cx.scene.chars[cx.me].pos_p) {
            self.change_next_pattern(cx);
            self.entry_cmnd_target(cx);
            Self::cursor_off(cx, false);
        }
    }

    /// `OnThinkReturn` (0x0047dac0).
    fn on_return(&mut self, cx: &mut Cx) {
        match self.act_proccess {
            0 => {
                self.erase_cmnd_target(cx);
                Self::cursor_off(cx, true);
                self.spd_up(0x42c8_0000, 0x42c8_0000);
                self.move_dirc = self.center_dirc;
                self.act_proccess += 1;
            }
            1 if ee::lt(self.center_dist, 0x43c8_0000) => {
                self.move_spd = 0;
                self.move_vector = [0; 4];
                self.entry_cmnd_target(cx);
                Self::cursor_off(cx, false);
                self.change_next_pattern(cx);
            }
            _ => {}
        }
    }

    /// `OnThinkWander` (0x0047dbd0).
    fn on_wander(&mut self, cx: &mut Cx) {
        let me = cx.me;
        if self.stop != 0 || cx.scene.chars[me].cond[cond::HOLD] != 0 {
            self.move_spd = 0;
            return;
        }
        self.move_spd = 0x4234_0000;
        let c = self.act_count;
        self.act_count += 1;
        if c >= 61 || !self.valid_area(cx, cx.scene.chars[me].pos_p) {
            self.change_next_pattern(cx);
            return;
        }
        if self.act_proccess != 0 {
            return;
        }
        self.move_dirc = ee::add(PI, self.target_dirc);
        let t = cx.scene.chars[me].target_char;
        if !cx.valid(t) || t.is_some_and(|t| cx.dead(t)) {
            let t = self.select_target(cx, 0, FAR);
            cx.scene.chars[me].target_char = t;
            if !cx.valid(t) || t.is_some_and(|t| cx.dead(t)) {
                self.move_dirc = self.center_dirc;
            } else {
                self.calc_target_info(cx);
                self.move_dirc = ee::add(PI, self.center_dirc);
            }
        }
        if ee::lt(self.move_dirc, PI) {
            self.move_dirc = ee::add(self.move_dirc, TWO_PI);
        } else if !ee::le(self.move_dirc, PI) {
            self.move_dirc = ee::sub(self.move_dirc, TWO_PI);
        }
        self.act_proccess += 1;
    }

    /// `OnThinkDash` (0x0047ddf0).
    fn on_dash(&mut self, cx: &mut Cx) {
        self.spd_up(0x4248_0000, 0x4248_0000);
        let me = cx.me;
        if self.act_proccess == 0 {
            self.animate.set(self.transparency, HALF, 0x4170_0000);
            self.erase_cmnd_target(cx);
            Self::cursor_off(cx, true);
            self.act_proccess += 1;
            cx.se3d(229);
        }
        match self.act_proccess {
            1 => {
                if self.act_count & 3 == 3 {
                    cx.out(Out::AfterImage);
                    if self.act_count == 3 {
                        cx.se3d(57);
                    }
                }
                if self.act_count >= 121 {
                    self.act_proccess += 1;
                    return;
                }
                let p = w2p(cx, self.dash_pos);
                let pp = cx.scene.chars[me].pos_p;
                self.move_dirc = get_dirc(pp, p);
                if ee::lt(dist(cx, pp, p), 0x4348_0000) {
                    self.act_proccess += 1;
                }
                self.dirc[2] = self.move_dirc;
                self.animate.animate();
                self.set_transparency = self.animate.transparency;
                self.act_count += 1;
            }
            2 => {
                self.entry_cmnd_target(cx);
                Self::cursor_off(cx, false);
                self.change_next_pattern(cx);
                self.set_transparency = ONE;
            }
            _ => {}
        }
    }

    // --- attacks -------------------------------------------------------------------------------

    fn effect(&mut self, cx: &mut Cx, kind: EffKind) -> i32 {
        let pos = cx.pos();
        let dirc = self.dirc;
        self.effect_at(cx, kind, pos, dirc)
    }

    fn effect_at(&mut self, cx: &mut Cx, kind: EffKind, pos: V4, dirc: V4) -> i32 {
        let id = self.effects.create(kind);
        cx.out(Out::Effect { id, kind, pos, dirc });
        id
    }

    /// `OnCrossAtk` (0x0047e030): the target held while the cross swings,
    /// struck at frame 40.
    fn on_cross(&mut self, cx: &mut Cx) {
        let f = self.anm.frame();
        if f == 15 || f == 35 {
            cx.se3d(169);
        }
        let me = cx.me;
        let t = cx.scene.chars[me].target_char;
        if let Some(tp) = t
            && cx.valid(t)
            && !cx.dead(tp)
        {
            cx.affect(tp, 5, 32767);
            if self.anm.frame() == 40 {
                self.skill_damage(cx, t, None, SKILL_CROSS);
                cx.affect(tp, 6, 0);
                cx.se3d(171);
            }
        }
        if self.anm_status != 0 {
            self.change_next_pattern(cx);
        } else {
            self.spd_down(0, 0x40a0_0000);
            self.set_dirc(self.target_dirc);
        }
    }

    /// `OnWaveAtk` (0x0047e1a0).
    fn on_wave(&mut self, cx: &mut Cx) {
        match self.act_proccess {
            0 => {
                let c = self.act_count;
                self.act_count += 1;
                if c >= 73 {
                    self.anm_wave.set(ANM_WAVE, cx.clips);
                    self.act_count = 0;
                    self.act_proccess += 1;
                    cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
                    self.effect(cx, EffKind::WaveShock);
                }
                match self.anm.frame() {
                    10 => cx.se3d(223),
                    65 => cx.se3d(224),
                    _ => {}
                }
            }
            1 => {
                let c = self.act_count;
                self.act_count += 1;
                if c == 25 {
                    let pos = cx.pos();
                    self.skill_damage(cx, None, Some(pos), SKILL_WAVE);
                }
                cx.quake_cam(0x420c_0000);
                let done = self.anm_wave.forward();
                cx.out(Out::DrawWave);
                if self.anm_status != 0 && done {
                    self.change_next_pattern(cx);
                }
            }
            _ => {}
        }
        self.spd_down(0, 0x40a0_0000);
    }

    /// `OnDataDrainAtk` (0x0047e410): a member drained through menu 74.
    fn on_drain(&mut self, cx: &mut Cx) {
        let c = self.act_count;
        self.act_count += 1;
        let me = cx.me;
        if self.act_proccess == 0 {
            let mut t = self.select_target(cx, 0, FAR);
            cx.scene.chars[me].target_char = t;
            if !cx.valid(t) {
                t = self.select_target(cx, 0, FAR);
                cx.scene.chars[me].target_char = t;
                if t.is_none() {
                    self.change_next_pattern(cx);
                    return;
                }
            }
            self.calc_target_info(cx);
            self.lock_player(cx, false);
            cx.out(Out::Cinema(Some(-1)));
            self.act_count = 0;
            self.act_proccess += 1;
        }
        match self.act_proccess {
            1 if c == 45 => {
                let id = cx.scene.chars[me].target_char.map_or(-1, |t| i32::from(cx.scene.chars[t].id()));
                let slot = cx.party.slot_of(id);
                cx.out(Out::StreamMenu { stream: 20, mask: 1 << slot });
                self.act_count = 0;
                self.act_proccess += 1;
            }
            2 if cx.env.menu_type == -1 => {
                let c2 = self.act_count;
                self.act_count += 1;
                if c2 >= 2 {
                    let t = cx.scene.chars[me].target_char;
                    if let Some(tp) = t
                        && cx.valid(t)
                    {
                        cx.affect(tp, 13, 0);
                    }
                    self.unlock_player();
                    cx.out(Out::Cinema(None));
                    self.change_next_pattern(cx);
                }
            }
            _ => {}
        }
        self.spd_down(0, 0x40a0_0000);
    }

    /// `OnMagicAtk` (0x0047e6a0): its steps wait on its effects.
    fn on_magic(&mut self, cx: &mut Cx) {
        let me = cx.me;
        match self.act_proccess {
            0 => {
                let t = cx.scene.chars[me].target_char;
                if !cx.valid(t) {
                    let t = self.select_target(cx, 0, FAR);
                    cx.scene.chars[me].target_char = t;
                    if !cx.valid(t) {
                        self.change_next_pattern(cx);
                        return;
                    }
                    self.calc_target_info(cx);
                }
                self.lock_player(cx, true);
                if self.use_stage_eff && self.stage_eff_id >= 0 {
                    cx.out(Out::StageEnd { rgba: 0, t: -1 });
                    self.stage_eff_id = -1;
                }
                cx.out(Out::SwitchLayer);
                if self.use_stage_eff {
                    self.stage_eff_id = 0;
                    cx.out(Out::StageBegin { rgba: 0x6400_0000, t: [15, 15, 32767] });
                }
                self.erase_cmnd_target(cx);
                Self::cursor_off(cx, true);
                self.anm.set(ANM_MAGIC, cx.clips);
                self.effect(cx, EffKind::MagicSquare { n: 0 });
                cx.se3d(225);
                cx.out(Out::Cinema(Some(2)));
                self.act_proccess += 1;
            }
            1 => {
                let c = self.act_count;
                self.act_count += 1;
                if c == 30 {
                    for m in cx.party.members.into_iter().flatten() {
                        if cx.valid(Some(m)) && !cx.dead(m) {
                            let mut p = cx.scene.chars[m].pos;
                            p[2] = 0x447a_0000;
                            let rot = [0x3fc9_0fdb, 0, 0, ONE];
                            let k = EffKind::ForceGenerator {
                                num: 8,
                                life: 100,
                                speed: 0x4120_0000,
                                r0: 0x4348_0000,
                                r1: 0x4348_0000,
                                clt: 8,
                            };
                            self.eff_force = Some(self.effect_at(cx, k, p, rot));
                            cx.se3d(226);
                        }
                    }
                    self.act_proccess += 1;
                }
            }
            2 => {
                let c = self.act_count;
                self.act_count += 1;
                if c == 130 {
                    cx.se3d(227);
                }
                if self.effects.enabled(self.eff_force) {
                    return;
                }
                let rot = [0, 0, self.target_dirc, 0];
                self.eff_force = Some(self.effect_at(cx, EffKind::AutoSamonRing { n: 35 }, self.target_pos, rot));
                self.act_count = 0;
                self.act_proccess += 1;
            }
            3 => {
                if self.effects.enabled(self.eff_force) {
                    return;
                }
                let mut none = true;
                for m in cx.party.members.into_iter().flatten() {
                    if cx.valid(Some(m)) && !cx.dead(m) {
                        let p = cx.scene.chars[m].pos;
                        self.eff_ice_break = Some(self.effect_at(cx, EffKind::IceBreak, p, VF0));
                        none = false;
                    }
                }
                if none {
                    let p = self.target_pos;
                    self.eff_ice_break = Some(self.effect_at(cx, EffKind::IceBreak, p, VF0));
                }
                self.draw_sw = 0;
                if self.use_stage_eff && self.stage_eff_id >= 0 {
                    cx.out(Out::StageEnd { rgba: 0x6400_0000, t: 15 });
                    self.stage_eff_id = -1;
                }
                cx.out(Out::Se { se: 228 });
                self.act_proccess += 1;
            }
            4 => {
                let c = self.act_count;
                self.act_count += 1;
                if c == 15 {
                    cx.out(Out::SwitchLayer);
                }
                if self.effects.enabled(self.eff_ice_break) {
                    cx.out(Out::Reverse);
                    return;
                }
                self.draw_sw = 1;
                self.entry_cmnd_target(cx);
                Self::cursor_off(cx, false);
                self.unlock_player();
                if self.use_stage_eff && self.stage_eff_id >= 0 {
                    cx.out(Out::StageEnd { rgba: 0x6400_0000, t: 15 });
                    self.stage_eff_id = -1;
                }
                self.eff_ice_break = None;
                let tp = self.target_pos;
                self.skill_damage(cx, None, Some(tp), SKILL_MAGIC);
                let pos = cx.pos();
                cx.out(Out::Se3dNote { se: 66, pos, note: 52 });
                cx.se3d(65);
                cx.se3d(56);
                cx.out(Out::Cinema(None));
                self.change_next_pattern(cx);
            }
            _ => {}
        }
    }

    // --- Action ---------------------------------------------------------------------------------

    /// `ccBoss01::Action` (0x0047bf80).
    fn action(&mut self, cx: &mut Cx) {
        match self.act_num {
            act::EPITAPH => {
                self.cheat_hp = 0;
                self.set_pattern_tbl(Tbl::Epitaph);
                let tbl = Tbl::Epitaph.words(&cx.data.skeith).to_vec();
                // The index is discarded, as in the constructor.
                let _ = self.exec_pattern_index(cx, &tbl, 0);
                self.pat_mode = 2;
                self.change_action(cx, act::EPITAPH_NEUTRAL, 3, true);
                self.move_spd = 0;
            }
            act::DEAD => match self.act_proccess {
                0 => {
                    self.move_spd = 0;
                    self.move_vector = [0; 4];
                    crate::fellow::delete_cmnd(cx.scene, cx.me);
                    cx.out(Out::DeleteCmnd);
                    self.eff_dead = Some(self.begin_dead_effect(cx, 0x44fa_0000, 0x44fa_0000));
                    self.animate_dead.set(ONE, 0, 0x41f0_0000);
                    self.act_proccess += 1;
                }
                1 => {
                    if self.eff_dead.is_some() && !self.effects.enabled(self.eff_dead) {
                        self.eff_dead = None;
                    }
                    if self.anm_status != 0 && self.eff_dead.is_none() {
                        self.act_proccess += 1;
                    }
                }
                2 => {
                    let c = self.act_count;
                    self.act_count += 1;
                    if c >= 30 {
                        self.exit = 1;
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    /// `ccBoss::CalcEffectCameraPos(p, d, out)` (OUT gcmn 0x0046ffb0): `d`
    /// from `p` along the way from it to the centre.
    fn calc_effect_camera_pos(&self, cx: &Cx, p: V4, d: F) -> V4 {
        let pp = w2p(cx, p);
        let r = get_dirc(pp, self.center_pos_p);
        let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI), r);
        let v = geom::apply_matrix(&m, [d, 0, 0, ONE]);
        p2w(cx, geom::vadd(pp, v))
    }

    /// `ccBoss::BeginDeadEffect(dist, height)` (0x0045ee90): the camera
    /// behind the boss, the party held, the music out, the effect.
    fn begin_dead_effect(&mut self, cx: &mut Cx, dist: F, height: F) -> i32 {
        let me = cx.me;
        let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), self.dirc[2]), NEG_HALF_PI);
        let v = geom::apply_matrix(&m, [dist, 0, height, ONE]);
        let eye = p2w(cx, geom::vadd(cx.scene.chars[me].pos_p, v));
        let id = i32::from(cx.scene.chars[me].id());
        let view = cx.pos();
        cx.out(Out::DeadCamera { eye, view, music_fade: !(7..10).contains(&id) });
        self.lock_player(cx, false);
        self.effect(cx, EffKind::Dead)
    }

    // --- Affect ----------------------------------------------------------------------------------

    /// `ccBoss01::Affect` (0x0047bdf0): what an `EntryAffect` on the boss
    /// does at once (`bossAffectFunc`, 0x0045f090): `ccBoss::Affect`, then
    /// Skeith's own - the drain's 4500 HP, and the Super patterns once a
    /// hit leaves its protect gauge at half.
    pub fn affect(&mut self, cx: &mut Cx) {
        match self.class {
            Class::Innis(_) => return innis::affect(self, cx),
            Class::Magus(_) => return magus::affect(self, cx),
            Class::MagusLeaf(_) => return magus::leaf::affect(self, cx),
            Class::KyviaCore(_) => return kyvia::core::affect(self, cx),
            Class::Gomora(_) => return kyvia::gomora::affect(self, cx),
            Class::Fidchell(_) => return fidchell::affect(self, cx),
            Class::Macha(_) => return macha::affect(self, cx),
            Class::Tarvos(_) => return tarvos::affect(self, cx),
            Class::Corbenik(_) => return corbenik::first::affect(self, cx),
            Class::CorbenikSeed(_) => return corbenik::seed::affect(self, cx),
            Class::CorbenikB(_) => return corbenik::second::affect(self, cx),
            Class::CorbenikC(_) => return corbenik::third::affect(self, cx),
            Class::CorbenikKiller(_) => return corbenik::eye::affect(self, cx),
            Class::Gorre(_) => return gorre::affect(self, cx),
            Class::GorreBrother(_) => return gorre::brother_affect(self, cx),
            _ => {}
        }
        let me = cx.me;
        let t = cx.scene.chars[me].affect.ty;
        if t != 21 && (self.act_forbid == 2 || self.lock_player != 0) {
            return;
        }
        self.base_affect(cx);
        let t = cx.scene.chars[me].affect.ty;
        if t == 21 {
            let ch = &mut cx.scene.chars[me];
            ch.hp = 4500;
            ch.max_hp = 4500;
        } else if (t == 1 || t == 3) && self.pat_mode == 0 {
            let pp = cx.scene.param(me).map_or(0, |p| p.pp);
            let id = usize::try_from(cx.scene.chars[me].id()).unwrap_or(0);
            let max = cx.t.bosses.get(id).map_or(0, |r| r.max_pp);
            if pp >= max >> 1 {
                self.set_pattern_tbl(Tbl::Super);
                let tbl = Tbl::Super.words(&cx.data.skeith).to_vec();
                let _ = self.exec_pattern_index(cx, &tbl, 0);
                self.pat_mode = 1;
            }
        }
    }

    /// `ccBoss::Affect` (0x0045c730).
    fn base_affect(&mut self, cx: &mut Cx) {
        self.base_affect_with(cx, &mut |b, cx, n, forbid, af| b.change_action(cx, n, forbid, af));
    }

    /// `ccBoss::Affect` with the class's `ChangeAction` (`ca`), which it
    /// calls through the vtable.
    pub(crate) fn base_affect_with(&mut self, cx: &mut Cx, ca: &mut dyn FnMut(&mut Boss, &mut Cx, i16, i16, bool)) {
        let me = cx.me;
        let (t, p0, person) = {
            let a = &cx.scene.chars[me].affect;
            (a.ty, a.param[0], a.person)
        };
        if t != 21 && (self.act_forbid == 2 || self.lock_player != 0) {
            return;
        }
        let hp = cx.scene.chars[me].hp;
        let mhp = cx.scene.chars[me].max_hp;
        match t {
            6 => self.stop = 0,
            5 => self.stop = 1,
            21 => {
                let v = (i32::from(mhp) / 10) as i16;
                let ch = &mut cx.scene.chars[me];
                ch.hp = v;
                ch.max_hp = v;
                if let Some(p) = cx.scene.param_mut(me) {
                    p.pp = -1;
                    p.pp_count = 0;
                }
            }
            13 => {
                ca(self, cx, act::EPITAPH, 2, true);
                cx.scene.chars[me].affect.ty = 0;
            }
            7 | 9 => {
                cx.out(Out::FlyFont { kind: 20, n: i32::from(p0) });
                cx.scene.chars[me].hp = (hp + p0).min(mhp);
            }
            1 | 3 => {
                if cx.annihilated() || cx.game_over {
                    return;
                }
                cx.out(Out::FlyFont { kind: 2, n: i32::from(p0) });
                if p0 < 0 {
                    return;
                }
                cx.out(Out::HitMark { by: person });
                // cheatHP: a tenth of the hit, never below half.
                let mut h = if self.cheat_hp != 0 { (hp - p0 / 10).max(mhp / 2) } else { hp - p0 };
                if h <= 0 {
                    h = 0;
                    cx.out(Out::ClearSpcCondition);
                    affect::clear_conditions(&mut cx.scene.chars[me]);
                    ca(self, cx, act::DEAD, 2, true);
                } else if t == 1 {
                    let n = if cx.env.count & 1 != 0 { act::DAMAGE0 } else { act::DAMAGE1 };
                    ca(self, cx, n, 0, true);
                }
                cx.scene.chars[me].hp = h;
            }
            _ => {}
        }
    }

    /// What affects asked for since the last frame.
    pub fn take_pending(&mut self) -> Vec<Out> {
        std::mem::take(&mut self.pending)
    }

    /// The blur's writes its affects asked since its last frame, out of
    /// [`Boss::pending`] (the manager's pass applies them before `Main`).
    pub fn take_blur(&mut self) -> Vec<blur::Set> {
        let mut sets = Vec::new();
        self.pending.retain(|o| match o {
            Out::Blur(s) => {
                sets.push(*s);
                false
            }
            _ => true,
        });
        sets
    }
}

/// What a boss's affect needs from outside the scene ([`AffectCtx::boss`]).
pub struct BossEnv<'a> {
    pub t: &'a Tables,
    pub data: &'a BossData,
    pub clips: &'a dyn Fn(&str) -> Option<(u32, bool)>,
    pub env: &'a Env,
    /// `compulsionGameOver`.
    pub game_over: bool,
    /// The fight's `ccRand`, for an Affect that draws it (Tarvos's blows'
    /// notes, a table its gauge brings); None draws from a throwaway.
    pub cc: Option<&'a std::cell::RefCell<crate::rand::Genrand>>,
}

/// `bossAffectFunc` (gcmn 0x0045f090) from `EntryAffect` on the boss
/// `on`: its `Affect` at once, what it asks kept in [`Boss::pending`].
/// Nothing without a [`BossEnv`] in the context.
pub fn entry(scene: &mut Scene, ctx: &AffectCtx, on: usize, rng: &mut dyn Rng, ev: &mut Events) {
    let Some(benv) = ctx.boss else {
        let a = &scene.chars[on].affect;
        let q = (a.ty, a.param, a.person);
        let Some(b) = scene.chars[on].foe_state_mut().and_then(|f| f.boss.as_mut()) else { return };
        b.queued.push(q);
        // Every class's Affect takes a drain (13) at once unless held, and
        // leaves the type 0; kept at 13 until the boss's frame, it would
        // stop the drain menu's 21 (the world paused) at `EntryAffect`.
        if q.0 == 13 && b.act_forbid != 2 && b.lock_player == 0 {
            scene.chars[on].affect.ty = 0;
        }
        return;
    };
    let Some(mut boss) = scene.chars[on].foe_state_mut().and_then(|f| f.boss.take()) else { return };
    let mut spare = crate::rand::Rand::default();
    let mut held = benv.cc.map(|c| c.borrow_mut());
    let cc: &mut dyn Rng = match held.as_deref_mut() {
        Some(g) => g,
        None => &mut spare,
    };
    let mut collide = |_| None;
    let mut land = |_| 0;
    let mut cx = Cx {
        t: benv.t,
        data: benv.data,
        scene: &mut *scene,
        party: ctx.party,
        world: World::default(),
        env: benv.env,
        actx: ctx,
        rand: rng,
        cc,
        clips: benv.clips,
        collide: &mut collide,
        part_hits: &mut NoHits,
        game_over: benv.game_over,
        // Affect never quakes.
        boss_cam: false,
        cam: CamView::default(),
        land: &mut land,
        disc: DiscView::default(),
        gen_alive: &|_| true,
        me: on,
        out: Vec::new(),
        ev: Events::new(),
    };
    boss.affect(&mut cx);
    let (out, e) = (cx.out, cx.ev);
    boss.pending.extend(out);
    ev.extend(e);
    if let Some(f) = scene.chars[on].foe_state_mut() {
        f.boss = Some(boss);
    }
}

/// The affects [`entry`] queued, each put back in the boss's affect
/// fields and applied as `bossAffectFunc` would have: the port's stand-in
/// for the game's immediate call from inside Kite's and the members'
/// frames, run first thing in the boss's task of the same frame.
pub fn apply_queued(scene: &mut Scene, ctx: &AffectCtx, on: usize, rng: &mut dyn Rng, ev: &mut Events) {
    let q = match scene.chars[on].foe_state_mut().and_then(|f| f.boss.as_mut()) {
        Some(b) => std::mem::take(&mut b.queued),
        None => return,
    };
    for (ty, param, person) in q {
        let a = &mut scene.chars[on].affect;
        a.ty = ty;
        a.param = param;
        a.person = person;
        entry(scene, ctx, on, rng, ev);
    }
}
