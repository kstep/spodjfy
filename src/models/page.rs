use crate::models::common::Empty;
use rspotify::model::{Cursor, CursorBasedPage, Page};

pub trait PageLike<T> {
    type Offset: Clone;

    fn items(&self) -> &[T];

    fn total(&self) -> u32 { self.items().len() as u32 }

    fn init_offset() -> Self::Offset;

    fn num_offset(&self) -> u32 { 0 }

    fn next_offset(&self) -> Option<Self::Offset> { None }
}

impl<T> PageLike<T> for Vec<T> {
    type Offset = ();

    fn items(&self) -> &[T] { &self }

    fn init_offset() -> Self::Offset {}
}

impl<T> PageLike<T> for Page<T> {
    type Offset = u32;

    fn items(&self) -> &[T] { &self.items }

    fn total(&self) -> u32 { self.total }

    fn init_offset() -> Self::Offset { 0 }

    fn num_offset(&self) -> u32 { self.offset }

    fn next_offset(&self) -> Option<Self::Offset> {
        if self.next.is_some() {
            Some(self.offset + self.limit)
        } else {
            None
        }
    }
}

impl<T> PageLike<T> for CursorBasedPage<T> {
    type Offset = String;

    fn items(&self) -> &[T] { &self.items }

    fn total(&self) -> u32 { self.total.unwrap_or(0) }

    fn init_offset() -> Self::Offset { String::new() }

    fn next_offset(&self) -> Option<Self::Offset> { self.cursors.after.clone() }
}

impl<T> Empty for Page<T> {
    fn empty() -> Self {
        Page {
            href: String::new(),
            items: Vec::new(),
            limit: 0,
            next: None,
            offset: 0,
            previous: None,
            total: 0,
        }
    }

    fn is_empty(&self) -> bool { self.items.is_empty() }
}

impl<T> Empty for CursorBasedPage<T> {
    fn empty() -> Self {
        CursorBasedPage {
            href: String::new(),
            items: Vec::new(),
            limit: 0,
            next: None,
            cursors: Cursor { after: None },
            total: None,
        }
    }

    fn is_empty(&self) -> bool { self.items.is_empty() }
}
