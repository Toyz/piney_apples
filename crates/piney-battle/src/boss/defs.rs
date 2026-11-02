//! The bosses as defs: a [`BossDef`] says what a boss is, its class, its
//! look (its file, body and the clumps its images and parts draw with),
//! its camera and how it is made. Every boss class derives from `ccBoss`
//! (`piney_data::classes`), so every def is [`BASE_BOSS`] with its
//! differences written over it, a part's [`BASE_PART`]. [`BOSSES`] is
//! `bossFunc`, a def a code.

use piney_data::classes::BossGameClass;

use super::systems::{Hook, Kind, Step};
use super::{Boss, BossBase, Cx, F, ONE, V4};
use super::{corbenik, fidchell, gorre, innis, kyvia, macha, magus, skeith, tarvos};

/// Which body draws a clip of the boss's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyPick {
    Body,
    /// The first image under the Epitaph's clips (by their names' start),
    /// the body under the rest.
    Epitaph(&'static str),
    /// The image whose nodes the clip poses.
    ByClip,
}

/// Which image draws a part's clip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartBody {
    /// The first part the first image, the others the second.
    InOrder,
    /// The first image.
    First,
    /// The image whose nodes its clip poses.
    ByClip,
}

/// The boss camera its constructor makes: none, or `InitBossCamera` at
/// this transfer, then what the constructor asked of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CamDef {
    None,
    Transfer(V4),
}

/// Where a boss is made: Kite's place and heading, the arena's centre
/// (`DMY_center01`), whether Corbenik is in its third form (`saveData`
/// +0x652f, `eventStatus[55]`, 1) and the look's `DMY_dummy1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spawn {
    pub kite_pos: V4,
    pub kite_dirc: V4,
    pub center: V4,
    pub third: bool,
    pub dummy: V4,
}

/// What a boss is: one `static` a def, so a def is its address (no copies:
/// a `const` def named from two crates need not share one).
#[derive(Debug)]
pub struct BossDef {
    /// `bossFunc`'s code (none for Corbenik's later forms and the parts).
    pub code: Option<i32>,
    pub class: BossGameClass,
    /// Its file and body clump, the clumps of that file its images and
    /// parts draw with, Innis's mirrors' shards, which body draws its
    /// clips and which image a part's.
    pub file: &'static str,
    pub clump: &'static str,
    pub images: &'static [&'static str],
    pub shards: bool,
    pub body: BodyPick,
    pub part: PartBody,
    pub cam: CamDef,
    /// The body hit's radius, when not the base's width.
    pub radius: Option<F>,
    pub make: fn(&mut Cx, &Spawn) -> Boss,
    /// Its systems, as its constructor starts them, and the steps of its
    /// frame (`Main`), of its `Affect`, of the effect manager's pass before
    /// its task, and of the hand-over after it, in the game's order.
    pub systems: &'static [Kind],
    pub order: &'static [Step],
    pub affect: &'static [Step],
    pub pass: &'static [Step],
    pub hand_over: &'static [Step],
}

/// One def is one boss: two are the same def or none.
impl PartialEq for BossDef {
    fn eq(&self, other: &BossDef) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for BossDef {}

impl BossDef {
    /// Code `code`'s def, None for a code the port lacks.
    pub fn of(code: i32) -> Option<&'static BossDef> {
        BOSSES.iter().copied().find(|d| d.code == Some(code))
    }
}

/// A bare `ccBoss` at the scene's boss.
fn bare(_: &mut Cx, _: &Spawn) -> Boss {
    Boss::build(&BASE_BOSS, BossBase::default())
}

/// `ccBoss`: `x11`'s `CMP_trall` for a look, no camera.
pub static BASE_BOSS: BossDef = BossDef {
    code: None,
    class: BossGameClass::CcBoss,
    file: "",
    clump: "CMP_trall",
    images: &[],
    shards: false,
    body: BodyPick::Body,
    part: PartBody::InOrder,
    cam: CamDef::None,
    radius: None,
    make: bare,
    systems: &[],
    order: &[Step::Live(&[Step::Prologue, Step::TickStop, Step::LockParty, Step::Move, Step::ForwardAndDraw])],
    affect: &[Step::BaseAffect],
    pass: &[],
    hand_over: &[],
};

/// A part: a `ccBoss` of its own character whose frame its master runs.
pub static BASE_PART: BossDef = BossDef { order: &[], ..BASE_BOSS };

/// `InitBossCamera(z, y)`'s `transfer`: (200, 1000), the eye 1000 behind
/// Kite and 200 up at its nearest (Skeith's, Tarvos's, Corbenik's second
/// form's).
pub const CAM_TRANSFER: V4 = [0, 0x447a_0000, 0x4348_0000, ONE];

/// Skeith (`ccBoss01`, `x11`): his wave and his own; `ccBoss01::Main`
/// (gcmn 0x0047bd50) is `ccBoss::Main`'s blocks around his `Think` and
/// `Action`, his `PreDrawAnm` (0x0047c340) before the body's, then the
/// hold let go and his `DrawCross`; his `Affect` his own.
pub static SKEITH: BossDef = BossDef {
    code: Some(0),
    class: BossGameClass::CcBoss01,
    file: "x11",
    cam: CamDef::Transfer(CAM_TRANSFER),
    make: |cx, s| skeith::new(cx, s.kite_pos, s.kite_dirc, s.center),
    systems: &[Kind::Wave, Kind::Skeith],
    order: &[
        Step::TickEffects,
        Step::Live(&[
            Step::Prologue,
            Step::TickStop,
            Step::Run(Kind::Skeith, Hook::Think),
            Step::Run(Kind::Skeith, Hook::Action),
            Step::LockParty,
            Step::Move,
            Step::Run(Kind::Skeith, Hook::PreDraw),
            Step::ForwardAndDraw,
        ]),
        Step::ClearHold,
        Step::Run(Kind::Skeith, Hook::Late),
    ],
    affect: &[Step::Run(Kind::Skeith, Hook::Affect)],
    ..BASE_BOSS
};

/// Innis (`ccBoss02`, `x21`): its body, its three images (`Mon1`-`Mon3`'s
/// models), its mirrors' shards. `InitBossCamera(200, 900)`. Its wave, its
/// missiles' pass, its images and its own; `ccThBossEffect`'s pass, then
/// `ccBoss02::Main` (0x00495d50): `ccBoss::Main` with its own `Move`, its
/// lock with act 5's forbid, the clip forward at its alpha before the
/// body's `Draw`, `Slave()` (each image's `Main`) and the hold let go.
pub static INNIS: BossDef = BossDef {
    code: Some(1),
    class: BossGameClass::CcBoss02,
    file: "x21",
    images: &["CMP_ex21mon1", "CMP_ex21mon2", "CMP_ex21mon3"],
    shards: true,
    cam: CamDef::Transfer([0, 0x4461_0000, 0x4348_0000, ONE]),
    make: |cx, s| innis::new(cx, s.kite_pos, s.kite_dirc, s.center),
    systems: &[Kind::Wave, Kind::Missiles, Kind::Images, Kind::Innis],
    order: &[
        Step::Run(Kind::Missiles, Hook::Pass),
        Step::Live(&[
            Step::Prologue,
            Step::TickStop,
            Step::Run(Kind::Innis, Hook::Think),
            Step::Run(Kind::Innis, Hook::Action),
            Step::Run(Kind::Innis, Hook::Move),
            Step::Run(Kind::Innis, Hook::Lock),
            Step::Run(Kind::Innis, Hook::PreDraw),
            Step::Draw,
            Step::Run(Kind::Images, Hook::Action),
            Step::ClearHold,
        ]),
    ],
    affect: &[Step::Run(Kind::Innis, Hook::Affect)],
    ..BASE_BOSS
};

/// Magus (`ccBoss03`, `x31`): its body and its leaves' model; it turns
/// camera 3 itself. Its wave, leaves, laser, needles and its own;
/// `ccThBoss03`'s frame (MUT gcmn 0x0049f580): what it reads of the body,
/// the leaves lent, the effects' pass (the needles'), the attack's wait;
/// `ccBoss::Main` with its own `Move` and `PreDrawAnm` and `Slave()`; then
/// `CalcCamera`, the PP count, the holds let go.
pub static MAGUS: BossDef = BossDef {
    code: Some(2),
    class: BossGameClass::CcBoss03,
    file: "x31",
    images: &["CMP_ex31leaf"],
    part: PartBody::First,
    make: |cx, s| magus::new(cx, s.kite_pos, s.kite_dirc, s.center),
    systems: &[Kind::Wave, Kind::Leaves, Kind::Laser, Kind::Needles, Kind::Magus],
    order: &[
        Step::Run(Kind::Leaves, Hook::Early),
        Step::Run(Kind::Laser, Hook::Early),
        Step::Run(Kind::Leaves, Hook::Lend),
        Step::Run(Kind::Needles, Hook::Pass),
        Step::Run(Kind::Magus, Hook::Early),
        Step::Live(&[
            Step::Prologue,
            Step::TickStop,
            Step::Run(Kind::Magus, Hook::Think),
            Step::LockParty,
            Step::Run(Kind::Magus, Hook::Move),
            Step::Run(Kind::Magus, Hook::PreDraw),
            Step::Run(Kind::Laser, Hook::PreDraw),
            Step::Draw,
            Step::Run(Kind::Leaves, Hook::Action),
            Step::ClearHold,
        ]),
        Step::Run(Kind::Magus, Hook::Late),
        Step::ClearHold,
        Step::Run(Kind::Leaves, Hook::Late),
        Step::Run(Kind::Leaves, Hook::Return),
    ],
    affect: &[Step::Run(Kind::Magus, Hook::Affect)],
    ..BASE_BOSS
};

/// Fidchell (`ccBoss04`, `x41`): its body, and the body its Epitaph's
/// clips (`ANM_ex4x*`) move. `InitBossCamera(250, 1500)`.
pub static FIDCHELL: BossDef = BossDef {
    code: Some(3),
    class: BossGameClass::CcBoss04,
    file: fidchell::FILE,
    clump: "CMP_trall1",
    images: &["CMP_trallx"],
    body: BodyPick::Epitaph("ANM_ex4x"),
    cam: CamDef::Transfer([0, 0x44bb_8000, 0x437a_0000, ONE]),
    make: |cx, s| fidchell::new(cx, s.kite_pos, s.kite_dirc, s.center),
    systems: &[Kind::Wave, Kind::Fidchell],
    // ccThBossEffect's pass (its effects' Draw), then Outbreak's
    // ccBoss::Main (OUT gcmn 0x0046d490) around its Think; its Action
    // and Slave are the base's (nothing).
    order: &[
        Step::Run(Kind::Fidchell, Hook::Pass),
        Step::Live(&[
            Step::Prologue,
            Step::TickStop,
            Step::Run(Kind::Fidchell, Hook::Think),
            Step::LockParty,
            Step::Move,
            Step::ForwardAndDraw,
            Step::ClearHold,
        ]),
    ],
    affect: &[Step::Run(Kind::Fidchell, Hook::Affect)],
    ..BASE_BOSS
};

/// Gorre (`ccBoss05`, `x51`): its body (which its Kerse's `ANM_ex51act1`
/// poses) and its brothers' clumps, whose objects their clips draw: the
/// first's `ANM_ex51*`, the second's `ANM_ex52*`, both brothers' Epitaph
/// `ANM_ex5x*`. `InitBossCamera(200, 1500)`. Its brothers, its own and its
/// Kerse; `ccBoss05::Main` (OUT gcmn 0x004aa210): the effects' count (one
/// array for the three), then its own up to `Slave`, each brother's `Main`
/// (`Slave`) and the hold let go. Its `Affect` is empty.
pub static GORRE: BossDef = BossDef {
    code: Some(4),
    class: BossGameClass::CcBoss05,
    file: gorre::FILE,
    images: &["CMP_trall", "CMP_trall2", "CMP_trallx"],
    part: PartBody::ByClip,
    cam: CamDef::Transfer([0, 0x44bb_8000, 0x4348_0000, ONE]),
    make: |cx, s| gorre::new(cx, s.kite_pos, s.kite_dirc, s.center),
    systems: &[Kind::Brothers, Kind::Gorre, Kind::Kerse],
    order: &[
        Step::Run(Kind::Brothers, Hook::Lend),
        Step::TickEffects,
        Step::Run(Kind::Brothers, Hook::Tick),
        Step::Live(&[Step::Run(Kind::Gorre, Hook::Think), Step::Run(Kind::Brothers, Hook::Action), Step::ClearHold]),
        Step::Run(Kind::Brothers, Hook::Return),
    ],
    affect: &[],
    ..BASE_BOSS
};

/// Macha (`ccBoss06`, `x61`): the body, and the body its Epitaph's clips
/// (`ANM_ex6x*`) move. `InitBossCamera(250, 1000)`. Its wave and its own;
/// `ccThBossEffect`'s pass, then `ccBoss06::Main` (QUA gcmn 0x003a4270):
/// `ccBoss::Main` (Outbreak's code) around its `Think`, the hold let go
/// again and the PP count.
pub static MACHA: BossDef = BossDef {
    code: Some(5),
    class: BossGameClass::CcBoss06,
    file: macha::FILE,
    clump: "CMP_trall1",
    images: &["CMP_trall2"],
    body: BodyPick::Epitaph("ANM_ex6x"),
    cam: CamDef::Transfer([0, 0x447a_0000, 0x437a_0000, ONE]),
    make: |cx, s| macha::new(cx, s.kite_pos, s.kite_dirc, s.center),
    systems: &[Kind::Wave, Kind::Macha],
    order: &[
        Step::Run(Kind::Macha, Hook::Pass),
        Step::Live(&[
            Step::Prologue,
            Step::TickStop,
            Step::Run(Kind::Macha, Hook::Think),
            Step::LockParty,
            Step::Move,
            Step::ForwardAndDraw,
            Step::ClearHold,
        ]),
        Step::ClearHold,
        Step::Run(Kind::Macha, Hook::Late),
    ],
    affect: &[Step::Run(Kind::Macha, Hook::Affect)],
    ..BASE_BOSS
};

/// Tarvos (`ccBoss07`, `x71`): the body, and the body its Epitaph's clips
/// (`ANM_ex7x*`) move.
pub static TARVOS: BossDef = BossDef {
    code: Some(6),
    class: BossGameClass::CcBoss07,
    file: tarvos::FILE,
    images: &["CMP_trall2"],
    body: BodyPick::Epitaph("ANM_ex7x"),
    cam: CamDef::Transfer(CAM_TRANSFER),
    make: |cx, s| tarvos::new(cx, s.kite_pos, s.kite_dirc, s.center),
    systems: &[Kind::Wave, Kind::Tarvos],
    // ccThBossEffect's pass, then ccBoss07::Main (QUA gcmn 0x003a91e0):
    // ccBoss::Main (Outbreak's code) around its Think, its own PreDrawAnm
    // before the body's Draw; the hold let go again and the PP count.
    order: &[
        Step::Run(Kind::Tarvos, Hook::Pass),
        Step::Live(&[
            Step::Prologue,
            Step::TickStop,
            Step::Run(Kind::Tarvos, Hook::Think),
            Step::LockParty,
            Step::Move,
            Step::Run(Kind::Tarvos, Hook::PreDraw),
            Step::Draw,
            Step::ClearHold,
        ]),
        Step::ClearHold,
        Step::Run(Kind::Tarvos, Hook::Late),
    ],
    affect: &[Step::Run(Kind::Tarvos, Hook::Affect)],
    ..BASE_BOSS
};

/// Corbenik (`ccBoss08`, `x81`): its forms' and parts' clumps, the first
/// form's the body ([`corbenik::BODIES`]). Code 7 makes the first form,
/// or the third when the save says (`ccThBoss08`); the first makes no
/// camera. The first form is its own (its seeds lent for the frame and
/// for its `Affect`); `ccThBoss08`'s frame: the effects' pass, then
/// `ccBoss08::Main` (0x003ad000), `ccBoss::Main` with its own
/// `PreDrawAnm` and `Slave` (each seed's `Main`), the hold let go. Once it
/// exits its hand-over makes the second form.
pub static CORBENIK: BossDef = BossDef {
    code: Some(7),
    class: BossGameClass::CcBoss08,
    file: corbenik::FILE,
    clump: corbenik::BODIES[0],
    images: &corbenik::BODIES,
    body: BodyPick::ByClip,
    part: PartBody::ByClip,
    make: |cx, s| match s.third {
        true => corbenik::third::new(cx, s.kite_pos, s.kite_dirc, s.center, s.dummy),
        false => corbenik::first::new(cx, s.kite_pos, s.kite_dirc, s.center),
    },
    systems: &[Kind::CorbenikFirst],
    order: &[
        Step::Run(Kind::CorbenikFirst, Hook::Lend),
        Step::Run(Kind::CorbenikFirst, Hook::Pass),
        Step::Live(&[
            Step::Prologue,
            Step::TickStop,
            Step::Run(Kind::CorbenikFirst, Hook::Think),
            Step::LockParty,
            Step::Move,
            Step::Run(Kind::CorbenikFirst, Hook::PreDraw),
            Step::Draw,
            Step::Run(Kind::CorbenikFirst, Hook::Action),
        ]),
        Step::Run(Kind::CorbenikFirst, Hook::Return),
        Step::ClearHold,
    ],
    affect: &[
        Step::Run(Kind::CorbenikFirst, Hook::Lend),
        Step::Run(Kind::CorbenikFirst, Hook::Affect),
        Step::Run(Kind::CorbenikFirst, Hook::Return),
    ],
    hand_over: &[Step::Run(Kind::CorbenikFirst, Hook::HandOver)],
    ..BASE_BOSS
};

/// Corbenik's second form (`ccBoss08_b`), which the first's task makes as
/// it exits: `InitBossCamera(200, 1000)`. Its own (its roots lent for the
/// frame); the effects' pass, then `ccBoss08_b::Main` (0x003b2610):
/// `ccBoss::Main` with the barrier's message's wait before its `Think`, its
/// own `PreDrawAnm` and `Slave` (each root's `Main`), the hold let go.
pub static CORBENIK_SECOND: BossDef = BossDef {
    code: None,
    class: BossGameClass::CcBoss08B,
    cam: CamDef::Transfer(CAM_TRANSFER),
    systems: &[Kind::CorbenikSecond],
    order: &[
        Step::Run(Kind::CorbenikSecond, Hook::Lend),
        Step::Run(Kind::CorbenikSecond, Hook::Pass),
        Step::Live(&[
            Step::Prologue,
            Step::TickStop,
            Step::Run(Kind::CorbenikSecond, Hook::Think),
            Step::LockParty,
            Step::Move,
            Step::Run(Kind::CorbenikSecond, Hook::PreDraw),
            Step::Draw,
            Step::Run(Kind::CorbenikSecond, Hook::Action),
            Step::ClearHold,
        ]),
        Step::Run(Kind::CorbenikSecond, Hook::Return),
    ],
    affect: &[Step::Run(Kind::CorbenikSecond, Hook::Affect)],
    hand_over: &[],
    ..CORBENIK
};

/// Corbenik's third form (`ccBoss08_c`): `InitBossCamera(980, 1500)`. Its
/// own (its eyes and killer eyes lent for the frame and for its
/// `Affect`); the effects' pass, then `ccBoss08_c::Main` (0x003b99b0, the
/// base's): its own `PreDrawAnm` and `Draw` (the body's matrix read),
/// `Slave` (each eye's `Main`, then each killer eye's), the hold let go.
pub static CORBENIK_THIRD: BossDef = BossDef {
    code: None,
    class: BossGameClass::CcBoss08C,
    cam: CamDef::Transfer([0, corbenik::third::CAM_Y, corbenik::third::CAM_Z, ONE]),
    systems: &[Kind::CorbenikThird],
    order: &[
        Step::Run(Kind::CorbenikThird, Hook::Lend),
        Step::Run(Kind::CorbenikThird, Hook::Pass),
        Step::Live(&[
            Step::Prologue,
            Step::TickStop,
            Step::Run(Kind::CorbenikThird, Hook::Think),
            Step::LockParty,
            Step::Move,
            Step::Run(Kind::CorbenikThird, Hook::PreDraw),
            Step::Run(Kind::CorbenikThird, Hook::Draw),
            Step::Run(Kind::CorbenikThird, Hook::Action),
            Step::ClearHold,
        ]),
        Step::Run(Kind::CorbenikThird, Hook::Return),
    ],
    affect: &[
        Step::Run(Kind::CorbenikThird, Hook::Lend),
        Step::Run(Kind::CorbenikThird, Hook::Affect),
        Step::Run(Kind::CorbenikThird, Hook::Return),
    ],
    hand_over: &[],
    ..CORBENIK
};

/// Kyvia's fights (`ccBossKyvia01`-`04`): the body in the fight's file,
/// and its core's and gomoras' clumps there (the same names in each),
/// whose objects their clips (`ANM_ex0cnut0`, `ANM_ex0gnut0`) draw and
/// morph. `InitBossCamera(450, 1050)`, the fourth fight's (250, 700). Its
/// core and the core's gomoras, its disc and its own; `ccBossKyvia01::Main`
/// (MUT gcmn 0x004d4410): the core and gomoras lent, `ccBoss::Main` around
/// `CheckDiscMove` and its `Think` and `Action`, its lock (the cinema off as
/// the party is let go), then `Slave()` (the core's `Main`). Its `Affect`
/// is `ccBoss`'s; the effects' pass before the blur is its own.
pub static KYVIA: BossDef = BossDef {
    code: Some(12),
    class: BossGameClass::CcBossKyvia01,
    file: kyvia::Fight::First.file(),
    clump: "CMP_trallex01",
    images: &["CMP_trall2", "CMP_trall3"],
    cam: CamDef::Transfer([0, 0x4483_4000, 0x43e1_0000, ONE]),
    radius: Some(kyvia::BODY_RADIUS),
    make: |cx, _| kyvia::new(cx, kyvia::Fight::First),
    systems: &[Kind::SlaveCore, Kind::Gomoras, Kind::Disc, Kind::Kyvia],
    order: &[
        Step::Run(Kind::SlaveCore, Hook::Lend),
        Step::Run(Kind::Gomoras, Hook::Lend),
        Step::Live(&[
            Step::Prologue,
            Step::TickStop,
            Step::Run(Kind::Disc, Hook::Think),
            Step::Run(Kind::Kyvia, Hook::Think),
            Step::Run(Kind::Kyvia, Hook::Action),
            Step::Move,
            Step::Run(Kind::Kyvia, Hook::Lock),
            Step::ForwardAndDraw,
            Step::Run(Kind::SlaveCore, Hook::Action),
        ]),
        Step::Run(Kind::Gomoras, Hook::Return),
        Step::Run(Kind::SlaveCore, Hook::Return),
    ],
    pass: &[
        Step::Run(Kind::SlaveCore, Hook::Lend),
        Step::Run(Kind::Gomoras, Hook::Lend),
        Step::Run(Kind::Kyvia, Hook::Pass),
        Step::Run(Kind::Gomoras, Hook::Return),
        Step::Run(Kind::SlaveCore, Hook::Return),
    ],
    ..BASE_BOSS
};

pub static KYVIA_SECOND: BossDef = BossDef {
    code: Some(13),
    class: BossGameClass::CcBossKyvia02,
    file: kyvia::Fight::Second.file(),
    clump: "CMP_trallex02",
    make: |cx, _| kyvia::new(cx, kyvia::Fight::Second),
    ..KYVIA
};

pub static KYVIA_THIRD: BossDef = BossDef {
    code: Some(14),
    class: BossGameClass::CcBossKyvia03,
    file: kyvia::Fight::Third.file(),
    clump: "CMP_trallex03",
    make: |cx, _| kyvia::new(cx, kyvia::Fight::Third),
    ..KYVIA
};

/// The fourth fight (`Main` QUA gcmn 0x003db950) has its places beside the disc
/// (their walls first in its `Think`) and its own `Move`, and lets the
/// body's hold go after `Slave()`.
pub static KYVIA_FOURTH: BossDef = BossDef {
    code: Some(15),
    class: BossGameClass::CcBossKyvia04,
    file: kyvia::Fight::Fourth.file(),
    clump: "CMP_trallex04",
    cam: CamDef::Transfer([0, 0x442f_0000, 0x437a_0000, ONE]),
    make: |cx, _| kyvia::new(cx, kyvia::Fight::Fourth),
    systems: &[Kind::SlaveCore, Kind::Gomoras, Kind::Disc, Kind::Places, Kind::Kyvia],
    order: &[
        Step::Run(Kind::SlaveCore, Hook::Lend),
        Step::Run(Kind::Gomoras, Hook::Lend),
        Step::Live(&[
            Step::Prologue,
            Step::TickStop,
            Step::Run(Kind::Places, Hook::Think),
            Step::Run(Kind::Disc, Hook::Think),
            Step::Run(Kind::Kyvia, Hook::Think),
            Step::Run(Kind::Kyvia, Hook::Action),
            Step::Run(Kind::Kyvia, Hook::Move),
            Step::Run(Kind::Kyvia, Hook::Lock),
            Step::ForwardAndDraw,
            Step::Run(Kind::SlaveCore, Hook::Action),
            Step::ClearHold,
        ]),
        Step::Run(Kind::Gomoras, Hook::Return),
        Step::Run(Kind::SlaveCore, Hook::Return),
    ],
    ..KYVIA
};

// --- the parts ---------------------------------------------------------------------------

/// Magus's leaf (`ccBoss03Leaf`): its own, its `Affect` its own; its frame
/// Magus's `Slave()`.
pub static LEAF: BossDef = BossDef {
    class: BossGameClass::CcBoss03Leaf,
    systems: &[Kind::Leaf],
    affect: &[Step::Run(Kind::Leaf, Hook::Affect)],
    ..BASE_PART
};
/// Kyvia's core (`kyviaCore`): its own, its `Affect` its own; its frame
/// the body's `Slave()`.
pub static CORE: BossDef = BossDef {
    class: BossGameClass::KyviaCore,
    systems: &[Kind::KyviaCore],
    affect: &[Step::Run(Kind::KyviaCore, Hook::Affect)],
    ..BASE_PART
};
/// A gomora (`kyviaGomora`): its own, its `Affect` its own; its frame the
/// core's `Slave()`.
pub static GOMORA: BossDef = BossDef {
    class: BossGameClass::KyviaGomora,
    systems: &[Kind::Gomora],
    affect: &[Step::Run(Kind::Gomora, Hook::Affect)],
    ..BASE_PART
};
/// A Gorre brother (`ccBoss05Brother`): its own, its `Affect` its own;
/// its frame Gorre's `Slave`.
pub static BROTHER: BossDef = BossDef {
    class: BossGameClass::CcBoss05Brother,
    systems: &[Kind::Brother],
    affect: &[Step::Run(Kind::Brother, Hook::Affect)],
    ..BASE_PART
};
/// Corbenik's seed (`ccBoss08Seed`): its own, its `Affect` its own; its
/// frame the first form's `Slave`.
pub static SEED: BossDef = BossDef {
    class: BossGameClass::CcBoss08Seed,
    systems: &[Kind::Seed],
    affect: &[Step::Run(Kind::Seed, Hook::Affect)],
    ..BASE_PART
};
/// A root of the second form (`ccBoss08Root`): its own; its `Affect`
/// `ccBoss`'s.
pub static ROOT: BossDef = BossDef { class: BossGameClass::CcBoss08Root, systems: &[Kind::Root], ..BASE_PART };
/// The third form's eye (`ccBoss08Eye`): its own; its `Affect` `ccBoss`'s.
pub static EYE: BossDef = BossDef { class: BossGameClass::CcBoss08Eye, systems: &[Kind::Eye], ..BASE_PART };
/// A killer eye (`ccBoss08KillerEye`): its own, its `Affect` its own.
pub static KILLER_EYE: BossDef = BossDef {
    class: BossGameClass::CcBoss08KillerEye,
    systems: &[Kind::Killer],
    affect: &[Step::Run(Kind::Killer, Hook::Affect)],
    ..BASE_PART
};

/// `bossFunc`: the def each code starts.
pub static BOSSES: [&BossDef; 12] = [
    &SKEITH,
    &INNIS,
    &MAGUS,
    &FIDCHELL,
    &GORRE,
    &MACHA,
    &TARVOS,
    &CORBENIK,
    &KYVIA,
    &KYVIA_SECOND,
    &KYVIA_THIRD,
    &KYVIA_FOURTH,
];

/// Every def: the codes', Corbenik's later forms and the parts.
pub static ALL: [&BossDef; 23] = [
    &BASE_BOSS,
    &SKEITH,
    &INNIS,
    &MAGUS,
    &FIDCHELL,
    &GORRE,
    &MACHA,
    &TARVOS,
    &CORBENIK,
    &CORBENIK_SECOND,
    &CORBENIK_THIRD,
    &KYVIA,
    &KYVIA_SECOND,
    &KYVIA_THIRD,
    &KYVIA_FOURTH,
    &LEAF,
    &CORE,
    &GOMORA,
    &BROTHER,
    &SEED,
    &ROOT,
    &EYE,
    &KILLER_EYE,
];

#[cfg(test)]
mod tests {
    use piney_data::classes::GameClass;

    use super::*;

    /// Each def's class derives from `ccBoss` in the game's tree, which
    /// [`BASE_BOSS`] (and so [`BASE_PART`]) is.
    #[test]
    fn each_def_is_a_ccboss() {
        for d in ALL.iter().filter(|d| d.class != BossGameClass::CcBoss) {
            let class = GameClass::from(d.class);
            assert_eq!(class.base(), Some(GameClass::from(BASE_BOSS.class)), "{}", class.name());
        }
    }

    /// `bossFunc`'s codes are each one def's, with a file to look like.
    #[test]
    fn each_code_one_def() {
        for d in BOSSES {
            let code = d.code.unwrap();
            assert_eq!(BossDef::of(code).map(|x| x.class), Some(d.class));
            assert!(!d.file.is_empty(), "code {code}");
        }
        assert!(BossDef::of(8).is_none());
    }

    /// A def is its address: each static one, the code's the same one.
    #[test]
    fn each_def_one_address() {
        for (k, a) in ALL.iter().enumerate() {
            assert!(ALL[k + 1..].iter().all(|b| !std::ptr::eq(*a, *b)), "{:?}", a.class);
        }
        assert!(BOSSES.iter().all(|d| ALL.contains(d)));
        assert_eq!(BossDef::of(6), Some(&TARVOS));
        assert_ne!(BASE_PART, BASE_BOSS);
    }
}
