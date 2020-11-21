use crate::loaders::HasDuration;
use glib::{IsA, Type};
use rspotify::model::page::{CursorBasedPage, Page};

pub trait PageLike<T> {
    type Offset: Clone;
    fn items(&self) -> &[T];
    fn total(&self) -> u32 {
        self.items().len() as u32
    }
    fn init_offset() -> Self::Offset;
    fn num_offset(&self) -> u32 {
        0
    }
    fn next_offset(&self) -> Option<Self::Offset> {
        None
    }
}

impl<T> PageLike<T> for Vec<T> {
    type Offset = ();
    fn items(&self) -> &[T] {
        &self
    }
    fn init_offset() -> Self::Offset {}
}

impl<T: HasDuration> HasDuration for Vec<T> {
    fn duration(&self) -> u32 {
        self.iter().map(|item| item.duration()).sum()
    }

    fn duration_exact(&self) -> bool {
        self.iter().all(|item| item.duration_exact())
    }
}

impl<T> PageLike<T> for Page<T> {
    type Offset = u32;
    fn items(&self) -> &[T] {
        &self.items
    }
    fn total(&self) -> u32 {
        self.total
    }
    fn init_offset() -> Self::Offset {
        0
    }
    fn num_offset(&self) -> u32 {
        self.offset
    }
    fn next_offset(&self) -> Option<Self::Offset> {
        if self.next.is_some() {
            Some(self.offset + self.limit)
        } else {
            None
        }
    }
}

impl<T: HasDuration> HasDuration for Page<T> {
    fn duration(&self) -> u32 {
        self.items.iter().map(|item| item.duration()).sum()
    }

    fn duration_exact(&self) -> bool {
        (self.items.len() == self.total as usize)
            && self.items.iter().all(|item| item.duration_exact())
    }
}

impl<T> PageLike<T> for CursorBasedPage<T> {
    type Offset = String;
    fn items(&self) -> &[T] {
        &self.items
    }
    fn total(&self) -> u32 {
        self.total.unwrap_or(0)
    }
    fn init_offset() -> Self::Offset {
        String::new()
    }
    fn next_offset(&self) -> Option<Self::Offset> {
        self.next.clone()
    }
}

impl<T: HasDuration> HasDuration for CursorBasedPage<T> {
    fn duration(&self) -> u32 {
        self.items.iter().map(|item| item.duration()).sum()
    }

    fn duration_exact(&self) -> bool {
        (self.items.len() == self.total.unwrap_or(0) as usize)
            && self.items.iter().all(|item| item.duration_exact())
    }
}

pub trait RowLike {
    fn content_types() -> Vec<Type>;
    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter;
}
