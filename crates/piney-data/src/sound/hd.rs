//! The `.hd` bank header (`docs/formats/snddata.md`), as `tools/scei.py`
//! reads it: `Vers`, `Head`, then the `Prog`, `Sset`, `Smpl` and `Vagi`
//! chunks, each a count and a table of offsets into itself
//! (0xffffffff for an empty slot).
//!
//! Every chunk starts with the creator and type as little-endian u32s, so
//! the bytes read backwards: `IECSsreV` is `SCEI` `Vers`.

use crate::{Bytes, Result, format_err};

const NONE: u32 = 0xffff_ffff;

/// A `Vagi` entry: where a VAG's ADPCM starts in the `.bd` and its rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vag {
    pub offset: u32,
    /// Hz.
    pub rate: u16,
    /// 1 exactly when the ADPCM ends in a looping frame.
    pub looped: bool,
    pub reserved: u8,
}

/// A 42-byte `Smpl` record. Field names past `vag`, the velocity range,
/// `base_note`, `pan`, `volume` and the ADSR words are Sony's SDK names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sample {
    pub vag: u16,
    pub vel_low: u8,
    pub vel_crossfade: u8,
    pub vel_high: u8,
    pub vel_follow_pitch: i8,
    pub vel_follow_pitch_center: u8,
    pub vel_follow_pitch_curve: u8,
    pub vel_follow_amp: i8,
    pub vel_follow_amp_center: u8,
    pub vel_follow_amp_curve: u8,
    pub base_note: u8,
    pub detune: i8,
    pub pan: i8,
    pub group: u8,
    pub priority: u8,
    pub volume: u8,
    pub reserved: u8,
    pub adsr1: u16,
    pub adsr2: u16,
    pub kf_ar: i8,
    pub kf_ar_center: u8,
    pub kf_dr: i8,
    pub kf_dr_center: u8,
    pub kf_sr: i8,
    pub kf_sr_center: u8,
    pub kf_rr: i8,
    pub kf_rr_center: u8,
    pub kf_sl: i8,
    pub kf_sl_center: u8,
    pub pitch_lfo_delay: u16,
    pub pitch_lfo_fade: u16,
    pub amp_lfo_delay: u16,
    pub amp_lfo_fade: u16,
    pub lfo_attr: u8,
    pub spu_attr: u8,
}

pub const SAMPLE_SIZE: usize = 42;

/// A `Sset` entry: the samples a split can sound, by velocity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SampleSet {
    pub vel_curve: u8,
    pub vel_low: u8,
    pub vel_high: u8,
    pub samples: Vec<u16>,
}

/// A 20-byte split of a program: a key range and its sample set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Split {
    pub sample_set: u16,
    pub key_low: u8,
    pub key_crossfade: u8,
    pub key_high: u8,
    pub number: u8,
    pub bend_low: u16,
    pub bend_high: u16,
    pub key_follow_pitch: i8,
    pub key_follow_pitch_center: u8,
    pub key_follow_amp: i8,
    pub key_follow_amp_center: u8,
    pub key_follow_pan: i8,
    pub key_follow_pan_center: u8,
    pub volume: u8,
    pub pan: i8,
    pub transpose: i8,
    pub detune: i8,
}

pub const SPLIT_SIZE: usize = 20;

/// A `Prog` entry: the 36-byte header and its splits.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Program {
    pub split_offset: u32,
    pub n_split: u8,
    pub split_size: u8,
    pub volume: u8,
    pub pan: i8,
    pub transpose: i8,
    pub detune: i8,
    pub key_follow_pan: i8,
    pub key_follow_pan_center: u8,
    pub attr: u8,
    pub reserved: u8,
    pub lfo_wave: u8,
    pub lfo_wave2: u8,
    pub lfo_phase: u8,
    pub lfo_phase2: u8,
    pub lfo_random: u8,
    pub lfo_random2: u8,
    pub lfo_cycle: u16,
    pub lfo_cycle2: u16,
    pub lfo_pitch_depth: i16,
    pub lfo_pitch_depth2: i16,
    pub lfo_midi_pitch_depth: i16,
    pub lfo_midi_pitch_depth2: i16,
    pub lfo_amp_depth: i8,
    pub lfo_amp_depth2: i8,
    pub lfo_midi_amp_depth: i8,
    pub lfo_midi_amp_depth2: i8,
    pub splits: Vec<Split>,
}

pub const PROGRAM_SIZE: usize = 36;

/// A parsed `.hd`. The tables are indexed by slot, `None` where the offset
/// table says 0xffffffff.
#[derive(Clone, Debug, Default)]
pub struct Hd {
    pub version: (u8, u8),
    pub hd_size: u32,
    pub bd_size: u32,
    pub se_timbre: u32,
    pub vags: Vec<Option<Vag>>,
    pub samples: Vec<Option<Sample>>,
    pub sample_sets: Vec<Option<SampleSet>>,
    pub programs: Vec<Option<Program>>,
}

/// `(creator, type, size)`, with the four-character codes the right way round.
fn chunk_at(d: &[u8], p: usize) -> Result<([u8; 4], [u8; 4], u32)> {
    let mut c: [u8; 4] = d.slice_at(p, 4)?.try_into().unwrap();
    let mut k: [u8; 4] = d.slice_at(p + 4, 4)?.try_into().unwrap();
    c.reverse();
    k.reverse();
    Ok((c, k, d.u32_at(p + 8)?))
}

fn expect(d: &[u8], p: usize, kind: &[u8; 4]) -> Result<()> {
    let (c, k, _) = chunk_at(d, p)?;
    if &c != b"SCEI" || &k != kind {
        return format_err(format!(
            "expected SCEI/{} at 0x{p:x}, found {}/{}",
            String::from_utf8_lossy(kind),
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&k)
        ));
    }
    Ok(())
}

/// The absolute offsets of a chunk's entries, `None` for empty slots.
pub(crate) fn table(d: &[u8], p: usize, kind: &[u8; 4]) -> Result<Vec<Option<usize>>> {
    expect(d, p, kind)?;
    let top = d.u32_at(p + 12)? as usize;
    (0..=top)
        .map(|i| {
            let o = d.u32_at(p + 16 + 4 * i)?;
            Ok((o != NONE).then_some(p + o as usize))
        })
        .collect()
}

fn sample_at(d: &[u8], o: usize) -> Result<Sample> {
    let r = d.slice_at(o, SAMPLE_SIZE)?;
    let u16_at = |a: usize| u16::from_le_bytes([r[a], r[a + 1]]);
    Ok(Sample {
        vag: u16_at(0),
        vel_low: r[2],
        vel_crossfade: r[3],
        vel_high: r[4],
        vel_follow_pitch: r[5] as i8,
        vel_follow_pitch_center: r[6],
        vel_follow_pitch_curve: r[7],
        vel_follow_amp: r[8] as i8,
        vel_follow_amp_center: r[9],
        vel_follow_amp_curve: r[10],
        base_note: r[11],
        detune: r[12] as i8,
        pan: r[13] as i8,
        group: r[14],
        priority: r[15],
        volume: r[16],
        reserved: r[17],
        adsr1: u16_at(18),
        adsr2: u16_at(20),
        kf_ar: r[22] as i8,
        kf_ar_center: r[23],
        kf_dr: r[24] as i8,
        kf_dr_center: r[25],
        kf_sr: r[26] as i8,
        kf_sr_center: r[27],
        kf_rr: r[28] as i8,
        kf_rr_center: r[29],
        kf_sl: r[30] as i8,
        kf_sl_center: r[31],
        pitch_lfo_delay: u16_at(32),
        pitch_lfo_fade: u16_at(34),
        amp_lfo_delay: u16_at(36),
        amp_lfo_fade: u16_at(38),
        lfo_attr: r[40],
        spu_attr: r[41],
    })
}

fn split_at(d: &[u8], o: usize) -> Result<Split> {
    let r = d.slice_at(o, SPLIT_SIZE)?;
    let u16_at = |a: usize| u16::from_le_bytes([r[a], r[a + 1]]);
    Ok(Split {
        sample_set: u16_at(0),
        key_low: r[2],
        key_crossfade: r[3],
        key_high: r[4],
        number: r[5],
        bend_low: u16_at(6),
        bend_high: u16_at(8),
        key_follow_pitch: r[10] as i8,
        key_follow_pitch_center: r[11],
        key_follow_amp: r[12] as i8,
        key_follow_amp_center: r[13],
        key_follow_pan: r[14] as i8,
        key_follow_pan_center: r[15],
        volume: r[16],
        pan: r[17] as i8,
        transpose: r[18] as i8,
        detune: r[19] as i8,
    })
}

fn program_at(d: &[u8], o: usize) -> Result<Program> {
    let r = d.slice_at(o, PROGRAM_SIZE)?;
    let u16_at = |a: usize| u16::from_le_bytes([r[a], r[a + 1]]);
    let i16_at = |a: usize| i16::from_le_bytes([r[a], r[a + 1]]);
    let mut p = Program {
        split_offset: d.u32_at(o)?,
        n_split: r[4],
        split_size: r[5],
        volume: r[6],
        pan: r[7] as i8,
        transpose: r[8] as i8,
        detune: r[9] as i8,
        key_follow_pan: r[10] as i8,
        key_follow_pan_center: r[11],
        attr: r[12],
        reserved: r[13],
        lfo_wave: r[14],
        lfo_wave2: r[15],
        lfo_phase: r[16],
        lfo_phase2: r[17],
        lfo_random: r[18],
        lfo_random2: r[19],
        lfo_cycle: u16_at(20),
        lfo_cycle2: u16_at(22),
        lfo_pitch_depth: i16_at(24),
        lfo_pitch_depth2: i16_at(26),
        lfo_midi_pitch_depth: i16_at(28),
        lfo_midi_pitch_depth2: i16_at(30),
        lfo_amp_depth: r[32] as i8,
        lfo_amp_depth2: r[33] as i8,
        lfo_midi_amp_depth: r[34] as i8,
        lfo_midi_amp_depth2: r[35] as i8,
        splits: Vec::new(),
    };
    for j in 0..p.n_split as usize {
        p.splits.push(split_at(d, o + p.split_offset as usize + j * p.split_size as usize)?);
    }
    Ok(p)
}

impl Hd {
    pub fn parse(d: &[u8]) -> Result<Hd> {
        expect(d, 0, b"Vers")?;
        expect(d, 16, b"Head")?;
        let mut hd = Hd {
            version: (d.u8_at(14)?, d.u8_at(15)?),
            hd_size: d.u32_at(28)?,
            bd_size: d.u32_at(32)?,
            se_timbre: d.u32_at(52)?,
            ..Hd::default()
        };
        let (prog, sset, smpl, vagi) = (d.u32_at(36)?, d.u32_at(40)?, d.u32_at(44)?, d.u32_at(48)?);
        if vagi != NONE {
            for o in table(d, vagi as usize, b"Vagi")? {
                hd.vags.push(match o {
                    Some(o) => Some(Vag {
                        offset: d.u32_at(o)?,
                        rate: d.u16_at(o + 4)?,
                        looped: d.u8_at(o + 6)? != 0,
                        reserved: d.u8_at(o + 7)?,
                    }),
                    None => None,
                });
            }
        }
        if smpl != NONE {
            for o in table(d, smpl as usize, b"Smpl")? {
                hd.samples.push(o.map(|o| sample_at(d, o)).transpose()?);
            }
        }
        if sset != NONE {
            for o in table(d, sset as usize, b"Sset")? {
                hd.sample_sets.push(match o {
                    Some(o) => {
                        let n = d.u8_at(o + 3)? as usize;
                        let samples = (0..n).map(|i| d.u16_at(o + 4 + 2 * i)).collect::<Result<_>>()?;
                        Some(SampleSet {
                            vel_curve: d.u8_at(o)?,
                            vel_low: d.u8_at(o + 1)?,
                            vel_high: d.u8_at(o + 2)?,
                            samples,
                        })
                    }
                    None => None,
                });
            }
        }
        if prog != NONE {
            for o in table(d, prog as usize, b"Prog")? {
                hd.programs.push(o.map(|o| program_at(d, o)).transpose()?);
            }
        }
        Ok(hd)
    }

    /// Each VAG's `.bd` range `(start, end)`, by `Vagi` slot: it runs to the
    /// next VAG's start in offset order, the last to the end of the body.
    pub fn vag_ranges(&self) -> Vec<Option<(u32, u32)>> {
        let mut order: Vec<(u32, usize)> =
            self.vags.iter().enumerate().filter_map(|(i, v)| v.map(|v| (v.offset, i))).collect();
        order.sort();
        let mut out = vec![None; self.vags.len()];
        for (k, &(start, i)) in order.iter().enumerate() {
            let end = order.get(k + 1).map(|&(o, _)| o).unwrap_or(self.bd_size);
            out[i] = Some((start, end));
        }
        out
    }
}
