//! PSS movies (`PSS/*.PSS`, `docs/formats/pss.md`): an MPEG-2 program stream
//! of 16,384-byte packs, as `sceMpegDemuxPssRing` (INF SLUS_202.67:0x00116c68)
//! reads it, ending in `00 00 01 B9`. Video is stream `0xE0`; audio, in
//! `OPENING.PSS` only, is private stream 1 (`0xBD`) whose payloads start with
//! the sub-stream header `FF A0 00 ch`, the first also with a 40-byte `SShd` /
//! `SSbd` header. It is 16-bit PCM, 48,000 Hz stereo, in 512-byte blocks of
//! 256 left then 256 right samples, the SPU2's own streaming format.

use crate::{Bytes, Result, format_err};

pub const PACK: u8 = 0xba;
pub const SYSTEM_HEADER: u8 = 0xbb;
pub const PRIVATE_STREAM_1: u8 = 0xbd;
pub const PADDING: u8 = 0xbe;
pub const END: u8 = 0xb9;

/// What a PES packet carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stream {
    /// MPEG video `0xE0 + n`.
    Video(u8),
    /// Private stream 1 with the PSS sub-stream header `FF kind 00 channel`:
    /// kind `0xA0` PCM (`sceMpegStrPCM`), `0xA1` PS-ADPCM (`sceMpegStrADPCM`).
    Audio { kind: u8, channel: u8 },
    /// `0xBB`, `0xBE`, or anything else by its stream id.
    Other(u8),
}

/// One PES packet: its stream, its time stamps (90 kHz) and where its
/// payload is in the file (after the PES header and, for audio, after the
/// 4-byte sub-stream header).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Packet {
    pub stream: Stream,
    pub pts: Option<u64>,
    pub dts: Option<u64>,
    pub offset: usize,
    pub len: usize,
}

/// Every PES packet of a PSS file, in file order.
pub fn packets(bytes: &[u8]) -> Result<Vec<Packet>> {
    let mut out = Vec::new();
    let mut p = 0usize;
    while p + 4 <= bytes.len() {
        if bytes[p..p + 3] != [0, 0, 1] {
            // Zero padding (none on the disc, but a sector-rounded copy).
            if bytes[p..].iter().all(|&b| b == 0) {
                break;
            }
            return format_err(format!("PSS: no start code at 0x{p:x}"));
        }
        let id = bytes[p + 3];
        match id {
            PACK => {
                // MPEG-2 pack header: 10 bytes after the code, the last's low
                // three bits the stuffing length.
                let b4 = bytes.u8_at(p + 4)?;
                if b4 & 0xc0 != 0x40 {
                    return format_err(format!("PSS: MPEG-1 pack header at 0x{p:x}"));
                }
                p += 14 + (bytes.u8_at(p + 13)? & 7) as usize;
            }
            END => break,
            _ if id < 0xb9 => return format_err(format!("PSS: start code {id:02x} outside a PES packet at 0x{p:x}")),
            _ => {
                let len = u16::from_be_bytes([bytes.u8_at(p + 4)?, bytes.u8_at(p + 5)?]) as usize;
                let body = bytes.slice_at(p + 6, len)?;
                let start = p + 6;
                p = start + len;
                if id == SYSTEM_HEADER || id == PADDING {
                    out.push(Packet { stream: Stream::Other(id), pts: None, dts: None, offset: start, len });
                    continue;
                }
                let (pts, dts, header) = pes_header(body)?;
                let mut offset = start + header;
                let mut n = len - header;
                let stream = if (0xe0..0xf0).contains(&id) {
                    Stream::Video(id - 0xe0)
                } else if id == PRIVATE_STREAM_1 {
                    let sub = bytes.slice_at(offset, 4)?;
                    if sub[0] != 0xff || sub[2] != 0 {
                        return format_err(format!("PSS: private stream sub-header {sub:02x?} at 0x{offset:x}"));
                    }
                    offset += 4;
                    n -= 4;
                    Stream::Audio { kind: sub[1], channel: sub[3] }
                } else {
                    Stream::Other(id)
                };
                out.push(Packet { stream, pts, dts, offset, len: n });
            }
        }
    }
    Ok(out)
}

/// The MPEG-2 PES header: PTS, DTS and its length (3 + header_data_length).
fn pes_header(body: &[u8]) -> Result<(Option<u64>, Option<u64>, usize)> {
    let flags = body.u8_at(0)?;
    if flags & 0xc0 != 0x80 {
        return format_err("PSS: not an MPEG-2 PES header");
    }
    let pd = body.u8_at(1)? >> 6;
    let hl = body.u8_at(2)? as usize;
    let ts = |at: usize| -> Result<u64> {
        let b = body.slice_at(at, 5)?;
        Ok((u64::from(b[0] >> 1) & 7) << 30
            | u64::from(b[1]) << 22
            | u64::from(b[2] >> 1) << 15
            | u64::from(b[3]) << 7
            | u64::from(b[4] >> 1))
    };
    let pts = if pd & 2 != 0 { Some(ts(3)?) } else { None };
    let dts = if pd == 3 { Some(ts(8)?) } else { None };
    if 3 + hl > body.len() {
        return format_err("PSS: PES header runs past its packet");
    }
    Ok((pts, dts, 3 + hl))
}

/// `SpuStreamHeader` (DWARF, mpeg.cpp): the `SShd` chunk at the head of the
/// audio, 0x18 bytes after its id and size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundHeader {
    /// `type`: 1 in `OPENING.PSS`, 16-bit PCM. The game never reads it.
    pub format: i32,
    /// `rate`: 48,000.
    pub rate: i32,
    /// `ch`: 2.
    pub channels: i32,
    /// `interSize`: bytes of one channel before the next channel's, 0x200.
    pub interleave: i32,
    /// `loopStart`, `loopEnd`: -1 (no loop).
    pub loop_start: i32,
    pub loop_end: i32,
}

/// The audio of a PSS: its header and the sound bytes after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Audio {
    /// The private stream's sub-stream kind: 0xA0 PCM, 0xA1 PS-ADPCM.
    pub kind: u8,
    pub header: SoundHeader,
    /// `SSbd`'s size: the sound bytes the header promises.
    pub body_size: u32,
    /// The sound bytes, as many as the packets carry.
    pub data: Vec<u8>,
}

impl Audio {
    /// The header's size in the stream: `SShd` (8 + 0x18) and `SSbd` (8).
    pub const HEADER: usize = 40;

    fn parse(kind: u8, raw: &[u8]) -> Result<Audio> {
        if raw.len() < Self::HEADER || &raw[0..4] != b"SShd" || &raw[32..36] != b"SSbd" {
            return format_err("PSS: audio does not start with SShd / SSbd");
        }
        if raw.u32_at(4)? != 0x18 {
            return format_err("PSS: SShd size is not 0x18");
        }
        let header = SoundHeader {
            format: raw.i32_at(8)?,
            rate: raw.i32_at(12)?,
            channels: raw.i32_at(16)?,
            interleave: raw.i32_at(20)?,
            loop_start: raw.i32_at(24)?,
            loop_end: raw.i32_at(28)?,
        };
        Ok(Audio { kind, header, body_size: raw.u32_at(36)?, data: raw[Self::HEADER..].to_vec() })
    }

    /// The sound as interleaved 16-bit samples (left, right, left, ...) for
    /// PCM audio: each `interleave`-byte block of every channel in turn,
    /// little-endian. A trailing partial block set is dropped.
    pub fn pcm(&self) -> Result<Vec<i16>> {
        if self.kind != 0xa0 {
            return format_err(format!("PSS: audio kind {:02x} is not PCM", self.kind));
        }
        let ch = self.header.channels.max(1) as usize;
        let il = self.header.interleave as usize;
        if il == 0 || !il.is_multiple_of(2) {
            return format_err("PSS: bad audio interleave");
        }
        let set = il * ch;
        let per = il / 2;
        let mut out = Vec::with_capacity(self.data.len() / 2);
        for blocks in self.data.chunks_exact(set) {
            for i in 0..per {
                for c in 0..ch {
                    let at = c * il + 2 * i;
                    out.push(i16::from_le_bytes([blocks[at], blocks[at + 1]]));
                }
            }
        }
        Ok(out)
    }
}

/// A PSS file taken apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pss {
    /// The MPEG-2 video elementary stream (stream 0xE0), whole.
    pub video: Vec<u8>,
    /// The first video packet's PTS (90 kHz).
    pub video_pts: Option<u64>,
    /// Audio channel 0, if the file has any.
    pub audio: Option<Audio>,
    /// The first audio packet's PTS.
    pub audio_pts: Option<u64>,
    /// PES packets by kind: video, audio, other.
    pub counts: [usize; 3],
}

/// Demultiplex a whole PSS file.
pub fn demux(bytes: &[u8]) -> Result<Pss> {
    let mut video = Vec::new();
    let mut raw_audio = Vec::new();
    let mut audio_kind = None;
    let (mut video_pts, mut audio_pts) = (None, None);
    let mut counts = [0usize; 3];
    for p in packets(bytes)? {
        let payload = &bytes[p.offset..p.offset + p.len];
        match p.stream {
            Stream::Video(0) => {
                counts[0] += 1;
                video_pts = video_pts.or(p.pts);
                video.extend_from_slice(payload);
            }
            Stream::Audio { kind, channel: 0 } => {
                counts[1] += 1;
                if audio_kind.is_some_and(|k| k != kind) {
                    return format_err("PSS: audio changes kind");
                }
                audio_kind = Some(kind);
                audio_pts = audio_pts.or(p.pts);
                raw_audio.extend_from_slice(payload);
            }
            _ => counts[2] += 1,
        }
    }
    if video.is_empty() {
        return format_err("PSS: no video stream");
    }
    let audio = match audio_kind {
        Some(kind) => Some(Audio::parse(kind, &raw_audio)?),
        None => None,
    };
    Ok(Pss { video, video_pts, audio, audio_pts, counts })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pes(id: u8, pts: Option<u64>, payload: &[u8]) -> Vec<u8> {
        let mut h = vec![0x81u8, 0, 0];
        if let Some(t) = pts {
            h[1] = 0x80;
            h.extend_from_slice(&[
                0x21 | ((t >> 29) as u8 & 0x0e),
                (t >> 22) as u8,
                ((t >> 14) as u8) | 1,
                (t >> 7) as u8,
                ((t << 1) as u8) | 1,
            ]);
        }
        h[2] = (h.len() - 3) as u8;
        let mut out = vec![0, 0, 1, id];
        out.extend_from_slice(&((h.len() + payload.len()) as u16).to_be_bytes());
        out.extend_from_slice(&h);
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn demuxes_video_and_pcm() {
        let mut f = vec![0, 0, 1, PACK, 0x44, 0, 4, 0, 4, 1, 1, 4, 3, 0xf8];
        f.extend(pes(0xe0, Some(3600), &[0, 0, 1, 0xb3, 9]));
        let mut a = vec![0xff, 0xa0, 0, 0];
        a.extend_from_slice(b"SShd");
        for v in [0x18i32, 1, 48000, 2, 4, -1, -1] {
            a.extend_from_slice(&v.to_le_bytes());
        }
        a.extend_from_slice(b"SSbd");
        a.extend_from_slice(&8u32.to_le_bytes());
        a.extend_from_slice(&[1, 0, 2, 0, 3, 0, 4, 0]);
        f.extend(pes(PRIVATE_STREAM_1, Some(3000), &a));
        f.extend(pes(0xe0, None, &[7]));
        f.extend_from_slice(&[0, 0, 1, END, 0, 0, 0, 0]);
        let d = demux(&f).unwrap();
        assert_eq!(d.video, [0, 0, 1, 0xb3, 9, 7]);
        assert_eq!(d.video_pts, Some(3600));
        assert_eq!(d.audio_pts, Some(3000));
        let au = d.audio.unwrap();
        assert_eq!(au.header.rate, 48000);
        assert_eq!(au.body_size, 8);
        // Blocks of 4 bytes: left 1, 2; right 3, 4.
        assert_eq!(au.pcm().unwrap(), [1, 3, 2, 4]);
    }
}
