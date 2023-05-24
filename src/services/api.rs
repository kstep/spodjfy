use async_trait::async_trait;
use rspotify::{
    ClientResult,
    model::{
        AudioAnalysis, AudioFeatures, Category, CurrentPlaybackContext, CursorBasedPage, Device, FullAlbum, FullArtist,
        FullEpisode, FullPlaylist, FullShow, FullTrack, Page, PlayHistory, PlaylistItem, PrivateUser, PublicUser, RepeatState,
        SavedAlbum, SavedTrack, Show, SimplifiedAlbum, SimplifiedEpisode, SimplifiedPlaylist, SimplifiedShow, SimplifiedTrack,
        Type,
    },
};
use rspotify::model::{AlbumId, ArtistId, EpisodeId, PlaylistId, ShowId, TrackId, UserId};
use rspotify::prelude::{Id, PlayableId, PlayContextId};
use serde_json::{Map, Value};

pub trait ThreadSafe: Send + Sync + 'static {}
impl<T> ThreadSafe for T where T: Send + Sync + 'static {}

#[async_trait]
pub trait PlaybackQueueApi {
    async fn dequeue_tracks(&mut self, uris: &[String]) -> ClientResult<()>;
    async fn enqueue_tracks(&mut self, uris: &[String]) -> ClientResult<()>;
    async fn get_queue_tracks(&self) -> ClientResult<Vec<FullTrack>>;
}

#[async_trait]
pub trait PlaylistsStorageApi {
    async fn get_playlist(&self, uri: &str) -> ClientResult<FullPlaylist>;
    async fn get_playlists(&self, uris: &[String]) -> ClientResult<Vec<FullPlaylist>>;
    async fn get_my_playlists(&self, offset: u32, limit: u32) -> ClientResult<Page<SimplifiedPlaylist>>;
    async fn get_user_playlists(&self, user_id: &str, offset: u32, limit: u32) -> ClientResult<Page<SimplifiedPlaylist>>;
    async fn get_featured_playlists(&self, offset: u32, limit: u32) -> ClientResult<Page<SimplifiedPlaylist>>;
    async fn get_categories(&self, offset: u32, limit: u32) -> ClientResult<Page<Category>>;
    async fn get_category_playlists(&self, category_id: &str, offset: u32, limit: u32) -> ClientResult<Page<SimplifiedPlaylist>>;

    async fn add_my_playlists(&self, uris: &[String], public: bool) -> ClientResult<()>;
    async fn remove_my_playlists(&self, uris: &[String]) -> ClientResult<()>;
    async fn are_my_playlists(&self, uris: &[String]) -> ClientResult<Vec<bool>>;
}

#[async_trait]
pub trait AlbumsStorageApi {
    async fn get_album(&self, id: &AlbumId) -> ClientResult<FullAlbum>;
    async fn get_albums(&self, ids: &[AlbumId]) -> ClientResult<Vec<FullAlbum>>;
    async fn get_my_albums(&self, offset: u32, limit: u32) -> ClientResult<Page<SavedAlbum>>;
    async fn get_artist_albums(&self, id: &ArtistId, offset: u32, limit: u32) -> ClientResult<Page<SimplifiedAlbum>>;
    async fn get_new_releases(&self, offset: u32, limit: u32) -> ClientResult<Page<SimplifiedAlbum>>;

    async fn add_my_albums(&self, uris: &[AlbumId]) -> ClientResult<()>;
    async fn remove_my_albums(&self, uris: &[AlbumId]) -> ClientResult<()>;
    async fn are_my_albums(&self, uris: &[AlbumId]) -> ClientResult<Vec<bool>>;
}

#[async_trait]
pub trait ArtistsStorageApi {
    async fn get_artist(&self, id: &ArtistId) -> ClientResult<FullArtist>;
    async fn get_artists(&self, uris: &[ArtistId]) -> ClientResult<Vec<FullArtist>>;
    async fn get_my_artists(&self, cursor: Option<String>, limit: u32) -> ClientResult<CursorBasedPage<FullArtist>>;
    async fn get_my_top_artists(&self, offset: u32, limit: u32) -> ClientResult<Page<FullArtist>>;
    async fn get_artist_related_artists(&self, id: &ArtistId) -> ClientResult<Vec<FullArtist>>;

    async fn add_my_artists(&self, uris: &[ArtistId]) -> ClientResult<()>;
    async fn remove_my_artists(&self, uris: &[ArtistId]) -> ClientResult<()>;
    async fn are_my_artists(&self, uris: &[ArtistId]) -> ClientResult<Vec<bool>>;
}

#[async_trait]
pub trait UsersStorageApi {
    async fn get_my_profile(&self) -> ClientResult<PrivateUser>;
    async fn get_user_profile(&self, uri: &UserId) -> ClientResult<PublicUser>;

    async fn add_my_users(&self, uris: &[UserId]) -> ClientResult<()>;
    async fn remove_my_users(&self, uris: &[UserId]) -> ClientResult<()>;
    async fn are_my_users(&self, uris: &[UserId]) -> ClientResult<Vec<bool>>;
}

#[async_trait]
pub trait ShowsStorageApi {
    async fn get_show(&self, id: &ShowId) -> ClientResult<FullShow>;
    async fn get_shows(&self, ids: &[ShowId]) -> ClientResult<Vec<SimplifiedShow>>;
    async fn get_my_shows(&self, offset: u32, limit: u32) -> ClientResult<Page<Show>>;

    async fn add_my_shows(&self, ids: &[ShowId]) -> ClientResult<()>;
    async fn remove_my_shows(&self, ids: &[ShowId]) -> ClientResult<()>;
    async fn are_my_shows(&self, ids: &[ShowId]) -> ClientResult<Vec<bool>>;
}

#[async_trait]
pub trait EpisodesStorageApi {
    async fn get_episode(&self, uri: &EpisodeId) -> ClientResult<FullEpisode>;
    async fn get_episodes(&self, uris: &[EpisodeId]) -> ClientResult<Vec<FullEpisode>>;
    async fn get_show_episodes(&self, uri: &ShowId, offset: u32, limit: u32) -> ClientResult<Page<SimplifiedEpisode>>;
}

#[async_trait]
pub trait TracksStorageApi {
    async fn get_track(&self, id: &TrackId) -> ClientResult<FullTrack>;
    async fn get_tracks(&self, ids: &[TrackId]) -> ClientResult<Vec<FullTrack>>;
    async fn get_track_analysis(&self, id: &TrackId) -> ClientResult<AudioAnalysis>;
    async fn get_tracks_features(&self, ids: &[TrackId]) -> ClientResult<Vec<AudioFeatures>>;
    async fn get_my_tracks(&self, offset: u32, limit: u32) -> ClientResult<Page<SavedTrack>>;
    async fn get_my_top_tracks(&self, offset: u32, limit: u32) -> ClientResult<Page<FullTrack>>;
    async fn get_recent_tracks(&self, limit: u32) -> ClientResult<Vec<PlayHistory>>;
    async fn get_playlist_tracks(&self, id: &PlaylistId, offset: u32, limit: u32) -> ClientResult<Page<PlaylistItem>>;
    async fn get_album_tracks(&self, id: &AlbumId, offset: u32, limit: u32) -> ClientResult<Page<SimplifiedTrack>>;
    async fn get_artist_top_tracks(&self, id: &ArtistId) -> ClientResult<Vec<FullTrack>>;

    async fn add_my_tracks(&self, ids: &[TrackId]) -> ClientResult<()>;
    async fn remove_my_tracks(&self, ids: &[TrackId]) -> ClientResult<()>;
    async fn are_my_tracks(&self, ids: &[TrackId]) -> ClientResult<Vec<bool>>;
}

#[async_trait]
pub trait PlaybackControlApi {
    async fn get_playback_state(&self) -> ClientResult<Option<CurrentPlaybackContext>>;
    async fn play_context(&self, id: &dyn PlayContextId, start_uri: Option<&dyn PlayableId>) -> ClientResult<()>;
    async fn play_tracks(&self, ids: &[TrackId]) -> ClientResult<()>;
    async fn start_playback(&self) -> ClientResult<()>;
    async fn pause_playback(&self) -> ClientResult<()>;
    async fn play_next_track(&self) -> ClientResult<()>;
    async fn play_prev_track(&self) -> ClientResult<()>;
    async fn seek_track(&self, pos: u32) -> ClientResult<()>;
    async fn set_volume(&self, value: u8) -> ClientResult<()>;
    async fn set_shuffle(&self, value: bool) -> ClientResult<()>;
    async fn set_repeat_mode(&self, mode: RepeatState) -> ClientResult<()>;

    async fn get_my_devices(&self) -> ClientResult<Vec<Device>>;
    async fn use_device(&self, id: &str, play: bool) -> ClientResult<()>;
}

#[async_trait]
pub trait SearchApi {
    async fn get_recommended_tracks(
        &self,
        seed_genres: Option<Vec<String>>,
        seed_artists: Option<Vec<String>>,
        seed_tracks: Option<Vec<String>>,
        tunables: Map<String, Value>,
        limit: u32,
    ) -> ClientResult<Vec<SimplifiedTrack>>;
}

pub trait LibraryStorageApi:
    PlaylistsStorageApi
    + ArtistsStorageApi
    + AlbumsStorageApi
    + TracksStorageApi
    + PlaylistsStorageApi
    + UsersStorageApi
    + ShowsStorageApi
{}

impl<T> LibraryStorageApi for T where
    T: PlaylistsStorageApi + ArtistsStorageApi + AlbumsStorageApi + TracksStorageApi + UsersStorageApi + ShowsStorageApi
{}
