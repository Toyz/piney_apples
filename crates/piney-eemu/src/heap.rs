//! Outbreak's `ccHeap` (main 0x00100390-0x00100cf0) as a model: the same
//! blocks for the same requests. `ee_run`'s `EE_HEAP_MODEL` runs it beside
//! the game and stops at the first handout it would place elsewhere
//! (docs/engine/heap.md).

use std::collections::HashMap;

/// A free list's node: one of `freeTbl`'s 256 small lists (one per 16-byte
/// size), the large list (`freeTbl` +0x800), or a hole by its address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Node {
    Small(usize),
    Large,
    Hole(u32),
}

#[derive(Clone, Copy, Debug)]
struct Links {
    next: Node,
    prev: Node,
}

/// A hole: the room between two held blocks. From 32 bytes it is on a
/// free list, its first 16 bytes the record: +0 / +4 the links, +8 the
/// block below, +12 its size.
#[derive(Clone, Copy, Debug)]
struct Hole {
    links: Links,
    /// +8: the held block it follows.
    owner: u32,
    /// +12
    size: u32,
}

/// A held block's 16-byte header: +0 the block below, +4 the block
/// above, +8 its size.
#[derive(Clone, Copy, Debug)]
struct Block {
    prev: u32,
    next: u32,
    size: u32,
}

/// `_ccMalloc`'s flags (a3).
pub mod flags {
    /// Return 0 when nothing fits (else it writes to address 0).
    pub const MAY_FAIL: u32 = 1;
    /// From the top down (`ccMallocB`): the highest gap that fits.
    pub const FROM_TOP: u32 = 2;
}

#[derive(Clone, Debug)]
pub struct Heap {
    blocks: HashMap<u32, Block>,
    holes: HashMap<u32, Hole>,
    small: Vec<Links>,
    large: Links,
    top: u32,
    end: u32,
    /// `useEnd`: the end of the highest block handed out from the bottom.
    use_end: u32,
    /// `useEndB`: the lowest block handed out from the top.
    use_end_b: u32,
}

impl Heap {
    /// `ccHeap::InitFirst` (0x00100390): the two 16-byte end blocks at
    /// `heapTop` and `heapEnd`, one hole between them.
    pub fn new(top: u32, end: u32) -> Heap {
        let empty = |n| Links { next: n, prev: n };
        let mut h = Heap {
            blocks: HashMap::new(),
            holes: HashMap::new(),
            small: (0..256).map(|i| empty(Node::Small(i))).collect(),
            large: empty(Node::Large),
            top,
            end,
            use_end: top + 16,
            use_end_b: end,
        };
        h.blocks.insert(top, Block { prev: 0, next: end, size: 16 });
        h.blocks.insert(end, Block { prev: top, next: 0, size: 16 });
        h.add_hole(top + 16, end - (top + 16), top);
        h
    }

    fn links(&mut self, n: Node) -> &mut Links {
        match n {
            Node::Small(i) => &mut self.small[i],
            Node::Large => &mut self.large,
            Node::Hole(a) => &mut self.holes.get_mut(&a).expect("a listed hole").links,
        }
    }

    fn insert_after(&mut self, at: Node, a: u32) {
        let next = self.links(at).next;
        self.links(Node::Hole(a)).next = next;
        self.links(Node::Hole(a)).prev = at;
        self.links(next).prev = Node::Hole(a);
        self.links(at).next = Node::Hole(a);
    }

    fn insert_before(&mut self, at: Node, a: u32) {
        let prev = self.links(at).prev;
        self.insert_after(prev, a);
    }

    /// `AddTbl` (0x001005b0) with the hole's +8 written after it.
    fn add_hole(&mut self, a: u32, size: u32, owner: u32) {
        let at = Links { next: Node::Hole(a), prev: Node::Hole(a) };
        self.holes.insert(a, Hole { links: at, owner, size });
        if size < 0x1000 {
            // Ahead of a first hole at or above it; else after the last
            // one below it, looking only at the last (an address between
            // the two goes to the front).
            let list = Node::Small((size >> 4) as usize);
            match self.links(list).next {
                Node::Hole(first) if first >= a => self.insert_after(list, a),
                _ => match self.links(list).prev {
                    Node::Hole(last) if last < a => self.insert_after(Node::Hole(last), a),
                    _ => self.insert_after(list, a),
                },
            }
        } else {
            // In address order.
            let mut n = self.large.next;
            while let Node::Hole(x) = n {
                if a < x {
                    break;
                }
                n = self.links(n).next;
            }
            self.insert_before(n, a);
        }
    }

    fn unlink(&mut self, a: u32) {
        let h = self.holes.remove(&a).expect("a listed hole");
        self.links(h.links.next).prev = h.links.prev;
        self.links(h.links.prev).next = h.links.next;
    }

    /// `SearchFree` (0x00100500): under 4096, the first hole of the first
    /// non-empty small list from the size's own up; else (or with none)
    /// the lowest hole on the large list that fits.
    fn search(&self, size: u32) -> Option<u32> {
        if size < 0x1000 {
            let hit = self.small[(size >> 4) as usize..].iter().find_map(|l| match l.next {
                Node::Hole(a) => Some(a),
                _ => None,
            });
            if hit.is_some() {
                return hit;
            }
        }
        let mut n = self.large.next;
        while let Node::Hole(a) = n {
            if self.holes[&a].size >= size {
                return Some(a);
            }
            n = self.holes[&a].links.next;
        }
        None
    }

    fn size(&self, b: u32) -> u32 {
        self.blocks[&b].size
    }

    /// The hole above `b`'s end, when one is listed there.
    fn unlink_if_listed(&mut self, a: u32) {
        if self.holes.contains_key(&a) {
            self.unlink(a);
        }
    }

    /// `_ccMalloc(align, n, _, flags)` (0x001006e0): the payload's address,
    /// None when nothing fits.
    pub fn malloc(&mut self, align: u32, n: u32, flags: u32) -> Option<u32> {
        let mut size = (align + 31 + n) & !15;
        let (below, above, mut at) = if flags & flags::FROM_TOP == 0 {
            let hole = self.search(size)?;
            let below = self.holes[&hole].owner;
            let above = self.blocks[&below].next;
            self.unlink(hole);
            (below, above, hole)
        } else {
            let (mut above, mut below) = (self.end, self.blocks[&self.end].prev);
            loop {
                let gap = above - (below + self.size(below));
                if gap >= size || below == self.top {
                    break;
                }
                above = below;
                below = self.blocks[&below].prev;
            }
            let start = below + self.size(below);
            if start < self.use_end_b && above - start >= 32 {
                self.unlink_if_listed(start);
            }
            let at = above.checked_sub(size).filter(|&a| a >= self.use_end)?;
            (below, above, at)
        };
        if align != 0 {
            let off = (at + 16) & (align - 1);
            if off != 0 {
                at += align - off;
                size -= align - off;
            }
        }
        self.blocks.insert(at, Block { prev: below, next: above, size });
        self.blocks.get_mut(&above).unwrap().prev = at;
        self.blocks.get_mut(&below).unwrap().next = at;
        let below_end = below + self.size(below);
        if flags & flags::FROM_TOP == 0 {
            self.use_end = self.use_end.max(at + size);
            if above - (at + size) >= 32 {
                self.add_hole(at + size, above - (at + size), at);
            }
            if at - below_end >= 32 {
                self.add_hole(below_end, at - below_end, below);
            }
        } else if at < self.use_end_b {
            self.use_end_b = at;
            if at - below_end >= 32 {
                self.add_hole(below_end, at - below_end, below);
            }
        }
        Some(at + 16)
    }

    /// `_ccFree` (0x001009a0). A block from the top that is not the lowest
    /// one leaves no hole on a list.
    pub fn free(&mut self, p: u32) {
        if p == 0 {
            return;
        }
        let b = p - 16;
        let Block { prev, next, size } = self.blocks.remove(&b).expect("a held block");
        self.blocks.get_mut(&prev).unwrap().next = next;
        self.blocks.get_mut(&next).unwrap().prev = prev;
        let start = prev + self.size(prev);
        if b < self.use_end {
            if self.use_end == b + size {
                self.use_end = start;
            }
            if next - (b + size) >= 32 {
                self.unlink_if_listed(b + size);
            }
            if b - start >= 32 {
                self.unlink_if_listed(start);
            }
        } else if self.use_end_b == b {
            if b - start >= 32 {
                self.unlink_if_listed(start);
            }
            self.use_end_b = next;
        } else {
            return;
        }
        if next - start >= 32 {
            self.add_hole(start, next - start, prev);
        }
    }

    /// `ccMemReduce(p, n)` (0x00100b60): the block cut to `n`, the rest
    /// a hole with whatever lay above it.
    pub fn reduce(&mut self, p: u32, n: u32) {
        let b = p - 16;
        let new = (n + 31) & !15;
        let Block { next, size: old, .. } = self.blocks[&b];
        if new == old {
            return;
        }
        if b < self.use_end {
            if self.use_end == b + old {
                self.use_end = b + new;
            }
            if next - (b + old) >= 32 {
                self.unlink_if_listed(b + old);
            }
            if next - (b + new) >= 32 {
                self.add_hole(b + new, next - (b + new), b);
            }
        } else if self.use_end_b == b {
            self.use_end_b = next;
        }
        self.blocks.get_mut(&b).unwrap().size = new;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOP: u32 = 0x0077_3790;
    const END: u32 = 0x01ff_3ff0;

    #[test]
    fn the_first_block_follows_the_top_block() {
        let mut h = Heap::new(TOP, END);
        assert_eq!(h.malloc(0, 0x4e0, 0), Some(TOP + 32));
        // 0x4e0 + 31 rounded down to 16: 0x4f0 with the header.
        assert_eq!(h.malloc(0, 0x38, 0), Some(TOP + 16 + 0x4f0 + 16));
    }

    #[test]
    fn a_large_request_takes_the_lowest_hole_that_fits() {
        let mut h = Heap::new(TOP, END);
        let a = h.malloc(0, 0x2000, 0).unwrap();
        let _b = h.malloc(0, 0x100, 0).unwrap();
        let c = h.malloc(0, 0x3000, 0).unwrap();
        let _d = h.malloc(0, 0x100, 0).unwrap();
        h.free(c);
        h.free(a);
        // Both holes fit 0x1800; the lower one wins, and nothing is cleared.
        assert_eq!(h.malloc(0, 0x1800, 0), Some(a));
        // 0x2800 fits only the upper one.
        assert_eq!(h.malloc(0, 0x2800, 0), Some(c));
    }

    #[test]
    fn a_small_request_takes_the_smallest_size_first() {
        let mut h = Heap::new(TOP, END);
        let a = h.malloc(0, 0x60, 0).unwrap();
        let _x = h.malloc(0, 0x10, 0).unwrap();
        let b = h.malloc(0, 0x20, 0).unwrap();
        let _y = h.malloc(0, 0x10, 0).unwrap();
        h.free(a);
        h.free(b);
        // b's hole (0x30) is a smaller size than a's (0x70): it goes first.
        assert_eq!(h.malloc(0, 0x10, 0), Some(b));
    }

    #[test]
    fn from_the_top_takes_the_highest_gap() {
        let mut h = Heap::new(TOP, END);
        let t = h.malloc(0, 0x20000, flags::FROM_TOP).unwrap();
        assert_eq!(t, END - 0x20010 + 16);
        // A bottom request still starts at the bottom.
        assert_eq!(h.malloc(0, 0x10, 0), Some(TOP + 32));
    }

    #[test]
    fn alignment_moves_the_block_up() {
        let mut h = Heap::new(TOP, END);
        // The payload moves up to the next 0x80 line, the block with it.
        let p = h.malloc(0x80, 0x20000, 0).unwrap();
        assert_eq!(p, (TOP + 32 + 0x7f) & !0x7f);
    }

    #[test]
    fn a_reduced_block_leaves_its_tail_a_hole() {
        let mut h = Heap::new(TOP, END);
        let a = h.malloc(0, 0x858, 0).unwrap();
        let _b = h.malloc(0, 0x18, 0).unwrap();
        h.reduce(a, 0x7c8);
        // The 0x870-byte block is now 0x7e0: a 0x90-byte hole above it.
        assert_eq!(h.malloc(0, 0x30, 0), Some(a - 16 + 0x7e0 + 16));
    }
}
