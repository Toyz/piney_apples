//! `--dvd [SPEED]`: the original's disc reads timed, so the loading
//! display between areas stays up about as long as it did on a PS2. The
//! port's own reads take no time (the files are in memory), so without
//! this the card only shows its 47 frames over the new area's fade in
//! (`crate::loaddisp`).
//!
//! The time is an estimate, not a measurement. It counts the archive
//! members the scene's set-up reads that the last scene's did not
//! (`ccFileListLoad` loads only those, `loadCheck` main 0x00165530): a seek
//! for each run of them lying end to end on the disc, then their bytes at
//! SPEED times DVD 1x (1,385,000 bytes a second). The PS2's drive reads DVD
//! at up to 4x, at constant angular velocity, so slower near the centre of
//! the disc; the default is 3x. The EE's inflate of each file is not
//! counted.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

/// DVD 1x, in bytes a second.
const DVD_1X: f64 = 1_385_000.0;
/// The speed `--dvd` alone takes.
pub const DEFAULT_SPEED: f64 = 3.0;
/// An estimate of one seek (a DVD drive's random access is 100-150 ms).
const SEEK: f64 = 0.12;
const FRAME_RATE: f64 = 60.0;

/// The disc's timing, when `--dvd` asked for it.
#[derive(Clone, Copy, Debug)]
pub struct Dvd {
    /// Bytes a second.
    pub rate: f64,
    /// Seconds a file.
    pub seek: f64,
}

static DVD: OnceLock<Dvd> = OnceLock::new();

/// `--dvd SPEED` for this run.
pub fn set(speed: f64) {
    let _ = DVD.set(Dvd { rate: speed.max(0.1) * DVD_1X, seek: SEEK });
}

/// The timing, when on.
pub fn get() -> Option<Dvd> {
    DVD.get().copied()
}

impl Dvd {
    /// Frames to read `files` (name: offset and length in the archive),
    /// leaving out those already `resident` from the last scene.
    pub fn frames(&self, files: &BTreeMap<String, (usize, usize)>, resident: &BTreeSet<String>) -> u32 {
        let mut new: Vec<(usize, usize)> =
            files.iter().filter(|(n, _)| !resident.contains(*n)).map(|(_, &at)| at).collect();
        new.sort();
        let seeks = new.iter().enumerate().filter(|&(i, &(o, _))| i == 0 || new[i - 1].0 + new[i - 1].1 != o).count();
        let bytes: usize = new.iter().map(|&(_, l)| l).sum();
        let secs = seeks as f64 * self.seek + bytes as f64 / self.rate;
        (secs * FRAME_RATE).ceil() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_new_files_take_time() {
        let dvd = Dvd { rate: 3.0 * DVD_1X, seek: SEEK };
        let at = |n: &str, o, l| (n.to_string(), (o, l));
        // One second of reading; the first two end to end, one seek, the
        // third apart, another.
        let files: BTreeMap<String, (usize, usize)> =
            [at("field_a.cmp", 0, 4_000_000), at("bg_a1.cmp", 4_000_000, 155_000), at("x104.cmp", 9_000_000, 0)]
                .into_iter()
                .collect();
        assert_eq!(dvd.frames(&files, &Default::default()), 75);
        let resident = ["field_a.cmp".to_string()].into_iter().collect();
        assert_eq!(dvd.frames(&files, &resident), 17);
        assert_eq!(dvd.frames(&BTreeMap::new(), &Default::default()), 0);
    }
}
