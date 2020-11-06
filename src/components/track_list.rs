use crate::components::spotify::{SpotifyCmd, SpotifyProxy};
use crate::utils::ImageLoader;
use gdk_pixbuf::Pixbuf;
use glib::StaticType;
use gtk::prelude::*;
use gtk::{
    CellRendererExt, GtkMenuExt, GtkMenuItemExt, Inhibit, TreeModelExt, TreeViewColumn, TreeViewExt,
};
use itertools::Itertools;
use relm::vendor::fragile::Fragile;
use relm::{EventStream, Relm, Update, Widget};
use relm_derive::Msg;
use rspotify::model::album::SimplifiedAlbum;
use rspotify::model::artist::SimplifiedArtist;
use rspotify::model::audio::AudioFeatures;
use rspotify::model::image::Image;
use rspotify::model::page::Page;
use rspotify::model::playlist::{FullPlaylist, PlaylistTrack};
use rspotify::model::show::FullEpisode;
use rspotify::model::track::{FullTrack, SavedTrack, SimplifiedTrack};
use rspotify::model::PlayingItem;
use std::sync::Arc;

pub trait TrackContainer: 'static {
    type Id;
    type Track: TrackLike + 'static;
}

pub trait ControlSpotifyContext {
    const PAGE_LIMIT: u32;
    fn load_tracks_page(&self, offset: u32);
    fn play_tracks(&self, uris: Vec<String>);
}

pub trait TrackLike {
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
}

impl TrackLike for PlaylistTrack {
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
}

macro_rules! impl_track_like_for_playing_item {
    ($($method:ident -> $tpe:ty),+) => {
        impl TrackLike for PlayingItem {
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
    duration -> u32
}

#[derive(Msg)]
pub enum TrackListMsg<T: TrackContainer> {
    Clear,
    Reset(T::Id),
    Reload,
    LoadPage(u32),
    NewPage(Page<T::Track>),

    LoadThumb(String, gtk::TreeIter),
    NewThumb(gdk_pixbuf::Pixbuf, gtk::TreeIter),
    PlayChosenTracks,
    PlayingNewTrack,
    LoadTracksInfo(Vec<String>, Vec<gtk::TreeIter>),
    NewTracksInfo(Vec<AudioFeatures>, Vec<gtk::TreeIter>),

    NewBpm(gtk::TreePath, f32),
    Click(gdk::EventButton),
}

const THUMB_SIZE: i32 = 32;

const COL_TRACK_ID: u32 = 0;
const COL_TRACK_THUMB: u32 = 1;
const COL_TRACK_NAME: u32 = 2;
const COL_TRACK_ARTISTS: u32 = 3;
const COL_TRACK_NUMBER: u32 = 4;
const COL_TRACK_ALBUM: u32 = 5;
const COL_TRACK_CAN_PLAY: u32 = 6;
const COL_TRACK_DURATION: u32 = 7;
const COL_TRACK_DURATION_MS: u32 = 8;
const COL_TRACK_URI: u32 = 9;
const COL_TRACK_BPM: u32 = 10;

pub struct TrackListModel<T: TrackContainer> {
    stream: EventStream<TrackListMsg<T>>,
    spotify: Arc<SpotifyProxy>,
    store: gtk::ListStore,
    image_loader: ImageLoader,
    parent_id: Option<T::Id>,
}

pub struct TrackList<T: TrackContainer> {
    model: TrackListModel<T>,
    root: gtk::ScrolledWindow,
    tracks_view: gtk::TreeView,
    context_menu: gtk::Menu,
}

impl TrackContainer for () {
    type Id = ();
    type Track = SavedTrack;
}

impl ControlSpotifyContext for TrackList<()> {
    const PAGE_LIMIT: u32 = 10;

    fn load_tracks_page(&self, offset: u32) {
        self.model.spotify.ask(
            self.model.stream.clone(),
            move |tx| SpotifyCmd::GetFavoriteTracks {
                tx,
                limit: Self::PAGE_LIMIT,
                offset,
            },
            TrackListMsg::NewPage,
        );
    }

    fn play_tracks(&self, uris: Vec<String>) {
        self.model.spotify.tell(SpotifyCmd::PlayTracks { uris });
    }
}

impl TrackContainer for FullPlaylist {
    type Id = String;
    type Track = PlaylistTrack;
}

impl ControlSpotifyContext for TrackList<FullPlaylist> {
    const PAGE_LIMIT: u32 = 20;

    fn load_tracks_page(&self, offset: u32) {
        if let Some(ref parent_id) = self.model.parent_id {
            let parent_id = parent_id.clone();
            self.model.spotify.ask(
                self.model.stream.clone(),
                move |tx| SpotifyCmd::GetPlaylistTracks {
                    tx,
                    uri: parent_id,
                    limit: Self::PAGE_LIMIT,
                    offset,
                },
                TrackListMsg::NewPage,
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

impl<T> Update for TrackList<T>
where
    T: TrackContainer,
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
        ]);

        let stream = relm.stream().clone();

        TrackListModel {
            stream,
            spotify,
            store,
            image_loader: ImageLoader::new_with_resize(THUMB_SIZE),
            parent_id: None,
        }
    }

    fn update(&mut self, event: Self::Msg) {
        use TrackListMsg::*;
        match event {
            Clear => {
                self.model.store.clear();
            }
            Reset(parent_id) => {
                self.model.parent_id.replace(parent_id);
                self.model.store.clear();
                self.model.stream.emit(LoadPage(0))
            }
            Reload => {
                self.model.store.clear();
                self.model.stream.emit(LoadPage(0))
            }
            LoadPage(offset) => {
                self.load_tracks_page(offset);
            }
            NewPage(page) => {
                let stream = &self.model.stream;
                let store = &self.model.store;
                let tracks = page.items;

                let mut uris = Vec::with_capacity(tracks.len());
                let mut iters = Vec::with_capacity(tracks.len());

                for track in tracks {
                    let pos = store.insert_with_values(
                        None,
                        &[
                            COL_TRACK_ID,
                            COL_TRACK_NAME,
                            COL_TRACK_ARTISTS,
                            COL_TRACK_NUMBER,
                            COL_TRACK_ALBUM,
                            COL_TRACK_CAN_PLAY,
                            COL_TRACK_DURATION,
                            COL_TRACK_DURATION_MS,
                            COL_TRACK_URI,
                        ],
                        &[
                            &track.id(),
                            &track.name(),
                            &track.artists().iter().map(|artist| &artist.name).join(", "),
                            &track.number(),
                            &track.album().map(|album| &*album.name),
                            &track.is_playable(),
                            &crate::utils::humanize_time(track.duration()),
                            &track.duration(),
                            &track.uri(),
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
                }

                if page.next.is_some() {
                    stream.emit(LoadPage(page.offset + Self::PAGE_LIMIT));
                }

                stream.emit(LoadTracksInfo(uris, iters));
            }
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
                self.model.image_loader.load_from_url(url, move |loaded| {
                    if let Ok(pb) = loaded {
                        stream.into_inner().emit(NewThumb(pb, pos.into_inner()));
                    }
                });
            }
            NewThumb(thumb, pos) => {
                self.model
                    .store
                    .set_value(&pos, COL_TRACK_THUMB, &thumb.to_value());
            }
            Click(event) if event.get_button() == 3 => {
                self.context_menu.popup_at_pointer(Some(&event));
            }
            Click(event) if event.get_event_type() == gdk::EventType::DoubleButtonPress => {
                self.model.stream.emit(PlayChosenTracks);
            }
            Click(_) => {}
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

                //let uri = uris.first().cloned();
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
    T: TrackContainer,
    TrackList<T>: ControlSpotifyContext,
{
    type Root = gtk::ScrolledWindow;

    fn root(&self) -> Self::Root {
        self.root.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let root = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);

        let tracks_view = gtk::TreeViewBuilder::new()
            .model(&model.store)
            .expand(true)
            .build();

        tracks_view
            .get_selection()
            .set_mode(gtk::SelectionMode::Multiple);

        let base_column = gtk::TreeViewColumnBuilder::new()
            .resizable(true)
            .reorderable(true)
            .expand(true);

        tracks_view.append_column(&{
            let icon_cell = gtk::CellRendererPixbuf::new();
            icon_cell.set_property_icon_name(Some("audio-x-generic-symbolic"));

            let column = TreeViewColumn::new();
            column.pack_start(&icon_cell, true);
            column.add_attribute(&icon_cell, "pixbuf", COL_TRACK_THUMB as i32);
            column
        });

        tracks_view.append_column(&{
            let text_cell = gtk::CellRendererText::new();
            let column = base_column
                .clone()
                .title("Title")
                .sort_column_id(COL_TRACK_NAME as i32)
                .build();
            column.pack_start(&text_cell, true);
            column.add_attribute(&text_cell, "text", COL_TRACK_NAME as i32);
            column
        });

        tracks_view.append_column(&{
            let text_cell = gtk::CellRendererText::new();
            text_cell.set_alignment(1.0, 0.5);
            let column = base_column
                .clone()
                .title("Duration")
                .sort_column_id(COL_TRACK_DURATION_MS as i32)
                .build();
            column.pack_start(&text_cell, true);
            column.add_attribute(&text_cell, "text", COL_TRACK_DURATION as i32);
            column
        });

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

        let stream = relm.stream().clone();
        tracks_view.connect_button_press_event(move |_, event| {
            stream.emit(TrackListMsg::Click(event.clone()));
            Inhibit(false)
        });

        root.add(&tracks_view);

        let context_menu = gtk::Menu::new();

        context_menu.append(&{
            let item = gtk::MenuItem::with_label("Play now");
            let stream = relm.stream().clone();
            item.connect_activate(move |_| stream.emit(TrackListMsg::PlayChosenTracks));
            item
        });

        context_menu.append(&{
            let item = gtk::MenuItem::with_label("Remove from library");
            item
        });

        context_menu.show_all();

        root.add(&context_menu);
        root.show_all();

        TrackList {
            model,
            root,
            tracks_view,
            context_menu,
        }
    }
}
