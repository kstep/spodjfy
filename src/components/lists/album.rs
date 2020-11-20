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

use crate::components::lists::common::{ContainerList, ContainerListMsg};
use crate::loaders::album::*;
use crate::loaders::common::{ContainerLoader, HasImages, MissingColumns};
use crate::loaders::paged::RowLike;
use glib::Cast;
use gtk::prelude::*;
use gtk::{CellRendererExt, CellRendererTextExt, TreeModelExt, TreeViewExt};
use relm::{EventStream, Relm, Widget};

#[doc(hidden)]
const THUMB_SIZE: i32 = 48;

pub type AlbumList<Loader> = ContainerList<Loader, gtk::TreeView>;

impl<Loader> ContainerList<Loader, gtk::TreeView>
where
    Loader: ContainerLoader,
    Loader::Item: MissingColumns,
{
    pub fn create_items_view<S: IsA<gtk::TreeModel>>(
        stream: EventStream<ContainerListMsg<Loader>>,
        store: &S,
    ) -> (gtk::TreeView, i32) {
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

        (albums_view, THUMB_SIZE)
    }
}

impl<Loader> Widget for ContainerList<Loader, gtk::TreeView>
where
    Loader: ContainerLoader,
    Loader::Item: RowLike + HasImages + MissingColumns,
{
    type Root = gtk::Box;

    fn root(&self) -> Self::Root {
        self.root.clone()
    }

    fn view(relm: &Relm<Self>, mut model: Self::Model) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let scroller = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);

        let (items_view, thumb_size) = Self::create_items_view(relm.stream().clone(), &model.store);
        model.image_loader.resize = thumb_size;

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

        ContainerList {
            root,
            items_view,
            status_bar,
            progress_bar,
            refresh_btn,
            model,
        }
    }
}
