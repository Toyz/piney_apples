//! Not the game's: `--webp OUT.webp`, a `--shot` run's frames as one
//! animated WebP, for sharing what the port draws. Each frame lasts the
//! vertical blanks the game spent on it (the mode's frame rate, times the
//! frames skipped between two kept ones) at 59.94 a second; lossy, at the
//! quality asked for.

use webp_animation::{Encoder, EncoderOptions, EncodingConfig, EncodingType, LossyEncodingConfig};

/// NTSC's vertical blanks a second, the game's clock.
const VBLANK_HZ: f64 = 60_000.0 / 1001.0;

pub struct Recorder {
    encoder: Encoder,
    width: u32,
    height: u32,
    /// Vertical blanks from the first frame to the one being added.
    vblanks: u64,
    frames: usize,
}

impl Recorder {
    pub fn new(width: u32, height: u32, quality: f32) -> Result<Recorder, String> {
        // Every frame a key frame: between key frames libwebp marks the
        // pixels it takes for unchanged transparent, and in lossy frames the
        // error builds up into blocks over the picture.
        let options = EncoderOptions {
            kmin: 0,
            kmax: 1,
            encoding_config: Some(EncodingConfig {
                quality,
                encoding_type: EncodingType::Lossy(LossyEncodingConfig::default()),
                ..EncodingConfig::default()
            }),
            ..EncoderOptions::default()
        };
        let encoder = Encoder::new_with_options((width, height), options).map_err(|e| format!("webp: {e:?}"))?;
        Ok(Recorder { encoder, width, height, vblanks: 0, frames: 0 })
    }

    fn ms(&self) -> i32 {
        (self.vblanks as f64 * 1000.0 / VBLANK_HZ).round() as i32
    }

    /// A frame (`rgba`, top row first, the recorder's size), shown after
    /// `vblanks` since the frame before it (none for the first).
    pub fn add(&mut self, rgba: &[u8], vblanks: u32) -> Result<(), String> {
        if self.frames > 0 {
            self.vblanks += u64::from(vblanks);
        }
        if rgba.len() != (self.width * self.height * 4) as usize {
            return Err(format!("webp: a {}-byte frame for {}x{}", rgba.len(), self.width, self.height));
        }
        let at = self.ms();
        self.encoder.add_frame(rgba, at).map_err(|e| format!("webp frame {}: {e:?}", self.frames))?;
        self.frames += 1;
        Ok(())
    }

    /// The file, the last frame held for `vblanks`; its frame count.
    pub fn finish(mut self, path: &str, vblanks: u32) -> Result<usize, String> {
        self.vblanks += u64::from(vblanks);
        let end = self.ms();
        let data = self.encoder.finalize(end).map_err(|e| format!("webp: {e:?}"))?;
        std::fs::write(path, &*data).map_err(|e| format!("{path}: {e}"))?;
        Ok(self.frames)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Three 8x8 frames, two game frames apart, the last held for two
    /// more: a looping RIFF WEBP whose frames (ANMF, 24-bit durations at
    /// +12 of each) add up to the six frames' 100 ms at 59.94 a second.
    #[test]
    fn frames_timed_on_the_game_clock() {
        let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/shots".into());
        let path = format!("{dir}/recorder_test.webp");
        let mut r = Recorder::new(8, 8, 80.0).unwrap();
        for k in 0..3u8 {
            let rgba: Vec<u8> = (0..64).flat_map(|i| [k * 80, i as u8 * 4, 255 - k * 80, 255]).collect();
            r.add(&rgba, 2).unwrap();
        }
        assert_eq!(r.finish(&path, 2).unwrap(), 3);
        let d = std::fs::read(&path).unwrap();
        assert_eq!((&d[..4], &d[8..12]), (&b"RIFF"[..], &b"WEBP"[..]));
        let (mut i, mut total, mut frames) = (12, 0u32, 0);
        while i + 8 <= d.len() {
            let n = u32::from_le_bytes(d[i + 4..i + 8].try_into().unwrap()) as usize;
            if &d[i..i + 4] == b"ANMF" {
                let b = &d[i + 8..];
                total += u32::from(b[12]) | u32::from(b[13]) << 8 | u32::from(b[14]) << 16;
                frames += 1;
            }
            i += 8 + n + (n & 1);
        }
        assert_eq!(frames, 3);
        assert_eq!(total, (6.0 * 1000.0 / VBLANK_HZ).round() as u32);
        let _ = std::fs::remove_file(&path);
    }
}
