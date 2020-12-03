use async_trait::async_trait;
use rspotify::{
    client::ClientResult,
    model::{
        AudioAnalysis, AudioFeatures, Category, CurrentPlaybackContext, CursorBasedPage, Device, FullAlbum, FullArtist,
        FullEpisode, FullPlaylist, FullShow, FullTrack, Page, PlayHistory, PlaylistItem, PrivateUser, PublicUser, RepeatState,
        SavedAlbum, SavedTrack, Show, SimplifiedAlbum, SimplifiedEpisode, SimplifiedPlaylist, SimplifiedShow, SimplifiedTrack,
        Type,
    },
};
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
    async fn get_album(&self, uri: &str) -> ClientResult<FullAlbum>;
    async fn get_albums(&self, uris: &[String]) -> ClientResult<Vec<FullAlbum>>;
    async fn get_my_albums(&self, offset: u32, limit: u32) -> ClientResult<Page<SavedAlbum>>;
    async fn get_artist_albums(&self, uri: &str, offset: u32, limit: u32) -> ClientResult<Page<SimplifiedAlbum>>;
    async fn get_new_releases(&self, offset: u32, limit: u32) -> ClientResult<Page<SimplifiedAlbum>>;

    async fn add_my_albums(&self, uris: &[String]) -> ClientResult<()>;
    async fn remove_my_albums(&self, uris: &[String]) -> ClientResult<()>;
    async fn are_my_albums(&self, uris: &[String]) -> ClientResult<Vec<bool>>;
}

#[async_trait]
pub trait ArtistsStorageApi {
    async fn get_artist(&self, uri: &str) -> ClientResult<FullArtist>;
    async fn get_artists(&self, uris: &[String]) -> ClientResult<Vec<FullArtist>>;
    async fn get_my_artists(&self, cursor: Option<String>, limit: u32) -> ClientResult<CursorBasedPage<FullArtist>>;
    async fn get_my_top_artists(&self, offset: u32, limit: u32) -> ClientResult<Page<FullArtist>>;
    async fn get_artist_related_artists(&self, uri: &str) -> ClientResult<Vec<FullArtist>>;

    async fn add_my_artists(&self, uris: &[String]) -> ClientResult<()>;
    async fn remove_my_artists(&self, uris: &[String]) -> ClientResult<()>;
    async fn are_my_artists(&self, uris: &[String]) -> ClientResult<Vec<bool>>;
}

#[async_trait]
pub trait UsersStorageApi {
    async fn get_my_profile(&self) -> ClientResult<PrivateUser>;
    async fn get_user_profile(&self, uri: &str) -> ClientResult<PublicUser>;

    async fn add_my_users(&self, uris: &[String]) -> ClientResult<()>;
    async fn remove_my_users(&self, uris: &[String]) -> ClientResult<()>;
    async fn are_my_users(&self, uris: &[String]) -> ClientResult<Vec<bool>>;
}

#[async_trait]
pub trait ShowsStorageApi {
    async fn get_show(&self, uri: &str) -> ClientResult<FullShow>;
    async fn get_shows(&self, uris: &[String]) -> ClientResult<Vec<SimplifiedShow>>;
    async fn get_my_shows(&self, offset: u32, limit: u32) -> ClientResult<Page<Show>>;

    async fn add_my_shows(&self, uris: &[String]) -> ClientResult<()>;
    async fn remove_my_shows(&self, uris: &[String]) -> ClientResult<()>;
    async fn are_my_shows(&self, uris: &[String]) -> ClientResult<Vec<bool>>;
}

#[async_trait]
pub trait EpisodesStorageApi {
    async fn get_episode(&self, uri: &str) -> ClientResult<FullEpisode>;
    async fn get_episodes(&self, uris: &[String]) -> ClientResult<Vec<FullEpisode>>;
    async fn get_show_episodes(&self, uri: &str, offset: u32, limit: u32) -> ClientResult<Page<SimplifiedEpisode>>;
}

#[async_trait]
pub trait TracksStorageApi {
    async fn get_track(&self, uri: &str) -> ClientResult<FullTrack>;
    async fn get_tracks(&self, uris: &[String]) -> ClientResult<Vec<FullTrack>>;
    async fn get_track_analysis(&self, uri: &str) -> ClientResult<AudioAnalysis>;
    async fn get_tracks_features(&self, uris: &[String]) -> ClientResult<Vec<AudioFeatures>>;
    async fn get_my_tracks(&self, offset: u32, limit: u32) -> ClientResult<Page<SavedTrack>>;
    async fn get_my_top_tracks(&self, offset: u32, limit: u32) -> ClientResult<Page<FullTrack>>;
    async fn get_recent_tracks(&self, limit: u32) -> ClientResult<Vec<PlayHistory>>;
    async fn get_playlist_tracks(&self, uri: &str, offset: u32, limit: u32) -> ClientResult<Page<PlaylistItem>>;
    async fn get_album_tracks(&self, uri: &str, offset: u32, limit: u32) -> ClientResult<Page<SimplifiedTrack>>;
    async fn get_artist_top_tracks(&self, uri: &str) -> ClientResult<Vec<FullTrack>>;

    async fn add_my_tracks(&self, uris: &[String]) -> ClientResult<()>;
    async fn remove_my_tracks(&self, uris: &[String]) -> ClientResult<()>;
    async fn are_my_tracks(&self, uris: &[String]) -> ClientResult<Vec<bool>>;
}

#[async_trait]
pub trait PlaybackControlApi {
    async fn get_playback_state(&self) -> ClientResult<Option<CurrentPlaybackContext>>;
    async fn play_context(&self, uri: String, start_uri: Option<String>) -> ClientResult<()>;
    async fn play_tracks(&self, uris: &[String]) -> ClientResult<()>;
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

#[async_trait]
pub trait LibraryStorageApi:
    PlaylistsStorageApi
    + ArtistsStorageApi
    + AlbumsStorageApi
    + TracksStorageApi
    + PlaylistsStorageApi
    + UsersStorageApi
    + ShowsStorageApi
{
    async fn are_in_my_library(&self, tpe: Type, uris: &[String]) -> ClientResult<Vec<bool>> {
        match tpe {
            Type::Artist => self.are_my_artists(uris).await,
            Type::Album => self.are_my_albums(uris).await,
            Type::Track => self.are_my_tracks(uris).await,
            Type::Playlist => self.are_my_playlists(uris).await,
            Type::User => self.are_my_users(uris).await,
            Type::Show => self.are_my_shows(uris).await,
            Type::Episode => Ok(vec![false; uris.len()]),
        }
    }

    async fn add_to_my_library(&self, tpe: Type, uris: &[String]) -> ClientResult<()> {
        match tpe {
            Type::Artist => self.add_my_artists(uris).await,
            Type::Album => self.add_my_albums(uris).await,
            Type::Track => self.add_my_tracks(uris).await,
            Type::Playlist => self.add_my_playlists(uris, false).await,
            Type::User => self.add_my_users(uris).await,
            Type::Show => self.add_my_shows(uris).await,
            Type::Episode => Ok(()),
        }
    }

    async fn remove_from_my_library(&self, tpe: Type, uris: &[String]) -> ClientResult<()> {
        match tpe {
            Type::Artist => self.remove_my_artists(uris).await,
            Type::Album => self.remove_my_albums(uris).await,
            Type::Track => self.remove_my_tracks(uris).await,
            Type::Playlist => self.remove_my_playlists(uris).await,
            Type::User => self.remove_my_users(uris).await,
            Type::Show => self.remove_my_shows(uris).await,
            Type::Episode => Ok(()),
        }
    }
}

impl<T> LibraryStorageApi for T where
    T: PlaylistsStorageApi + ArtistsStorageApi + AlbumsStorageApi + TracksStorageApi + UsersStorageApi + ShowsStorageApi
{
}
