//! A 16-bit PCM WAV writer for headless renders, as `tools/wav.py` writes
//! them (no `smpl` chunk).

use std::io::{self, Write};

/// `pcm` is interleaved when `channels` > 1.
pub fn encode(pcm: &[i16], rate: u32, channels: u16) -> Vec<u8> {
    let data_len = (pcm.len() * 2) as u32;
    let block = 2 * channels as u32;
    let mut out = Vec::with_capacity(44 + pcm.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * block).to_le_bytes());
    out.extend_from_slice(&(block as u16).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

pub fn write(path: impl AsRef<std::path::Path>, pcm: &[i16], rate: u32, channels: u16) -> io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    f.write_all(&encode(pcm, rate, channels))
}

#[cfg(test)]
mod tests {
    #[test]
    fn header() {
        let w = super::encode(&[1, -1, 2, -2], 48_000, 2);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(w[4..8].try_into().unwrap()), 36 + 8);
        assert_eq!(u32::from_le_bytes(w[24..28].try_into().unwrap()), 48_000);
        assert_eq!(w.len(), 44 + 8);
    }
}
