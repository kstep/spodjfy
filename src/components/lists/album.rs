//! # Albums list component
//!
//! A component to show list of albums of a given parent (e.g. artist, user followed albums, etc).
//!
//! Parameters:
//!   - `Arc<SpotifyProxy>` - a reference to spotify proxy
//!
//! Usage:
//!
//! ```
//!# use std::sync::{Arc, mpsc::channel};
//!# use crate::spodjfy::servers::spotify::SpotifyProxy;
//!# macro_rules! view { ($body:tt*) => {} }
//!# let (tx, rx) = channel();
//!# let spotify = Arc::new(SpotifyProxy::new(tx));
//! use spodjfy::components::lists::album::AlbumList;
//! use spodjfy::loaders::album::SavedLoader;
//!
//! view! {
//!     AlbumList::<SavedLoader>(spotify.clone())
//! }
//! ```

use crate::components::lists::common::{ContainerListMsg, GetSelectedRows};
use crate::loaders::album::*;
use crate::loaders::common::ContainerLoader;
use crate::loaders::image::ImageLoader;
use crate::loaders::paged::PageLike;
use crate::servers::spotify::SpotifyProxy;
use glib::{Cast, StaticType};
use gtk::prelude::*;
use gtk::{CellRendererExt, CellRendererTextExt, TreeModelExt, TreeViewExt};
use relm::vendor::fragile::Fragile;
use relm::{EventStream, Relm, Update, Widget};
use std::sync::Arc;

#[doc(hidden)]
const THUMB_SIZE: i32 = 48;

#[doc(hidden)]
pub struct AlbumListModel<Loader: ContainerLoader> {
    stream: EventStream<ContainerListMsg<Loader>>,
    spotify: Arc<SpotifyProxy>,
    store: gtk::ListStore,
    items_loader: Option<Loader>,
    image_loader: ImageLoader,
    total_items: u32,
}

pub struct AlbumList<Loader: ContainerLoader> {
    root: gtk::Box,
    items_view: gtk::TreeView,
    status_bar: gtk::Statusbar,
    model: AlbumListModel<Loader>,
    progress_bar: gtk::ProgressBar,
    refresh_btn: gtk::Button,
}

impl<Loader: ContainerLoader> AlbumList<Loader>
where
    Loader::Item: AlbumLike,
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

    fn create_items_view<S: IsA<gtk::TreeModel>>(
        stream: EventStream<ContainerListMsg<Loader>>,
        store: &S,
    ) -> gtk::TreeView {
        let albums_view = gtk::TreeViewBuilder::new()
            .model(store)
            .expand(true)
            .reorderable(true)
            .build();

        albums_view.connect_row_activated(move |view, path, _| {
            if let Some((uri, name)) = view.get_model().and_then(|model| {
                model.get_iter(path).and_then(|pos| {
                    model
                        .get_value(&pos, COL_ALBUM_URI as i32)
                        .get::<String>()
                        .ok()
                        .flatten()
                        .zip(
                            model
                                .get_value(&pos, COL_ALBUM_NAME as i32)
                                .get::<String>()
                                .ok()
                                .flatten(),
                        )
                })
            }) {
                stream.emit(ContainerListMsg::OpenItem(uri, name));
            }
        });

        let base_column = gtk::TreeViewColumnBuilder::new()
            .expand(true)
            .resizable(true)
            .reorderable(true);

        let unavailable_columns = Loader::Item::unavailable_columns();

        if !unavailable_columns.contains(&COL_ALBUM_THUMB) {
            albums_view.append_column(&{
                let cell = gtk::CellRendererPixbuf::new();
                let col = gtk::TreeViewColumn::new();
                col.pack_start(&cell, true);
                col.add_attribute(&cell, "pixbuf", COL_ALBUM_THUMB as i32);
                col
            });
        }

        if !unavailable_columns.contains(&COL_ALBUM_TYPE) {
            albums_view.append_column(&{
                let cell = gtk::CellRendererText::new();
                let col = base_column
                    .clone()
                    .sort_column_id(COL_ALBUM_TYPE as i32)
                    .expand(false)
                    .build();
                col.pack_start(&cell, true);
                col.add_attribute(&cell, "text", COL_ALBUM_TYPE as i32);
                gtk::TreeViewColumnExt::set_cell_data_func(
                    &col,
                    &cell,
                    Some(Box::new(move |_col, cell, model, pos| {
                        if let (Some(cell), Ok(Some(value))) = (
                            cell.downcast_ref::<gtk::CellRendererText>(),
                            model.get_value(pos, COL_ALBUM_TYPE as i32).get::<u8>(),
                        ) {
                            cell.set_property_text(Some(match value {
                                0 => "\u{1F4BF}", // album
                                1 => "\u{1F3B5}", // single
                                2 => "\u{1F468}", // appears on
                                3 => "\u{1F4DA}", // compilation
                                _ => "?",
                            }));
                        }
                    })),
                );
                col
            });
        }

        if !unavailable_columns.contains(&COL_ALBUM_NAME) {
            albums_view.append_column(&{
                let cell = gtk::CellRendererText::new();
                let col = base_column
                    .clone()
                    .title("Title")
                    .sort_column_id(COL_ALBUM_NAME as i32)
                    .build();
                col.pack_start(&cell, true);
                col.add_attribute(&cell, "text", COL_ALBUM_NAME as i32);
                col
            });
        }

        if !unavailable_columns.contains(&COL_ALBUM_TOTAL_TRACKS) {
            albums_view.append_column(&{
                let cell = gtk::CellRendererText::new();
                cell.set_alignment(1.0, 0.5);
                let col = base_column
                    .clone()
                    .title("Tracks")
                    .expand(false)
                    .sort_column_id(COL_ALBUM_TOTAL_TRACKS as i32)
                    .build();
                col.pack_start(&cell, true);
                col.add_attribute(&cell, "text", COL_ALBUM_TOTAL_TRACKS as i32);
                col
            });
        }

        if !unavailable_columns.contains(&COL_ALBUM_DURATION) {
            albums_view.append_column(&{
                let cell = gtk::CellRendererText::new();
                cell.set_alignment(1.0, 0.5);
                let col = base_column
                    .clone()
                    .title("Duration")
                    .expand(false)
                    .sort_column_id(COL_ALBUM_DURATION as i32)
                    .build();
                col.pack_start(&cell, true);
                col.add_attribute(&cell, "text", COL_ALBUM_DURATION as i32);
                gtk::TreeViewColumnExt::set_cell_data_func(
                    &col,
                    &cell,
                    Some(Box::new(move |_col, cell, model, pos| {
                        if let (Some(cell), Ok(Some(value))) = (
                            cell.downcast_ref::<gtk::CellRendererText>(),
                            model.get_value(pos, COL_ALBUM_DURATION as i32).get::<u32>(),
                        ) {
                            cell.set_property_text(Some(&crate::utils::humanize_time(value)));
                        }
                    })),
                );
                col
            });
        }

        if !unavailable_columns.contains(&COL_ALBUM_RELEASE_DATE) {
            albums_view.append_column(&{
                let cell = gtk::CellRendererText::new();
                cell.set_alignment(1.0, 0.5);
                let col = base_column
                    .clone()
                    .title("Released")
                    .expand(false)
                    .sort_column_id(COL_ALBUM_RELEASE_DATE as i32)
                    .build();
                col.pack_start(&cell, true);
                col.add_attribute(&cell, "text", COL_ALBUM_RELEASE_DATE as i32);
                col
            });
        }

        if !unavailable_columns.contains(&COL_ALBUM_GENRES) {
            albums_view.append_column(&{
                let cell = gtk::CellRendererText::new();
                let col = base_column
                    .clone()
                    .title("Genres")
                    .sort_column_id(COL_ALBUM_GENRES as i32)
                    .build();
                col.pack_start(&cell, true);
                col.add_attribute(&cell, "text", COL_ALBUM_GENRES as i32);
                col
            });
        }

        if !unavailable_columns.contains(&COL_ALBUM_ARTISTS) {
            albums_view.append_column(&{
                let cell = gtk::CellRendererText::new();
                let col = base_column
                    .clone()
                    .title("Artists")
                    .sort_column_id(COL_ALBUM_ARTISTS as i32)
                    .build();
                col.pack_start(&cell, true);
                col.add_attribute(&cell, "text", COL_ALBUM_ARTISTS as i32);
                col
            });
        }

        albums_view
    }
}

impl<Loader: ContainerLoader> Update for AlbumList<Loader>
where
    Loader::Item: AlbumLike,
{
    type Model = AlbumListModel<Loader>;
    type ModelParam = Arc<SpotifyProxy>;
    type Msg = ContainerListMsg<Loader>;

    fn model(relm: &Relm<Self>, spotify: Self::ModelParam) -> Self::Model {
        let stream = relm.stream().clone();

        let store = gtk::ListStore::new(&[
            gdk_pixbuf::Pixbuf::static_type(), // thumb
            String::static_type(),             // uri
            String::static_type(),             // name
            String::static_type(),             // release date
            u32::static_type(),                // total tracks
            String::static_type(),             // artists
            String::static_type(),             // genres
            u8::static_type(),                 // type
            u32::static_type(),                // duration
        ]);

        let image_loader = ImageLoader::new();

        AlbumListModel {
            stream,
            spotify,
            store,
            image_loader,
            items_loader: None,
            total_items: 0,
        }
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
                    let pos = item.insert_into_store(store);

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

impl<Loader> Widget for AlbumList<Loader>
where
    Loader: ContainerLoader,
    Loader::Item: AlbumLike,
{
    type Root = gtk::Box;

    fn root(&self) -> Self::Root {
        self.root.clone()
    }

    fn view(relm: &Relm<Self>, mut model: AlbumListModel<Loader>) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let scroller = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);

        let items_view = Self::create_items_view(relm.stream().clone(), &model.store);
        model.image_loader.resize = THUMB_SIZE;

        scroller.add(&items_view);

        root.add(&scroller);

        let status_bar = gtk::Statusbar::new();

        let progress_bar = gtk::ProgressBarBuilder::new()
            .valign(gtk::Align::Center)
            .width_request(200)
            .visible(false)
            .show_text(true)
            .build();
        status_bar.pack_end(&progress_bar, false, true, 0);

        let refresh_btn =
            gtk::Button::from_icon_name(Some("view-refresh"), gtk::IconSize::SmallToolbar);
        let stream = relm.stream().clone();
        refresh_btn.connect_clicked(move |_| stream.emit(ContainerListMsg::Reload));
        status_bar.pack_start(&refresh_btn, false, false, 0);

        root.add(&status_bar);

        root.show_all();

        AlbumList {
            root,
            items_view,
            status_bar,
            progress_bar,
            refresh_btn,
            model,
        }
    }
}

fn extract_uri_name(model: &gtk::TreeModel, path: &gtk::TreePath) -> Option<(String, String)> {
    model.get_iter(path).and_then(|pos| {
        model
            .get_value(&pos, COL_ALBUM_URI as i32)
            .get::<String>()
            .ok()
            .flatten()
            .zip(
                model
                    .get_value(&pos, COL_ALBUM_NAME as i32)
                    .get::<String>()
                    .ok()
                    .flatten(),
            )
    })
}
