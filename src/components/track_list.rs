use crate::loaders::image::ImageLoader;
use crate::loaders::paged::{PageLike, RowLike};
use crate::loaders::track::*;
use crate::servers::spotify::{SpotifyCmd, SpotifyProxy};
use gdk_pixbuf::Pixbuf;
use glib::StaticType;
use gtk::prelude::*;
use gtk::{
    ButtonExt, CellRendererExt, GtkMenuExt, GtkMenuItemExt, Inhibit, ProgressBarExt, StatusbarExt,
    TreeModelExt, TreeViewColumn, TreeViewExt,
};
use relm::vendor::fragile::Fragile;
use relm::{EventStream, Relm, Update, Widget};
use relm_derive::Msg;
use rspotify::model::audio::AudioFeatures;
use std::sync::Arc;

pub trait ControlSpotifyContext {
    fn play_tracks(&self, uris: Vec<String>);
}

#[derive(Msg)]
pub enum TrackListMsg<Loader: TracksLoader> {
    Clear,
    Reset(Loader::ParentId, bool),
    Reload,
    LoadPage(<Loader::Page as PageLike<Loader::Track>>::Offset, usize),
    NewPage(Loader::Page, usize),

    LoadThumb(String, gtk::TreeIter),
    NewThumb(gdk_pixbuf::Pixbuf, gtk::TreeIter),
    PlayChosenTracks,
    PlayTracks(Vec<String>),
    PlayingNewTrack,
    LoadTracksInfo(Vec<String>, Vec<gtk::TreeIter>),
    NewTracksInfo(Vec<AudioFeatures>, Vec<gtk::TreeIter>),

    NewBpm(gtk::TreePath, f32),
    OpenContextMenu(gdk::EventButton),

    GoToTrack(String),
    GoToChosenTrackAlbum,
    GoToChosenTrackArtist,
    EnqueueChosenTracks,
    AddChosenTracks,
    SaveChosenTracks,
    RecommendTracks,
    UnsaveChosenTracks,
}

const THUMB_SIZE: i32 = 32;

pub struct TrackListModel<Loader: TracksLoader> {
    stream: EventStream<TrackListMsg<Loader>>,
    spotify: Arc<SpotifyProxy>,
    store: gtk::ListStore,
    image_loader: ImageLoader,
    tracks_loader: Option<Loader>,
    total_tracks: u32,
    total_duration: u32,
}

pub struct TrackList<Loader: TracksLoader> {
    model: TrackListModel<Loader>,
    root: gtk::Box,
    tracks_view: gtk::TreeView,
    context_menu: gtk::Menu,
    status_bar: gtk::Statusbar,
    progress_bar: gtk::ProgressBar,
    refresh_btn: gtk::Button,
}

impl ControlSpotifyContext for TrackList<RecentLoader> {
    fn play_tracks(&self, uris: Vec<String>) {
        self.model
            .spotify
            .tell(SpotifyCmd::PlayTracks { uris })
            .unwrap();
    }
}

impl ControlSpotifyContext for TrackList<SavedLoader> {
    fn play_tracks(&self, uris: Vec<String>) {
        self.model
            .spotify
            .tell(SpotifyCmd::PlayTracks { uris })
            .unwrap();
    }
}

impl<T> ControlSpotifyContext for TrackList<T>
where
    T: TracksLoader<ParentId = String>,
{
    fn play_tracks(&self, uris: Vec<String>) {
        if let Some(ref loader) = self.model.tracks_loader {
            self.model
                .spotify
                .tell(SpotifyCmd::PlayContext {
                    uri: loader.parent_id(),
                    start_uri: uris.first().cloned(),
                })
                .unwrap();
        }
    }
}

impl<Loader: TracksLoader> TrackList<Loader> {
    fn clear_store(&mut self) {
        self.model.store.clear();
        self.model.total_duration = 0;
        self.model.total_tracks = 0;
    }

    fn start_load(&mut self) {
        if let Some(ref mut loader) = self.model.tracks_loader {
            *loader = Loader::new(loader.parent_id());
            let epoch = loader.uuid();
            self.refresh_btn.set_visible(false);
            self.progress_bar.set_fraction(0.0);
            self.progress_bar.set_visible(true);
            self.progress_bar.pulse();
            self.model
                .stream
                .emit(TrackListMsg::LoadPage(Loader::Page::init_offset(), epoch));
        }
    }

    fn load_tracks_page(&self, offset: <Loader::Page as PageLike<Loader::Track>>::Offset) {
        if let Some(ref loader) = self.model.tracks_loader {
            let epoch = loader.uuid();
            let loader = loader.clone();
            self.model
                .spotify
                .ask(
                    self.model.stream.clone(),
                    move |tx| loader.load_tracks_page(tx, offset),
                    move |reply| TrackListMsg::NewPage(reply, epoch),
                )
                .unwrap();
        }
    }

    fn get_selected_tracks_uris(&self) -> Vec<String> {
        let select = self.tracks_view.get_selection();
        let (rows, model) = select.get_selected_rows();
        rows.into_iter()
            .filter_map(|path| model.get_iter(&path))
            .filter_map(|pos| {
                model
                    .get_value(&pos, COL_TRACK_URI as i32)
                    .get::<String>()
                    .ok()
                    .flatten()
            })
            .collect::<Vec<_>>()
    }
}

impl<Loader> Update for TrackList<Loader>
where
    Loader: TracksLoader,
    TrackList<Loader>: ControlSpotifyContext,
{
    type Model = TrackListModel<Loader>;
    type ModelParam = Arc<SpotifyProxy>;
    type Msg = TrackListMsg<Loader>;

    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> Self::Model {
        let store = gtk::ListStore::new(&[
            String::static_type(), // id
            Pixbuf::static_type(), // thumb
            String::static_type(), // name
            String::static_type(), // artists
            u32::static_type(),    // number
            String::static_type(), // album
            bool::static_type(),   // is playable
            String::static_type(), // formatted duration
            u32::static_type(),    // duration in ms
            String::static_type(), // track uri
            f32::static_type(),    // bpm
            String::static_type(), // duration from start
            String::static_type(), // release date
        ]);

        let stream = relm.stream().clone();

        TrackListModel {
            stream,
            spotify,
            store,
            image_loader: ImageLoader::new_with_resize(THUMB_SIZE),
            tracks_loader: None,
            total_duration: 0,
            total_tracks: 0,
        }
    }

    fn update(&mut self, event: Self::Msg) {
        use TrackListMsg::*;
        match event {
            Clear => {
                self.clear_store();
            }
            Reset(parent_id, reload) => {
                self.model.tracks_loader = Some(Loader::new(parent_id));
                self.clear_store();
                if reload {
                    self.start_load()
                }
            }
            Reload => {
                self.clear_store();
                self.start_load();
            }
            LoadPage(offset, epoch) => {
                if epoch
                    == self
                        .model
                        .tracks_loader
                        .as_ref()
                        .map_or(0, |ldr| ldr.uuid())
                {
                    self.load_tracks_page(offset);
                }
            }
            NewPage(page, epoch) => {
                if epoch
                    != self
                        .model
                        .tracks_loader
                        .as_ref()
                        .map_or(0, |ldr| ldr.uuid())
                {
                    return;
                }

                let stream = &self.model.stream;
                let store = &self.model.store;
                let tracks = page.items();
                let offset = page.num_offset();

                self.progress_bar
                    .set_fraction((offset as f64 + tracks.len() as f64) / page.total() as f64);

                let mut uris = Vec::with_capacity(tracks.len());
                let mut iters = Vec::with_capacity(tracks.len());

                let mut page_duration = 0;
                for (idx, track) in tracks.iter().enumerate() {
                    let pos = track.append_to_store(store);
                    store.set(
                        &pos,
                        &[COL_TRACK_NUMBER, COL_TRACK_TIMELINE],
                        &[
                            &(idx as u32 + offset + 1),
                            &crate::utils::humanize_time(self.model.total_duration + page_duration),
                        ],
                    );

                    let image = track.images().and_then(|images| {
                        crate::loaders::image::find_best_thumb(images, THUMB_SIZE)
                    });

                    if let Some(url) = image {
                        stream.emit(LoadThumb(url.to_owned(), pos.clone()));
                    }

                    uris.push(track.uri().to_owned());
                    iters.push(pos);
                    page_duration += track.duration();
                }

                self.model.total_duration += page_duration;

                if let Some(next_offset) = page.next_offset() {
                    stream.emit(LoadPage(next_offset, epoch));
                } else {
                    self.model.total_tracks = page.total();

                    let status_ctx = self.status_bar.get_context_id("totals");
                    self.progress_bar.set_visible(false);
                    self.refresh_btn.set_visible(true);
                    self.status_bar.remove_all(status_ctx);
                    self.status_bar.push(
                        status_ctx,
                        &format!(
                            "Total tracks: {}, total duration: {}",
                            self.model.total_tracks,
                            crate::utils::humanize_time(self.model.total_duration)
                        ),
                    );
                }

                if !Loader::Track::unavailable_columns().contains(&COL_TRACK_BPM) {
                    stream.emit(LoadTracksInfo(uris, iters));
                }
            }
            LoadTracksInfo(uris, iters) => {
                self.model
                    .spotify
                    .ask(
                        self.model.stream.clone(),
                        |tx| SpotifyCmd::GetTracksFeatures { tx, uris },
                        move |feats| NewTracksInfo(feats, iters.clone()),
                    )
                    .unwrap();
            }
            NewTracksInfo(info, iters) => {
                let store = &self.model.store;
                for (idx, pos) in iters.iter().enumerate() {
                    store.set(pos, &[COL_TRACK_BPM], &[&info[idx].tempo]);
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
                self.model
                    .store
                    .set_value(&pos, COL_TRACK_THUMB, &thumb.to_value());
            }
            GoToTrack(track_id) => {
                let store = &self.model.store;
                if let Some(pos) = store.get_iter_first() {
                    loop {
                        if let Ok(Some(uri)) =
                            store.get_value(&pos, COL_TRACK_URI as i32).get::<&str>()
                        {
                            if uri == track_id {
                                let select = self.tracks_view.get_selection();
                                select.unselect_all();
                                select.select_iter(&pos);

                                self.tracks_view.scroll_to_cell(
                                    store.get_path(&pos).as_ref(),
                                    None::<&gtk::TreeViewColumn>,
                                    false,
                                    0.0,
                                    0.0,
                                );

                                break;
                            }
                        }
                        if !store.iter_next(&pos) {
                            break;
                        }
                    }
                }
            }
            OpenContextMenu(event) => {
                self.context_menu.popup_at_pointer(Some(&event));
            }
            PlayChosenTracks => {
                let uris = self.get_selected_tracks_uris();
                self.model.stream.emit(PlayTracks(uris));
            }
            EnqueueChosenTracks => {
                let uris = self.get_selected_tracks_uris();
                self.model
                    .spotify
                    .tell(SpotifyCmd::EnqueueTracks { uris })
                    .unwrap();
            }
            AddChosenTracks => {}
            SaveChosenTracks => {
                let uris = self.get_selected_tracks_uris();
                self.model
                    .spotify
                    .tell(SpotifyCmd::AddMyTracks { uris })
                    .unwrap();
            }
            UnsaveChosenTracks => {
                let uris = self.get_selected_tracks_uris();
                self.model
                    .spotify
                    .tell(SpotifyCmd::RemoveMyTracks { uris })
                    .unwrap();
            }
            RecommendTracks => {}
            GoToChosenTrackAlbum => {}
            GoToChosenTrackArtist => {}
            PlayTracks(uris) => {
                self.play_tracks(uris);
                self.model.stream.emit(PlayingNewTrack);
            }
            PlayingNewTrack => {}
            NewBpm(path, bpm) => {
                let store = &self.model.store;
                if let Some(iter) = store.get_iter(&path) {
                    store.set_value(&iter, COL_TRACK_BPM, &bpm.to_value());
                }
            }
        }
    }
}

impl<Loader> Widget for TrackList<Loader>
where
    Loader: TracksLoader,
    TrackList<Loader>: ControlSpotifyContext,
{
    type Root = gtk::Box;

    fn root(&self) -> Self::Root {
        self.root.clone()
    }

    #[allow(clippy::redundant_clone)]
    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let scroller = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);

        let tracks_view = gtk::TreeViewBuilder::new()
            .model(&model.store)
            .expand(true)
            .reorderable(true)
            .build();

        tracks_view
            .get_selection()
            .set_mode(gtk::SelectionMode::Multiple);

        let base_column = gtk::TreeViewColumnBuilder::new()
            .resizable(true)
            .reorderable(true)
            .expand(true);

        let unavailable_columns = Loader::Track::unavailable_columns();

        if !unavailable_columns.contains(&COL_TRACK_NUMBER) {
            tracks_view.append_column(&{
                let text_cell = gtk::CellRendererText::new();
                text_cell.set_alignment(1.0, 0.5);

                let column = base_column
                    .clone()
                    .expand(false)
                    .title("#")
                    .sort_column_id(COL_TRACK_NUMBER as i32)
                    .alignment(1.0)
                    .build();
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_NUMBER as i32);
                column
            });
        }

        if !unavailable_columns.contains(&COL_TRACK_THUMB) {
            tracks_view.append_column(&{
                let icon_cell = gtk::CellRendererPixbuf::new();
                icon_cell.set_property_icon_name(Some("audio-x-generic-symbolic"));

                let column = TreeViewColumn::new();
                column.pack_start(&icon_cell, true);
                column.add_attribute(&icon_cell, "pixbuf", COL_TRACK_THUMB as i32);
                column
            });
        }

        if !unavailable_columns.contains(&COL_TRACK_NAME) {
            tracks_view.append_column(&{
                let text_cell = gtk::CellRendererText::new();
                let column = base_column
                    .clone()
                    .title("Title")
                    .sort_column_id(COL_TRACK_NAME as i32)
                    .build();
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_NAME as i32);
                column.add_attribute(&text_cell, "strikethrough", COL_TRACK_CANT_PLAY as i32);
                column
            });
        }

        if !unavailable_columns.contains(&COL_TRACK_DURATION) {
            tracks_view.append_column(&{
                let text_cell = gtk::CellRendererText::new();
                text_cell.set_alignment(1.0, 0.5);
                let column = base_column
                    .clone()
                    .expand(false)
                    .title("Duration")
                    .sort_column_id(COL_TRACK_DURATION_MS as i32)
                    .build();
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_DURATION as i32);
                column
            });
        }

        if !unavailable_columns.contains(&COL_TRACK_TIMELINE) {
            tracks_view.append_column(&{
                let text_cell = gtk::CellRendererText::new();
                text_cell.set_alignment(1.0, 0.5);
                let column = base_column
                    .clone()
                    .expand(false)
                    .title("Timeline")
                    .sort_column_id(COL_TRACK_NUMBER as i32)
                    .build();
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_TIMELINE as i32);
                column
            });
        }

        if !unavailable_columns.contains(&COL_TRACK_BPM) {
            tracks_view.append_column(&{
                let text_cell = gtk::CellRendererTextBuilder::new()
                    .xalign(1.0)
                    .editable(true)
                    .mode(gtk::CellRendererMode::Editable)
                    .build();

                let stream = relm.stream().clone();
                text_cell.connect_edited(move |_, path, new_text| {
                    if let Ok(bpm) = new_text.parse::<f32>() {
                        stream.emit(TrackListMsg::NewBpm(path, bpm));
                    }
                });
                let column = base_column
                    .clone()
                    .expand(false)
                    .title("BPM")
                    .sort_column_id(COL_TRACK_BPM as i32)
                    .build();
                <TreeViewColumn as TreeViewColumnExt>::set_cell_data_func(
                    &column,
                    &text_cell,
                    Some(Box::new(|_layout, cell, model, iter| {
                        let bpm: f32 = model
                            .get_value(iter, COL_TRACK_BPM as i32)
                            .get()
                            .ok()
                            .flatten()
                            .unwrap_or(0.0);
                        let _ = cell.set_property("text", &format!("{:.0}", bpm));
                    })),
                );
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_BPM as i32);
                column
            });
        }

        if !unavailable_columns.contains(&COL_TRACK_RELEASE_DATE) {
            tracks_view.append_column(&{
                let text_cell = gtk::CellRendererText::new();
                text_cell.set_alignment(1.0, 0.5);

                let column = base_column
                    .clone()
                    .expand(false)
                    .title("Released")
                    .sort_column_id(COL_TRACK_RELEASE_DATE as i32)
                    .build();
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_RELEASE_DATE as i32);
                column
            });
        }

        if !unavailable_columns.contains(&COL_TRACK_ARTISTS) {
            tracks_view.append_column(&{
                let text_cell = gtk::CellRendererText::new();
                let column = base_column
                    .clone()
                    .title("Artists")
                    .sort_column_id(COL_TRACK_ARTISTS as i32)
                    .build();
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_ARTISTS as i32);
                column
            });
        }

        if !unavailable_columns.contains(&COL_TRACK_ALBUM) {
            tracks_view.append_column(&{
                let text_cell = gtk::CellRendererText::new();
                let column = base_column
                    .clone()
                    .title("Album")
                    .sort_column_id(COL_TRACK_ALBUM as i32)
                    .build();
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_ALBUM as i32);
                column
            });
        }

        let stream = relm.stream().clone();
        tracks_view.connect_button_press_event(move |_, event| {
            if event.get_button() == 3 {
                stream.emit(TrackListMsg::OpenContextMenu(event.clone()));
                Inhibit(true)
            } else {
                Inhibit(false)
            }
        });

        tracks_view.set_search_column(COL_TRACK_NAME as i32);
        tracks_view.set_enable_search(true);
        tracks_view.set_search_equal_func(|model, col, needle, pos| {
            if let Ok(Some(haystack)) = model.get_value(pos, col).get::<&str>() {
                let haystack = haystack.to_ascii_lowercase();
                let needle = needle.to_ascii_lowercase();
                !haystack.contains(&needle)
            } else {
                true
            }
        });

        let stream = relm.stream().clone();
        tracks_view.connect_row_activated(move |tree, path, _col| {
            if let Some(track_uri) = tree.get_model().and_then(|store| {
                store.get_iter(path).and_then(|pos| {
                    store
                        .get_value(&pos, COL_TRACK_URI as i32)
                        .get::<String>()
                        .ok()
                        .flatten()
                })
            }) {
                stream.emit(TrackListMsg::PlayTracks(vec![track_uri]));
            }
        });

        scroller.add(&tracks_view);
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
        refresh_btn.connect_clicked(move |_| stream.emit(TrackListMsg::Reload));
        status_bar.pack_start(&refresh_btn, false, false, 0);

        root.add(&status_bar);

        let context_menu = gtk::Menu::new();

        macro_rules! menu {
            ($menu:ident, $relm:ident, $($item:tt),+) => {
                $($menu.append(&{
                    menu!(@ $relm, $item)
                });)+
            };
            (@ $relm:ident, ($title:literal => $msg:ident)) => {{
                let item = gtk::MenuItem::with_label($title);
                let stream = $relm.stream().clone();
                item.connect_activate(move |_| stream.emit(TrackListMsg::$msg));
                item
            }};
            (@ $relm:ident, (===)) => {
                gtk::SeparatorMenuItem::new()
            };
        }

        menu! {context_menu, relm,
            ("Play now" => PlayChosenTracks),
            ("Add to queue" => EnqueueChosenTracks),
            ("Add to library" => SaveChosenTracks),
            ("Add to playlist…" => AddChosenTracks),
            (===),
            ("Go to album" => GoToChosenTrackAlbum),
            ("Go to artist" => GoToChosenTrackArtist),
            ("Recommend similar" => RecommendTracks),
            (===),
            ("Remove from library" => UnsaveChosenTracks)
            //("Remove from playlist" => RemoveChosenTracks)
        };

        context_menu.show_all();

        root.add(&context_menu);
        root.show_all();

        TrackList {
            model,
            root,
            tracks_view,
            context_menu,
            status_bar,
            progress_bar,
            refresh_btn,
        }
    }
}
