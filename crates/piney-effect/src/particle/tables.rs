//! The particle system's tables (`tables::effect`, INF main): the generator
//! rows (`particleGeneratorTbl` 0x003402f0), the force fields
//! (`particleForceFieldTbl` 0x00343850), `particleTbl` (0x00340220),
//! `particleEffectTbl` (0x00344ad0), `particleCcsAnmTbl` (0x003739e0, looked
//! up as `InitParticleCtrl` does), `Polyhedron82Table` (0x0033ef90), the
//! static generators' force fields, and `ccParticle::Setup`'s texture-id map,
//! decoded from each volume's code (Outbreak and Quarantine recompiled it).
//! The layouts are in docs/engine/particles.md.

use piney_data::tables::effect;
use piney_data::tables::types::{ParticleForceFieldParam, ParticleGeneratorParam};
use piney_data::volume::Volume;

use crate::ee::{F, V4};
use crate::files::{Assets, ObjRef};

/// `particleGeneratorTbl`'s rows.
pub const GENERATOR_ROWS: usize = 244;
/// `particleTbl`'s own rows: `Setup` indexes it by texture id up to 244 (id
/// 237 falls through its switch), so 245 are read, rows 101 on the
/// halfwords that follow, as the game reads them.
pub const PARTICLE_ROWS: usize = 101;

/// A `ccParticleGeneratorParam` (0x38 bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GenParam {
    /// Where the volume keeps the row: its identity, as the game's pointers
    /// to it.
    pub va: u32,
    /// Its row of `particleGeneratorTbl`; `usize::MAX` for a row of
    /// another table (GCMN.PRG's spells', the bosses').
    pub row: usize,
    /// +0x00 bits 0-3 `gType`: 0 `pGenRate` a second, 1 one a frame, 2
    /// `pGenRate` at once when none are out.
    pub g_type: u8,
    /// bits 4-7 `rType`: where a particle starts (0 the generator, 1 a
    /// circle, 2 a sphere, 3 the polyhedron, 4 the line to `pos2`, 5 a
    /// ring about that line).
    pub r_type: u8,
    /// bits 8-11 `dType`: which way it flies (0 and 5 a cone about
    /// `pDirc`/`pRange`, 1 out from the centre, 2 in to it, 4 anywhere up;
    /// 3 none, and `rType` 1/4/5 spread regularly).
    pub d_type: u8,
    /// bit 12 `gSync` (the generator follows `syncPos`), 13 `pSync` (its
    /// particles are placed relative to it), 14 `pRotRnd`.
    pub g_sync: bool,
    pub p_sync: bool,
    pub p_rot_rnd: bool,
    /// +0x02 bits 0-3 `pPatRnd`: texture ids to pick among.
    pub p_pat_rnd: u8,
    /// +0x04 `gLife` (-1 forever), +0x08 `gRadius`.
    pub g_life: i16,
    pub g_radius: F,
    /// +0x0c `pLife`, +0x0e `pTex`, +0x10 `pDirc`, +0x12 `pRange`.
    pub p_life: i16,
    pub p_tex: i16,
    pub p_dirc: i16,
    pub p_range: i16,
    /// +0x14 `pIV` (initial speed), +0x18 `pGenRate`, +0x1c `gRadiusRR`,
    /// +0x20 `pLifeRR`, +0x24 `pIVRR` (the random reductions, fractions).
    pub p_iv: F,
    pub p_gen_rate: F,
    pub g_radius_rr: F,
    pub p_life_rr: F,
    pub p_iv_rr: F,
    /// +0x28 `pDircRR`, +0x2a `pRangeRR`, +0x2c `pFadeIn`, +0x2e `pFadeOut`.
    pub p_dirc_rr: i16,
    pub p_range_rr: i16,
    pub p_fade_in: i16,
    pub p_fade_out: i16,
    /// +0x30 `ffNum[4]`: rows of `particleForceFieldTbl`, -1 none.
    pub ff_num: [i16; 4],
}

impl GenParam {
    /// The generated row at `va`; `row` its place in
    /// `particleGeneratorTbl`.
    pub fn of(p: &ParticleGeneratorParam, va: u32, row: usize) -> GenParam {
        GenParam {
            va,
            row,
            g_type: (p.g_type & 15) as u8,
            r_type: (p.r_type & 15) as u8,
            d_type: (p.d_type & 15) as u8,
            g_sync: p.g_sync != 0,
            p_sync: p.p_sync != 0,
            p_rot_rnd: p.p_rot_rnd != 0,
            p_pat_rnd: (p.p_pat_rnd & 15) as u8,
            g_life: p.g_life,
            g_radius: p.g_radius.to_bits(),
            p_life: p.p_life,
            p_tex: p.p_tex,
            p_dirc: p.p_dirc,
            p_range: p.p_range,
            p_iv: p.p_iv.to_bits(),
            p_gen_rate: p.p_gen_rate.to_bits(),
            g_radius_rr: p.g_radius_rr.to_bits(),
            p_life_rr: p.p_life_rr.to_bits(),
            p_iv_rr: p.p_ivrr.to_bits(),
            p_dirc_rr: p.p_dirc_rr,
            p_range_rr: p.p_range_rr,
            p_fade_in: p.p_fade_in,
            p_fade_out: p.p_fade_out,
            ff_num: p.ff_num,
        }
    }

    /// A row of another table than `particleGeneratorTbl`: row `k` of the
    /// table at `table`.
    pub fn other(rows: &[ParticleGeneratorParam], table: u32, k: usize) -> GenParam {
        GenParam::of(&rows[k], table + 0x38 * k as u32, usize::MAX)
    }
}

/// A `ccParticleForceFieldParam` (0x20 bytes): what one force does to a
/// particle each frame (`ccParticleForceField::Calc`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FfParam {
    /// Where the volume keeps the row.
    pub va: u32,
    /// +0x00 bit 0 `calcType`: 1 along the particle's direction (or the
    /// variant each force type names), 0 in world terms.
    pub calc_type: u8,
    /// +0x01 `fieldType`: 0-3 act (3 also ends a particle that reaches the
    /// generator), anything else does nothing.
    pub field_type: u8,
    /// +0x02 `forceType`: 0-16, see [`super::force`].
    pub force_type: u8,
    /// +0x04 `rotate` (65536 a turn), +0x08 `radius`, +0x10 `force`.
    pub rotate: u16,
    pub radius: F,
    pub force: V4,
}

impl FfParam {
    pub fn of(p: &ParticleForceFieldParam, va: u32) -> FfParam {
        FfParam {
            va,
            calc_type: p.calc_type & 1,
            field_type: p.field_type,
            force_type: p.force_type,
            rotate: p.rotate,
            radius: p.radius.to_bits(),
            force: p.force.map(f32::to_bits),
        }
    }

    /// Row `k` of the table at `table`.
    pub fn row(rows: &[ParticleForceFieldParam], table: u32, k: usize) -> FfParam {
        FfParam::of(&rows[k], table + 0x20 * k as u32)
    }
}

/// A `ccParticleParam`: what a particle draws.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PartParam {
    /// bits 0-3 `style`: 0 a `ccEff` sprite, 1 a `ccClump`, 2 a `ccAnm`.
    pub style: u8,
    /// bits 4-7 `anm`: 2 the sprite's patterns play once, 3 loop; with
    /// `pat`, 1-3 (sprites) or 3 start at a random pattern.
    pub anm: u8,
    /// bits 8-15 `pat`.
    pub pat: u8,
}

/// A `ccParticleEffectParam` (0x20): up to four generators about a
/// character (`startParticleEffect`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EffectParam {
    /// +0x00 `sync[4]`: 2 the offset is above the head, 3 at mid height.
    pub sync: [i16; 4],
    /// +0x08 `geneNum[4]`: `particleGeneratorTbl` rows, -1 none.
    pub gene_num: [i16; 4],
    /// +0x10 `offset[4]`: each generator's height.
    pub offset: [F; 4],
}

/// Everything the particle system reads from the executable, with the
/// objects `particleCcsAdrs` holds resolved in the effect files.
#[derive(Clone, Debug, Default)]
pub struct Tables {
    pub generators: Vec<GenParam>,
    pub force_fields: Vec<FfParam>,
    pub particles: Vec<PartParam>,
    pub effects: Vec<EffectParam>,
    /// `particleCcsAnmTbl`: (file stem, object name).
    pub ccs_anm: Vec<(String, String)>,
    /// `particleCcsAdrs[245]`: each row's object (`GetChunkAdrsF`), None
    /// when a file lacks it.
    pub adrs: Vec<Option<ObjRef>>,
    pub polyhedron: Vec<V4>,
    /// `ccpffpSmoke1..4`, `hitPhotonDummyF1/F2`.
    pub smoke_ff: Vec<FfParam>,
    pub hit_photon_ff: Vec<FfParam>,
    /// `Setup`'s texture-id switch: from `tex_first`, (particleTbl row,
    /// CLUT flag) for each id; ids outside are their own row with no CLUT.
    pub tex_first: i32,
    pub tex_map: Vec<(i32, i32)>,
}

impl Tables {
    /// The volume's, the objects looked up in `assets`' files.
    pub fn read(volume: Volume, assets: &Assets) -> Tables {
        let t = effect::of(volume);
        let ccs_anm: Vec<(String, String)> = t
            .ccs_anm()
            .iter()
            .map(|r| (r.ccs.unwrap_or("").to_ascii_lowercase(), r.anm.unwrap_or("").to_string()))
            .collect();
        let adrs = ccs_anm.iter().map(|(f, o)| assets.find(f, o)).collect();
        let sw = t.tex_switch();
        Tables {
            generators: (0..t.generators().len())
                .map(|i| GenParam::of(&t.generators()[i], t.generators_va() + 0x38 * i as u32, i))
                .collect(),
            force_fields: (0..t.force_fields().len())
                .map(|i| FfParam::row(t.force_fields(), t.force_fields_va(), i))
                .collect(),
            particles: t
                .particles()
                .iter()
                .map(|p| PartParam { style: (p.style & 15) as u8, anm: (p.anm & 15) as u8, pat: p.pat as u8 })
                .collect(),
            effects: t
                .particle_effects()
                .iter()
                .map(|e| EffectParam { sync: e.sync, gene_num: e.gene_num, offset: e.offset.map(f32::to_bits) })
                .collect(),
            ccs_anm,
            adrs,
            polyhedron: t.polyhedron().iter().map(|p| p.map(f32::to_bits)).collect(),
            smoke_ff: vec![
                FfParam::of(&t.smoke_ff1(), t.smoke_ff1_va()),
                FfParam::of(&t.smoke_ff2(), t.smoke_ff2_va()),
                FfParam::of(&t.smoke_ff3(), t.smoke_ff3_va()),
                FfParam::of(&t.smoke_ff4(), t.smoke_ff4_va()),
            ],
            hit_photon_ff: vec![
                FfParam::of(&t.hit_photon_ff1(), t.hit_photon_ff1_va()),
                FfParam::of(&t.hit_photon_ff2(), t.hit_photon_ff2_va()),
            ],
            tex_first: sw.first,
            tex_map: sw.map.iter().map(|m| (m[0], m[1])).collect(),
        }
    }

    /// `particleTbl[row]` (past its 101 rows, the bytes that follow), zero
    /// past what is read.
    pub fn part(&self, row: i32) -> PartParam {
        usize::try_from(row).ok().and_then(|r| self.particles.get(r)).copied().unwrap_or_default()
    }

    /// `particleCcsAdrs[i]`.
    pub fn adrs(&self, i: i32) -> Option<ObjRef> {
        usize::try_from(i).ok().and_then(|r| self.adrs.get(r)).copied().flatten()
    }

    /// `Setup`'s switch on a texture id: the `particleTbl` row it draws
    /// and whether (nonzero) it swaps in the CLUT `particleCcsAdrs[tex]`.
    pub fn tex_base(&self, tex: i32) -> (i32, i32) {
        match usize::try_from(tex - self.tex_first).ok().and_then(|i| self.tex_map.get(i)) {
            Some(&m) => m,
            None => (tex, 0),
        }
    }
}
