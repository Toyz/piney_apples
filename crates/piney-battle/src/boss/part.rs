//! A boss's parts (Kyvia's core and gomoras, Magus's leaves, Gorre's
//! brothers, Corbenik's seeds, roots, eyes and killer eyes): bosses of
//! their own characters, taken out of them for the master's frame and put
//! back after ([`Part`], [`Parts`]).

use std::ops::{Deref, DerefMut};

use super::{Boss, Class, Cx};

/// A part's own members, as its [`Class`] variant holds them.
pub trait PartClass: Default + Sized {
    fn wrap(x: Box<Self>) -> Class;
    /// The members, or the class back when it is another.
    fn unwrap(c: Class) -> Result<Box<Self>, Class>;
}

/// One of the fight's parts as the master's frame holds it: its `ccBoss`,
/// its own members and its character.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Part<T> {
    pub b: Box<Boss>,
    pub x: Box<T>,
    pub me: usize,
}

impl<T: PartClass> Part<T> {
    /// The part taken out of character `me`. A slot with no boss, or with
    /// another class, is left as it was.
    pub fn take(cx: &mut Cx, me: usize) -> Option<Part<T>> {
        let mut b = cx.scene.chars.get_mut(me).and_then(|c| c.foe_state_mut()).and_then(|f| f.boss.take())?;
        match T::unwrap(std::mem::replace(&mut b.class, Class::Plain)) {
            Ok(x) => Some(Part { b, x, me }),
            Err(other) => {
                b.class = other;
                if let Some(f) = cx.scene.chars[me].foe_state_mut() {
                    f.boss = Some(b);
                }
                None
            }
        }
    }

    /// The part put back into its character.
    pub fn put(self, cx: &mut Cx) {
        let Part { mut b, x, me } = self;
        b.class = T::wrap(x);
        if let Some(f) = cx.scene.chars[me].foe_state_mut() {
            f.boss = Some(b);
        }
    }

    /// [`Part::take`] where the code indexes its parts: a default stand-in
    /// at `me` when there was none to take, and whether there was.
    pub fn take_or_stand_in(cx: &mut Cx, me: usize) -> (Part<T>, bool) {
        match Part::take(cx, me) {
            Some(p) => (p, true),
            None => (Part { me, ..Part::default() }, false),
        }
    }

    /// [`Part::put`], unless [`Part::take_or_stand_in`] found none (a
    /// stand-in is never written over what the character holds).
    pub fn put_back(self, cx: &mut Cx, found: bool) {
        if found {
            self.put(cx);
        }
    }
}

/// A master's parts by index for its frame, each with whether its
/// character held one ([`Part::take_or_stand_in`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Parts<T> {
    list: Vec<Part<T>>,
    found: Vec<bool>,
}

impl<T: PartClass> Parts<T> {
    /// Each of `mes` taken out of its character, in order.
    pub fn take(cx: &mut Cx, mes: &[usize]) -> Parts<T> {
        let (list, found) = mes.iter().map(|&me| Part::take_or_stand_in(cx, me)).unzip();
        Parts { list, found }
    }

    /// Every part put back; the list left empty.
    pub fn put(&mut self, cx: &mut Cx) {
        for (p, found) in std::mem::take(&mut self.list).into_iter().zip(std::mem::take(&mut self.found)) {
            p.put_back(cx, found);
        }
    }
}

impl<T> Deref for Parts<T> {
    type Target = [Part<T>];

    fn deref(&self) -> &[Part<T>] {
        &self.list
    }
}

impl<T> DerefMut for Parts<T> {
    fn deref_mut(&mut self) -> &mut [Part<T>] {
        &mut self.list
    }
}

impl<'a, T> IntoIterator for &'a Parts<T> {
    type Item = &'a Part<T>;
    type IntoIter = std::slice::Iter<'a, Part<T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.list.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Parts<T> {
    type Item = &'a mut Part<T>;
    type IntoIter = std::slice::IterMut<'a, Part<T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.list.iter_mut()
    }
}

/// `PartClass` for the parts' variants.
macro_rules! part_class {
    ($($t:ty => $v:ident),* $(,)?) => {$(
        impl PartClass for $t {
            fn wrap(x: Box<Self>) -> Class {
                Class::$v(x)
            }

            fn unwrap(c: Class) -> Result<Box<Self>, Class> {
                match c {
                    Class::$v(x) => Ok(x),
                    other => Err(other),
                }
            }
        }
    )*};
}

part_class! {
    super::kyvia::core::Core => KyviaCore,
    super::kyvia::gomora::Gomora => Gomora,
    super::magus::leaf::Leaf => MagusLeaf,
    super::gorre::brother::Brother => GorreBrother,
    super::corbenik::seed::Seed => CorbenikSeed,
    super::corbenik::second::Root => CorbenikRoot,
    super::corbenik::eye::Eye => CorbenikEye,
    super::corbenik::eye::Killer => CorbenikKiller,
}
