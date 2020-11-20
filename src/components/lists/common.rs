use crate::loaders::common::ContainerLoader;
use crate::loaders::paged::PageLike;
use gtk::{IconViewExt, TreeSelectionExt, TreeViewExt};
use relm_derive::Msg;

#[derive(Msg)]
pub enum ContainerListMsg<Loader: ContainerLoader> {
    /// Clear list
    Clear,
    /// Reset list, set new parent id to list from,
    /// if second argument is `true`, also reload list
    Reset(Loader::ParentId, bool),
    /// Reload current list
    Reload,
    /// Load one page from Spotify with an offset
    LoadPage(<Loader::Page as PageLike<Loader::Item>>::Offset),
    /// New page from Spotify arrived
    NewPage(Loader::Page),
    /// Load item thumbnail from URI for a row
    LoadThumb(String, gtk::TreeIter),
    /// New item thumbnail image for a row arrived
    NewThumb(gdk_pixbuf::Pixbuf, gtk::TreeIter),
    /// Item in list is activated (e.g. double-clicked)
    OpenChosenItem,
    /// Open given item (uri, name), show tracks list for the item
    OpenItem(String, String),
}

pub trait GetSelectedRows {
    fn get_selected_rows(&self) -> (Vec<gtk::TreePath>, gtk::TreeModel);
}

impl GetSelectedRows for gtk::TreeView {
    fn get_selected_rows(&self) -> (Vec<gtk::TreePath>, gtk::TreeModel) {
        let select = self.get_selection();
        select.get_selected_rows()
    }
}

impl GetSelectedRows for gtk::IconView {
    fn get_selected_rows(&self) -> (Vec<gtk::TreePath>, gtk::TreeModel) {
        let items = self.get_selected_items();
        (items, self.get_model().unwrap())
    }
}
