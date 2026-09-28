//! SEWORDS.IRX's channel 0 streaming a voice line: `wordPlay` (module offset
//! 0x2c74) sets it up, the thread `_BgmPlay` (0x1fb4) refills it on each
//! transfer interrupt, and the SPU2's auto-DMA plays it into core 0's
//! sound-data input at 48 kHz (`docs/formats/voice.md`). The model keeps the
//! module's own buffer (`BgmInit(0, 0xc000)`) and runs its steps on it byte
//! for byte, so a line plays every packet but its last, `4096 * (ceil(size /
//! 8192) - 1)` samples, and one of 16 KiB to 32 KiB plays its first two
//! packets twice: checked against the module in `tools/eemu.py`.

/// `BgmInit(ch, 0xc000)`: the buffer `wordPlay` allocates.
pub const ALLOC: usize = 0xc000;
/// `gSPacketSize` in mono mode (`BgmSetMode(ch, 0x10)`): a third of the
/// buffer, one half of the SPU buffer.
pub const SPU_PACKET: usize = ALLOC / 3;
/// `gRPacketSize`: the bytes read from the disc per half.
pub const RAW_PACKET: usize = SPU_PACKET / 2;
/// Samples in a half: 16 blocks of 256.
pub const HALF_SAMPLES: usize = SPU_PACKET / 4;
/// `gBuffRaw = gBuffSpu + 2 * gSPacketSize`.
const RAW: usize = 2 * SPU_PACKET;
/// The interrupts `_BgmPlay` lets pass after the end is flagged.
const TERMINATE: u32 = 2;
/// With `usedrive` 2 (the EE's `sewordCmd(0x130, 2)`, a DVD) the last
/// packet is read to a whole sector.
const SECTOR: i32 = 2048;

/// The bytes of the voice file from the line's offset on that the stream
/// can read: the first four packets, or the line to its sector's end.
pub fn read_len(size: i32) -> usize {
    (4 * RAW_PACKET).max((size.max(0) as usize).div_ceil(SECTOR as usize) * SECTOR as usize)
}

/// Channel 0 streaming one line.
pub struct Word {
    /// `gBuffSpu` then `gBuffRaw`.
    mem: Vec<u8>,
    /// The file from the line's offset on (see [`read_len`]).
    file: Vec<u8>,
    /// The file position, from the line's offset.
    fp: usize,
    /// `gWave[0]`: the line's size and the bytes counted read.
    size: i32,
    pos: i32,
    /// `gBgmMode & 0x8000`.
    end: bool,
    terminate: u32,
    /// The half transferring, and the sample within it.
    half: usize,
    at: usize,
    stopped: bool,
}

impl Word {
    /// `wordPlay`'s set-up, from `BgmInit` to `BgmStart`: `None` when
    /// `BgmPreLoad`'s reads come up short (the module then closes the file
    /// and plays nothing).
    pub fn open(file: Vec<u8>, size: i32) -> Option<Word> {
        let mut w = Word {
            mem: vec![0; ALLOC],
            file,
            fp: 0,
            size,
            pos: 0,
            end: false,
            terminate: 0,
            half: 0,
            at: 0,
            stopped: false,
        };
        let two = 2 * RAW_PACKET as i32;
        if w.read(RAW, two) != two {
            return None;
        }
        w.pos += two;
        w.raw_to_spu(0);
        w.raw_to_spu(1);
        if two < w.size - w.pos {
            if w.read(RAW, two) != two {
                return None;
            }
            w.pos += two;
        }
        Some(w)
    }

    /// The next sample into the sound-data input, `None` once the thread
    /// has stopped the transfer.
    pub fn next_sample(&mut self) -> Option<(i16, i16)> {
        if self.stopped {
            return None;
        }
        if self.at == HALF_SAMPLES {
            let played = self.half;
            self.half ^= 1;
            self.at = 0;
            self.interrupt(played);
            if self.stopped {
                return None;
            }
        }
        let b = self.half * SPU_PACKET + (self.at / 256) * 1024 + (self.at % 256) * 2;
        let l = i16::from_le_bytes([self.mem[b], self.mem[b + 1]]);
        let r = i16::from_le_bytes([self.mem[b + 512], self.mem[b + 513]]);
        self.at += 1;
        Some((l, r))
    }

    pub fn stopped(&self) -> bool {
        self.stopped
    }

    /// `read(gFd, buf, n)`: what is there, none for a size of 0 or less.
    /// On the console a negative size (a line under 16 KiB: the skill-word
    /// tables' closing `{0, 0}` rows) goes to CDVDMAN's blocking stream
    /// read, which by its code never returns and copies past the buffer
    /// (`docs/formats/voice.md`); here the line ends instead.
    fn read(&mut self, at: usize, n: i32) -> i32 {
        if n <= 0 {
            return 0;
        }
        let n = (n as usize).min(self.file.len().saturating_sub(self.fp));
        self.mem[at..at + n].copy_from_slice(&self.file[self.fp..self.fp + n]);
        self.fp += n;
        n as i32
    }

    /// `BgmRaw2Spu(ch, h, mode)` in mono (`_BgmRaw2SpuMono`, 0xaf0).
    fn raw_to_spu(&mut self, h: usize) {
        for b in 0..SPU_PACKET / 1024 {
            let src = RAW + h * RAW_PACKET + b * 512;
            let dst = h * SPU_PACKET + b * 1024;
            self.mem.copy_within(src..src + 512, dst);
            self.mem.copy_within(src..src + 512, dst + 512);
        }
    }

    /// `_BgmPlay`'s loop once half `h` has played (`1 -
    /// sceSdBlockTransStatus >> 24`).
    fn interrupt(&mut self, h: usize) {
        self.raw_to_spu(h);
        if self.end {
            if self.terminate < TERMINATE {
                self.terminate += 1;
            } else {
                // _BgmStop, BgmSetVolumeDirect(ch, 0), loopEnd.
                self.stopped = true;
            }
            return;
        }
        let packet = RAW_PACKET as i32;
        let at = RAW + h * RAW_PACKET;
        let left = self.size - self.pos;
        if packet < left {
            let n = self.read(at, packet);
            // "retry": a short read leaves the half for the next interrupt.
            if n < packet {
                return;
            }
            self.pos += n;
            return;
        }
        let mut n = left;
        if n % SECTOR != 0 {
            n = (n / SECTOR + 1) * SECTOR;
        }
        if self.read(at, n) < n {
            return;
        }
        // Not looping (mode 0x10): the packet's rest zeroed, the end flagged.
        let from = (at as i64 + n as i64) as usize;
        let count = ((packet - n) >> 1) as usize;
        self.mem[from..from + 2 * count].fill(0);
        self.end = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(samples: usize) -> Vec<u8> {
        (0..samples).flat_map(|i| ((i % 30000) as i16 + 1).to_le_bytes()).collect()
    }

    fn play(file: Vec<u8>, size: i32) -> Vec<i16> {
        let mut w = Word::open(file, size).unwrap();
        let mut out = Vec::new();
        while let Some((l, r)) = w.next_sample() {
            assert_eq!(l, r);
            out.push(l);
        }
        out
    }

    #[test]
    fn every_packet_but_the_last() {
        for size in [40_756, 40_960, 49_152, 49_154, 441_604] {
            let file = line(read_len(size) / 2);
            let heard = play(file.clone(), size);
            let packets = (size as usize).div_ceil(RAW_PACKET) - 1;
            assert_eq!(heard.len(), packets * HALF_SAMPLES, "{size}");
            let want: Vec<i16> = file.as_chunks::<2>().0.iter().map(|c| i16::from_le_bytes(*c)).collect();
            assert_eq!(heard[..], want[..heard.len()], "{size}");
        }
    }

    #[test]
    fn a_short_line_plays_its_first_two_packets_twice() {
        let file = line(read_len(30_000) / 2);
        let heard = play(file, 30_000);
        let p = HALF_SAMPLES;
        assert_eq!(heard.len(), 5 * p);
        assert_eq!(heard[..2 * p], heard[2 * p..4 * p]);
    }

    #[test]
    fn a_short_file_plays_nothing() {
        assert!(Word::open(vec![0; 1000], 1000).is_none());
    }
}
