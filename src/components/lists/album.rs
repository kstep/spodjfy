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

use crate::components::lists::common::{
    ContainerList, ContainerListMsg, GetSelectedRows, ItemsListView,
};
use crate::loaders::album::*;
use crate::loaders::common::{ContainerLoader, MissingColumns};
use glib::Cast;
use gtk::prelude::*;
use gtk::{CellRendererExt, CellRendererTextExt, TreeModel, TreeModelExt, TreePath, TreeViewExt};
use relm::EventStream;

#[doc(hidden)]
const THUMB_SIZE: i32 = 48;

pub struct AlbumView(gtk::TreeView);
impl From<gtk::TreeView> for AlbumView {
    fn from(view: gtk::TreeView) -> Self {
        AlbumView(view)
    }
}
impl AsRef<gtk::Widget> for AlbumView {
    fn as_ref(&self) -> &gtk::Widget {
        self.0.upcast_ref()
    }
}
impl GetSelectedRows for AlbumView {
    fn get_selected_rows(&self) -> (Vec<TreePath>, TreeModel) {
        self.0.get_selected_rows()
    }
}

impl<Loader> ItemsListView<Loader> for AlbumView
where
    Loader: ContainerLoader,
    Loader::Item: MissingColumns,
{
    type CustomMsg = ();

    fn create<S: IsA<gtk::TreeModel>>(
        stream: EventStream<ContainerListMsg<Loader>>,
        store: &S,
    ) -> Self {
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
                stream.emit(ContainerListMsg::ActivateItem(uri, name));
            }
        });

        let base_column = gtk::TreeViewColumnBuilder::new()
            .expand(true)
            .resizable(true)
            .reorderable(true);

        let missing_columns = Loader::Item::missing_columns();

        if !missing_columns.contains(&COL_ALBUM_THUMB) {
            albums_view.append_column(&{
                let cell = gtk::CellRendererPixbuf::new();
                let col = gtk::TreeViewColumn::new();
                col.pack_start(&cell, true);
                col.add_attribute(&cell, "pixbuf", COL_ALBUM_THUMB as i32);
                col
            });
        }

        if !missing_columns.contains(&COL_ALBUM_TYPE) {
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

        if !missing_columns.contains(&COL_ALBUM_NAME) {
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

        if !missing_columns.contains(&COL_ALBUM_TOTAL_TRACKS) {
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

        if !missing_columns.contains(&COL_ALBUM_DURATION) {
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

        if !missing_columns.contains(&COL_ALBUM_RELEASE_DATE) {
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

        if !missing_columns.contains(&COL_ALBUM_GENRES) {
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

        if !missing_columns.contains(&COL_ALBUM_ARTISTS) {
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

        AlbumView(albums_view)
    }

    fn thumb_size(&self) -> i32 {
        THUMB_SIZE
    }
}

pub type AlbumList<Loader> = ContainerList<Loader, AlbumView>;
