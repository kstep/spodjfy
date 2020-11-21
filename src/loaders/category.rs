use crate::loaders::{
    ContainerLoader, HasDuration, HasImages, RowLike, COL_ITEM_NAME, COL_ITEM_THUMB, COL_ITEM_URI,
};
use crate::servers::spotify::{ResultSender, SpotifyCmd};
use gdk_pixbuf::Pixbuf;
use glib::{IsA, StaticType, Type};
use gtk::prelude::GtkListStoreExtManual;
use rspotify::model::{Category, Image, Page};

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

#[derive(Clone, Copy)]
pub struct CategoriesLoader;

impl ContainerLoader for CategoriesLoader {
    type ParentId = ();
    type Item = Category;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;

    fn new(_id: Self::ParentId) -> Self {
        CategoriesLoader
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetCategories {
            tx,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}

pub const COL_CATEGORY_ID: u32 = COL_ITEM_URI;
pub const COL_CATEGORY_ICON: u32 = COL_ITEM_THUMB;
pub const COL_CATEGORY_NAME: u32 = COL_ITEM_NAME;
