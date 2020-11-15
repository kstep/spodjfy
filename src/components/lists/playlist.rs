use crate::loaders::image::ImageLoader;
use crate::loaders::paged::PageLike;
use crate::loaders::playlist::*;
use crate::servers::spotify::SpotifyProxy;
use glib::{Cast, StaticType};
use gtk::prelude::*;
use gtk::{CellRendererExt, CellRendererTextExt, TreeModelExt, TreeViewExt};
use relm::vendor::fragile::Fragile;
use relm::{EventStream, Relm, Update, Widget};
use relm_derive::Msg;
use std::sync::Arc;

const TREE_THUMB_SIZE: i32 = 48;
const ICON_THUMB_SIZE: i32 = 128;

#[derive(Msg)]
pub enum PlaylistListMsg<Loader: PlaylistsLoader> {
    Clear,
    Reset(Loader::ParentId, bool),
    Reload,
    LoadPage(<Loader::Page as PageLike<Loader::Playlist>>::Offset),
    NewPage(Loader::Page),
    LoadThumb(String, gtk::TreeIter),
    NewThumb(gdk_pixbuf::Pixbuf, gtk::TreeIter),
    OpenChosenPlaylist,
    OpenPlaylist(String, String),
}

pub struct PlaylistListModel<Loader: PlaylistsLoader> {
    stream: EventStream<PlaylistListMsg<Loader>>,
    spotify: Arc<SpotifyProxy>,
    store: gtk::ListStore,
    playlists_loader: Option<Loader>,
    image_loader: ImageLoader,
    total_playlists: u32,
    playlists_view: PlaylistView,
}

pub enum PlaylistView {
    Tree(gtk::TreeView),
    Icon(gtk::IconView),
}
impl PlaylistView {
    fn thumb_size(&self) -> i32 {
        match self {
            PlaylistView::Icon(_) => ICON_THUMB_SIZE,
            PlaylistView::Tree(_) => TREE_THUMB_SIZE,
        }
    }
    fn get_selected_items(&self) -> (Vec<gtk::TreePath>, gtk::TreeModel) {
        match self {
            PlaylistView::Tree(view) => {
                let select = view.get_selection();
                select.get_selected_rows()
            }
            PlaylistView::Icon(view) => {
                let items = view.get_selected_items();
                (items, view.get_model().unwrap())
            }
        }
    }

    fn widget(&self) -> &gtk::Widget {
        match self {
            PlaylistView::Tree(view) => view.upcast_ref::<gtk::Widget>(),
            PlaylistView::Icon(view) => view.upcast_ref::<gtk::Widget>(),
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

pub struct PlaylistList<Loader: PlaylistsLoader> {
    root: gtk::Box,
    status_bar: gtk::Statusbar,
    model: PlaylistListModel<Loader>,
    progress_bar: gtk::ProgressBar,
    refresh_btn: gtk::Button,
}

impl<Loader: PlaylistsLoader> PlaylistList<Loader> {
    fn clear_store(&mut self) {
        self.model.store.clear();
        self.model.total_playlists = 0;

        let status_ctx = self.status_bar.get_context_id("totals");
        self.status_bar.remove_all(status_ctx);
    }

    fn start_load(&mut self) {
        if let Some(ref mut loader) = self.model.playlists_loader {
            *loader = Loader::new(loader.parent_id());
            self.refresh_btn.set_visible(false);
            self.progress_bar.set_fraction(0.0);
            self.progress_bar.set_visible(true);
            self.progress_bar.pulse();
            self.model
                .stream
                .emit(PlaylistListMsg::LoadPage(Loader::Page::init_offset()));
        }
    }

    fn finish_load(&self) {
        let status_ctx = self.status_bar.get_context_id("totals");
        self.progress_bar.set_visible(false);
        self.refresh_btn.set_visible(true);
        self.status_bar.remove_all(status_ctx);
        self.status_bar.push(
            status_ctx,
            &format!("Total playlists: {}", self.model.total_playlists),
        );
    }

    fn build_playlists_view(relm: &Relm<Self>, store: &gtk::ListStore) -> PlaylistView {
        if Loader::Playlist::unavailable_columns().is_empty() {
            Self::build_tree_view(relm, store).into()
        } else {
            Self::build_icon_view(relm, store).into()
        }
    }

    fn build_icon_view(relm: &Relm<Self>, store: &gtk::ListStore) -> gtk::IconView {
        let playlists_view = gtk::IconViewBuilder::new()
            .model(store)
            .expand(true)
            .reorderable(true)
            .text_column(COL_PLAYLIST_NAME as i32)
            .pixbuf_column(COL_PLAYLIST_THUMB as i32)
            .item_orientation(gtk::Orientation::Horizontal)
            .item_padding(10)
            .item_width(256)
            .build();

        let cells = playlists_view.get_cells();
        if let Some(cell) = cells.last() {
            cell.set_alignment(0.0, 0.0);
            cell.set_padding(10, 0);
            playlists_view.set_cell_data_func(
                cell,
                Some(Box::new(move |_layout, cell, model, pos| {
                    if let (Ok(Some(name)), Ok(Some(tracks)), Some(cell)) = (
                        model.get_value(pos, COL_PLAYLIST_NAME as i32).get::<&str>(),
                        model
                            .get_value(pos, COL_PLAYLIST_TOTAL_TRACKS as i32)
                            .get::<u32>(),
                        cell.downcast_ref::<gtk::CellRendererText>(),
                    ) {
                        cell.set_property_markup(Some(&format!(
                            "{}\n<i>Tracks: {}</i>",
                            name, tracks
                        )));
                    }
                })),
            );
        }

        let stream = relm.stream().clone();
        playlists_view.connect_item_activated(move |view, path| {
            if let Some((uri, name)) = view
                .get_model()
                .and_then(|model| extract_uri_name(&model, path))
            {
                stream.emit(PlaylistListMsg::OpenPlaylist(uri, name));
            }
        });

        playlists_view
    }

    fn build_tree_view(relm: &Relm<Self>, store: &gtk::ListStore) -> gtk::TreeView {
        let playlists_view = gtk::TreeViewBuilder::new()
            .model(store)
            .expand(true)
            .reorderable(true)
            .build();

        let stream = relm.stream().clone();
        playlists_view.connect_row_activated(move |view, path, _| {
            if let Some((uri, name)) = view
                .get_model()
                .and_then(|model| extract_uri_name(&model, path))
            {
                stream.emit(PlaylistListMsg::OpenPlaylist(uri, name));
            }
        });

        let base_column = gtk::TreeViewColumnBuilder::new()
            .expand(true)
            .resizable(true)
            .reorderable(true);

        let unavailable_columns = Loader::Playlist::unavailable_columns();

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
}

impl<Loader: PlaylistsLoader> Update for PlaylistList<Loader> {
    type Model = PlaylistListModel<Loader>;
    type ModelParam = Arc<SpotifyProxy>;
    type Msg = PlaylistListMsg<Loader>;

    fn model(relm: &Relm<Self>, spotify: Self::ModelParam) -> Self::Model {
        let stream = relm.stream().clone();

        let store = gtk::ListStore::new(&[
            gdk_pixbuf::Pixbuf::static_type(), // thumb
            String::static_type(),             // uri
            String::static_type(),             // name
            u32::static_type(),                // total tracks
            u32::static_type(),                // duration
            String::static_type(),             // description
        ]);

        let playlists_view = Self::build_playlists_view(relm, &store);
        let image_loader = ImageLoader::new_with_resize(playlists_view.thumb_size());

        PlaylistListModel {
            stream,
            spotify,
            store,
            image_loader,
            playlists_view,
            playlists_loader: None,
            total_playlists: 0,
        }
    }

    fn update(&mut self, event: Self::Msg) {
        use PlaylistListMsg::*;
        match event {
            Clear => {
                self.clear_store();
            }
            Reset(artist_id, reload) => {
                self.model.playlists_loader = Some(Loader::new(artist_id));
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
                if let Some(ref loader) = self.model.playlists_loader {
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
                let playlists = page.items();

                self.progress_bar.set_fraction(
                    (page.num_offset() as f64 + playlists.len() as f64) / page.total() as f64,
                );

                for playlist in playlists {
                    let pos = store.insert_with_values(
                        None,
                        &[
                            COL_PLAYLIST_URI,
                            COL_PLAYLIST_NAME,
                            COL_PLAYLIST_TOTAL_TRACKS,
                            COL_PLAYLIST_DURATION,
                            COL_PLAYLIST_DESCRIPTION,
                        ],
                        &[
                            &playlist.uri(),
                            &playlist.name(),
                            &playlist.total_tracks(),
                            &playlist.duration(),
                            &playlist.description(),
                        ],
                    );

                    let image = crate::loaders::image::find_best_thumb(
                        playlist.images(),
                        self.model.image_loader.size(),
                    );
                    if let Some(url) = image {
                        stream.emit(LoadThumb(url.to_owned(), pos));
                    }
                }

                if let Some(next_offset) = page.next_offset() {
                    stream.emit(LoadPage(next_offset));
                } else {
                    self.model.total_playlists = page.total();
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
            OpenChosenPlaylist => {
                let (rows, model) = self.model.playlists_view.get_selected_items();

                if let Some((uri, name)) =
                    rows.first().and_then(|path| extract_uri_name(&model, path))
                {
                    self.model.stream.emit(OpenPlaylist(uri, name));
                }
            }
            OpenPlaylist(_, _) => {}
        }
    }
}

impl<Loader: PlaylistsLoader> Widget for PlaylistList<Loader> {
    type Root = gtk::Box;

    fn root(&self) -> Self::Root {
        self.root.clone()
    }

    fn view(relm: &Relm<Self>, model: PlaylistListModel<Loader>) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let scroller = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);

        let playlists_view = &model.playlists_view;

        scroller.add(playlists_view.widget());

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
        refresh_btn.connect_clicked(move |_| stream.emit(PlaylistListMsg::Reload));
        status_bar.pack_start(&refresh_btn, false, false, 0);

        root.add(&status_bar);

        root.show_all();

        PlaylistList {
            root,
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
