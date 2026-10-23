//! The characters of a battle and the game's command lists: `cmndPcRoot`
//! (the party), `cmndEneRoot` (enemies and bosses) and `cmndObjRoot`
//! (objects), linked through `ccChar.cmndLink`. `ccCheckTarget(ch)`
//! (gcmn 0x00519920) is "on one of the three lists"; area rules walk the
//! party's or the foes' list in its order.

use crate::chara::{Char, Param, ParamAt};

/// The characters and the lists, by index into `chars`.
#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub chars: Vec<Char>,
    pub pc_list: Vec<usize>,
    pub ene_list: Vec<usize>,
    pub obj_list: Vec<usize>,
}

impl Scene {
    /// `ccCheckTarget(chars[i])`.
    pub fn listed(&self, i: usize) -> bool {
        self.pc_list.contains(&i) || self.ene_list.contains(&i) || self.obj_list.contains(&i)
    }

    /// The list an area rule walks for targets of type `tyb`: the party's
    /// for bits 0x6, the foes' for 0xe0, none otherwise.
    pub fn side(&self, tyb: i32) -> Option<Vec<usize>> {
        if tyb & 6 != 0 {
            Some(self.pc_list.clone())
        } else if tyb & crate::param::ty::FOE != 0 {
            Some(self.ene_list.clone())
        } else {
            None
        }
    }

    /// The character that owns the block `chars[c]`'s `param` points at:
    /// itself, or the owner it shares with.
    pub fn param_owner(&self, c: usize) -> usize {
        self.chars[c].foe_state().and_then(|f| f.owner()).unwrap_or(c)
    }

    /// The foe block `chars[c]`'s `param` points at, through its owner.
    pub fn param(&self, c: usize) -> Option<&Param> {
        self.chars[self.param_owner(c)].param()
    }

    pub fn param_mut(&mut self, c: usize) -> Option<&mut Param> {
        let o = self.param_owner(c);
        self.chars[o].param_mut()
    }

    /// `chars[c]`'s `real` block ([`Char::real`]), a foe's through its
    /// `param`.
    pub fn real(&self, c: usize) -> &crate::param::Elm {
        match self.param(c) {
            Some(p) if self.chars[c].foe_state().is_some() => &p.real,
            _ => self.chars[c].real(),
        }
    }

    /// `chars[c]`'s `temp` and `time` blocks ([`Char::temp_time`]), a
    /// foe's through its `param`.
    pub fn temp_time(&self, c: usize) -> Option<(&crate::param::Elm, &crate::param::Elm)> {
        match self.param(c) {
            Some(p) if self.chars[c].foe_state().is_some() => Some((&p.temp, &p.time)),
            _ => self.chars[c].temp_time(),
        }
    }

    /// `SetBaseParam(row, &owner's param)`'s pointer: `chars[c]`'s
    /// `param` is `owner`'s block from now on (its own is dropped).
    pub fn share_param(&mut self, c: usize, owner: usize) {
        let owner = self.param_owner(owner);
        if c != owner
            && let Some(f) = self.chars[c].foe_state_mut()
        {
            f.param = ParamAt::Of(owner);
        }
    }

    /// `chars[c]` as code that reads one character sees it: a sharer
    /// holding a copy of its owner's block (for reading; writes go through
    /// [`Scene::lend`]).
    pub fn view(&self, c: usize) -> std::borrow::Cow<'_, Char> {
        let o = self.param_owner(c);
        match (o != c).then(|| self.chars[o].param()).flatten() {
            Some(p) => {
                let mut ch = self.chars[c].clone();
                if let Some(f) = ch.foe_state_mut() {
                    f.param = ParamAt::Own(p.clone());
                }
                std::borrow::Cow::Owned(ch)
            }
            None => std::borrow::Cow::Borrowed(&self.chars[c]),
        }
    }

    /// `SetBaseParam`'s pointer to the character's own block: a fresh
    /// one if it pointed at another's (the row sets every field next).
    pub fn own_param(&mut self, c: usize) {
        if let Some(f) = self.chars[c].foe_state_mut()
            && f.owner().is_some()
        {
            f.param = ParamAt::default();
        }
    }

    /// `f` on `chars[c]` with the block its `param` points at: a sharer is
    /// lent its owner's for the call (the owner points back at it until
    /// then), so code that takes one character reads and writes the
    /// shared block, as the game's `ccChar*` does.
    pub fn lend<R>(&mut self, c: usize, f: impl FnOnce(&mut Char) -> R) -> R {
        self.lend_in(c, |chars| f(&mut chars[c]))
    }

    /// [`Scene::lend`] with every character in reach: `chars[c]` holds
    /// the block for the call.
    pub fn lend_in<R>(&mut self, c: usize, f: impl FnOnce(&mut [Char]) -> R) -> R {
        let o = self.param_owner(c);
        if o == c {
            return f(&mut self.chars);
        }
        // The block moves to `to`, and `from` points at it there.
        let hand = |chars: &mut [Char], from: usize, to: usize| {
            let block = chars[from].foe_state_mut().map(|x| std::mem::replace(&mut x.param, ParamAt::Of(to)));
            if let (Some(b), Some(x)) = (block, chars[to].foe_state_mut()) {
                x.param = b;
            }
        };
        hand(&mut self.chars, o, c);
        let r = f(&mut self.chars);
        hand(&mut self.chars, c, o);
        r
    }

    /// Add a character; `list` 0 party, 1 foes, 2 objects, anything else
    /// none. Returns its index.
    pub fn add(&mut self, ch: Char, list: u8) -> usize {
        let i = self.chars.len();
        self.chars.push(ch);
        match list {
            0 => self.pc_list.push(i),
            1 => self.ene_list.push(i),
            2 => self.obj_list.push(i),
            _ => {}
        }
        i
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::param::FoeRow;

    /// A sharer reads and writes its owner's block: lent for one
    /// character's code, through the owner otherwise; its own block is gone.
    #[test]
    fn a_sharer_reads_and_writes_its_owners_block() {
        let mut s = Scene::default();
        let owner = s.add(Char::foe(FoeRow::default()), 1);
        let sharer = s.add(Char::foe(FoeRow::default()), 1);
        s.share_param(sharer, owner);
        assert_eq!(s.param_owner(sharer), owner);
        s.lend(sharer, |ch| {
            let p = ch.param_mut().unwrap();
            p.temp[0] = -8;
            p.time[0] = 30;
        });
        assert_eq!(s.param(owner).unwrap().time[0], 30);
        s.param_mut(owner).unwrap().pp = 7;
        assert_eq!(s.param(sharer).unwrap().pp, 7);
        assert_eq!(s.view(sharer).param().unwrap().temp[0], -8);
        assert_eq!(s.chars[sharer].foe_state().unwrap().owner(), Some(owner));
        s.own_param(sharer);
        assert_eq!(s.param(sharer).unwrap().pp, 0);
    }
}
