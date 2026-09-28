//! The MPEG-2 video decoder (ISO/IEC 13818-2), for what the PSS streams
//! use: main profile, 4:2:0, progressive frame pictures with
//! `frame_pred_frame_dct` set (so every prediction is a frame prediction
//! and every DCT a frame DCT), I, P and B pictures, loaded intra matrices,
//! the non-linear quantiser scale, both intra VLC tables and both scans.
//! Field pictures, field or dual-prime prediction, concealment motion
//! vectors and the scalable extensions are refused with an error; no PSS
//! on the Infection disc uses them (`docs/formats/pss.md`).

use piney_data::{Error, Result};

use crate::bits::Bits;
use crate::idct;
use crate::vlc::{
    ADDR_ESCAPE, ADDR_STUFFING, DCT_EOB, DCT_ESCAPE, MB_BACKWARD, MB_FORWARD, MB_INTRA, MB_PATTERN, MB_QUANT, Vlc,
    tables,
};

fn err<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error::Format(format!("mpeg: {}", msg.into())))
}

/// The zig-zag scan (Figure 7-2) and the alternate scan (Figure 7-3):
/// scan position to coefficient index, row-major.
const SCAN: [[u8; 64]; 2] = [
    [
        0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20, 13, 6, 7, 14,
        21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59, 52, 45, 38, 31, 39, 46, 53, 60,
        61, 54, 47, 55, 62, 63,
    ],
    [
        0, 8, 16, 24, 1, 9, 2, 10, 17, 25, 32, 40, 48, 56, 57, 49, 41, 33, 26, 18, 3, 11, 4, 12, 19, 27, 34, 42, 50,
        58, 35, 43, 51, 59, 20, 28, 5, 13, 6, 14, 21, 29, 36, 44, 52, 60, 37, 45, 53, 61, 22, 30, 7, 15, 23, 31, 38,
        46, 54, 62, 39, 47, 55, 63,
    ],
];

/// The default intra quantiser matrix (6.3.11), row-major.
const DEFAULT_INTRA: [u8; 64] = [
    8, 16, 19, 22, 26, 27, 29, 34, 16, 16, 22, 24, 27, 29, 34, 37, 19, 22, 26, 27, 29, 34, 34, 38, 22, 22, 26, 27, 29,
    34, 37, 40, 22, 26, 27, 29, 32, 35, 40, 48, 26, 27, 29, 32, 35, 40, 48, 58, 26, 27, 29, 34, 38, 46, 56, 69, 27, 29,
    35, 38, 46, 56, 69, 83,
];

/// quantiser_scale for q_scale_type 1 (Table 7-6).
const NONLINEAR_SCALE: [u8; 32] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 18, 20, 22, 24, 28, 32, 36, 40, 44, 48, 52, 56, 64, 72, 80, 88, 96, 104,
    112,
];

/// The sequence header and sequence extension.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sequence {
    pub width: u16,
    pub height: u16,
    /// aspect_ratio_information: 1 square samples.
    pub aspect: u8,
    pub frame_rate_code: u8,
    /// bit_rate in 400 bit/s units, with its extension.
    pub bit_rate: u32,
    pub vbv_buffer_size: u32,
    /// The intra and non-intra matrices, row-major.
    pub intra_matrix: [u8; 64],
    pub non_intra_matrix: [u8; 64],
    /// Whether the intra matrix was loaded (not the default).
    pub loaded_intra: bool,
    pub loaded_non_intra: bool,
    /// Whether a sequence extension came (MPEG-2).
    pub mpeg2: bool,
    pub profile_level: u8,
    pub progressive: bool,
    /// chroma_format: 1 is 4:2:0.
    pub chroma_format: u8,
    pub low_delay: bool,
    pub frame_rate_ext: (u8, u8),
}

impl Sequence {
    /// Frames a second as a fraction (Table 6-4 with the extension's n, d).
    pub fn frame_rate(&self) -> (u32, u32) {
        let (n, d) = match self.frame_rate_code {
            1 => (24000, 1001),
            2 => (24, 1),
            3 => (25, 1),
            4 => (30000, 1001),
            5 => (30, 1),
            6 => (50, 1),
            7 => (60000, 1001),
            8 => (60, 1),
            _ => (0, 1),
        };
        (n * (u32::from(self.frame_rate_ext.0) + 1), d * (u32::from(self.frame_rate_ext.1) + 1))
    }
}

/// picture_coding_type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    I = 1,
    P = 2,
    B = 3,
}

/// The picture header and picture coding extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PictureHeader {
    pub temporal_reference: u16,
    pub kind: Kind,
    /// f_code[s][t]: s 0 forward, 1 backward; t 0 horizontal, 1 vertical.
    pub f_code: [[u8; 2]; 2],
    pub intra_dc_precision: u8,
    pub picture_structure: u8,
    pub top_field_first: bool,
    pub frame_pred_frame_dct: bool,
    pub concealment_motion_vectors: bool,
    pub q_scale_type: bool,
    pub intra_vlc_format: bool,
    pub alternate_scan: bool,
    pub repeat_first_field: bool,
    pub progressive_frame: bool,
}

/// A decoded picture, in display order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub width: usize,
    pub height: usize,
    /// Y, `width` x `height`, row-major.
    pub y: Vec<u8>,
    /// Cb and Cr, `(width + 1) / 2` x `(height + 1) / 2`.
    pub cb: Vec<u8>,
    pub cr: Vec<u8>,
    pub header: PictureHeader,
    /// Its place in decode order, from 0.
    pub decode_index: u32,
}

impl Picture {
    /// The picture as planar YUV 4:2:0 (Y, then Cb, then Cr), the layout
    /// of ffmpeg's `-pix_fmt yuv420p`.
    pub fn yuv420p(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.y.len() + 2 * self.cb.len());
        out.extend_from_slice(&self.y);
        out.extend_from_slice(&self.cb);
        out.extend_from_slice(&self.cr);
        out
    }
}

/// A frame being decoded or kept for reference: whole macroblocks.
#[derive(Clone)]
struct Frame {
    y: Vec<u8>,
    cb: Vec<u8>,
    cr: Vec<u8>,
    header: PictureHeader,
    decode_index: u32,
}

/// The geometry of the frames: macroblocks across and down, strides.
#[derive(Clone, Copy)]
struct Geometry {
    mb_width: usize,
    mb_height: usize,
    stride: usize,
    cstride: usize,
    lines: usize,
    clines: usize,
}

impl Geometry {
    fn new(seq: &Sequence) -> Geometry {
        let mb_width = usize::from(seq.width).div_ceil(16);
        let mb_height = usize::from(seq.height).div_ceil(16);
        Geometry {
            mb_width,
            mb_height,
            stride: mb_width * 16,
            cstride: mb_width * 8,
            lines: mb_height * 16,
            clines: mb_height * 8,
        }
    }

    fn frame(&self, header: PictureHeader, decode_index: u32) -> Frame {
        Frame {
            y: vec![0; self.stride * self.lines],
            cb: vec![0; self.cstride * self.clines],
            cr: vec![0; self.cstride * self.clines],
            header,
            decode_index,
        }
    }
}

/// The decoder over a whole video elementary stream.
pub struct Decoder {
    data: Vec<u8>,
    /// The index of the code byte of every start code (`00 00 01 xx`).
    starts: Vec<usize>,
    next: usize,
    seq: Option<Sequence>,
    /// The older and the newer reference picture.
    past: Option<Frame>,
    future: Option<Frame>,
    /// Whether `future` has been output.
    future_shown: bool,
    decoded: u32,
    output: u32,
}

impl Decoder {
    pub fn new(data: Vec<u8>) -> Decoder {
        let mut starts = Vec::new();
        let mut i = 2;
        while i < data.len() {
            if data[i] > 1 {
                i += 3;
            } else if data[i] == 1 && data[i - 1] == 0 && data[i - 2] == 0 {
                if i + 1 < data.len() {
                    starts.push(i + 1);
                }
                i += 3;
            } else {
                i += 1;
            }
        }
        Decoder {
            data,
            starts,
            next: 0,
            seq: None,
            past: None,
            future: None,
            future_shown: true,
            decoded: 0,
            output: 0,
        }
    }

    /// The sequence header, once one has been read.
    pub fn sequence(&self) -> Option<&Sequence> {
        self.seq.as_ref()
    }

    /// Pictures decoded so far (decode order).
    pub fn decoded(&self) -> u32 {
        self.decoded
    }

    /// Pictures returned so far (display order).
    pub fn output(&self) -> u32 {
        self.output
    }

    /// Read the headers ahead of the first picture, so [`Decoder::sequence`]
    /// (with its extensions) has them before any picture is decoded.
    pub fn read_sequence(&mut self) -> Result<&Sequence> {
        while let Some(&at) = self.starts.get(self.next) {
            match self.data[at] {
                0xb3 | 0xb5 | 0xb2 => self.header_unit(at)?,
                _ => break,
            }
            self.next += 1;
        }
        match &self.seq {
            Some(s) => Ok(s),
            None => err("no sequence header before the first picture"),
        }
    }

    /// The bytes of the unit whose code byte is at `starts[i]`, after the
    /// code byte, up to the next start code.
    fn unit(&self, i: usize) -> &[u8] {
        let at = self.starts[i] + 1;
        let end = self.starts.get(i + 1).map_or(self.data.len(), |&n| n - 4 + 1);
        &self.data[at.min(end)..end]
    }

    fn header_unit(&mut self, at: usize) -> Result<()> {
        let unit = self.unit(self.next).to_vec();
        match self.data[at] {
            0xb3 => self.sequence_header(&unit),
            0xb5 => {
                let mut b = Bits::new(&unit);
                match b.read(4) {
                    1 => self.sequence_extension(&mut b),
                    3 => self.quant_matrix_extension(&mut b),
                    // Sequence display (2) and the others: nothing the
                    // decode needs.
                    _ => Ok(()),
                }
            }
            _ => Ok(()),
        }
    }

    fn sequence_header(&mut self, unit: &[u8]) -> Result<()> {
        let mut b = Bits::new(unit);
        let width = b.read(12) as u16;
        let height = b.read(12) as u16;
        let aspect = b.read(4) as u8;
        let frame_rate_code = b.read(4) as u8;
        let bit_rate = b.read(18);
        b.skip(1);
        let vbv_buffer_size = b.read(10);
        b.skip(1);
        let mut intra_matrix = DEFAULT_INTRA;
        let mut non_intra_matrix = [16u8; 64];
        let loaded_intra = b.bit();
        if loaded_intra {
            for &pos in &SCAN[0] {
                intra_matrix[pos as usize] = b.read(8) as u8;
            }
        }
        let loaded_non_intra = b.bit();
        if loaded_non_intra {
            for &pos in &SCAN[0] {
                non_intra_matrix[pos as usize] = b.read(8) as u8;
            }
        }
        if b.overrun() || width == 0 || height == 0 {
            return err("short sequence header");
        }
        if intra_matrix.contains(&0) || non_intra_matrix.contains(&0) {
            return err("a zero in a quantiser matrix");
        }
        let (mpeg2, profile_level, progressive, chroma_format, low_delay, frame_rate_ext) = match &self.seq {
            Some(s) => (s.mpeg2, s.profile_level, s.progressive, s.chroma_format, s.low_delay, s.frame_rate_ext),
            None => (false, 0, true, 1, false, (0, 0)),
        };
        if self.seq.as_ref().is_some_and(|s| (s.width, s.height) != (width, height)) {
            return err("the picture size changes mid-stream");
        }
        self.seq = Some(Sequence {
            width,
            height,
            aspect,
            frame_rate_code,
            bit_rate,
            vbv_buffer_size,
            intra_matrix,
            non_intra_matrix,
            loaded_intra,
            loaded_non_intra,
            mpeg2,
            profile_level,
            progressive,
            chroma_format,
            low_delay,
            frame_rate_ext,
        });
        Ok(())
    }

    fn sequence_extension(&mut self, b: &mut Bits) -> Result<()> {
        let Some(seq) = self.seq.as_mut() else {
            return err("a sequence extension before the sequence header");
        };
        seq.profile_level = b.read(8) as u8;
        seq.progressive = b.bit();
        seq.chroma_format = b.read(2) as u8;
        let hx = b.read(2) as u16;
        let vx = b.read(2) as u16;
        seq.width |= hx << 12;
        seq.height |= vx << 12;
        seq.bit_rate |= b.read(12) << 18;
        b.skip(1);
        seq.vbv_buffer_size |= b.read(8) << 10;
        seq.low_delay = b.bit();
        seq.frame_rate_ext = (b.read(2) as u8, b.read(5) as u8);
        seq.mpeg2 = true;
        if seq.chroma_format != 1 {
            return err(format!("chroma_format {} (only 4:2:0)", seq.chroma_format));
        }
        Ok(())
    }

    fn quant_matrix_extension(&mut self, b: &mut Bits) -> Result<()> {
        let Some(seq) = self.seq.as_mut() else {
            return err("a quant matrix extension before the sequence header");
        };
        for matrix in [&mut seq.intra_matrix, &mut seq.non_intra_matrix] {
            if b.bit() {
                for &pos in &SCAN[0] {
                    matrix[pos as usize] = b.read(8) as u8;
                }
            }
        }
        // The chroma matrices are for 4:2:2 and 4:4:4 only.
        Ok(())
    }

    /// The next picture in display order, or None at the end of the
    /// stream.
    pub fn next_picture(&mut self) -> Result<Option<Picture>> {
        loop {
            let Some(&at) = self.starts.get(self.next) else {
                return Ok(self.flush());
            };
            match self.data[at] {
                0x00 => {
                    if let Some(p) = self.picture()? {
                        return Ok(Some(p));
                    }
                }
                0xb7 => {
                    // sequence_end_code: the last reference is shown.
                    self.next += 1;
                    if let Some(p) = self.flush() {
                        return Ok(Some(p));
                    }
                }
                0x01..=0xaf => return err(format!("a slice outside a picture at 0x{at:x}")),
                _ => {
                    self.header_unit(at)?;
                    self.next += 1;
                }
            }
        }
    }

    fn flush(&mut self) -> Option<Picture> {
        if self.future_shown {
            return None;
        }
        self.future_shown = true;
        let seq = self.seq.as_ref()?;
        let p = self.future.as_ref().map(|f| crop(seq, f));
        self.output += p.is_some() as u32;
        p
    }

    /// Decode the picture whose header is at `starts[next]`; its display
    /// order output, if any.
    fn picture(&mut self) -> Result<Option<Picture>> {
        let Some(seq) = self.seq.clone() else {
            return err("a picture before the sequence header");
        };
        let mut b = Bits::new(self.unit(self.next));
        let temporal_reference = b.read(10) as u16;
        let kind = match b.read(3) {
            1 => Kind::I,
            2 => Kind::P,
            3 => Kind::B,
            k => return err(format!("picture_coding_type {k}")),
        };
        self.next += 1;
        let mut h = PictureHeader {
            temporal_reference,
            kind,
            f_code: [[15; 2]; 2],
            intra_dc_precision: 0,
            picture_structure: 3,
            top_field_first: false,
            frame_pred_frame_dct: true,
            concealment_motion_vectors: false,
            q_scale_type: false,
            intra_vlc_format: false,
            alternate_scan: false,
            repeat_first_field: false,
            progressive_frame: true,
        };
        let mut coding_ext = false;
        // The picture's extensions and user data, then its slices.
        while let Some(&at) = self.starts.get(self.next) {
            match self.data[at] {
                0xb5 => {
                    let unit = self.unit(self.next);
                    let mut b = Bits::new(unit);
                    match b.read(4) {
                        8 => {
                            for s in 0..2 {
                                for t in 0..2 {
                                    h.f_code[s][t] = b.read(4) as u8;
                                }
                            }
                            h.intra_dc_precision = b.read(2) as u8;
                            h.picture_structure = b.read(2) as u8;
                            h.top_field_first = b.bit();
                            h.frame_pred_frame_dct = b.bit();
                            h.concealment_motion_vectors = b.bit();
                            h.q_scale_type = b.bit();
                            h.intra_vlc_format = b.bit();
                            h.alternate_scan = b.bit();
                            h.repeat_first_field = b.bit();
                            b.skip(1);
                            h.progressive_frame = b.bit();
                            coding_ext = true;
                        }
                        3 => {
                            let unit = unit.to_vec();
                            self.quant_matrix_extension(&mut Bits::new(&unit[..]).skipped(4))?;
                        }
                        _ => {}
                    }
                }
                0xb2 => {}
                _ => break,
            }
            self.next += 1;
        }
        if !coding_ext {
            return err("an MPEG-1 picture (no picture coding extension)");
        }
        if h.picture_structure != 3 {
            return err("a field picture");
        }
        if !h.frame_pred_frame_dct {
            return err("frame_pred_frame_dct 0 (field prediction and field DCT)");
        }
        if h.concealment_motion_vectors {
            return err("concealment motion vectors");
        }
        let seq = self.seq.clone().unwrap_or(seq);
        let g = Geometry::new(&seq);
        let mut cur = g.frame(h, self.decoded);
        let (fwd, bwd) = match kind {
            Kind::I => (None, None),
            Kind::P => (self.future.as_ref(), None),
            Kind::B => (self.past.as_ref(), self.future.as_ref()),
        };
        if kind == Kind::P && fwd.is_none() || kind == Kind::B && (fwd.is_none() || bwd.is_none()) {
            return err("a predicted picture without its reference");
        }
        let ctx = Ctx { seq: &seq, h: &h, g, fwd, bwd };
        while let Some(&at) = self.starts.get(self.next) {
            let code = self.data[at];
            if !(0x01..=0xaf).contains(&code) {
                break;
            }
            let row = usize::from(code) - 1;
            if row >= g.mb_height {
                return err(format!("slice row {row} below the picture"));
            }
            ctx.slice(&mut cur, self.unit(self.next), row)?;
            self.next += 1;
        }
        self.decoded += 1;
        let out = match kind {
            Kind::B => Some(crop(&seq, &cur)),
            Kind::I | Kind::P => {
                let shown = std::mem::replace(&mut self.future_shown, false);
                let old = self.future.replace(cur);
                let out = if shown { None } else { old.as_ref().map(|f| crop(&seq, f)) };
                self.past = old;
                out
            }
        };
        self.output += out.is_some() as u32;
        Ok(out)
    }
}

impl Bits<'_> {
    fn skipped(mut self, n: u32) -> Self {
        self.skip(n);
        self
    }
}

/// The visible part of a frame.
fn crop(seq: &Sequence, f: &Frame) -> Picture {
    let (w, h) = (usize::from(seq.width), usize::from(seq.height));
    let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
    let g = Geometry::new(seq);
    let plane = |src: &[u8], stride: usize, w: usize, h: usize| {
        if stride == w {
            return src[..w * h].to_vec();
        }
        let mut out = Vec::with_capacity(w * h);
        for row in src.chunks_exact(stride).take(h) {
            out.extend_from_slice(&row[..w]);
        }
        out
    };
    Picture {
        width: w,
        height: h,
        y: plane(&f.y, g.stride, w, h),
        cb: plane(&f.cb, g.cstride, cw, ch),
        cr: plane(&f.cr, g.cstride, cw, ch),
        header: f.header,
        decode_index: f.decode_index,
    }
}

/// What a slice needs of the picture.
struct Ctx<'a> {
    seq: &'a Sequence,
    h: &'a PictureHeader,
    g: Geometry,
    fwd: Option<&'a Frame>,
    bwd: Option<&'a Frame>,
}

/// A macroblock's prediction: which directions, and each one's vector in
/// half samples.
#[derive(Clone, Copy, Default)]
struct Motion {
    flags: i32,
    mv: [[i32; 2]; 2],
}

impl Ctx<'_> {
    fn slice(&self, cur: &mut Frame, unit: &[u8], row: usize) -> Result<()> {
        let t = tables();
        let h = self.h;
        let mut b = Bits::new(unit);
        let mut qcode = b.read(5);
        if b.peek(1) == 1 {
            // intra_slice_flag, intra_slice, reserved_bits, then any
            // extra_information_slice bytes.
            b.skip(1 + 1 + 7);
            while b.peek(1) == 1 {
                b.skip(9);
            }
        }
        b.skip(1);
        let dc_reset = 1i32 << (7 + h.intra_dc_precision);
        let mut dc = [dc_reset; 3];
        let mut pmv = [[0i32; 2]; 2];
        let mut prev = Motion::default();
        let mut addr: Option<usize> = None;
        let mb_type = &t.mb_type[h.kind as usize - 1];
        loop {
            let mut inc = 0usize;
            loop {
                match t.addr_inc.decode(&mut b) {
                    Some(ADDR_ESCAPE) => inc += 33,
                    Some(ADDR_STUFFING) => {}
                    Some(n) => {
                        inc += n as usize;
                        break;
                    }
                    None => return err(format!("bad macroblock_address_increment in row {row}")),
                }
            }
            let a = match addr {
                None => row * self.g.mb_width + inc - 1,
                Some(last) => {
                    // Skipped macroblocks.
                    for skipped in last + 1..last + inc {
                        match h.kind {
                            Kind::I => return err("a skipped macroblock in an I picture"),
                            Kind::P => {
                                pmv = [[0; 2]; 2];
                                let m = Motion { flags: MB_FORWARD, mv: [[0; 2]; 2] };
                                self.predict(cur, skipped, m);
                            }
                            Kind::B => {
                                if prev.flags & MB_INTRA != 0 {
                                    return err("a skipped macroblock after an intra one in a B picture");
                                }
                                self.predict(cur, skipped, prev);
                            }
                        }
                        dc = [dc_reset; 3];
                    }
                    last + inc
                }
            };
            if a >= self.g.mb_width * self.g.mb_height {
                return err("a macroblock past the end of the picture");
            }
            addr = Some(a);

            let Some(flags) = mb_type.decode(&mut b) else {
                return err(format!("bad macroblock_type at macroblock {a}"));
            };
            if flags & MB_QUANT != 0 {
                qcode = b.read(5);
            }
            let qscale = if h.q_scale_type { i32::from(NONLINEAR_SCALE[qcode as usize]) } else { 2 * qcode as i32 };
            let mut m = Motion { flags, mv: [[0; 2]; 2] };
            for (s, bit) in [MB_FORWARD, MB_BACKWARD].into_iter().enumerate() {
                if flags & bit != 0 {
                    for (c, p) in pmv[s].iter_mut().enumerate() {
                        *p = motion_component(&mut b, &t.motion, h.f_code[s][c], *p)?;
                    }
                    m.mv[s] = pmv[s];
                }
            }
            let cbp = if flags & MB_PATTERN != 0 {
                match t.cbp.decode(&mut b) {
                    Some(v) => v as u32,
                    None => return err(format!("bad coded_block_pattern at macroblock {a}")),
                }
            } else if flags & MB_INTRA != 0 {
                0x3f
            } else {
                0
            };

            let (mx, my) = (a % self.g.mb_width, a / self.g.mb_width);
            let mut block = [0i16; 64];
            if flags & MB_INTRA != 0 {
                pmv = [[0; 2]; 2];
                for i in 0..6 {
                    let comp = if i < 4 { 0 } else { i - 3 };
                    self.intra_block(&mut b, &mut block, comp, &mut dc[comp], qscale)?;
                    let (plane, at, stride) = self.block_at(cur, mx, my, i);
                    idct::put(&mut block, plane, at, stride);
                }
            } else {
                dc = [dc_reset; 3];
                if h.kind == Kind::P && flags & MB_FORWARD == 0 {
                    // No MC: the zero vector, and the predictors reset.
                    pmv = [[0; 2]; 2];
                    m.flags |= MB_FORWARD;
                }
                self.predict(cur, a, m);
                for i in 0..6 {
                    if cbp & (0x20 >> i) != 0 {
                        self.non_intra_block(&mut b, &mut block, qscale)?;
                        let (plane, at, stride) = self.block_at(cur, mx, my, i);
                        idct::add(&mut block, plane, at, stride);
                    }
                }
            }
            prev = m;
            if b.overrun() {
                return err(format!("slice in row {row} runs past its data"));
            }
            // A slice ends where 23 zero bits start the next start code.
            if b.peek(23) == 0 {
                return Ok(());
            }
        }
    }

    /// Block `i` of macroblock (mx, my): its plane, offset and stride.
    fn block_at<'f>(&self, cur: &'f mut Frame, mx: usize, my: usize, i: usize) -> (&'f mut [u8], usize, usize) {
        match i {
            0..4 => {
                let x = mx * 16 + (i & 1) * 8;
                let y = my * 16 + (i >> 1) * 8;
                (&mut cur.y, y * self.g.stride + x, self.g.stride)
            }
            4 => (&mut cur.cb, my * 8 * self.g.cstride + mx * 8, self.g.cstride),
            _ => (&mut cur.cr, my * 8 * self.g.cstride + mx * 8, self.g.cstride),
        }
    }

    /// An intra block: the DC differential, then the AC coefficients, each
    /// inverse quantised (7.4.2.1, 7.4.2.3), saturated and mismatch
    /// controlled (7.4.3, 7.4.4), into `block` in natural order.
    fn intra_block(
        &self,
        b: &mut Bits,
        block: &mut [i16; 64],
        comp: usize,
        dc_pred: &mut i32,
        qscale: i32,
    ) -> Result<()> {
        let t = tables();
        let h = self.h;
        let size = if comp == 0 { t.dc_luma.decode(b) } else { t.dc_chroma.decode(b) };
        let Some(size) = size else {
            return err("bad dct_dc_size");
        };
        if size > 0 {
            let v = b.read(size as u32) as i32;
            *dc_pred += if v & (1 << (size - 1)) == 0 { v - (1 << size) + 1 } else { v };
        }
        block.fill(0);
        let dc = (*dc_pred << (3 - h.intra_dc_precision)).clamp(-2048, 2047);
        let mut sum = dc;
        block[0] = dc as i16;
        let table = if h.intra_vlc_format { &t.dct_one } else { &t.dct_zero };
        let scan = &SCAN[h.alternate_scan as usize];
        let w = &self.seq.intra_matrix;
        let mut i = 0usize;
        while let Some((run, level)) = coefficient(b, table)? {
            i += run + 1;
            if i > 63 {
                return err("DCT coefficients run past 64");
            }
            let pos = scan[i] as usize;
            let mag = (level.abs() * qscale * i32::from(w[pos])) >> 4;
            let v = if level < 0 { -mag } else { mag }.clamp(-2048, 2047);
            sum += v;
            block[pos] = v as i16;
        }
        if sum & 1 == 0 {
            block[63] ^= 1;
        }
        Ok(())
    }

    /// A non-intra block of the residual (7.4.2.3 with k = sign).
    fn non_intra_block(&self, b: &mut Bits, block: &mut [i16; 64], qscale: i32) -> Result<()> {
        let t = tables();
        let scan = &SCAN[self.h.alternate_scan as usize];
        let w = &self.seq.non_intra_matrix;
        block.fill(0);
        let mut sum = 0i32;
        let mut i = 0usize;
        let mut first = true;
        loop {
            let (run, level) = if first && b.peek(1) == 1 {
                // The first coefficient's short code for run 0, level 1.
                b.skip(1);
                (0, if b.bit() { -1 } else { 1 })
            } else {
                match coefficient(b, &t.dct_zero)? {
                    Some(c) => c,
                    None => break,
                }
            };
            let at = if first { run } else { i + run + 1 };
            first = false;
            if at > 63 {
                return err("DCT coefficients run past 64");
            }
            i = at;
            let pos = scan[i] as usize;
            let mag = ((2 * level.abs() + 1) * qscale * i32::from(w[pos])) >> 5;
            let v = if level < 0 { -mag } else { mag }.clamp(-2048, 2047);
            sum += v;
            block[pos] = v as i16;
        }
        if sum & 1 == 0 {
            block[63] ^= 1;
        }
        Ok(())
    }

    /// The prediction of macroblock `a` from the reference pictures, into
    /// `cur`: forward, backward, or their average (7.6).
    fn predict(&self, cur: &mut Frame, a: usize, m: Motion) {
        let (mx, my) = (a % self.g.mb_width, a / self.g.mb_width);
        let mut first = true;
        for (s, r) in [self.fwd, self.bwd].into_iter().enumerate() {
            let bit = if s == 0 { MB_FORWARD } else { MB_BACKWARD };
            if m.flags & bit == 0 {
                continue;
            }
            let Some(r) = r else { continue };
            let [vx, vy] = m.mv[s];
            let g = &self.g;
            mc(&mut cur.y, &r.y, g.stride, g.lines, mx * 16, my * 16, 16, vx, vy, !first);
            // 4:2:0 chroma vectors: halved, towards zero (7.6.3.7).
            let (cx, cy) = (vx / 2, vy / 2);
            mc(&mut cur.cb, &r.cb, g.cstride, g.clines, mx * 8, my * 8, 8, cx, cy, !first);
            mc(&mut cur.cr, &r.cr, g.cstride, g.clines, mx * 8, my * 8, 8, cx, cy, !first);
            first = false;
        }
    }
}

/// One motion vector component (7.6.3.1): motion_code, the residual, the
/// predictor, and the wrap into the range f_code allows.
fn motion_component(b: &mut Bits, vlc: &Vlc, f_code: u8, pred: i32) -> Result<i32> {
    let Some(code) = vlc.decode(b) else {
        return err("bad motion_code");
    };
    if !(1..=9).contains(&f_code) {
        return err(format!("f_code {f_code} used"));
    }
    let r_size = u32::from(f_code) - 1;
    let delta = if r_size == 0 || code == 0 {
        code
    } else {
        let residual = b.read(r_size) as i32;
        let d = ((code.abs() - 1) << r_size) + residual + 1;
        if code < 0 { -d } else { d }
    };
    let f = 1i32 << r_size;
    let (low, high, range) = (-16 * f, 16 * f - 1, 32 * f);
    let mut v = pred + delta;
    if v < low {
        v += range;
    } else if v > high {
        v -= range;
    }
    Ok(v)
}

/// The next DCT coefficient: (run, signed level), or None at end of block.
#[inline]
fn coefficient(b: &mut Bits, table: &Vlc) -> Result<Option<(usize, i32)>> {
    match table.decode(b) {
        Some(DCT_EOB) => Ok(None),
        Some(DCT_ESCAPE) => {
            let run = b.read(6) as usize;
            let level = ((b.read(12) << 20) as i32) >> 20;
            if level == 0 || level == -2048 {
                return err("a forbidden escape level");
            }
            Ok(Some((run, level)))
        }
        Some(v) => {
            let level = v & 0xff;
            Ok(Some(((v >> 8) as usize, if b.bit() { -level } else { level })))
        }
        None => err("bad DCT coefficient code"),
    }
}

/// Half-sample motion compensation of one `size` x `size` block at (x, y)
/// by vector (vx, vy) in half samples, from `src` into `dst` (both with
/// `stride` and `lines`). `avg` averages with what `dst` holds (the second
/// direction of a bidirectional prediction), rounding up.
#[allow(clippy::too_many_arguments)]
fn mc(
    dst: &mut [u8],
    src: &[u8],
    stride: usize,
    lines: usize,
    x: usize,
    y: usize,
    size: usize,
    vx: i32,
    vy: i32,
    avg: bool,
) {
    let sx = x as i32 + (vx >> 1);
    let sy = y as i32 + (vy >> 1);
    let (hx, hy) = ((vx & 1) as usize, (vy & 1) as usize);
    let inside = sx >= 0 && sy >= 0 && sx as usize + size + hx <= stride && sy as usize + size + hy <= lines;
    let at = |xx: i32, yy: i32| -> u32 {
        if inside {
            u32::from(src[yy as usize * stride + xx as usize])
        } else {
            let xx = xx.clamp(0, stride as i32 - 1) as usize;
            let yy = yy.clamp(0, lines as i32 - 1) as usize;
            u32::from(src[yy * stride + xx])
        }
    };
    for j in 0..size as i32 {
        let row = (y + j as usize) * stride + x;
        for i in 0..size as i32 {
            let (px, py) = (sx + i, sy + j);
            let p = match (hx, hy) {
                (0, 0) => at(px, py),
                (1, 0) => (at(px, py) + at(px + 1, py) + 1) >> 1,
                (0, 1) => (at(px, py) + at(px, py + 1) + 1) >> 1,
                _ => (at(px, py) + at(px + 1, py) + at(px, py + 1) + at(px + 1, py + 1) + 2) >> 2,
            };
            let d = &mut dst[row + i as usize];
            *d = if avg { ((u32::from(*d) + p + 1) >> 1) as u8 } else { p as u8 };
        }
    }
}
