//! # Playlists list component
//!
//! A component to show list of playlists of a given parent (e.g. current user playlists, some other user playlists, etc).
//!
//! Parameters:
//!   - `Arc<SpotifyProxy>` - a reference to spotify proxy
//!
//! Usage:
//!
//! ```
//!# use std::sync::{Arc, mpsc::channel};
//!# use spodjfy::servers::spotify::SpotifyProxy;
//!# macro_rules! view { ($body:tt*) => {} }
//!# let (tx, rx) = channel();
//!# let spotify = Arc::new(SpotifyProxy::new(tx));
//! use spodjfy::components::lists::playlist::PlaylistList;
//! use spodjfy::loaders::playlist::SavedLoader;
//!
//! view! {
//!     PlaylistList::<SavedLoader>(spotify.clone())
//! }
//! ```
use crate::components::lists::{ContainerList, ContainerMsg, GetSelectedRows, ItemsListView};
use crate::loaders::playlist::*;
use crate::loaders::{ContainerLoader, MissingColumns};
use glib::Cast;
use gtk::prelude::*;
use gtk::{CellRendererExt, CellRendererTextExt, TreeModelExt, TreeViewExt};
use relm::EventStream;

const TREE_THUMB_SIZE: i32 = 48;
const ICON_THUMB_SIZE: i32 = 128;
const ICON_ITEM_SIZE: i32 = (ICON_THUMB_SIZE as f32 * 2.25) as i32;

pub type PlaylistList<Loader> = ContainerList<Loader, PlaylistView>;

pub enum PlaylistView {
    Tree(gtk::TreeView),
    Icon(gtk::IconView),
}

impl AsRef<gtk::Widget> for PlaylistView {
    fn as_ref(&self) -> &gtk::Widget {
        match self {
            PlaylistView::Icon(view) => view.upcast_ref(),
            PlaylistView::Tree(view) => view.upcast_ref(),
        }
    }
}

impl PlaylistView {
    fn build_icon_view<Loader, Message, Store>(
        stream: EventStream<Message>,
        store: &Store,
    ) -> gtk::IconView
    where
        Loader: ContainerLoader,
        Loader::Item: MissingColumns,
        Store: IsA<gtk::TreeModel>,
        ContainerMsg<Loader>: Into<Message>,
        Message: 'static,
    {
        let playlists_view = gtk::IconViewBuilder::new()
            .model(store)
            .expand(true)
            .reorderable(true)
            .text_column(COL_PLAYLIST_NAME as i32)
            .pixbuf_column(COL_PLAYLIST_THUMB as i32)
            .item_orientation(gtk::Orientation::Horizontal)
            .item_padding(10)
            .item_width(ICON_ITEM_SIZE)
            .build();

        let cells = playlists_view.get_cells();
        if let Some(cell) = cells.last() {
            cell.set_alignment(0.0, 0.0);
            cell.set_padding(10, 0);
            playlists_view.set_cell_data_func(
                cell,
                Some(Box::new(move |_layout, cell, model, pos| {
                    if let (Ok(Some(name)), Ok(Some(publisher)), Ok(Some(tracks)), Some(cell)) = (
                        model.get_value(pos, COL_PLAYLIST_NAME as i32).get::<&str>(),
                        model
                            .get_value(pos, COL_PLAYLIST_PUBLISHER as i32)
                            .get::<&str>(),
                        model
                            .get_value(pos, COL_PLAYLIST_TOTAL_TRACKS as i32)
                            .get::<u32>(),
                        cell.downcast_ref::<gtk::CellRendererText>(),
                    ) {
                        let info = if tracks > 0 {
                            format!("{} by {}\n<i>Tracks: {}</i>", name, publisher, tracks)
                        } else {
                            format!("{} by {}", name, publisher)
                        };

                        cell.set_property_markup(Some(&info));
                    }
                })),
            );
        }

        playlists_view.connect_item_activated(move |view, path| {
            if let Some((uri, name)) = view
                .get_model()
                .and_then(|model| crate::utils::extract_uri_name(&model, path))
            {
                stream.emit(ContainerMsg::ActivateItem(uri, name).into());
            }
        });

        playlists_view
    }

    #[allow(clippy::redundant_clone)]
    fn build_tree_view<Loader, Message, Store>(
        stream: EventStream<Message>,
        store: &Store,
    ) -> gtk::TreeView
    where
        Loader: ContainerLoader,
        Loader::Item: MissingColumns,
        Store: IsA<gtk::TreeModel>,
        ContainerMsg<Loader>: Into<Message>,
        Message: 'static,
    {
        let playlists_view = gtk::TreeViewBuilder::new()
            .model(store)
            .expand(true)
            .reorderable(true)
            .build();

        playlists_view.connect_row_activated(move |view, path, _| {
            if let Some((uri, name)) = view
                .get_model()
                .and_then(|model| crate::utils::extract_uri_name(&model, path))
            {
                stream.emit(ContainerMsg::ActivateItem(uri, name).into());
            }
        });

        let base_column = gtk::TreeViewColumnBuilder::new()
            .expand(true)
            .resizable(true)
            .reorderable(true);

        let missing_columns = Loader::Item::missing_columns();

        if !missing_columns.contains(&COL_PLAYLIST_THUMB) {
            playlists_view.append_column(&{
                let cell = gtk::CellRendererPixbuf::new();
                let col = gtk::TreeViewColumn::new();
                col.pack_start(&cell, true);
                col.add_attribute(&cell, "pixbuf", COL_PLAYLIST_THUMB as i32);
                col
            });
        }

        if !missing_columns.contains(&COL_PLAYLIST_NAME) {
            playlists_view.append_column(&{
                let cell = gtk::CellRendererText::new();
                let col = base_column
                    .clone()
                    .title("Title")
                    .sort_column_id(COL_PLAYLIST_NAME as i32)
                    .build();
                col.pack_start(&cell, true);
                col.add_attribute(&cell, "text", COL_PLAYLIST_NAME as i32);
                col
            });
        }

        if !missing_columns.contains(&COL_PLAYLIST_PUBLISHER) {
            playlists_view.append_column(&{
                let cell = gtk::CellRendererText::new();
                let col = base_column
                    .clone()
                    .title("Publisher")
                    .sort_column_id(COL_PLAYLIST_PUBLISHER as i32)
                    .build();
                col.pack_start(&cell, true);
                col.add_attribute(&cell, "text", COL_PLAYLIST_PUBLISHER as i32);
                col
            });
        }

        if !missing_columns.contains(&COL_PLAYLIST_DESCRIPTION) {
            playlists_view.append_column(&{
                let cell = gtk::CellRendererText::new();
                let col = base_column
                    .clone()
                    .title("Description")
                    .sort_column_id(COL_PLAYLIST_DESCRIPTION as i32)
                    .build();
                col.pack_start(&cell, true);
                col.add_attribute(&cell, "text", COL_PLAYLIST_DESCRIPTION as i32);
                col
            });
        }

        if !missing_columns.contains(&COL_PLAYLIST_TOTAL_TRACKS) {
            playlists_view.append_column(&{
                let cell = gtk::CellRendererText::new();
                cell.set_alignment(1.0, 0.5);
                let col = base_column
                    .clone()
                    .title("Tracks")
                    .expand(false)
                    .sort_column_id(COL_PLAYLIST_TOTAL_TRACKS as i32)
                    .build();
                col.pack_start(&cell, true);
                col.add_attribute(&cell, "text", COL_PLAYLIST_TOTAL_TRACKS as i32);
                col
            });
        }

        if !missing_columns.contains(&COL_PLAYLIST_DURATION) {
            playlists_view.append_column(&{
                let cell = gtk::CellRendererText::new();
                cell.set_alignment(1.0, 0.5);
                let col = base_column
                    .clone()
                    .title("Duration")
                    .expand(false)
                    .sort_column_id(COL_PLAYLIST_DURATION as i32)
                    .build();
                col.pack_start(&cell, true);
                col.add_attribute(&cell, "text", COL_PLAYLIST_DURATION as i32);
                gtk::TreeViewColumnExt::set_cell_data_func(
                    &col,
                    &cell,
                    Some(Box::new(move |_col, cell, model, pos| {
                        if let (Some(cell), Ok(Some(value))) = (
                            cell.downcast_ref::<gtk::CellRendererText>(),
                            model
                                .get_value(pos, COL_PLAYLIST_DURATION as i32)
                                .get::<u32>(),
                        ) {
                            cell.set_property_text(Some(&crate::utils::humanize_time(value)));
                        }
                    })),
                );
                col
            });
        }

        playlists_view
    }
}
impl GetSelectedRows for PlaylistView {
    fn get_selected_rows(&self) -> (Vec<gtk::TreePath>, gtk::TreeModel) {
        match self {
            PlaylistView::Tree(view) => view.get_selected_rows(),
            PlaylistView::Icon(view) => view.get_selected_rows(),
        }
    }
}
impl From<gtk::TreeView> for PlaylistView {
    fn from(tree: gtk::TreeView) -> Self {
        Self::Tree(tree)
    }
}
impl From<gtk::IconView> for PlaylistView {
    fn from(icon: gtk::IconView) -> Self {
        Self::Icon(icon)
    }
}

impl<Loader, Message> ItemsListView<Loader, Message> for PlaylistView
where
    Loader: ContainerLoader,
    Loader::Item: MissingColumns,
    ContainerMsg<Loader>: Into<Message>,
    Message: 'static,
{
    fn create<Store: IsA<gtk::TreeModel>>(stream: EventStream<Message>, store: &Store) -> Self {
        if Loader::Item::missing_columns().is_empty() {
            PlaylistView::build_tree_view::<Loader, Message, Store>(stream, store).into()
        } else {
            PlaylistView::build_icon_view::<Loader, Message, Store>(stream, store).into()
        }
    }

    fn thumb_size(&self) -> i32 {
        match self {
            PlaylistView::Icon(_) => ICON_THUMB_SIZE,
            PlaylistView::Tree(_) => TREE_THUMB_SIZE,
        }
    }
}
