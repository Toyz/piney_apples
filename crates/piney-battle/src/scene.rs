//! The characters of a battle and the game's command lists: `cmndPcRoot`
//! (the party), `cmndEneRoot` (enemies and bosses) and `cmndObjRoot`
//! (objects), linked through `ccChar.cmndLink`. `ccCheckTarget(ch)`
//! (gcmn 0x00519920) is "on one of the three lists"; area rules walk the
//! party's or the foes' list in its order.

use crate::chara::Char;

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
