use crate::models::common::*;
use gdk_pixbuf::Pixbuf;
use glib::{IsA, StaticType, Type};
use gtk::prelude::GtkListStoreExtManual;
use rspotify::model::{Category, Image};

pub const COL_CATEGORY_ID: u32 = COL_ITEM_URI;
pub const COL_CATEGORY_ICON: u32 = COL_ITEM_THUMB;
pub const COL_CATEGORY_NAME: u32 = COL_ITEM_NAME;

impl HasDuration for Category {
    fn duration_exact(&self) -> bool {
        false
    }
}

impl HasImages for Category {
    fn images(&self) -> &[Image] {
        &self.icons
    }
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
        store.insert_with_values(
            None,
            &[COL_CATEGORY_ID, COL_CATEGORY_NAME],
            &[&self.id, &self.name],
        )
    }
}