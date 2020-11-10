use crate::components::spotify::{SpotifyCmd, SpotifyProxy};
use crate::utils::ImageLoader;
use gdk_pixbuf::Pixbuf;
use glib::StaticType;
use gtk::prelude::*;
use gtk::{
    CellRendererExt, GtkMenuExt, GtkMenuItemExt, Inhibit, ProgressBarExt, StatusbarExt,
    TreeModelExt, TreeViewColumn, TreeViewExt,
};
use itertools::Itertools;
use relm::vendor::fragile::Fragile;
use relm::{EventStream, Relm, Update, Widget};
use relm_derive::Msg;
use rspotify::model::album::{FullAlbum, SavedAlbum, SimplifiedAlbum};
use rspotify::model::artist::SimplifiedArtist;
use rspotify::model::audio::AudioFeatures;
use rspotify::model::image::Image;
use rspotify::model::page::Page;
use rspotify::model::playlist::{FullPlaylist, PlaylistTrack, SimplifiedPlaylist};
use rspotify::model::show::{FullEpisode, Show, SimplifiedEpisode};
use rspotify::model::track::{FullTrack, SavedTrack, SimplifiedTrack};
use rspotify::model::PlayingItem;
use std::sync::Arc;

pub trait TrackContainer: 'static {
    type Track: TrackLike;
}

pub trait ControlSpotifyContext {
    const PAGE_LIMIT: u32;
    fn load_tracks_page(&self, offset: u32);
    fn play_tracks(&self, uris: Vec<String>);
}

pub trait TrackLike {
    type ParentId;

    fn id(&self) -> &str;
    fn uri(&self) -> &str;
    fn name(&self) -> &str;
    fn artists(&self) -> &[SimplifiedArtist];
    fn number(&self) -> u32;
    fn album(&self) -> Option<&SimplifiedAlbum>;
    fn is_playable(&self) -> bool;
    fn duration(&self) -> u32;

    fn images(&self) -> Option<&Vec<Image>> {
        self.album().map(|album| &album.images)
    }

    fn unavailable_columns() -> &'static [u32] {
        &[]
    }
}

impl TrackLike for PlaylistTrack {
    type ParentId = String;

    fn id(&self) -> &str {
        self.track.as_ref().map(FullTrack::id).unwrap_or("")
    }

    fn uri(&self) -> &str {
        self.track.as_ref().map(FullTrack::uri).unwrap_or("")
    }

    fn name(&self) -> &str {
        self.track.as_ref().map(FullTrack::name).unwrap_or("")
    }

    fn artists(&self) -> &[SimplifiedArtist] {
        self.track.as_ref().map(FullTrack::artists).unwrap_or(&[])
    }

    fn number(&self) -> u32 {
        self.track.as_ref().map(FullTrack::number).unwrap_or(0)
    }

    fn album(&self) -> Option<&SimplifiedAlbum> {
        self.track.as_ref().and_then(FullTrack::album)
    }

    fn is_playable(&self) -> bool {
        self.track
            .as_ref()
            .map(FullTrack::is_playable)
            .unwrap_or(false)
    }

    fn duration(&self) -> u32 {
        self.track.as_ref().map(FullTrack::duration).unwrap_or(0)
    }
}

impl TrackLike for FullTrack {
    type ParentId = ();

    fn id(&self) -> &str {
        self.id.as_deref().unwrap_or("")
    }

    fn uri(&self) -> &str {
        &self.uri
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn artists(&self) -> &[SimplifiedArtist] {
        &self.artists
    }

    fn number(&self) -> u32 {
        self.track_number
    }

    fn album(&self) -> Option<&SimplifiedAlbum> {
        Some(&self.album)
    }

    fn is_playable(&self) -> bool {
        self.is_playable.unwrap_or(true)
    }

    fn duration(&self) -> u32 {
        self.duration_ms
    }
}

impl TrackLike for SimplifiedTrack {
    type ParentId = String;

    fn unavailable_columns() -> &'static [u32] {
        &[COL_TRACK_ALBUM, COL_TRACK_THUMB]
    }

    fn id(&self) -> &str {
        self.id.as_deref().unwrap_or("")
    }

    fn uri(&self) -> &str {
        &self.uri
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn artists(&self) -> &[SimplifiedArtist] {
        &self.artists
    }

    fn number(&self) -> u32 {
        self.track_number
    }

    fn album(&self) -> Option<&SimplifiedAlbum> {
        None
    }

    fn is_playable(&self) -> bool {
        true
    }

    fn duration(&self) -> u32 {
        self.duration_ms
    }
}

impl TrackLike for SavedTrack {
    type ParentId = String;

    fn id(&self) -> &str {
        self.track.id()
    }

    fn uri(&self) -> &str {
        self.track.uri()
    }

    fn name(&self) -> &str {
        self.track.name()
    }

    fn artists(&self) -> &[SimplifiedArtist] {
        self.track.artists()
    }

    fn number(&self) -> u32 {
        self.track.number()
    }

    fn album(&self) -> Option<&SimplifiedAlbum> {
        self.track.album()
    }

    fn is_playable(&self) -> bool {
        self.track.is_playable()
    }

    fn duration(&self) -> u32 {
        self.track.duration()
    }
}

impl TrackLike for FullEpisode {
    type ParentId = ();

    fn id(&self) -> &str {
        &self.id
    }

    fn uri(&self) -> &str {
        &self.uri
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn artists(&self) -> &[SimplifiedArtist] {
        &[]
    }

    fn number(&self) -> u32 {
        0
    }

    fn album(&self) -> Option<&SimplifiedAlbum> {
        None
    }

    fn is_playable(&self) -> bool {
        self.is_playable
    }

    fn duration(&self) -> u32 {
        self.duration_ms
    }

    fn images(&self) -> Option<&Vec<Image>> {
        Some(&self.images)
    }

    fn unavailable_columns() -> &'static [u32] {
        &[COL_TRACK_ARTISTS, COL_TRACK_ALBUM]
    }
}

impl TrackLike for SimplifiedEpisode {
    type ParentId = String;

    fn id(&self) -> &str {
        &self.id
    }

    fn uri(&self) -> &str {
        &self.uri
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn artists(&self) -> &[SimplifiedArtist] {
        &[]
    }

    fn number(&self) -> u32 {
        0
    }

    fn album(&self) -> Option<&SimplifiedAlbum> {
        None
    }

    fn is_playable(&self) -> bool {
        self.is_playable
    }

    fn duration(&self) -> u32 {
        self.duration_ms
    }

    fn images(&self) -> Option<&Vec<Image>> {
        Some(&self.images)
    }

    fn unavailable_columns() -> &'static [u32] {
        &[COL_TRACK_ARTISTS, COL_TRACK_ALBUM, COL_TRACK_BPM]
    }
}

macro_rules! impl_track_like_for_playing_item {
    ($($method:ident -> $tpe:ty),+) => {
        impl TrackLike for PlayingItem {
            type ParentId = ();
            $(fn $method(&self) -> $tpe {
                match self {
                    PlayingItem::Track(track) => track.$method(),
                    PlayingItem::Episode(episode) => episode.$method(),
                }
            })+
        }
    }
}
impl_track_like_for_playing_item! {
    id -> &str, uri -> &str, name -> &str,
    artists -> &[SimplifiedArtist], number -> u32,
    album -> Option<&SimplifiedAlbum>, is_playable -> bool,
    duration -> u32, images -> Option<&Vec<Image>>
}

#[derive(Msg)]
pub enum TrackListMsg<T: TrackLike> {
    Clear,
    Reset(T::ParentId, bool),
    Reload,
    LoadPage(u32, u32),
    NewPage(Page<T>, u32),

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
}

const THUMB_SIZE: i32 = 32;

const COL_TRACK_ID: u32 = 0;
const COL_TRACK_THUMB: u32 = 1;
const COL_TRACK_NAME: u32 = 2;
const COL_TRACK_ARTISTS: u32 = 3;
const COL_TRACK_NUMBER: u32 = 4;
const COL_TRACK_ALBUM: u32 = 5;
const COL_TRACK_CANT_PLAY: u32 = 6;
const COL_TRACK_DURATION: u32 = 7;
const COL_TRACK_DURATION_MS: u32 = 8;
const COL_TRACK_URI: u32 = 9;
const COL_TRACK_BPM: u32 = 10;
const COL_TRACK_TIMELINE: u32 = 11;

pub struct TrackListModel<T: TrackLike> {
    stream: EventStream<TrackListMsg<T>>,
    spotify: Arc<SpotifyProxy>,
    store: gtk::ListStore,
    image_loader: ImageLoader,
    parent_id: Option<T::ParentId>,
    total_tracks: u32,
    total_duration: u32,
    epoch: u32,
}

pub struct TrackList<T: TrackLike> {
    model: TrackListModel<T>,
    root: gtk::Box,
    tracks_view: gtk::TreeView,
    context_menu: gtk::Menu,
    status_bar: gtk::Statusbar,
    progress_bar: gtk::ProgressBar,
}

impl TrackContainer for () {
    type Track = SavedTrack;
}

impl ControlSpotifyContext for TrackList<SavedTrack> {
    const PAGE_LIMIT: u32 = 10;

    fn load_tracks_page(&self, offset: u32) {
        let epoch = self.model.epoch;
        self.model.spotify.ask(
            self.model.stream.clone(),
            move |tx| SpotifyCmd::GetMyTracks {
                tx,
                limit: Self::PAGE_LIMIT,
                offset,
            },
            move |reply| TrackListMsg::NewPage(reply, epoch),
        );
    }

    fn play_tracks(&self, uris: Vec<String>) {
        self.model.spotify.tell(SpotifyCmd::PlayTracks { uris });
    }
}

impl TrackContainer for SimplifiedPlaylist {
    type Track = PlaylistTrack;
}

impl ControlSpotifyContext for TrackList<PlaylistTrack> {
    const PAGE_LIMIT: u32 = 10;

    fn load_tracks_page(&self, offset: u32) {
        if let Some(ref parent_id) = self.model.parent_id {
            let parent_id = parent_id.clone();
            let epoch = self.model.epoch;
            self.model.spotify.ask(
                self.model.stream.clone(),
                move |tx| SpotifyCmd::GetPlaylistTracks {
                    tx,
                    uri: parent_id,
                    limit: Self::PAGE_LIMIT,
                    offset,
                },
                move |reply| TrackListMsg::NewPage(reply, epoch),
            );
        }
    }

    fn play_tracks(&self, uris: Vec<String>) {
        if let Some(ref parent_id) = self.model.parent_id {
            self.model.spotify.tell(SpotifyCmd::PlayContext {
                uri: parent_id.clone(),
                start_uri: uris.first().cloned(),
            });
        }
    }
}

impl TrackContainer for SimplifiedAlbum {
    type Track = SimplifiedTrack;
}

impl TrackContainer for FullAlbum {
    type Track = SimplifiedTrack;
}

impl TrackContainer for SavedAlbum {
    type Track = SimplifiedTrack;
}

impl TrackContainer for Show {
    type Track = SimplifiedEpisode;
}

impl ControlSpotifyContext for TrackList<SimplifiedEpisode> {
    const PAGE_LIMIT: u32 = 10;

    fn load_tracks_page(&self, offset: u32) {
        if let Some(ref parent_id) = self.model.parent_id {
            let parent_id = parent_id.clone();
            let epoch = self.model.epoch;
            self.model.spotify.ask(
                self.model.stream.clone(),
                move |tx| SpotifyCmd::GetShowEpisodes {
                    tx,
                    uri: parent_id,
                    limit: Self::PAGE_LIMIT,
                    offset,
                },
                move |reply| TrackListMsg::NewPage(reply, epoch),
            );
        }
    }

    fn play_tracks(&self, uris: Vec<String>) {
        if let Some(ref parent_id) = self.model.parent_id {
            self.model.spotify.tell(SpotifyCmd::PlayContext {
                uri: parent_id.clone(),
                start_uri: uris.first().cloned(),
            });
        }
    }
}

impl ControlSpotifyContext for TrackList<SimplifiedTrack> {
    const PAGE_LIMIT: u32 = 10;

    fn load_tracks_page(&self, offset: u32) {
        if let Some(ref parent_id) = self.model.parent_id {
            let parent_id = parent_id.clone();
            let epoch = self.model.epoch;
            self.model.spotify.ask(
                self.model.stream.clone(),
                move |tx| SpotifyCmd::GetAlbumTracks {
                    tx,
                    uri: parent_id,
                    limit: Self::PAGE_LIMIT,
                    offset,
                },
                move |reply| TrackListMsg::NewPage(reply, epoch),
            );
        }
    }

    fn play_tracks(&self, uris: Vec<String>) {
        if let Some(ref parent_id) = self.model.parent_id {
            self.model.spotify.tell(SpotifyCmd::PlayContext {
                uri: parent_id.clone(),
                start_uri: uris.first().cloned(),
            });
        }
    }
}

impl TrackContainer for FullPlaylist {
    type Track = PlaylistTrack;
}

impl<T: TrackLike> TrackList<T> {
    fn clear_store(&mut self) {
        self.model.store.clear();
        self.model.total_duration = 0;
        self.model.total_tracks = 0;
    }

    fn start_load(&mut self) {
        self.model.epoch += 1;
        self.progress_bar.set_fraction(0.0);
        self.progress_bar.set_visible(true);
        self.progress_bar.pulse();
        self.model
            .stream
            .emit(TrackListMsg::LoadPage(0, self.model.epoch))
    }
}

impl<T> Update for TrackList<T>
where
    T: TrackLike + 'static,
    TrackList<T>: ControlSpotifyContext,
{
    type Model = TrackListModel<T>;
    type ModelParam = Arc<SpotifyProxy>;
    type Msg = TrackListMsg<T>;

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
        ]);

        let stream = relm.stream().clone();

        TrackListModel {
            stream,
            spotify,
            store,
            image_loader: ImageLoader::new_with_resize(THUMB_SIZE),
            parent_id: None,
            total_duration: 0,
            total_tracks: 0,
            epoch: 0,
        }
    }

    fn update(&mut self, event: Self::Msg) {
        use TrackListMsg::*;
        match event {
            Clear => {
                self.clear_store();
            }
            Reset(parent_id, reload) => {
                self.model.parent_id.replace(parent_id);
                self.clear_store();
                if reload {
                    self.start_load()
                }
            }
            Reload => {
                self.clear_store();
                self.start_load();
            }
            LoadPage(offset, epoch) if epoch == self.model.epoch => {
                self.load_tracks_page(offset);
            }
            LoadPage(_, _) => {}
            NewPage(page, epoch) if epoch == self.model.epoch => {
                let stream = &self.model.stream;
                let store = &self.model.store;
                let tracks = page.items;
                let offset = page.offset;

                self.progress_bar
                    .set_fraction((page.offset as f64 + tracks.len() as f64) / page.total as f64);

                let mut uris = Vec::with_capacity(tracks.len());
                let mut iters = Vec::with_capacity(tracks.len());

                let mut page_duration = 0;
                for (idx, track) in tracks.into_iter().enumerate() {
                    let pos = store.insert_with_values(
                        None,
                        &[
                            COL_TRACK_ID,
                            COL_TRACK_NAME,
                            COL_TRACK_ARTISTS,
                            COL_TRACK_NUMBER,
                            COL_TRACK_ALBUM,
                            COL_TRACK_CANT_PLAY,
                            COL_TRACK_DURATION,
                            COL_TRACK_DURATION_MS,
                            COL_TRACK_URI,
                            COL_TRACK_TIMELINE,
                        ],
                        &[
                            &track.id(),
                            &track.name(),
                            &track.artists().iter().map(|artist| &artist.name).join(", "),
                            &(idx as u32 + offset + 1),
                            &track.album().map(|album| &*album.name),
                            &!track.is_playable(),
                            &crate::utils::humanize_time(track.duration()),
                            &track.duration(),
                            &track.uri(),
                            &crate::utils::humanize_time(self.model.total_duration + page_duration),
                        ],
                    );

                    let image = track
                        .images()
                        .and_then(|images| crate::utils::find_best_thumb(images, THUMB_SIZE));

                    if let Some(url) = image {
                        stream.emit(LoadThumb(url.to_owned(), pos.clone()));
                    }

                    uris.push(track.uri().to_owned());
                    iters.push(pos);
                    page_duration += track.duration();
                }

                self.model.total_duration += page_duration;

                if page.next.is_some() {
                    stream.emit(LoadPage(offset + Self::PAGE_LIMIT, self.model.epoch));
                } else {
                    self.model.total_tracks = page.total;

                    let status_ctx = self.status_bar.get_context_id("totals");
                    self.progress_bar.set_visible(false);
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

                if !<T as TrackLike>::unavailable_columns().contains(&COL_TRACK_BPM) {
                    stream.emit(LoadTracksInfo(uris, iters));
                }
            }
            NewPage(_, _) => {}
            LoadTracksInfo(uris, iters) => {
                self.model.spotify.ask(
                    self.model.stream.clone(),
                    |tx| SpotifyCmd::GetTracksFeatures { tx, uris },
                    move |feats| NewTracksInfo(feats, iters.clone()),
                );
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
                let tree = &self.tracks_view;
                let select = tree.get_selection();
                let (rows, model) = select.get_selected_rows();
                let uris = rows
                    .into_iter()
                    .filter_map(|path| model.get_iter(&path))
                    .filter_map(|pos| {
                        model
                            .get_value(&pos, COL_TRACK_URI as i32)
                            .get::<String>()
                            .ok()
                            .flatten()
                    })
                    .collect::<Vec<_>>();

                self.model.stream.emit(PlayTracks(uris));
            }
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

impl<T> Widget for TrackList<T>
where
    T: TrackLike + 'static,
    TrackList<T>: ControlSpotifyContext,
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

        let unavailable_columns = <T as TrackLike>::unavailable_columns();

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
        root.add(&status_bar);

        let context_menu = gtk::Menu::new();

        context_menu.append(&{
            let item = gtk::MenuItem::with_label("Play now");
            let stream = relm.stream().clone();
            item.connect_activate(move |_| stream.emit(TrackListMsg::PlayChosenTracks));
            item
        });

        context_menu.append(&gtk::MenuItem::with_label("Remove from library"));

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
        }
    }
}
