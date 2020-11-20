use crate::loaders::common::{ContainerLoader, HasImages};
use crate::loaders::image::ImageLoader;
use crate::loaders::paged::{PageLike, RowLike};
use crate::servers::spotify::SpotifyProxy;
use glib::{ToValue, Type};
use gtk::prelude::GtkListStoreExtManual;
use gtk::{
    GtkListStoreExt, IconViewExt, ProgressBarExt, StatusbarExt, TreeModelExt, TreeSelectionExt,
    TreeViewExt, WidgetExt,
};
use relm::vendor::fragile::Fragile;
use relm::{EventStream, Relm, Update};
use relm_derive::Msg;
use std::sync::Arc;

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

#[doc(hidden)]
pub struct ContainerListModel<Loader: ContainerLoader> {
    pub stream: EventStream<ContainerListMsg<Loader>>,
    pub spotify: Arc<SpotifyProxy>,
    pub store: gtk::ListStore,
    pub items_loader: Option<Loader>,
    pub image_loader: ImageLoader,
    pub total_items: u32,
}

impl<Loader: ContainerLoader> ContainerListModel<Loader> {
    pub fn new(
        stream: EventStream<ContainerListMsg<Loader>>,
        spotify: Arc<SpotifyProxy>,
        column_types: &[Type],
    ) -> Self {
        let store = gtk::ListStore::new(column_types);
        let image_loader = ImageLoader::new();

        Self {
            stream,
            store,
            spotify,
            image_loader,
            items_loader: None,
            total_items: 0,
        }
    }
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

pub struct ContainerList<Loader: ContainerLoader, ItemsView> {
    pub root: gtk::Box,
    pub items_view: ItemsView,
    pub status_bar: gtk::Statusbar,
    pub model: ContainerListModel<Loader>,
    pub progress_bar: gtk::ProgressBar,
    pub refresh_btn: gtk::Button,
}

impl<Loader, ItemsView> ContainerList<Loader, ItemsView>
where
    Loader: ContainerLoader,
    Loader::Item: RowLike,
{
    fn clear_store(&mut self) {
        self.model.store.clear();
        self.model.total_items = 0;

        let status_ctx = self.status_bar.get_context_id("totals");
        self.status_bar.remove_all(status_ctx);
    }

    fn start_load(&mut self) {
        if let Some(ref mut loader) = self.model.items_loader {
            *loader = Loader::new(loader.parent_id().clone());
            self.refresh_btn.set_visible(false);
            self.progress_bar.set_fraction(0.0);
            self.progress_bar.set_visible(true);
            self.progress_bar.pulse();
            self.model
                .stream
                .emit(ContainerListMsg::LoadPage(Loader::Page::init_offset()));
        }
    }

    fn finish_load(&self) {
        let status_ctx = self.status_bar.get_context_id("totals");
        self.progress_bar.set_visible(false);
        self.refresh_btn.set_visible(true);
        self.status_bar.remove_all(status_ctx);
        self.status_bar.push(
            status_ctx,
            &format!("Total items: {}", self.model.total_items),
        );
    }
}

impl<Loader, ItemsView> Update for ContainerList<Loader, ItemsView>
where
    Loader: ContainerLoader,
    Loader::Item: RowLike + HasImages,
    ItemsView: GetSelectedRows,
{
    type Model = ContainerListModel<Loader>;
    type ModelParam = Arc<SpotifyProxy>;
    type Msg = ContainerListMsg<Loader>;

    fn model(relm: &Relm<Self>, spotify: Self::ModelParam) -> Self::Model {
        ContainerListModel::new(
            relm.stream().clone(),
            spotify,
            &Loader::Item::content_types(),
        )
    }

    fn update(&mut self, event: Self::Msg) {
        use ContainerListMsg::*;
        match event {
            Clear => {
                self.clear_store();
            }
            Reset(artist_id, reload) => {
                self.model.items_loader = Some(Loader::new(artist_id));
                self.clear_store();
                if reload {
                    self.start_load();
                }
            }
            Reload => {
                self.clear_store();
                self.start_load();
            }
            LoadPage(offset) => {
                if let Some(ref loader) = self.model.items_loader {
                    let loader = loader.clone();
                    self.model
                        .spotify
                        .ask(
                            self.model.stream.clone(),
                            move |tx| loader.load_page(tx, offset),
                            NewPage,
                        )
                        .unwrap();
                }
            }
            NewPage(page) => {
                let stream = &self.model.stream;
                let store = &self.model.store;
                let items = page.items();

                self.progress_bar.set_fraction(
                    (page.num_offset() as f64 + items.len() as f64) / page.total() as f64,
                );

                for item in items {
                    let pos = item.append_to_store(store);

                    let image = crate::loaders::image::find_best_thumb(
                        item.images(),
                        self.model.image_loader.size(),
                    );
                    if let Some(url) = image {
                        stream.emit(LoadThumb(url.to_owned(), pos));
                    }
                }

                if let Some(next_offset) = page.next_offset() {
                    stream.emit(LoadPage(next_offset));
                } else {
                    self.model.total_items = page.total();
                    self.finish_load();
                }
            }
            LoadThumb(url, pos) => {
                let stream = Fragile::new(self.model.stream.clone());
                let pos = Fragile::new(pos);
                self.model.image_loader.load_from_url(&url, move |loaded| {
                    if let Ok(Some(pb)) = loaded {
                        stream.into_inner().emit(NewThumb(pb, pos.into_inner()));
                    }
                });
            }
            NewThumb(thumb, pos) => {
                self.model.store.set_value(&pos, 0, &thumb.to_value());
            }
            OpenChosenItem => {
                let (rows, model) = self.items_view.get_selected_rows();

                if let Some((uri, name)) =
                    rows.first().and_then(|path| extract_uri_name(&model, path))
                {
                    self.model.stream.emit(OpenItem(uri, name));
                }
            }
            OpenItem(_, _) => {}
        }
    }
}

fn extract_uri_name(model: &gtk::TreeModel, path: &gtk::TreePath) -> Option<(String, String)> {
    model.get_iter(path).and_then(|pos| {
        model
            .get_value(&pos, 1 as i32) // COL_ITEM_URI
            .get::<String>()
            .ok()
            .flatten()
            .zip(
                model
                    .get_value(&pos, 2 as i32) // COL_ITEM_NAME
                    .get::<String>()
                    .ok()
                    .flatten(),
            )
    })
}
