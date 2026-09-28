//! Fields (`docs/engine/field.md`): the open ground of an area, built from
//! `fieldSeed` by `WORLD::Generate` (`INF gcmn.prg:0x005a6da0`) after
//! `WORLD::Init` (0x005a4cd0) has drawn from the same RNG ([`generate`]).
//!
//! ```text
//! WORLD::Init    a fixed number of draws per field type and weather
//! MakeField      80 x 80 heights, fieldrand(64) each
//! story area     chips 19-21 x 19-21 taken, the centre flattened
//! MakeHill       1 large and 3/5/8 small FRACTAL2 hills by `ground`
//! SetDungeonEnter, SetLake (field types 2, 3, 8, 9, 10)
//! objects        key, sub, base and tree objects at free chips
//! SetCover       ground cover on every chip not hidden
//! start          the first free 2 x 2 chips passing fieldrand(100) >= 96
//! ```
//!
//! The heights are EE floats, and the port computes them with the EE's own
//! rules ([`ee`]), so the height map is the game's to the bit.
//!
//! The tables the generator reads - the object tables, the field and cover
//! models, the per-type constants read from `Generate`'s code, `WORLD::Init`'s
//! draw counts - are engine data, read from each volume's executable by
//! `piney-gen` (`placement::field`) into the build ([`tables_of`], [`INF`];
//! `plans/build-data.md`).
//!
//! `tools/field.py` is the reference; `tools/test_field_rs.py` compares this
//! port with it field by field, and `tools/test_field.py` compares
//! `field.py` with the game's own code.
//!
//! Units: a height cell is 600 world units and the map is `[x][y]`; a chip
//! is 2 x 2 cells (1200 units), and objects occupy whole chips of the
//! 40 x 40 chip grids. The map wraps at 80 cells (48,000 units).
//!
//! What the game draws of a field - the ground and cover tiles, the object
//! heights, the lit vertex colours, the lake's water and the background -
//! is the render module's ([`Field::scene`], [`Field::tile`],
//! [`Field::cover_mesh`], [`Field::get_height`], [`Light`]).

use std::fmt;
use std::sync::{LazyLock, OnceLock};

use crate::store::{Load, Reader};
use crate::volume::Volume;

pub mod ee;
mod generate;
mod render;

pub use crate::dungeon::Rng;
pub use generate::generate;

/// The volume's field tables (the build's `PINEY/TABLES/field.bin`, read
/// once a run). Mutation's equal Infection's; Outbreak and Quarantine take
/// their square roots from the FPU (`sqrt.s`).
pub fn tables_of(v: Volume) -> &'static Tables {
    static READ: [OnceLock<&'static Tables>; 4] = [const { OnceLock::new() }; 4];
    READ[v as usize].get_or_init(|| crate::store::group(v, "field"))
}

/// Infection's field tables, read on first use.
pub static INF: LazyLock<&'static Tables> = LazyLock::new(|| tables_of(Volume::Inf));
pub use render::{
    Background, COVER_RANGE, CoverMesh, DRAWN_CHIPS, FADE_FROM, Light, Scene, TILE_RANGE, Tile, WORLD_SIZE, Water,
    blank_height, cover_offset, object_colours,
};

/// Height cells per side of `FIELD.map`.
pub const MAP: usize = 80;
/// Chips per side of the chip grids.
pub const CHIPS: usize = 40;
/// One height cell, in world units.
pub const CELL: f32 = 600.0;
/// One chip, in world units.
pub const CHIP: f32 = 1200.0;
/// Field types 0-10; type 4's gate leads straight into a dungeon, but its
/// tables are there and it generates like the rest.
pub const FIELD_TYPES: usize = 11;
/// The weathers an area can have (`clamp_weather` keeps them in 0-9).
pub const WEATHERS: usize = 10;

/// `FOBJECT_INFO_TABLE.flat`: bits the setters test. Every row of
/// Infection's tables sets exactly one of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Flat(u32);

impl Flat {
    /// Level the ground under the object (`FIELD::Flat` over 3w x 3h cells)
    /// and hide its chips from the ground cover; z is 0.
    pub const LEVEL: Flat = Flat(0x2);
    /// The lake pieces' flag. Like [`Flat::LEVEL`], the site must be off the
    /// hills and is not height-tested; only `SetSubObject` also levels and
    /// hides for it.
    pub const LAKE: Flat = Flat(0x4);
    /// The site must be off the hills and its corner heights within 64 of
    /// each other (`FIELD::CheckAreaHeight`); only when neither of the bits
    /// above is set.
    pub const SLOPE: Flat = Flat(0x8);

    pub const fn from_bits(bits: u32) -> Flat {
        Flat(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Every bit of `other` is set.
    pub const fn contains(self, other: Flat) -> bool {
        self.0 & other.0 == other.0
    }

    /// Any bit of `other` is set.
    pub const fn intersects(self, other: Flat) -> bool {
        self.0 & other.0 != 0
    }
}

impl std::ops::BitOr for Flat {
    type Output = Flat;
    fn bitor(self, other: Flat) -> Flat {
        Flat(self.0 | other.0)
    }
}

/// `FOBJECT_INFO_TABLE.type`: which table the row was made for, by its
/// numbers. It is not [`Kind`], which numbers what `Generate` placed:
///
/// - the lake fields' own lake rows (`LakeObjTBL_C` ... `_P`) are [`Sub`];
///   the other field types' `LakeObjTABLE` entries point at their key rows,
///   which are [`Key`];
/// - tree rows are [`Base`];
/// - three of `BaseObjTBL_I`'s six rows (`CMP_sfi3tom1` - `3`, models of
///   the sub series) are [`Sub`].
///
/// Nothing in Infection reads it: the rows are reached only through the
/// six `FOBJECT_TABLE`s, which only `WORLD::Init` (for the models' vertex
/// colours) and `Generate`'s setters read, and those read the names, the
/// size and `flat`.
///
/// [`Sub`]: ObjectClass::Sub
/// [`Key`]: ObjectClass::Key
/// [`Base`]: ObjectClass::Base
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ObjectClass {
    Key = 0,
    Sub = 1,
    Base = 2,
    Enter = 3,
}

/// A `FOBJECT_INFO_TABLE` row (0x1c bytes): one object a table can pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectInfo {
    /// `anmname`: an Anime chunk in the field's CCS file. When set, the
    /// object is an animated one (`ccAnm::SetAnm`) ...
    pub anm: Option<&'static str>,
    /// ... else `clumpname`, a Clump chunk there, is drawn as it stands.
    pub clump: &'static str,
    /// `type`.
    pub class: ObjectClass,
    /// Chips across (x) and down (y).
    pub w: u32,
    pub h: u32,
    /// `rotflag`: 0, 1 or 2, and unread, like [`ObjectClass`]. Every object
    /// keeps the zero rotation its constructor gives it (`FOBJECT`,
    /// `FOBJECT2` hold no pointer to their row).
    pub rotflag: u8,
    pub flat: Flat,
}

/// A `FOBJECT_TABLE[11]`: the rows one kind of object picks from, by field
/// type.
#[derive(Debug)]
pub struct ObjectTable {
    /// The table's symbol, e.g. `KeyObjTABLE`.
    pub name: &'static str,
    pub by_type: [&'static [ObjectInfo]; FIELD_TYPES],
}

/// What `Generate` places, with the numbers the setters give
/// `FOBJECT(2).type` (and `field.py` uses; 7 is the lake's).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Kind {
    Entrance = 0,
    Key = 1,
    Sub = 2,
    Base = 3,
    Tree = 4,
    Lake = 7,
}

impl Kind {
    pub fn number(self) -> u32 {
        self as u32
    }
}

/// `WORLD.maxKeyObj` ... `maxTreeObj` before the per-type divisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counts {
    pub key: u32,
    pub sub: u32,
    pub base: u32,
    pub tree: u32,
}

/// One of `Generate`'s per-type reductions: field type `field_type` divides
/// the count of `kind` by `by` (C division).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Division {
    pub field_type: u32,
    pub kind: Kind,
    pub by: u32,
}

/// Hills per `ground`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hills {
    pub small: u32,
    pub large: u32,
}

/// A hill is `fieldrand(range) + base` cells square.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HillSize {
    pub range: u32,
    pub base: u32,
}

/// The square root `ccGetDist` takes (`field.py`'s `Data.sqrt`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sqrt {
    /// A call of newlib's `sqrtf`, rounding to nearest (Infection, Mutation).
    Newlib,
    /// The FPU's `sqrt.s` inline, truncating (Outbreak, Quarantine).
    Fpu,
}

impl Sqrt {
    pub fn apply(self, v: u32) -> u32 {
        match self {
            Sqrt::Newlib => ee::sqrtf(v),
            Sqrt::Fpu => ee::sqrt(v),
        }
    }
}

/// One executable's field tables. Infection's are [`INF`]; the object
/// tables, `SmallMeshName` and every constant are the same on all four
/// volumes, the square root is not (`docs/engine/field.md`).
#[derive(Debug)]
pub struct Tables {
    /// The executable's `volumeNum`.
    pub volume: u32,
    /// `fieldccs`: the CCS file (without `.cmp`) holding a field type's
    /// ground, cover and object models, which `WORLD::Init` loads when
    /// `WORLD.isHacked` is 2 - as `WORLD`'s constructor sets it. `None`
    /// for type 4.
    pub ccs: [Option<&'static str>; FIELD_TYPES],
    /// `fieldccs2`: the same for any other `isHacked`.
    pub ccs2: [Option<&'static str>; FIELD_TYPES],
    /// `BaseMeshName`: the ground tile model per type.
    pub base_mesh: [&'static [&'static str]; FIELD_TYPES],
    /// `SmallMeshName`: the ground cover models `SetCover` picks from.
    pub small_mesh: [&'static [&'static str]; FIELD_TYPES],
    pub enter: ObjectTable,
    pub lake: ObjectTable,
    pub key: ObjectTable,
    pub sub: ObjectTable,
    pub base: ObjectTable,
    pub tree: ObjectTable,
    /// `fieldrand` calls `WORLD::Init` makes, by field type and weather.
    pub init_draws: [[u16; WEATHERS]; FIELD_TYPES],
    /// Story areas `Generate` gives no dungeon entrance.
    pub no_entrance: &'static [i32],
    /// Field types that get a lake.
    pub lake_fields: &'static [u32],
    /// Field types whose trees stand in rows (`SetTreeObject_B`).
    pub tree_rows: &'static [u32],
    pub counts: Counts,
    pub divide: &'static [Division],
    /// The factor on every count by `object` 0, 1, and any other, as float
    /// bits.
    pub percent: [u32; 3],
    /// By `ground` 0-2.
    pub hills: [Hills; 3],
    pub large_hill: HillSize,
    pub small_hill: HillSize,
    /// The CCS file of `WORLD.effccs` ...
    pub effect_ccs: &'static str,
    /// ... and the lake's water surface animation in it, which `SetLake`
    /// plays three times at the lake object.
    pub water: &'static str,
    pub sqrt: Sqrt,
    /// Which height cell each vertex of the ground and cover tiles takes.
    pub mesh: MeshTables,
    pub backgrounds: Backgrounds,
}

/// A vertex rewritten from a height cell: (vertex index in the model's
/// first mmat, cell offset x, y).
pub type MeshCell = (u8, i8, i8);

/// What `FIELD_MESH::SetMESH2` (`gcmn 0x005af780`) and `FIELD::SetSmallMESH`
/// (0x005ade80) write into a template model: each listed vertex's z from a
/// height cell, and its RGB from the same cell's colour. Read by running
/// both in the EE interpreter (`piney-gen`'s `placement::field`); for every
/// field type the template's own vertex x and y lie on the cell they list
/// (`tests/field.rs`).
#[derive(Debug)]
pub struct MeshTables {
    /// The ground tile of chip (x, y): cells (2x + dx, 2y + dy).
    pub tile: &'static [MeshCell],
    pub tile_colour: &'static [MeshCell],
    /// A cover tile set up for cell (x, y): cells (x + dx, y + dy).
    pub cover: &'static [MeshCell],
    pub cover_colour: &'static [MeshCell],
}

/// A `BG_INFO_TABLE` row's fog, as `WORLD::Init` passes it to
/// `ccDrawEnv::SetFog`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fog {
    /// 0-255 each.
    pub colour: [f32; 3],
    pub near: f32,
    pub far: f32,
    /// `per`: the fog's strength at `far`, in percent.
    pub percent: f32,
}

/// A `BG_INFO_TABLE` row (0x34 bytes): one field type's background for one
/// weather.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BgRow {
    /// `p`: read only when `WORLD.isHacked` is not 2.
    pub p: Option<&'static str>,
    /// `clumpname0-3`: the sky, two cloud layers and the mountains, Clump
    /// chunks in the background CCS file.
    pub clumps: [&'static str; 4],
    /// `lgname`: the distant light the ground and the objects are lit by ...
    pub light: &'static str,
    /// ... and `anmname`, the animation whose frame 0 sets it and the
    /// ambient colour.
    pub anime: &'static str,
    pub fog: Fog,
}

/// What `WORLD::Init` loads and `WORLD::DrawBG` draws behind a field, by
/// field type then weather (`WORLD.bgNum`, the area's weather).
#[derive(Debug)]
pub struct Backgrounds {
    /// `backccs`: the CCS file, its `?` standing for `1` + weather.
    pub ccs: [&'static str; FIELD_TYPES],
    /// `backccs2`, for `WORLD.isHacked` other than 2.
    pub ccs2: [Option<&'static str>; FIELD_TYPES],
    /// `BGTBL`.
    pub rows: [&'static [BgRow]; FIELD_TYPES],
    /// `BGTBL2`.
    pub rows2: [&'static [BgRow]; FIELD_TYPES],
    /// `BgMatName`: the sky material whose V `DrawBG` scrolls.
    pub material: [&'static [&'static str]; FIELD_TYPES],
    pub material2: [&'static [&'static str]; FIELD_TYPES],
    /// Field type 6's extra clump (an aurora), by weather.
    pub aurora: &'static [&'static str],
}

impl Backgrounds {
    /// The background CCS file (without `.cmp`) for a field type and
    /// weather.
    pub fn ccs_name(&self, field_type: u32, weather: u32) -> String {
        self.ccs[field_type as usize].replace('?', &(weather + 1).to_string())
    }

    pub fn row(&self, field_type: u32, weather: u32) -> Option<&'static BgRow> {
        self.rows[field_type as usize].get(weather as usize)
    }
}

impl Tables {
    /// `WORLD::Init`'s draws; weathers above 9 count as 9.
    pub fn init_draws(&self, field_type: u32, weather: u32) -> u32 {
        u32::from(self.init_draws[field_type as usize][(weather as usize).min(WEATHERS - 1)])
    }

    /// The table a kind of object picks from.
    pub fn table(&self, kind: Kind) -> &ObjectTable {
        match kind {
            Kind::Entrance => &self.enter,
            Kind::Key => &self.key,
            Kind::Sub => &self.sub,
            Kind::Base => &self.base,
            Kind::Tree => &self.tree,
            Kind::Lake => &self.lake,
        }
    }

    /// How many objects of `kind` `Generate` tries to place.
    pub fn count(&self, field_type: u32, kind: Kind, object: u32) -> u32 {
        let mut n = match kind {
            Kind::Key => self.counts.key,
            Kind::Sub => self.counts.sub,
            Kind::Base => self.counts.base,
            Kind::Tree => self.counts.tree,
            Kind::Entrance | Kind::Lake => return 1,
        };
        for d in self.divide {
            if d.field_type == field_type && d.kind == kind {
                n /= d.by;
            }
        }
        let percent = self.percent[(object as usize).min(2)];
        ee::to_int(ee::mul(ee::from_int(n as i32), percent)).max(0) as u32
    }
}

// Read from the build's file, field by field (`piney_gen::placement::field`
// writes them).

impl Load for Flat {
    fn load(r: &mut Reader) -> Self {
        Flat(Load::load(r))
    }
}

impl Load for ObjectClass {
    fn load(r: &mut Reader) -> Self {
        match u8::load(r) {
            0 => ObjectClass::Key,
            1 => ObjectClass::Sub,
            2 => ObjectClass::Base,
            3 => ObjectClass::Enter,
            n => panic!("field: object type {n}"),
        }
    }
}

impl Load for ObjectInfo {
    fn load(r: &mut Reader) -> Self {
        ObjectInfo {
            anm: Load::load(r),
            clump: Load::load(r),
            class: Load::load(r),
            w: Load::load(r),
            h: Load::load(r),
            rotflag: Load::load(r),
            flat: Load::load(r),
        }
    }
}

impl Load for Kind {
    fn load(r: &mut Reader) -> Self {
        match u32::load(r) {
            0 => Kind::Entrance,
            1 => Kind::Key,
            2 => Kind::Sub,
            3 => Kind::Base,
            4 => Kind::Tree,
            7 => Kind::Lake,
            n => panic!("field: kind {n}"),
        }
    }
}

impl Load for Counts {
    fn load(r: &mut Reader) -> Self {
        Counts { key: Load::load(r), sub: Load::load(r), base: Load::load(r), tree: Load::load(r) }
    }
}

impl Load for Division {
    fn load(r: &mut Reader) -> Self {
        Division { field_type: Load::load(r), kind: Load::load(r), by: Load::load(r) }
    }
}

impl Load for Hills {
    fn load(r: &mut Reader) -> Self {
        Hills { small: Load::load(r), large: Load::load(r) }
    }
}

impl Load for HillSize {
    fn load(r: &mut Reader) -> Self {
        HillSize { range: Load::load(r), base: Load::load(r) }
    }
}

impl Load for Sqrt {
    fn load(r: &mut Reader) -> Self {
        if bool::load(r) { Sqrt::Fpu } else { Sqrt::Newlib }
    }
}

/// A list of [`MeshCell`]s: a count, then (vertex, dx, dy) bytes.
fn mesh_cells(r: &mut Reader) -> &'static [MeshCell] {
    let n = u32::load(r) as usize;
    let v: Vec<MeshCell> = (0..n).map(|_| (Load::load(r), Load::load(r), Load::load(r))).collect();
    Box::leak(v.into_boxed_slice())
}

impl Load for MeshTables {
    fn load(r: &mut Reader) -> Self {
        MeshTables {
            tile: mesh_cells(r),
            tile_colour: mesh_cells(r),
            cover: mesh_cells(r),
            cover_colour: mesh_cells(r),
        }
    }
}

impl Load for Fog {
    fn load(r: &mut Reader) -> Self {
        Fog { colour: Load::load(r), near: Load::load(r), far: Load::load(r), percent: Load::load(r) }
    }
}

impl Load for BgRow {
    fn load(r: &mut Reader) -> Self {
        BgRow {
            p: Load::load(r),
            clumps: Load::load(r),
            light: Load::load(r),
            anime: Load::load(r),
            fog: Load::load(r),
        }
    }
}

impl Load for Backgrounds {
    fn load(r: &mut Reader) -> Self {
        Backgrounds {
            ccs: Load::load(r),
            ccs2: Load::load(r),
            rows: Load::load(r),
            rows2: Load::load(r),
            material: Load::load(r),
            material2: Load::load(r),
            aurora: Load::load(r),
        }
    }
}

impl Load for Tables {
    /// The object tables' row arrays come once, before the tables, which
    /// name them by index per field type (the game's tables share them).
    fn load(r: &mut Reader) -> Self {
        let volume = Load::load(r);
        let ccs = Load::load(r);
        let ccs2 = Load::load(r);
        let base_mesh = Load::load(r);
        let small_mesh = Load::load(r);
        let arrays: &'static [&'static [ObjectInfo]] = Load::load(r);
        let table = |r: &mut Reader| ObjectTable {
            name: Load::load(r),
            by_type: <[u32; FIELD_TYPES]>::load(r).map(|k| arrays[k as usize]),
        };
        Tables {
            volume,
            ccs,
            ccs2,
            base_mesh,
            small_mesh,
            enter: table(r),
            lake: table(r),
            key: table(r),
            sub: table(r),
            base: table(r),
            tree: table(r),
            init_draws: Load::load(r),
            no_entrance: Load::load(r),
            lake_fields: Load::load(r),
            tree_rows: Load::load(r),
            counts: Load::load(r),
            divide: Load::load(r),
            percent: Load::load(r),
            hills: Load::load(r),
            large_hill: Load::load(r),
            small_hill: Load::load(r),
            effect_ccs: Load::load(r),
            water: Load::load(r),
            sqrt: Load::load(r),
            mesh: Load::load(r),
            backgrounds: Load::load(r),
        }
    }
}

/// What `WORLD::Generate` reads (`tools/field.py`'s `Field` arguments).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Params {
    /// `fieldSeed`, which `WORLD_MAN::GO` seeds the RNG with.
    pub seed: u32,
    /// `WORLD.fieldType`, 0-10.
    pub field_type: u32,
    /// The area's weather, which sets `WORLD::Init`'s draws.
    pub weather: u32,
    /// 0-2: 3, 5 or 8 small hills.
    pub ground: u32,
    /// 0-2: 80%, 90% or all of the objects.
    pub object: u32,
    /// `WORLD_MAN.eventAreaNumber` (and `ccGame.field`): the story area, or
    /// 0 for a random area.
    pub event: i32,
    /// `WORLD_MAN::IsProtectArea`: the story area's
    /// `EVENTAREA_INFO.protect[1]` is set, so the start must be more than
    /// 7,000 units from the dungeon entrance.
    pub protect: bool,
    /// Start `Generate` from the RNG as seeded, without `WORLD::Init`'s draws.
    pub skip_init: bool,
}

/// A hill: `size` cells square with its key grid at cell (x, y).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hill {
    pub x: u32,
    pub y: u32,
    pub size: u32,
}

/// A placed object.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Object {
    pub kind: Kind,
    /// The row of its table (for the tree rows of field type 1, the piece).
    pub index: usize,
    pub info: &'static ObjectInfo,
    /// Top-left chip and size in chips.
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    /// Its centre in world units (`FOBJECT(2).wp` x, y), as float bits.
    pub pos: [u32; 2],
    /// The height cell of its centre (`mx`, `my`).
    pub cell: [i32; 2],
    /// The game sets its z to 0 rather than asking `WORLD::GetHeight`: the
    /// entrance, the lake, field type 1's tree rows, and objects whose
    /// [`Flat`] bits level the ground.
    pub level: bool,
    /// Its z (`FOBJECT(2).wp`), as float bits: 0 when [`Object::level`],
    /// else `WORLD::GetHeight` at its centre on the heights as they were
    /// when it was placed.
    pub z: u32,
}

/// A ground cover tile (`FCOVER`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cover {
    /// The chip.
    pub x: u32,
    pub y: u32,
    /// 0-3: which corner, `FCOVER.pos` = (-300 or +300 by bit 0, -300 or
    /// +300 by bit 1, 2.5).
    pub quadrant: u32,
    /// A `SmallMeshName` model.
    pub mesh: &'static str,
}

/// A generated field.
#[derive(Clone, Debug)]
pub struct Field {
    pub params: Params,
    /// `WORLD::Init`'s draws, made before `Generate` (0 with `skip_init`).
    pub init_draws: u32,
    /// `FIELD.map[x][y]`: 80 x 80 heights, x-major, as EE float bits.
    pub map: Vec<u32>,
    /// `FIELD.check` then `FIELD.check2`, 40 x 40 bytes each (`[x][y]`),
    /// contiguous as in `FIELD`: chips taken by objects.
    pub check: Vec<u8>,
    /// `FIELD.check3`: chips hidden from the ground cover.
    pub check3: Vec<u8>,
    /// `FIELD.mnt`: chips under a hill.
    pub mnt: Vec<u8>,
    pub hills: Vec<Hill>,
    pub objects: Vec<Object>,
    pub covers: Vec<Cover>,
    /// The dungeon entrance's chip, and its world position (float bits; z 0).
    pub entrance: Option<[u32; 2]>,
    pub dungeon_pos: Option<[u32; 3]>,
    /// The start chip and the position `WORLD_MAN::SetStartPos` gets
    /// (float bits; z 0).
    pub start: [u32; 2],
    pub start_pos: [u32; 3],
    /// The RNG after the start position.
    pub rng: Rng,
}

/// Why [`generate`] gives up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenerateError {
    FieldType(u32),
    /// Not 0-2: the game's switch leaves its hill counters unset.
    Ground(u32),
    /// A loop the game would run forever found no site: the dungeon
    /// entrance, the lake or the start.
    NoSite(&'static str),
}

impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenerateError::FieldType(t) => write!(f, "field type {t} is not 0-10"),
            GenerateError::Ground(g) => write!(f, "ground {g} is not 0-2"),
            GenerateError::NoSite(what) => write!(f, "no site for the {what}"),
        }
    }
}

impl std::error::Error for GenerateError {}

/// Something to draw: a placed object's model in world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub kind: Kind,
    /// The CCS file (without `.cmp`) holding its model: the field's.
    pub ccs: &'static str,
    /// An Anime chunk to play, when the object has one ...
    pub anm: Option<&'static str>,
    /// ... else this Clump chunk.
    pub clump: &'static str,
    /// Its centre, z [`Object::z`]: no rotation, no scale.
    pub pos: [f32; 3],
}

impl Field {
    /// Height cell (x, y), wrapping at 80.
    pub fn height(&self, x: usize, y: usize) -> f32 {
        f32::from_bits(self.map[(x % MAP) * MAP + y % MAP])
    }

    /// The map's vertex (x, y) in world units: `[600 x, 600 y, height]`.
    /// Cell x = 80 is cell 0 again.
    pub fn vertex(&self, x: usize, y: usize) -> [f32; 3] {
        [CELL * x as f32, CELL * y as f32, self.height(x, y)]
    }

    /// The map as world-space heights, x-major.
    pub fn heights(&self) -> Vec<f32> {
        self.map.iter().map(|&b| f32::from_bits(b)).collect()
    }

    /// Every object to draw, with the file and chunk names its model is
    /// under. Empty for field type 4, which has no CCS file.
    pub fn placements(&self, tables: &Tables) -> Vec<Placement> {
        let Some(ccs) = tables.ccs[self.params.field_type as usize] else {
            return Vec::new();
        };
        self.objects
            .iter()
            .map(|o| {
                let pos = [o.pos[0], o.pos[1], o.z].map(f32::from_bits);
                Placement { kind: o.kind, ccs, anm: o.info.anm, clump: o.info.clump, pos }
            })
            .collect()
    }

    /// `FIELD.check`.
    pub fn check1(&self) -> &[u8] {
        &self.check[..CHIPS * CHIPS]
    }

    /// `FIELD.check2`.
    pub fn check2(&self) -> &[u8] {
        &self.check[CHIPS * CHIPS..]
    }
}
