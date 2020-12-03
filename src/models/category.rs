use crate::models::common::*;
use gdk_pixbuf::Pixbuf;
use glib::{IsA, StaticType, Type};
use gtk::prelude::GtkListStoreExtManual;
use rspotify::model::{Category, Image};

pub mod constants {
    use crate::models::{COL_ITEM_THUMB, COL_ITEM_NAME, COL_ITEM_URI};
    pub const COL_CATEGORY_ID: u32 = COL_ITEM_URI;
    pub const COL_CATEGORY_ICON: u32 = COL_ITEM_THUMB;
    pub const COL_CATEGORY_NAME: u32 = COL_ITEM_NAME;
}
pub use self::constants::*;

impl HasDuration for Category {
    fn duration_exact(&self) -> bool { false }
}

impl HasImages for Category {
    fn images(&self) -> &[Image] { &self.icons }
}

impl RowLike for Category {
    fn content_types() -> Vec<Type> {
        vec![
            Pixbuf::static_type(), // thumb
            String::static_type(), // id
            String::static_type(), // name
        ]
    }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter {
        store.insert_with_values(None, &[COL_CATEGORY_ID, COL_CATEGORY_NAME], &[&self.id, &self.name])
    }
}

impl Merge for Category {
    fn merge(self, other: Self) -> Self {
        Category {
            href: self.href.merge(other.href),
            icons: self.icons.merge(other.icons),
            id: self.id.merge(other.id),
            name: self.name.merge(other.name),
        }
    }
}
