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
use crate::components::lists::common::{ContainerListMsg, GetSelectedRows};
use crate::loaders::common::ContainerLoader;
use crate::loaders::image::ImageLoader;
use crate::loaders::paged::PageLike;
use crate::loaders::playlist::*;
use crate::servers::spotify::SpotifyProxy;
use glib::{Cast, StaticType};
use gtk::prelude::*;
use gtk::{CellRendererExt, CellRendererTextExt, TreeModelExt, TreeViewExt};
use relm::vendor::fragile::Fragile;
use relm::{EventStream, Relm, Update, Widget};
use std::sync::Arc;

const TREE_THUMB_SIZE: i32 = 48;
const ICON_THUMB_SIZE: i32 = 128;
const ICON_ITEM_SIZE: i32 = (ICON_THUMB_SIZE as f32 * 2.25) as i32;

pub struct PlaylistListModel<Loader: ContainerLoader> {
    stream: EventStream<ContainerListMsg<Loader>>,
    spotify: Arc<SpotifyProxy>,
    store: gtk::ListStore,
    items_loader: Option<Loader>,
    image_loader: ImageLoader,
    total_items: u32,
}

pub enum PlaylistView {
    Tree(gtk::TreeView),
    Icon(gtk::IconView),
}
impl PlaylistView {
    fn create<Loader: ContainerLoader, Comp: Update<Msg = ContainerListMsg<Loader>>>(
        events: EventStream<Comp::Msg>,
        store: &gtk::ListStore,
    ) -> PlaylistView
    where
        Loader::Item: PlaylistLike,
    {
        if Loader::Item::unavailable_columns().is_empty() {
            Self::build_tree_view::<Loader, Comp>(events, store).into()
        } else {
            Self::build_icon_view::<Loader, Comp>(events, store).into()
        }
    }

    fn build_icon_view<Loader: ContainerLoader, Comp: Update<Msg = ContainerListMsg<Loader>>>(
        stream: EventStream<Comp::Msg>,
        store: &gtk::ListStore,
    ) -> gtk::IconView
    where
        Loader::Item: PlaylistLike,
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
                .and_then(|model| extract_uri_name(&model, path))
            {
                stream.emit(ContainerListMsg::OpenItem(uri, name));
            }
        });

        playlists_view
    }

    fn build_tree_view<Loader: ContainerLoader, Comp: Update<Msg = ContainerListMsg<Loader>>>(
        stream: EventStream<Comp::Msg>,
        store: &gtk::ListStore,
    ) -> gtk::TreeView
    where
        Loader::Item: PlaylistLike,
    {
        let playlists_view = gtk::TreeViewBuilder::new()
            .model(store)
            .expand(true)
            .reorderable(true)
            .build();

        playlists_view.connect_row_activated(move |view, path, _| {
            if let Some((uri, name)) = view
                .get_model()
                .and_then(|model| extract_uri_name(&model, path))
            {
                stream.emit(ContainerListMsg::OpenItem(uri, name));
            }
        });

        let base_column = gtk::TreeViewColumnBuilder::new()
            .expand(true)
            .resizable(true)
            .reorderable(true);

        let unavailable_columns = Loader::Item::unavailable_columns();

        if !unavailable_columns.contains(&COL_PLAYLIST_THUMB) {
            playlists_view.append_column(&{
                let cell = gtk::CellRendererPixbuf::new();
                let col = gtk::TreeViewColumn::new();
                col.pack_start(&cell, true);
                col.add_attribute(&cell, "pixbuf", COL_PLAYLIST_THUMB as i32);
                col
            });
        }

        if !unavailable_columns.contains(&COL_PLAYLIST_NAME) {
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

        if !unavailable_columns.contains(&COL_PLAYLIST_PUBLISHER) {
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

        if !unavailable_columns.contains(&COL_PLAYLIST_DESCRIPTION) {
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

        if !unavailable_columns.contains(&COL_PLAYLIST_TOTAL_TRACKS) {
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

        if !unavailable_columns.contains(&COL_PLAYLIST_DURATION) {
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

    fn thumb_size(&self) -> i32 {
        match self {
            PlaylistView::Icon(_) => ICON_THUMB_SIZE,
            PlaylistView::Tree(_) => TREE_THUMB_SIZE,
        }
    }

    fn widget(&self) -> &gtk::Widget {
        match self {
            PlaylistView::Tree(view) => view.upcast_ref::<gtk::Widget>(),
            PlaylistView::Icon(view) => view.upcast_ref::<gtk::Widget>(),
        }
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

pub struct PlaylistList<Loader: ContainerLoader> {
    root: gtk::Box,
    items_view: PlaylistView,
    status_bar: gtk::Statusbar,
    model: PlaylistListModel<Loader>,
    progress_bar: gtk::ProgressBar,
    refresh_btn: gtk::Button,
}

impl<Loader> PlaylistList<Loader>
where
    Loader: ContainerLoader,
    Loader::Item: PlaylistLike,
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

impl<Loader> Update for PlaylistList<Loader>
where
    Loader: ContainerLoader,
    Loader::Item: PlaylistLike,
{
    type Model = PlaylistListModel<Loader>;
    type ModelParam = Arc<SpotifyProxy>;
    type Msg = ContainerListMsg<Loader>;

    fn model(relm: &Relm<Self>, spotify: Self::ModelParam) -> Self::Model {
        let stream = relm.stream().clone();

        let store = gtk::ListStore::new(&[
            gdk_pixbuf::Pixbuf::static_type(), // thumb
            String::static_type(),             // uri
            String::static_type(),             // name
            u32::static_type(),                // total tracks
            u32::static_type(),                // duration
            String::static_type(),             // description
            String::static_type(),             // publisher
        ]);

        let image_loader = ImageLoader::new();

        PlaylistListModel {
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

impl<Loader> Widget for PlaylistList<Loader>
where
    Loader: ContainerLoader,
    Loader::Item: PlaylistLike,
{
    type Root = gtk::Box;

    fn root(&self) -> Self::Root {
        self.root.clone()
    }

    fn view(relm: &Relm<Self>, mut model: PlaylistListModel<Loader>) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let scroller = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);

        let items_view = PlaylistView::create::<Loader, Self>(relm.stream().clone(), &model.store);
        model.image_loader.resize = items_view.thumb_size();

        scroller.add(items_view.widget());

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

        PlaylistList {
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
            .get_value(&pos, COL_PLAYLIST_URI as i32)
            .get::<String>()
            .ok()
            .flatten()
            .zip(
                model
                    .get_value(&pos, COL_PLAYLIST_NAME as i32)
                    .get::<String>()
                    .ok()
                    .flatten(),
            )
    })
}
