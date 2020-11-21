use crate::loaders::common::{
    ContainerLoader, HasImages, MissingColumns, COL_ITEM_NAME, COL_ITEM_THUMB, COL_ITEM_URI,
};
use crate::loaders::paged::RowLike;
use crate::loaders::HasDuration;
use crate::servers::spotify::{ResultSender, SpotifyCmd};
use glib::{IsA, StaticType, Type};
use gtk::prelude::GtkListStoreExtManual;
use itertools::Itertools;
use rspotify::model::album::SimplifiedAlbum;
use rspotify::model::artist::SimplifiedArtist;
use rspotify::model::image::Image;
use rspotify::model::page::Page;
use rspotify::model::playing::PlayHistory;
use rspotify::model::playlist::PlaylistTrack;
use rspotify::model::show::{FullEpisode, SimplifiedEpisode};
use rspotify::model::track::{FullTrack, SavedTrack, SimplifiedTrack};
use rspotify::model::PlayingItem;
use serde_json::{Map, Value};

#[derive(Clone, Copy)]
pub struct Seed<Val: Copy> {
    min: Option<Val>,
    max: Option<Val>,
    target: Option<Val>,
}

#[derive(Clone, Copy)]
pub enum Mode {
    Minor = 0,
    Major = 1,
}

#[derive(Clone)]
pub struct RecommendLoader {
    seed_artists: Option<Vec<String>>,
    seed_genres: Option<Vec<String>>,
    seed_tracks: Option<Vec<String>>,
    tunables: Map<String, Value>,
    /*
    accousticness: Option<Seed<f32>>,
    dancability: Option<Seed<f32>>,
    duration_ms: Option<Seed<u32>>,
    energy: Option<Seed<f32>>,
    instrumentalness: Option<Seed<f32>>,
    key: Option<Seed<u8>>,
    liveness: Option<Seed<f32>>,
    loadness: Option<Seed<f32>>,
    mode: Option<Mode>,
    popularity: Option<Seed<u8>>,
    speechness: Option<Seed<f32>>,
    tempo: Option<Seed<f32>>,
    time_signature: Option<Seed<u8>>,
    valence: Option<Seed<f32>>,
     */
}

impl RecommendLoader {
    fn extract_vec_string(
        params: &mut Map<String, Value>,
        key: &str,
        max_items: usize,
    ) -> Option<Vec<String>> {
        params.remove(key).and_then(|seed| match seed {
            Value::Array(values) => Some(
                values
                    .into_iter()
                    .take(max_items)
                    .flat_map(|value| match value {
                        Value::String(value) => Some(value),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
    }
}

impl ContainerLoader for RecommendLoader {
    type ParentId = Map<String, Value>;
    type Item = SimplifiedTrack;
    type Page = Vec<Self::Item>;
    const PAGE_LIMIT: u32 = 100;

    fn new(mut tunables: Self::ParentId) -> Self {
        let seed_artists = Self::extract_vec_string(&mut tunables, "seed_artists", 5);
        let seed_genres = Self::extract_vec_string(&mut tunables, "seed_genres", 5);
        let seed_tracks = Self::extract_vec_string(&mut tunables, "seed_tracks", 5);
        /*
        tunables.retain(|key| {
            matches!(
                &*key,
                "min_accousticness"
                    | "max_acousticness"
                    | "target_acousticness"
                    | "min_danceability"
                    | "max_danceability"
                    | "target_danceability"
                    | "min_duration_ms"
                    | "max_duration_ms"
                    | "target_duration_ms"
                    | "min_energy"
                    | "max_energy"
                    | "target_energy"
                    | "min_instrumentalness"
                    | "max_instrumentalness"
                    | "target_instrumentalness"
                    | "min_key"
                    | "max_key"
                    | "target_key"
                    | "min_liveness"
                    | "max_liveness"
                    | "target_liveness"
                    | "min_loadness"
                    | "max_loudness"
                    | "target_loudness"
                    | "min_mode"
                    | "max_mode"
                    | "target_mode"
                    | "min_popularity"
                    | "max_popularity"
                    | "target_popularity"
                    | "min_speechiness"
                    | "max_speechiness"
                    | "target_speechiness"
                    | "min_tempo"
                    | "max_tempo"
                    | "target_tempo"
                    | "max_time_signature"
                    | "min_time_signature"
                    | "target_time_signature"
                    | "min_valence"
                    | "max_valence"
                    | "target_valence"
            )
        });
         */

        Self {
            seed_artists,
            seed_genres,
            seed_tracks,
            tunables,
        }
    }

    fn parent_id(&self) -> &Self::ParentId {
        return &self.tunables;
        //let mut params = self.tunables.clone();
        //if let Some(ref seed_artists) = self.seed_artists {
        //    params.insert("seed_artists".into(), Value::from(seed_artists.clone()));
        //}
        //if let Some(ref seed_genres) = self.seed_genres {
        //    params.insert("seed_genres".into(), Value::from(seed_genres.clone()));
        //}
        //if let Some(ref seed_tracks) = self.seed_tracks {
        //    params.insert("seed_tracks".into(), Value::from(seed_tracks.clone()));
        //}
        //params
    }

    fn load_page(self, tx: ResultSender<Self::Page>, _offset: ()) -> SpotifyCmd {
        let RecommendLoader {
            seed_tracks,
            seed_genres,
            seed_artists,
            tunables,
        } = self;
        SpotifyCmd::GetRecommendedTracks {
            tx,
            seed_tracks,
            seed_genres,
            seed_artists,
            tunables,
            limit: Self::PAGE_LIMIT,
        }
    }
}

#[derive(Clone, Copy)]
pub struct SavedLoader;
impl ContainerLoader for SavedLoader {
    type ParentId = ();
    type Item = SavedTrack;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;

    fn new(_id: Self::ParentId) -> Self {
        SavedLoader
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetMyTracks {
            tx,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}

#[derive(Clone, Copy)]
pub struct RecentLoader;
impl ContainerLoader for RecentLoader {
    type ParentId = ();
    type Item = PlayHistory;
    type Page = Vec<Self::Item>;
    const PAGE_LIMIT: u32 = 50;

    fn new(_id: Self::ParentId) -> Self {
        RecentLoader
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    fn load_page(self, tx: ResultSender<Self::Page>, _offset: ()) -> SpotifyCmd {
        SpotifyCmd::GetRecentTracks {
            tx,
            limit: Self::PAGE_LIMIT,
        }
    }
}

#[derive(Clone, Copy)]
pub struct QueueLoader;

impl ContainerLoader for QueueLoader {
    type ParentId = ();
    type Item = FullTrack;
    type Page = Vec<Self::Item>;
    const PAGE_LIMIT: u32 = 0;

    fn new(_id: Self::ParentId) -> Self {
        QueueLoader
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    fn load_page(self, tx: ResultSender<Self::Page>, _offset: ()) -> SpotifyCmd {
        SpotifyCmd::GetQueueTracks { tx }
    }
}

#[derive(Clone)]
pub struct AlbumLoader {
    uri: String,
}

impl ContainerLoader for AlbumLoader {
    type ParentId = String;
    type Item = SimplifiedTrack;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 10;

    fn new(uri: Self::ParentId) -> Self {
        AlbumLoader { uri }
    }

    fn parent_id(&self) -> &Self::ParentId {
        &self.uri
    }

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        let uri = self.parent_id().clone();
        SpotifyCmd::GetAlbumTracks {
            tx,
            uri,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}

#[derive(Clone)]
pub struct PlaylistLoader {
    uri: String,
}

impl ContainerLoader for PlaylistLoader {
    type ParentId = String;
    type Item = PlaylistTrack;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 10;

    fn new(uri: Self::ParentId) -> Self {
        PlaylistLoader { uri }
    }

    fn parent_id(&self) -> &Self::ParentId {
        &self.uri
    }

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        let uri = self.parent_id().clone();
        SpotifyCmd::GetPlaylistTracks {
            tx,
            uri,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}

#[derive(Clone)]
pub struct MyTopTracksLoader;

impl ContainerLoader for MyTopTracksLoader {
    type ParentId = ();
    type Item = FullTrack;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;

    fn new(_uri: Self::ParentId) -> Self {
        MyTopTracksLoader
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetMyTopTracks {
            tx,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}

#[derive(Clone)]
pub struct ShowLoader {
    uri: String,
}

impl ContainerLoader for ShowLoader {
    type ParentId = String;
    type Item = SimplifiedEpisode;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 10;

    fn new(uri: Self::ParentId) -> Self {
        ShowLoader { uri }
    }

    fn parent_id(&self) -> &Self::ParentId {
        &self.uri
    }

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        let uri = self.parent_id().clone();
        SpotifyCmd::GetShowEpisodes {
            tx,
            uri,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}

pub trait TrackLike: HasDuration + HasImages {
    fn id(&self) -> &str;
    fn uri(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> Option<&str> {
        None
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
        true
    }
    fn release_date(&self) -> Option<&str> {
        self.album().and_then(|album| album.release_date.as_deref())
    }
}

pub const COL_TRACK_THUMB: u32 = COL_ITEM_THUMB;
pub const COL_TRACK_URI: u32 = COL_ITEM_URI;
pub const COL_TRACK_NAME: u32 = COL_ITEM_NAME;
pub const COL_TRACK_ARTISTS: u32 = 3;
pub const COL_TRACK_NUMBER: u32 = 4;
pub const COL_TRACK_ALBUM: u32 = 5;
pub const COL_TRACK_CANT_PLAY: u32 = 6;
pub const COL_TRACK_DURATION: u32 = 7;
pub const COL_TRACK_DURATION_MS: u32 = 8;
pub const COL_TRACK_BPM: u32 = 9;
pub const COL_TRACK_TIMELINE: u32 = 10;
pub const COL_TRACK_RELEASE_DATE: u32 = 11;
pub const COL_TRACK_DESCRIPTION: u32 = 12;
pub const COL_TRACK_ALBUM_URI: u32 = 13;
pub const COL_TRACK_ARTIST_URI: u32 = 14;

impl<T: TrackLike> RowLike for T {
    fn content_types() -> Vec<Type> {
        vec![
            gdk_pixbuf::Pixbuf::static_type(), // thumb
            String::static_type(),             // track uri
            String::static_type(),             // name
            String::static_type(),             // artists
            u32::static_type(),                // number
            String::static_type(),             // album
            bool::static_type(),               // is playable
            String::static_type(),             // formatted duration
            u32::static_type(),                // duration in ms
            f32::static_type(),                // bpm
            String::static_type(),             // duration from start
            String::static_type(),             // release date
            String::static_type(),             // description
            String::static_type(),             // album uri
            String::static_type(),             // first artist uri
        ]
    }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter {
        store.insert_with_values(
            None,
            &[
                COL_TRACK_URI,
                COL_TRACK_NAME,
                COL_TRACK_ARTISTS,
                COL_TRACK_ALBUM,
                COL_TRACK_CANT_PLAY,
                COL_TRACK_DURATION,
                COL_TRACK_DURATION_MS,
                COL_TRACK_RELEASE_DATE,
                COL_TRACK_DESCRIPTION,
                COL_TRACK_ALBUM_URI,
                COL_TRACK_ARTIST_URI,
            ],
            &[
                &self.uri(),
                &self.name(),
                &self.artists().iter().map(|artist| &artist.name).join(", "),
                &self.album().map(|album| &*album.name),
                &!self.is_playable(),
                &crate::utils::humanize_time(self.duration()),
                &self.duration(),
                &self.release_date(),
                &self.description(),
                &self.album().and_then(|album| album.uri.as_deref()),
                &self
                    .artists()
                    .iter()
                    .next()
                    .and_then(|artist| artist.uri.as_deref()),
            ],
        )
    }
}

impl TrackLike for PlayHistory {
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

    fn release_date(&self) -> Option<&str> {
        self.track.release_date()
    }
}

impl HasDuration for PlayHistory {
    fn duration(&self) -> u32 {
        self.track.duration_ms
    }
}

impl HasImages for PlayHistory {
    fn images(&self) -> &[Image] {
        self.album().map(|album| &*album.images).unwrap_or(&[])
    }
}

impl MissingColumns for PlayHistory {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_TRACK_DESCRIPTION]
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

    fn release_date(&self) -> Option<&str> {
        self.track.as_ref().and_then(FullTrack::release_date)
    }
}

impl HasDuration for PlaylistTrack {
    fn duration(&self) -> u32 {
        self.track.as_ref().map_or(0, |track| track.duration_ms)
    }
    fn duration_exact(&self) -> bool {
        self.track.is_some()
    }
}

impl HasImages for PlaylistTrack {
    fn images(&self) -> &[Image] {
        self.album().map(|album| &*album.images).unwrap_or(&[])
    }
}

impl MissingColumns for PlaylistTrack {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_TRACK_DESCRIPTION]
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

    fn release_date(&self) -> Option<&str> {
        self.album.release_date.as_deref()
    }
}

impl HasDuration for FullTrack {
    fn duration(&self) -> u32 {
        self.duration_ms
    }
}

impl HasImages for FullTrack {
    fn images(&self) -> &[Image] {
        self.album().map(|album| &*album.images).unwrap_or(&[])
    }
}

impl MissingColumns for FullTrack {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_TRACK_DESCRIPTION]
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
}

impl HasDuration for SimplifiedTrack {
    fn duration(&self) -> u32 {
        self.duration_ms
    }
}

impl HasImages for SimplifiedTrack {
    fn images(&self) -> &[Image] {
        self.album().map(|album| &*album.images).unwrap_or(&[])
    }
}

impl MissingColumns for SimplifiedTrack {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[
            COL_TRACK_ALBUM,
            COL_TRACK_THUMB,
            COL_TRACK_RELEASE_DATE,
            COL_TRACK_DESCRIPTION,
        ]
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

    fn release_date(&self) -> Option<&str> {
        self.track.release_date()
    }
}

impl HasDuration for SavedTrack {
    fn duration(&self) -> u32 {
        self.track.duration_ms
    }
}

impl HasImages for SavedTrack {
    fn images(&self) -> &[Image] {
        self.album().map(|album| &*album.images).unwrap_or(&[])
    }
}

impl MissingColumns for SavedTrack {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_TRACK_DESCRIPTION]
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

    fn description(&self) -> Option<&str> {
        Some(&self.description)
    }

    fn is_playable(&self) -> bool {
        self.is_playable
    }

    fn release_date(&self) -> Option<&str> {
        Some(&self.release_date)
    }
}

impl HasDuration for FullEpisode {
    fn duration(&self) -> u32 {
        self.duration_ms
    }
}

impl HasImages for FullEpisode {
    fn images(&self) -> &[Image] {
        &self.images
    }
}

impl MissingColumns for FullEpisode {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_TRACK_ARTISTS, COL_TRACK_ALBUM]
    }
}

impl TrackLike for SimplifiedEpisode {
    fn id(&self) -> &str {
        &self.id
    }

    fn uri(&self) -> &str {
        &self.uri
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        Some(&self.description)
    }

    fn is_playable(&self) -> bool {
        self.is_playable
    }

    fn release_date(&self) -> Option<&str> {
        Some(&self.release_date)
    }
}

impl HasDuration for SimplifiedEpisode {
    fn duration(&self) -> u32 {
        self.duration_ms
    }
}

impl HasImages for SimplifiedEpisode {
    fn images(&self) -> &[Image] {
        &self.images
    }
}

impl MissingColumns for SimplifiedEpisode {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_TRACK_ARTISTS, COL_TRACK_ALBUM, COL_TRACK_BPM]
    }
}

impl HasDuration for PlayingItem {
    fn duration(&self) -> u32 {
        match self {
            PlayingItem::Track(track) => track.duration_ms,
            PlayingItem::Episode(episode) => episode.duration_ms,
        }
    }
}

impl MissingColumns for PlayingItem {}

impl HasImages for PlayingItem {
    fn images(&self) -> &[Image] {
        match self {
            PlayingItem::Track(track) => track.images(),
            PlayingItem::Episode(episode) => episode.images(),
        }
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
    release_date -> Option<&str>,
    description -> Option<&str>
}
