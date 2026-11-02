//! A boss's parts (Kyvia's core and gomoras, Magus's leaves, Gorre's
//! brothers, Corbenik's seeds, roots, eyes and killer eyes): bosses of
//! their own characters, taken out of them for the master's frame and put
//! back after ([`Part`], [`Parts`]).

use std::ops::{Deref, DerefMut};

use super::{Boss, BossBase, BossDef, Class, Cx, SystemKind};

/// A part's own members, as its boss holds them: its [`Class`] variant,
/// or its system on the def's list.
pub trait PartClass: Default + Sized {
    /// The members out of the boss, None when it holds none.
    fn take_from(b: &mut Boss) -> Option<Box<Self>>;
    fn put_into(b: &mut Boss, x: Box<Self>);
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
        match T::take_from(&mut b) {
            Some(x) => Some(Part { b, x, me }),
            None => {
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
        T::put_into(&mut b, x);
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

    /// [`Parts::take`] with `own` in place of its character's (one already
    /// out of it: its own `Affect`), never put back.
    pub fn take_with(cx: &mut Cx, mes: &[usize], own: Part<T>) -> Parts<T> {
        let mut own = Some(own);
        let (list, found) = (mes.iter())
            .map(|&me| match own.take_if(|p| p.me == me) {
                Some(p) => (p, false),
                None => Part::take_or_stand_in(cx, me),
            })
            .unzip();
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

impl<T: SystemKind + Default> Part<T> {
    /// A part of `def` at character `me`: its boss with the def's other
    /// systems as their constructors start them, `x` its own.
    pub fn of(def: &'static BossDef, base: BossBase, x: T, me: usize) -> Part<T> {
        let mut b = Boss::build(def, base);
        let _ = b.systems.take::<T>();
        Part { b: Box::new(b), x: Box::new(x), me }
    }
}

/// `PartClass` for the parts that are systems on their def's list.
macro_rules! part_system {
    ($($t:ty),* $(,)?) => {$(
        impl PartClass for $t {
            fn take_from(b: &mut Boss) -> Option<Box<Self>> {
                b.systems.take::<$t>()
            }

            fn put_into(b: &mut Boss, x: Box<Self>) {
                b.systems.put(b.def, x);
            }
        }
    )*};
}

part_system! {
    super::gorre::brother::Brother,
    super::magus::leaf::Leaf,
    super::kyvia::core::Core,
    super::kyvia::gomora::Gomora,
}

/// `PartClass` for the parts' variants.
macro_rules! part_class {
    ($($t:ty => $v:ident),* $(,)?) => {$(
        impl PartClass for $t {
            fn take_from(b: &mut Boss) -> Option<Box<Self>> {
                match std::mem::replace(&mut b.class, Class::Plain) {
                    Class::$v(x) => Some(x),
                    other => {
                        b.class = other;
                        None
                    }
                }
            }

            fn put_into(b: &mut Boss, x: Box<Self>) {
                b.class = Class::$v(x);
            }
        }
    )*};
}

part_class! {
    super::corbenik::seed::Seed => CorbenikSeed,
    super::corbenik::second::Root => CorbenikRoot,
    super::corbenik::eye::Eye => CorbenikEye,
    super::corbenik::eye::Killer => CorbenikKiller,
}
