// TODO: Use Model
#![allow(dead_code)]

use glib::{IsA, Type};
use rspotify::model::{CursorBasedPage, Image, Page};
use serde_derive::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

pub mod constants {
    pub const COL_ITEM_THUMB: u32 = 0;
    pub const COL_ITEM_URI: u32 = 1;
    pub const COL_ITEM_NAME: u32 = 2;
}
pub use constants::*;

#[derive(Serialize, Deserialize)]
pub enum Model<F, S> {
    Full(F),
    Simple(S),
}

impl<S: ToFull> Model<S::Full, S> {
    fn make_full(&mut self) {
        match self {
            Self::Full(_) => {}
            Self::Simple(model) => *self = Self::Full(model.to_full()),
        }
    }

    fn into_full(self) -> S::Full {
        match self {
            Self::Full(model) => model,
            Self::Simple(model) => model.into_full(),
        }
    }
}

impl<F: ToSimple> Model<F, F::Simple> {
    pub fn make_simple(&mut self) {
        match self {
            Self::Full(model) => *self = Self::Simple(model.to_simple()),
            Self::Simple(_) => {}
        }
    }

    pub fn into_simple(self) -> F::Simple {
        match self {
            Self::Simple(model) => model,
            Self::Full(model) => model.into_simple(),
        }
    }
}

pub trait Wrapper {
    type For;

    fn unwrap(self) -> Self::For;
    fn wrap(original: Self::For) -> Self;
}

pub trait ToSimple: Sized {
    type Simple;

    fn to_simple(&self) -> Self::Simple;
    fn into_simple(self) -> Self::Simple { self.to_simple() }
}

pub trait ToFull: Sized {
    type Full: ToSimple;

    fn to_full(&self) -> Self::Full;
    fn into_full(self) -> Self::Full { self.to_full() }
}

pub trait Empty {
    fn empty() -> Self;

    fn is_empty(&self) -> bool;
}

impl<S: ToFull> Merge for Model<S::Full, S>
where
    S::Full: Merge,
{
    fn merge(self, other: Self) -> Self { Model::Full(self.into_full().merge(other.into_full())) }
}

impl Empty for String {
    fn empty() -> Self { String::new() }
    fn is_empty(&self) -> bool { String::is_empty(self) }
}

impl<T> Empty for Vec<T> {
    fn empty() -> Self { Vec::new() }
    fn is_empty(&self) -> bool { Vec::is_empty(self) }
}

impl Empty for bool {
    fn empty() -> Self { false }
    fn is_empty(&self) -> bool { !*self }
}

impl<T> Empty for Option<T> {
    fn empty() -> Self { None }
    fn is_empty(&self) -> bool { self.is_none() }
}

macro_rules! impl_empty_for_num {
    ($($ty:ty),+) => {
        $(impl Empty for $ty {
            fn empty() -> Self { 0 }
            fn is_empty(&self) -> bool { *self == 0 }
        })+
    }
}

impl_empty_for_num!(usize, u64, u32, u16, u8, isize, i64, i32, i16, i8);

impl Empty for f32 {
    fn empty() -> f32 { 0.0 }
    fn is_empty(&self) -> bool { *self == 0.0 }
}

impl Empty for f64 {
    fn empty() -> f64 { 0.0 }
    fn is_empty(&self) -> bool { *self == 0.0 }
}

impl<K, V> Empty for HashMap<K, V> {
    fn empty() -> Self { HashMap::new() }
    fn is_empty(&self) -> bool { HashMap::is_empty(self) }
}

impl<K: Ord, V> Empty for BTreeMap<K, V> {
    fn empty() -> Self { BTreeMap::new() }

    fn is_empty(&self) -> bool { BTreeMap::is_empty(self) }
}

pub trait Merge<Other = Self>: Sized {
    fn merge(self, other: Other) -> Self;
}

impl<T: Empty + Clone> Merge for T {
    fn merge(self, other: Self) -> Self {
        if self.is_empty() {
            other
        } else {
            self
        }
    }
}

impl<T: HasDuration> HasDuration for Vec<T> {
    fn duration(&self) -> u32 { self.iter().map(|item| item.duration()).sum() }

    fn duration_exact(&self) -> bool { self.iter().all(|item| item.duration_exact()) }
}

pub trait HasImages {
    fn images(&self) -> &[Image];
}

pub trait HasDuration {
    fn duration(&self) -> u32 { 0 }
    fn duration_exact(&self) -> bool { true }
}

pub trait HasUri {
    fn uri(&self) -> &str;
}

pub trait HasName {
    fn name(&self) -> &str;
}

pub trait MissingColumns {
    fn missing_columns() -> &'static [u32] { &[] }
}

impl<T: HasDuration> HasDuration for Page<T> {
    fn duration(&self) -> u32 { self.items.iter().map(|item| item.duration()).sum() }

    fn duration_exact(&self) -> bool {
        (self.items.len() == self.total as usize) && self.items.iter().all(|item| item.duration_exact())
    }
}

impl<T: HasDuration> HasDuration for CursorBasedPage<T> {
    fn duration(&self) -> u32 { self.items.iter().map(|item| item.duration()).sum() }

    fn duration_exact(&self) -> bool {
        (self.items.len() == self.total.unwrap_or(0) as usize) && self.items.iter().all(|item| item.duration_exact())
    }
}

pub trait RowLike {
    fn content_types() -> Vec<Type>;

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter;
}
