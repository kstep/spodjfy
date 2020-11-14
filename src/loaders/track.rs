use crate::loaders::paged::{PageLike, RowLike};
use crate::servers::spotify::{ResultSender, SpotifyCmd};
use glib::IsA;
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

pub trait TracksLoader: Clone + 'static {
    type ParentId;
    type Track: TrackLike;
    type Page: PageLike<Self::Track>;
    const PAGE_LIMIT: u32;
    fn new(id: Self::ParentId) -> Self;
    fn parent_id(&self) -> Self::ParentId;
    fn load_tracks_page(
        self,
        tx: ResultSender<Self::Page>,
        offset: <<Self as TracksLoader>::Page as PageLike<Self::Track>>::Offset,
    ) -> SpotifyCmd;
    fn uuid(&self) -> usize {
        self as *const _ as *const () as usize
    }
}

#[derive(Clone, Copy)]
pub struct SavedLoader;
impl TracksLoader for SavedLoader {
    type ParentId = ();
    type Track = SavedTrack;
    type Page = Page<Self::Track>;
    const PAGE_LIMIT: u32 = 20;

    fn new(_id: Self::ParentId) -> Self {
        SavedLoader
    }

    fn parent_id(&self) -> Self::ParentId {}

    fn load_tracks_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetMyTracks {
            tx,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}

#[derive(Clone, Copy)]
pub struct RecentLoader;
impl TracksLoader for RecentLoader {
    type ParentId = ();
    type Track = PlayHistory;
    type Page = Vec<Self::Track>;
    const PAGE_LIMIT: u32 = 50;

    fn new(_id: Self::ParentId) -> Self {
        RecentLoader
    }

    fn parent_id(&self) -> Self::ParentId {}

    fn load_tracks_page(self, tx: ResultSender<Self::Page>, _offset: ()) -> SpotifyCmd {
        SpotifyCmd::GetRecentTracks {
            tx,
            limit: Self::PAGE_LIMIT,
        }
    }
}

#[derive(Clone, Copy)]
pub struct QueueLoader;

impl TracksLoader for QueueLoader {
    type ParentId = ();
    type Track = FullTrack;
    type Page = Vec<Self::Track>;
    const PAGE_LIMIT: u32 = 0;

    fn new(_id: Self::ParentId) -> Self {
        QueueLoader
    }

    fn parent_id(&self) -> Self::ParentId {}

    fn load_tracks_page(self, tx: ResultSender<Self::Page>, _offset: ()) -> SpotifyCmd {
        SpotifyCmd::GetQueueTracks { tx }
    }
}

#[derive(Clone)]
pub struct AlbumLoader {
    uri: String,
}

impl TracksLoader for AlbumLoader {
    type ParentId = String;
    type Track = SimplifiedTrack;
    type Page = Page<Self::Track>;
    const PAGE_LIMIT: u32 = 10;

    fn new(uri: Self::ParentId) -> Self {
        AlbumLoader { uri }
    }

    fn parent_id(&self) -> Self::ParentId {
        self.uri.clone()
    }

    fn load_tracks_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        let uri = self.parent_id();
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

impl TracksLoader for PlaylistLoader {
    type ParentId = String;
    type Track = PlaylistTrack;
    type Page = Page<Self::Track>;
    const PAGE_LIMIT: u32 = 10;

    fn new(uri: Self::ParentId) -> Self {
        PlaylistLoader { uri }
    }

    fn parent_id(&self) -> Self::ParentId {
        self.uri.clone()
    }

    fn load_tracks_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        let uri = self.parent_id();
        SpotifyCmd::GetPlaylistTracks {
            tx,
            uri,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}

#[derive(Clone)]
pub struct ShowLoader {
    uri: String,
}

impl TracksLoader for ShowLoader {
    type ParentId = String;
    type Track = SimplifiedEpisode;
    type Page = Page<Self::Track>;
    const PAGE_LIMIT: u32 = 10;

    fn new(uri: Self::ParentId) -> Self {
        ShowLoader { uri }
    }

    fn parent_id(&self) -> Self::ParentId {
        self.uri.clone()
    }

    fn load_tracks_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        let uri = self.parent_id();
        SpotifyCmd::GetShowEpisodes {
            tx,
            uri,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}

pub trait TrackLike {
    fn id(&self) -> &str;
    fn uri(&self) -> &str;
    fn name(&self) -> &str;
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
    fn duration(&self) -> u32;
    fn release_date(&self) -> Option<&str> {
        self.album().and_then(|album| album.release_date.as_deref())
    }

    fn images(&self) -> Option<&Vec<Image>> {
        self.album().map(|album| &album.images)
    }

    fn unavailable_columns() -> &'static [u32] {
        &[]
    }
}

pub const COL_TRACK_ID: u32 = 0;
pub const COL_TRACK_THUMB: u32 = 1;
pub const COL_TRACK_NAME: u32 = 2;
pub const COL_TRACK_ARTISTS: u32 = 3;
pub const COL_TRACK_NUMBER: u32 = 4;
pub const COL_TRACK_ALBUM: u32 = 5;
pub const COL_TRACK_CANT_PLAY: u32 = 6;
pub const COL_TRACK_DURATION: u32 = 7;
pub const COL_TRACK_DURATION_MS: u32 = 8;
pub const COL_TRACK_URI: u32 = 9;
pub const COL_TRACK_BPM: u32 = 10;
pub const COL_TRACK_TIMELINE: u32 = 11;
pub const COL_TRACK_RELEASE_DATE: u32 = 12;

impl<T: TrackLike> RowLike for T {
    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter {
        store.insert_with_values(
            None,
            &[
                COL_TRACK_ID,
                COL_TRACK_NAME,
                COL_TRACK_ARTISTS,
                COL_TRACK_ALBUM,
                COL_TRACK_CANT_PLAY,
                COL_TRACK_DURATION,
                COL_TRACK_DURATION_MS,
                COL_TRACK_URI,
                COL_TRACK_RELEASE_DATE,
            ],
            &[
                &self.id(),
                &self.name(),
                &self.artists().iter().map(|artist| &artist.name).join(", "),
                &self.album().map(|album| &*album.name),
                &!self.is_playable(),
                &crate::utils::humanize_time(self.duration()),
                &self.duration(),
                &self.uri(),
                &self.release_date(),
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

    fn duration(&self) -> u32 {
        self.track.duration()
    }

    fn release_date(&self) -> Option<&str> {
        self.track.release_date()
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

    fn release_date(&self) -> Option<&str> {
        self.track.as_ref().and_then(FullTrack::release_date)
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

    fn release_date(&self) -> Option<&str> {
        self.album.release_date.as_deref()
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

    fn duration(&self) -> u32 {
        self.duration_ms
    }

    fn unavailable_columns() -> &'static [u32] {
        &[COL_TRACK_ALBUM, COL_TRACK_THUMB, COL_TRACK_RELEASE_DATE]
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

    fn release_date(&self) -> Option<&str> {
        self.track.release_date()
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

    fn is_playable(&self) -> bool {
        self.is_playable
    }

    fn duration(&self) -> u32 {
        self.duration_ms
    }

    fn release_date(&self) -> Option<&str> {
        Some(&self.release_date)
    }

    fn images(&self) -> Option<&Vec<Image>> {
        Some(&self.images)
    }

    fn unavailable_columns() -> &'static [u32] {
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

    fn is_playable(&self) -> bool {
        self.is_playable
    }

    fn duration(&self) -> u32 {
        self.duration_ms
    }

    fn release_date(&self) -> Option<&str> {
        Some(&self.release_date)
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
    duration -> u32, images -> Option<&Vec<Image>>,
    release_date -> Option<&str>
}
