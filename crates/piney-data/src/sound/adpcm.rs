//! PS-ADPCM, the SPU2's sample format (`docs/formats/snddata.md`), decoded
//! as `tools/adpcm.py` decodes it, bit for bit: 16-byte frames of 28 samples,
//! `shift | filter << 4` (shift 13-15 acts as 9), flags (bit 0 end, 1 repeat,
//! 2 loop start), 14 bytes of signed nibbles, low first; each sample is
//! `(nibble << 12 >> shift) + ((s1 * F0[filter] + s2 * F1[filter] + 32) >> 6)`
//! clamped to 16 bits. A sample ends with the frame whose end bit is set,
//! which is played; with the repeat bit too the voice jumps back to the last
//! loop-start frame, keeping its filter history.

pub const FRAME: usize = 16;
pub const PER_FRAME: usize = 28;

pub const END: u8 = 0x01;
pub const REPEAT: u8 = 0x02;
pub const LOOP_START: u8 = 0x04;

const F0: [i32; 5] = [0, 60, 115, 98, 122];
const F1: [i32; 5] = [0, 0, -52, -55, -60];

/// The filter history a decoder carries from sample to sample and frame to
/// frame: the last two outputs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct History {
    pub s1: i32,
    pub s2: i32,
}

/// Decode one 16-byte frame into `out`; returns how many samples hit the
/// 16-bit clamp. `frame` must be at least 16 bytes.
pub fn decode_frame(frame: &[u8], hist: &mut History, out: &mut [i16; PER_FRAME]) -> usize {
    let head = frame[0];
    let mut shift = (head & 15) as u32;
    if shift > 12 {
        shift = 9;
    }
    let f = ((head >> 4) as usize).min(4);
    let (f0, f1) = (F0[f], F1[f]);
    let (mut s1, mut s2) = (hist.s1, hist.s2);
    let mut clipped = 0;
    for (i, slot) in out.iter_mut().enumerate() {
        let b = frame[2 + i / 2];
        let n = if i & 1 == 0 { b & 15 } else { b >> 4 } as i32;
        let n = if n & 8 != 0 { n - 16 } else { n };
        let t = (n << 12) >> shift;
        let mut s = t + ((s1 * f0 + s2 * f1 + 32) >> 6);
        if s > 32767 {
            s = 32767;
            clipped += 1;
        } else if s < -32768 {
            s = -32768;
            clipped += 1;
        }
        *slot = s as i16;
        s2 = s1;
        s1 = s;
    }
    *hist = History { s1, s2 };
    clipped
}

/// One decoded sample, as `adpcm.decode` returns it.
#[derive(Clone, Debug, Default)]
pub struct Decoded {
    pub pcm: Vec<i16>,
    /// `(start, end)` in samples, end exclusive: from the last loop-start
    /// frame to the end of the frame with end + repeat.
    pub loop_range: Option<(usize, usize)>,
    /// Frames consumed, the end frame included.
    pub frames: usize,
    /// Samples that hit the 16-bit clamp.
    pub clipped: usize,
}

/// Decode from the start of `data` up to and including the first frame with
/// the end bit, or to the end of `data`.
pub fn decode(data: &[u8]) -> Decoded {
    let mut out = Decoded::default();
    let mut hist = History::default();
    let mut buf = [0i16; PER_FRAME];
    let mut loop_start = None;
    let mut p = 0;
    while p + FRAME <= data.len() {
        let flags = data[p + 1];
        if flags & LOOP_START != 0 {
            loop_start = Some(out.pcm.len());
        }
        out.clipped += decode_frame(&data[p..p + FRAME], &mut hist, &mut buf);
        out.pcm.extend_from_slice(&buf);
        p += FRAME;
        if flags & END != 0 {
            if flags & REPEAT != 0
                && let Some(s) = loop_start
            {
                out.loop_range = Some((s, out.pcm.len()));
            }
            break;
        }
    }
    out.frames = p / FRAME;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame from 28 signed nibbles, low nibble first.
    fn frame(shift: u8, filter: u8, flags: u8, nibbles: &[i32]) -> Vec<u8> {
        let mut f = vec![shift | filter << 4, flags];
        for pair in nibbles.chunks(2) {
            f.push((pair[0] as u8 & 15) | (pair[1] as u8 & 15) << 4);
        }
        f
    }

    #[test]
    fn filter0_shift12_is_the_nibbles() {
        let mut nib = vec![1, 2, -1, -8, 7, 0];
        nib.resize(28, 3);
        let d = decode(&frame(12, 0, END, &nib));
        assert_eq!(d.pcm.iter().map(|&s| s as i32).collect::<Vec<_>>(), nib);
        assert_eq!((d.frames, d.loop_range), (1, None));
    }

    #[test]
    fn filter1_by_hand() {
        let mut nib = vec![1];
        nib.resize(28, 0);
        let d = decode(&frame(0, 1, END, &nib));
        assert_eq!(&d.pcm[..4], &[4096, 3840, 3600, 3375]);
    }

    #[test]
    fn clamp_and_shift_13() {
        let mut nib = vec![7, 7];
        nib.resize(28, 0);
        let d = decode(&frame(0, 4, END, &nib));
        assert_eq!(&d.pcm[..2], &[28672, 32767]);
        assert_eq!(d.clipped, d.pcm.iter().filter(|&&s| s == 32767 || s == -32768).count());
        let a = decode(&frame(13, 0, END, &[5; 28])).pcm;
        let b = decode(&frame(9, 0, END, &[5; 28])).pcm;
        assert_eq!(a, b);
    }

    #[test]
    fn loop_points() {
        let mut data = frame(12, 0, 0, &[1; 28]);
        data.extend(frame(12, 0, LOOP_START, &[2; 28]));
        data.extend(frame(12, 0, END | REPEAT, &[3; 28]));
        data.extend(frame(12, 0, 0, &[4; 28]));
        let d = decode(&data);
        assert_eq!((d.frames, d.loop_range, d.pcm.len()), (3, Some((28, 84)), 84));
    }
}
