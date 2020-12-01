use crate::scopes::Scope::{self, *};
use rspotify::client::{ClientError, ClientResult, Spotify as Client};
use rspotify::model::{
    offset, AdditionalType, AudioFeatures, Category, CurrentPlaybackContext, CursorBasedPage,
    Device, FullAlbum, FullArtist, FullPlaylist, FullShow, FullTrack, Id, Page, PlayHistory,
    PlaylistItem, PrivateUser, PublicUser, RepeatState, SavedAlbum, SavedTrack, Show,
    SimplifiedAlbum, SimplifiedEpisode, SimplifiedPlaylist, SimplifiedTrack, TimeRange, Type,
};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::collections::VecDeque;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

const DEFAULT_REFRESH_TOKEN_TIMEOUT: u64 = 20 * 60;

pub type SpotifyRef = Arc<RwLock<Spotify>>;

pub struct Spotify {
    cache_path: PathBuf,
    client: Client,
    queue: VecDeque<String>,
}

#[derive(Debug)]
pub enum ProxyError {
    Receive,
    Send,
    Backpressure(usize),
    Timeout,
    Upstream,
}

impl From<tokio::time::Elapsed> for ProxyError {
    fn from(_err: tokio::time::Elapsed) -> ProxyError {
        ProxyError::Timeout
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for ProxyError {
    fn from(_err: std::sync::mpsc::SendError<T>) -> ProxyError {
        ProxyError::Send
    }
}

impl From<std::sync::mpsc::RecvError> for ProxyError {
    fn from(_err: std::sync::mpsc::RecvError) -> ProxyError {
        ProxyError::Receive
    }
}

impl Spotify {
    pub fn new(id: String, secret: String, cache_path: PathBuf) -> Self {
        Spotify {
            client: Self::create_client(id, secret, cache_path.clone()),
            queue: VecDeque::new(),
            cache_path,
        }
    }

    pub async fn load_token_from_cache(&mut self) {
        self.client.token = self.client.read_token_cache().await;
    }

    pub async fn get_recent_tracks(&self, limit: u32) -> ClientResult<Vec<PlayHistory>> {
        self.client
            .current_user_recently_played(limit)
            .await
            .map(|page| page.items)
    }

    pub async fn get_my_devices(&self) -> ClientResult<Vec<Device>> {
        self.client.device().await.map(|reply| reply.devices)
    }

    pub async fn use_device(&self, id: String, play: bool) -> ClientResult<()> {
        self.client.transfer_playback(&id, play).await
    }

    pub async fn play_tracks(&self, uris: Vec<String>) -> ClientResult<()> {
        if uris.is_empty() {
            return Ok(());
        }

        self.client
            .start_playback(None, None, Some(uris), None, None)
            .await
    }

    pub async fn play_context(&self, uri: String, start_uri: Option<String>) -> ClientResult<()> {
        if uri.starts_with("spotify:artist:") {
            self.client
                .start_playback(None, Some(uri), start_uri.map(|uri| vec![uri]), None, None)
                .await
        } else {
            self.client
                .start_playback(
                    None,
                    Some(uri),
                    None,
                    start_uri.and_then(offset::for_uri),
                    None,
                )
                .await
        }
    }

    pub async fn get_tracks_features(&self, uris: &[String]) -> ClientResult<Vec<AudioFeatures>> {
        if uris.is_empty() {
            return Ok(Vec::new());
        }

        self.client
            .tracks_features(uris.iter().map(Deref::deref))
            .await
            .map(|payload| {
                payload
                    .map(|features| features.audio_features)
                    .unwrap_or_else(Vec::new)
            })
    }

    pub async fn get_my_tracks(&self, offset: u32, limit: u32) -> ClientResult<Page<SavedTrack>> {
        self.client.current_user_saved_tracks(limit, offset).await
    }

    pub async fn get_my_playlists(
        &self,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedPlaylist>> {
        self.client.current_user_playlists(limit, offset).await
    }

    pub async fn get_user_playlists(
        &self,
        user_id: &str,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedPlaylist>> {
        self.client.user_playlists(user_id, limit, offset).await
    }

    pub async fn get_my_albums(&self, offset: u32, limit: u32) -> ClientResult<Page<SavedAlbum>> {
        self.client.current_user_saved_albums(limit, offset).await
    }

    pub fn setup_client(&mut self, id: String, secret: String) -> ClientResult<String> {
        self.client = Self::create_client(id, secret, self.cache_path.clone());
        self.client.get_authorize_url(false)
    }

    fn create_client(id: String, secret: String, cache_path: PathBuf) -> rspotify::client::Spotify {
        let oauth: rspotify::oauth2::OAuth = rspotify::oauth2::OAuthBuilder::default()
            .scope(Scope::stringify(&[
                UserFollowRead,
                UserReadRecentlyPlayed,
                UserReadPlaybackState,
                UserReadPlaybackPosition,
                UserTopRead,
                UserLibraryRead,
                UserModifyPlaybackState,
                UserReadCurrentlyPlaying,
                PlaylistReadPrivate,
                PlaylistReadCollaborative,
                UserLibraryModify,
                PlaylistModifyPrivate,
                PlaylistModifyPublic,
                UserFollowModify,
            ]))
            .redirect_uri("http://localhost:8888/callback")
            .build()
            .unwrap();

        let creds: rspotify::oauth2::Credentials = rspotify::oauth2::CredentialsBuilder::default()
            .id(id)
            .secret(secret)
            .build()
            .unwrap();

        rspotify::client::SpotifyBuilder::default()
            .oauth(oauth)
            .credentials(creds)
            .cache_path(cache_path)
            .build()
            .unwrap()
    }

    pub async fn authorize_user(&mut self, code: String) -> ClientResult<()> {
        if code.starts_with("http") {
            if let Some(code) = self.client.parse_response_code(&code) {
                self.client.request_user_token(&code).await
            } else {
                Err(ClientError::InvalidAuth("Invalid code URL".into()))
            }
        } else {
            self.client.request_user_token(&code).await
        }
    }

    pub fn get_authorize_url(&self) -> ClientResult<String> {
        self.client.get_authorize_url(false)
    }

    pub async fn get_playback_state(&self) -> ClientResult<Option<CurrentPlaybackContext>> {
        self.client
            .current_playback(
                None,
                Some(vec![AdditionalType::Track, AdditionalType::Episode]),
            )
            .await
    }

    pub async fn start_playback(&self) -> ClientResult<()> {
        self.client
            .start_playback(None, None, None, None, None)
            .await
    }

    pub async fn pause_playback(&self) -> ClientResult<()> {
        self.client.pause_playback(None).await
    }

    pub async fn play_next_track(&self) -> ClientResult<()> {
        self.client.next_track(None).await
    }

    pub async fn play_prev_track(&self) -> ClientResult<()> {
        self.client.previous_track(None).await
    }

    pub async fn get_playlist_tracks(
        &self,
        uri: &str,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<PlaylistItem>> {
        self.client
            .playlist_tracks(uri, None, limit, offset, None)
            .await
    }

    pub async fn get_artist_albums(
        &self,
        uri: &str,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedAlbum>> {
        self.client
            .artist_albums(uri, None, None, Some(limit), Some(offset))
            .await
    }

    pub async fn get_album_tracks(
        &self,
        uri: &str,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedTrack>> {
        self.client.album_track(uri, limit, offset).await
    }

    pub async fn get_my_artists(
        &self,
        cursor: Option<String>,
        limit: u32,
    ) -> ClientResult<CursorBasedPage<FullArtist>> {
        self.client
            .current_user_followed_artists(limit, cursor)
            .await
            .map(|artists| artists.artists)
    }

    pub async fn seek_track(&self, pos: u32) -> ClientResult<()> {
        self.client.seek_track(pos, None).await
    }

    pub async fn set_volume(&self, value: u8) -> ClientResult<()> {
        self.client.volume(value, None).await
    }

    pub async fn set_shuffle(&self, value: bool) -> ClientResult<()> {
        self.client.shuffle(value, None).await
    }

    pub async fn set_repeat_mode(&self, mode: RepeatState) -> ClientResult<()> {
        self.client.repeat(mode, None).await
    }

    pub async fn refresh_user_token(&mut self) -> ClientResult<()> {
        if let Some(refresh_token) = self
            .client
            .token
            .as_ref()
            .and_then(|t| t.refresh_token.as_deref())
        {
            let token = refresh_token.to_owned();
            self.client.refresh_user_token(&token).await
        } else {
            Err(ClientError::InvalidAuth("Missing refresh token".into()))
        }
    }

    pub async fn get_album(&self, uri: &str) -> ClientResult<FullAlbum> {
        self.client.album(uri).await
    }

    pub async fn get_artist(&self, uri: &str) -> ClientResult<FullArtist> {
        self.client.artist(uri).await
    }

    pub async fn get_playlist(&self, uri: &str) -> ClientResult<FullPlaylist> {
        self.client.playlist(uri, None, None).await
    }

    pub fn set_redirect_uri<'a>(&mut self, url: impl Into<Cow<'a, str>>) {
        if let Some(ref mut oauth) = self.client.oauth {
            oauth.redirect_uri = url.into().into_owned();
        }
    }

    pub async fn get_my_profile(&self) -> ClientResult<PrivateUser> {
        self.client.current_user().await
    }

    pub async fn get_user_profile(&self, user_id: &str) -> ClientResult<PublicUser> {
        self.client.user(user_id).await
    }

    pub async fn get_my_shows(&self, offset: u32, limit: u32) -> ClientResult<Page<Show>> {
        self.client.get_saved_show(limit, offset).await
    }

    pub async fn get_show(&self, uri: &str) -> ClientResult<FullShow> {
        let id = Id::from_id_or_uri(Type::Show, uri)?.id().to_owned();
        self.client.get_a_show(id, None).await
    }

    pub async fn get_show_episodes(
        &self,
        uri: &str,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedEpisode>> {
        let id = Id::from_id_or_uri(Type::Episode, uri)?.id().to_owned();
        self.client
            .get_shows_episodes(id, limit, offset, None)
            .await
    }

    pub async fn dequeue_tracks(&mut self, uris: &[String]) -> ClientResult<()> {
        self.queue.retain(|uri| !uris.contains(uri));
        Ok(())
    }

    pub async fn enqueue_tracks(&mut self, uris: Vec<String>) -> ClientResult<()> {
        if uris.is_empty() {
            return Ok(());
        }

        futures::future::try_join_all(
            uris.iter()
                .cloned()
                .map(|uri| self.client.add_item_to_queue(uri, None)),
        )
        .await?;
        self.queue.extend(uris);
        Ok(())
    }

    pub async fn get_queue_tracks(&self) -> ClientResult<Vec<FullTrack>> {
        if self.queue.is_empty() {
            return Ok(Vec::new());
        }

        let uris = self
            .queue
            .iter()
            .map(|uri| uri.as_str())
            .collect::<Vec<_>>();
        self.client
            .tracks(uris, None)
            .await
            .map(|reply| reply.tracks)
    }

    pub async fn add_my_tracks(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .current_user_saved_tracks_add(uris.iter().map(Deref::deref))
            .await
    }

    pub async fn remove_my_tracks(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .current_user_saved_tracks_delete(uris.iter().map(Deref::deref))
            .await
    }

    pub async fn add_my_albums(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .current_user_saved_albums_add(uris.iter().map(Deref::deref))
            .await
    }

    pub async fn remove_my_albums(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .current_user_saved_albums_delete(uris.iter().map(Deref::deref))
            .await
    }

    pub async fn add_my_artists(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .user_follow_artists(uris.iter().map(Deref::deref))
            .await
    }

    pub async fn remove_my_artists(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .user_unfollow_artists(uris.iter().map(Deref::deref))
            .await
    }

    pub async fn add_my_shows(&self, uris: &[String]) -> ClientResult<()> {
        self.client.save_shows(uris.iter().map(Deref::deref)).await
    }

    pub async fn remove_my_shows(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .remove_users_saved_shows(uris.iter().map(Deref::deref), None)
            .await
    }

    pub async fn add_my_playlists(&self, uris: &[String], public: bool) -> ClientResult<()> {
        futures::future::try_join_all(
            uris.iter()
                .map(|uri| self.client.playlist_follow(&uri, public)),
        )
        .await
        .map(|_| ())
    }

    pub async fn remove_my_playlists(&self, uris: &[String]) -> ClientResult<()> {
        futures::future::try_join_all(uris.iter().map(|uri| self.client.playlist_unfollow(&uri)))
            .await
            .map(|_| ())
    }

    pub async fn add_my_users(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .user_follow_users(uris.iter().map(Deref::deref))
            .await
    }

    pub async fn remove_my_users(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .user_unfollow_users(uris.iter().map(Deref::deref))
            .await
    }

    pub async fn are_my_albums(&self, uris: &[String]) -> ClientResult<Vec<bool>> {
        self.client
            .current_user_saved_albums_contains(uris.iter().map(Deref::deref))
            .await
    }

    pub async fn are_my_artists(&self, uris: &[String]) -> ClientResult<Vec<bool>> {
        self.client
            .user_artist_check_follow(uris.iter().map(Deref::deref))
            .await
    }

    pub async fn are_my_shows(&self, uris: &[String]) -> ClientResult<Vec<bool>> {
        self.client
            .check_users_saved_shows(uris.iter().map(Deref::deref))
            .await
    }

    pub async fn are_my_playlists(&self, uris: &[String]) -> ClientResult<Vec<bool>> {
        // TODO: dummy implementation
        Ok(std::iter::repeat(true).take(uris.len()).collect())
    }

    pub async fn are_my_users(&self, uris: &[String]) -> ClientResult<Vec<bool>> {
        // TODO: dummy implementation
        Ok(std::iter::repeat(false).take(uris.len()).collect())
    }

    pub async fn are_my_tracks(&self, uris: &[String]) -> ClientResult<Vec<bool>> {
        self.client
            .current_user_saved_tracks_contains(uris.iter().map(Deref::deref))
            .await
    }

    pub async fn get_categories(&self, offset: u32, limit: u32) -> ClientResult<Page<Category>> {
        self.client
            .categories(None, None, limit, offset)
            .await
            .map(|page| page.categories)
    }

    pub async fn get_category_playlists(
        &self,
        category_id: &str,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedPlaylist>> {
        self.client
            .category_playlists(category_id, None, limit, offset)
            .await
    }

    pub async fn get_featured_playlists(
        &self,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedPlaylist>> {
        self.client
            .featured_playlists(None, None, None, limit, offset)
            .await
            .map(|page| page.playlists)
    }

    pub async fn get_new_releases(
        &self,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedAlbum>> {
        self.client
            .new_releases(None, limit, offset)
            .await
            .map(|page| page.albums)
    }

    pub async fn get_my_top_tracks(
        &self,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<FullTrack>> {
        self.client
            .current_user_top_tracks(limit, offset, TimeRange::MediumTerm)
            .await
    }

    pub async fn get_my_top_artists(
        &self,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<FullArtist>> {
        self.client
            .current_user_top_artists(limit, offset, TimeRange::MediumTerm)
            .await
    }

    pub async fn get_artist_top_tracks(&self, uri: &str) -> ClientResult<Vec<FullTrack>> {
        self.client
            .artist_top_tracks(uri, None)
            .await
            .map(|reply| reply.tracks)
    }

    pub async fn get_artist_related_artists(&self, uri: &str) -> ClientResult<Vec<FullArtist>> {
        self.client
            .artist_related_artists(uri)
            .await
            .map(|reply| reply.artists)
    }

    pub async fn get_recommended_tracks(
        &self,
        seed_genres: Option<Vec<String>>,
        seed_artists: Option<Vec<String>>,
        seed_tracks: Option<Vec<String>>,
        tunables: Map<String, Value>,
        limit: u32,
    ) -> ClientResult<Vec<SimplifiedTrack>> {
        self.client
            .recommendations(
                seed_artists,
                seed_genres,
                seed_tracks,
                limit,
                None,
                &tunables,
            )
            .await
            .map(|recommended| recommended.tracks)
    }
}
