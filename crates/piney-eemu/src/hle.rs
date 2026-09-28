//! The C string and memory functions eemu runs in Python rather than
//! interpreting the game's MMI-vectorised versions (`eemu.HLE`), natively.
//! Each does exactly what eemu's Python does to a `bytearray` of RAM: a read
//! past the end is cut short, a string with no terminator is `ValueError`, and
//! a write that would change RAM's length is the `BufferError` a bytearray
//! with a live buffer export raises, which is what eemu_rs's RAM is.

use std::cmp::Ordering;

/// One of eemu's HLE functions (`memmove` is `memcpy` there too).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hle {
    Strcmp,
    Strncmp,
    Strlen,
    Strcpy,
    Strcat,
    Memcpy,
    Memset,
    Memcmp,
}

/// eemu's `HLE` table: symbol name -> function.
pub const TABLE: [(&str, Hle); 9] = [
    ("strcmp", Hle::Strcmp),
    ("strncmp", Hle::Strncmp),
    ("strlen", Hle::Strlen),
    ("strcpy", Hle::Strcpy),
    ("strcat", Hle::Strcat),
    ("memcpy", Hle::Memcpy),
    ("memmove", Hle::Memcpy),
    ("memset", Hle::Memset),
    ("memcmp", Hle::Memcmp),
];

/// How an HLE function fails, as the Python would.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HleError {
    /// `bytearray.index(0, start)` found no terminator: `ValueError`.
    NoTerminator,
    /// A slice assignment that would resize RAM: `BufferError`.
    Resize,
}

/// Python's clamped slice `[start:start + n]` of a buffer of `len` bytes.
fn span(len: usize, start: u64, n: u64) -> (usize, usize) {
    let a = start.min(len as u64) as usize;
    let b = start.saturating_add(n).min(len as u64) as usize;
    (a, b.max(a))
}

/// eemu's `_cstr`: the bytes from `a` up to the next 0.
fn cstr(ram: &[u8], a: u32) -> Result<&[u8], HleError> {
    let a = a as usize;
    if a > ram.len() {
        return Err(HleError::NoTerminator);
    }
    match ram[a..].iter().position(|&b| b == 0) {
        Some(n) => Ok(&ram[a..a + n]),
        None => Err(HleError::NoTerminator),
    }
}

fn order(x: &[u8], y: &[u8]) -> i64 {
    match x.cmp(y) {
        Ordering::Equal => 0,
        Ordering::Greater => 1,
        Ordering::Less => -1,
    }
}

/// The ranges `call` writes, for tracing: (address, length).
pub type Writes = Vec<(u32, u32)>;

/// Run `h` with the argument registers `a` (a0-a3 as 32-bit words). Returns
/// what the Python returns, before eemu's `set32` takes its low 32 bits, and
/// adds what it wrote to `writes` when given.
pub fn call(h: Hle, ram: &mut [u8], a: [u32; 4], writes: Option<&mut Writes>) -> Result<i64, HleError> {
    let [a0, a1, a2, _] = a;
    let mut writes = writes;
    let mut wrote = |at: u64, n: usize| {
        if let Some(w) = writes.as_deref_mut().filter(|_| n > 0) {
            w.push((at as u32, n as u32));
        }
    };
    Ok(match h {
        Hle::Strcmp => {
            let x = cstr(ram, a0)?;
            let y = cstr(ram, a1)?;
            for i in 0..=x.len().min(y.len()) {
                let cx = i64::from(x.get(i).copied().unwrap_or(0));
                let cy = i64::from(y.get(i).copied().unwrap_or(0));
                if cx != cy {
                    return Ok(cx - cy);
                }
            }
            0
        }
        Hle::Strncmp => {
            let x = cstr(ram, a0)?;
            let y = cstr(ram, a1)?;
            let n = a2 as usize;
            order(&x[..x.len().min(n)], &y[..y.len().min(n)])
        }
        Hle::Strlen => cstr(ram, a0)?.len() as i64,
        Hle::Strcpy => {
            let n = strcpy(ram, u64::from(a0), a1)?;
            wrote(u64::from(a0), n);
            i64::from(a0)
        }
        Hle::Strcat => {
            let d = u64::from(a0) + cstr(ram, a0)?.len() as u64;
            let n = strcpy(ram, d, a1)?;
            wrote(d, n);
            i64::from(a0)
        }
        Hle::Memcpy => {
            // m.mem[d:d + n] = bytes(m.mem[s:s + n]): both slices clamp.
            let (s0, s1) = span(ram.len(), u64::from(a1), u64::from(a2));
            let (d0, d1) = span(ram.len(), u64::from(a0), u64::from(a2));
            if d1 - d0 != s1 - s0 {
                return Err(HleError::Resize);
            }
            ram.copy_within(s0..s1, d0);
            wrote(u64::from(a0), s1 - s0);
            i64::from(a0)
        }
        Hle::Memset => {
            let (d0, d1) = span(ram.len(), u64::from(a0), u64::from(a2));
            if (d1 - d0) as u64 != u64::from(a2) {
                return Err(HleError::Resize);
            }
            ram[d0..d1].fill(a1 as u8);
            wrote(u64::from(a0), d1 - d0);
            i64::from(a0)
        }
        Hle::Memcmp => {
            let (x0, x1) = span(ram.len(), u64::from(a0), u64::from(a2));
            let (y0, y1) = span(ram.len(), u64::from(a1), u64::from(a2));
            order(&ram[x0..x1], &ram[y0..y1])
        }
    })
}

/// `_strcpy(m, d, s)`: returns the bytes written (the terminator included).
fn strcpy(ram: &mut [u8], d: u64, s: u32) -> Result<usize, HleError> {
    let n = cstr(ram, s)?.len();
    let s = s as usize;
    let (a, b) = span(ram.len(), d, n as u64 + 1);
    if b - a != n + 1 {
        return Err(HleError::Resize);
    }
    // The terminator is part of the source, so one copy moves n + 1 bytes;
    // copy_within is a memmove, like the Python's copy-then-assign.
    ram.copy_within(s..s + n + 1, a);
    Ok(n + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ram() -> Vec<u8> {
        let mut r = vec![0u8; 64];
        r[0..4].copy_from_slice(b"abc\0");
        r[8..12].copy_from_slice(b"abd\0");
        r[16..18].copy_from_slice(b"a\0");
        r
    }

    #[test]
    fn strings() {
        let mut r = ram();
        assert_eq!(call(Hle::Strcmp, &mut r, [0, 8, 0, 0], None), Ok(i64::from(b'c') - i64::from(b'd')));
        assert_eq!(call(Hle::Strcmp, &mut r, [0, 16, 0, 0], None), Ok(i64::from(b'b')));
        assert_eq!(call(Hle::Strcmp, &mut r, [16, 0, 0, 0], None), Ok(-i64::from(b'b')));
        assert_eq!(call(Hle::Strncmp, &mut r, [0, 8, 2, 0], None), Ok(0));
        assert_eq!(call(Hle::Strncmp, &mut r, [8, 0, 3, 0], None), Ok(1));
        assert_eq!(call(Hle::Strlen, &mut r, [0, 0, 0, 0], None), Ok(3));
        assert_eq!(call(Hle::Strcpy, &mut r, [32, 0, 0, 0], None), Ok(32));
        assert_eq!(&r[32..36], b"abc\0");
        assert_eq!(call(Hle::Strcat, &mut r, [32, 8, 0, 0], None), Ok(32));
        assert_eq!(&r[32..39], b"abcabd\0");
        // No terminator before the end of RAM.
        r[60..64].copy_from_slice(b"xxxx");
        assert_eq!(call(Hle::Strlen, &mut r, [60, 0, 0, 0], None), Err(HleError::NoTerminator));
        assert_eq!(call(Hle::Strlen, &mut r, [65, 0, 0, 0], None), Err(HleError::NoTerminator));
        // A copy running past the end would resize RAM.
        r[60..64].copy_from_slice(b"xx\0\0");
        assert_eq!(call(Hle::Strcpy, &mut r, [62, 0, 0, 0], None), Err(HleError::Resize));
    }

    #[test]
    fn memory() {
        let mut r = ram();
        let mut w = Writes::new();
        assert_eq!(call(Hle::Memcpy, &mut r, [2, 0, 8, 0], Some(&mut w)), Ok(2));
        assert_eq!(&r[0..10], b"ababc\0\0\0\0\0");
        assert_eq!(w, vec![(2, 8)]);
        assert_eq!(call(Hle::Memset, &mut r, [40, 0x1ff, 4, 0], None), Ok(40));
        assert_eq!(&r[40..44], &[0xff; 4]);
        assert_eq!(call(Hle::Memcmp, &mut r, [0, 2, 2, 0], None), Ok(0));
        assert_eq!(call(Hle::Memcmp, &mut r, [8, 0, 4, 0], None), Ok(-1));
        // Reads past the end are cut short; a shorter string compares lower.
        assert_eq!(call(Hle::Memcmp, &mut r, [62, 40, 4, 0], None), Ok(-1));
        assert_eq!(call(Hle::Memcpy, &mut r, [0, 62, 4, 0], None), Err(HleError::Resize));
        assert_eq!(call(Hle::Memset, &mut r, [62, 0, 4, 0], None), Err(HleError::Resize));
        assert_eq!(call(Hle::Memcpy, &mut r, [100, 0, 0, 0], None), Ok(100));
    }
}
