use crate::scopes::Scope::{self, *};
use futures_util::TryFutureExt;
use rspotify::client::{ClientError, ClientResult, Spotify as Client};
use rspotify::model::album::{FullAlbum, SavedAlbum, SimplifiedAlbum};
use rspotify::model::artist::FullArtist;
use rspotify::model::audio::AudioFeatures;
use rspotify::model::category::Category;
use rspotify::model::context::CurrentPlaybackContext;
use rspotify::model::device::Device;
use rspotify::model::page::{CursorBasedPage, Page};
use rspotify::model::playing::PlayHistory;
use rspotify::model::playlist::{FullPlaylist, PlaylistTrack, SimplifiedPlaylist};
use rspotify::model::show::{FullShow, Show, SimplifiedEpisode};
use rspotify::model::track::{FullTrack, SavedTrack, SimplifiedTrack};
use rspotify::model::{offset, AdditionalType, RepeatState, TimeRange};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::collections::VecDeque;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, SendError, Sender};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use derivative::Derivative;

const DEFAULT_REFRESH_TOKEN_TIMEOUT: u64 = 20 * 60;

#[derive(Clone)]
pub struct SpotifyProxy {
    spotify_tx: Sender<SpotifyCmd>,
    errors_stream: relm::EventStream<ClientError>,
}

impl SpotifyProxy {
    pub fn new(spotify_tx: Sender<SpotifyCmd>) -> (SpotifyProxy, relm::EventStream<ClientError>) {
        tokio::spawn(Self::refresh_token_thread(spotify_tx.clone()));

        let errors_stream = relm::EventStream::new();
        (
            SpotifyProxy {
                spotify_tx,
                errors_stream: errors_stream.clone(),
            },
            errors_stream,
        )
    }

    async fn refresh_token_thread(spotify_tx: Sender<SpotifyCmd>) -> Result<(), ()> {
        let mut refresh_token_timer =
            tokio::time::interval(Duration::from_secs(DEFAULT_REFRESH_TOKEN_TIMEOUT));
        loop {
            refresh_token_timer.tick().await;
            info!("refresh access token");
            spotify_tx
                .send(SpotifyCmd::RefreshUserToken)
                .map_err(|error| {
                    error!("refresh token thread stopped: {:?}", error);
                })?;
        }
    }

    pub fn tell(&self, cmd: SpotifyCmd) -> Result<(), SendError<SpotifyCmd>> {
        self.spotify_tx.send(cmd)
    }
    pub fn ask<T, F, R, M>(
        &self,
        stream: relm::EventStream<M>,
        make_command: F,
        convert_output: R,
    ) -> Result<(), SendError<SpotifyCmd>>
    where
        F: FnOnce(ResultSender<T>) -> SpotifyCmd + 'static,
        R: Fn(T) -> M + 'static,
        M: 'static,
    {
        let errors_stream = self.errors_stream.clone();
        let (_, tx) = relm::Channel::<ClientResult<T>>::new(move |reply| match reply {
            Ok(out) => stream.emit(convert_output(out)),
            Err(err) => {
                error!("spotify error: {:?}", err);
                errors_stream.emit(err)
            }
        });
        self.spotify_tx.send(make_command(tx))
    }
}

pub type ResultSender<T> = relm::Sender<ClientResult<T>>;

#[derive(Derivative)]
#[derivative(Debug)]
pub enum SpotifyCmd {
    SetupClient {
        #[derivative(Debug="ignore")]
        tx: ResultSender<String>,
        id: String,
        secret: String,
    },
    GetAuthorizeUrl {
        #[derivative(Debug="ignore")]
        tx: ResultSender<String>,
    },
    AuthorizeUser {
        #[derivative(Debug="ignore")]
        tx: ResultSender<()>,
        code: String,
    },
    RefreshUserToken,
    PausePlayback,
    StartPlayback,
    PlayPrevTrack,
    PlayNextTrack,
    GetMyShows {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Page<Show>>,
        offset: u32,
        limit: u32,
    },
    GetShowEpisodes {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Page<SimplifiedEpisode>>,
        limit: u32,
        offset: u32,
        uri: String,
    },
    GetMyArtists {
        #[derivative(Debug="ignore")]
        tx: ResultSender<CursorBasedPage<FullArtist>>,
        cursor: Option<String>,
        limit: u32,
    },
    GetMyAlbums {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Page<SavedAlbum>>,
        offset: u32,
        limit: u32,
    },
    GetMyPlaylists {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Page<SimplifiedPlaylist>>,
        offset: u32,
        limit: u32,
    },
    GetMyTracks {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Page<SavedTrack>>,
        offset: u32,
        limit: u32,
    },
    PlayTracks {
        uris: Vec<String>,
    },
    PlayContext {
        uri: String,
        start_uri: Option<String>,
    },
    GetTracksFeatures {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Vec<AudioFeatures>>,
        uris: Vec<String>,
    },
    GetMyDevices {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Vec<Device>>,
    },
    UseDevice {
        id: String,
    },
    SetVolume {
        value: u8,
    },
    SetShuffle {
        state: bool,
    },
    SetRepeatMode {
        mode: RepeatState,
    },
    GetPlaybackState {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Option<CurrentPlaybackContext>>,
    },
    GetPlaylistTracks {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Page<PlaylistTrack>>,
        limit: u32,
        offset: u32,
        uri: String,
    },
    GetAlbumTracks {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Page<SimplifiedTrack>>,
        limit: u32,
        offset: u32,
        uri: String,
    },
    SeekTrack {
        pos: u32,
    },
    GetPlaylist {
        #[derivative(Debug="ignore")]
        tx: ResultSender<FullPlaylist>,
        uri: String,
    },
    GetAlbum {
        #[derivative(Debug="ignore")]
        tx: ResultSender<FullAlbum>,
        uri: String,
    },
    GetArtist {
        #[derivative(Debug="ignore")]
        tx: ResultSender<FullArtist>,
        uri: String,
    },
    GetArtistAlbums {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Page<SimplifiedAlbum>>,
        uri: String,
        offset: u32,
        limit: u32,
    },
    GetShow {
        #[derivative(Debug="ignore")]
        tx: ResultSender<FullShow>,
        uri: String,
    },
    GetRecentTracks {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Vec<PlayHistory>>,
        limit: u32,
    },
    EnqueueTracks {
        uris: Vec<String>,
    },
    DequeueTracks {
        uris: Vec<String>,
    },
    AddMyTracks {
        uris: Vec<String>,
    },
    RemoveMyTracks {
        uris: Vec<String>,
    },
    GetQueueTracks {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Vec<FullTrack>>,
    },
    GetCategories {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Page<Category>>,
        offset: u32,
        limit: u32,
    },
    GetCategoryPlaylists {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Page<SimplifiedPlaylist>>,
        category_id: String,
        offset: u32,
        limit: u32,
    },
    GetFeaturedPlaylists {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Page<SimplifiedPlaylist>>,
        offset: u32,
        limit: u32,
    },
    GetNewReleases {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Page<SimplifiedAlbum>>,
        offset: u32,
        limit: u32,
    },
    GetMyTopTracks {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Page<FullTrack>>,
        offset: u32,
        limit: u32,
    },
    GetMyTopArtists {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Page<FullArtist>>,
        offset: u32,
        limit: u32,
    },
    GetArtistTopTracks {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Vec<FullTrack>>,
        uri: String,
    },
    GetRecommendedTracks {
        #[derivative(Debug="ignore")]
        tx: ResultSender<Vec<SimplifiedTrack>>,
        limit: u32,
        seed_artists: Option<Vec<String>>,
        seed_genres: Option<Vec<String>>,
        seed_tracks: Option<Vec<String>>,
        tunables: Map<String, Value>,
    },
}

pub struct Spotify {
    cache_path: PathBuf,
    client: Client,
    queue: VecDeque<String>,
}

#[derive(Debug)]
pub enum SpotifyError {
    Recv,
    Send,
    Timeout,
}

impl From<tokio::time::Elapsed> for SpotifyError {
    fn from(_err: tokio::time::Elapsed) -> SpotifyError {
        SpotifyError::Timeout
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for SpotifyError {
    fn from(_err: std::sync::mpsc::SendError<T>) -> SpotifyError {
        SpotifyError::Send
    }
}

impl From<std::sync::mpsc::RecvError> for SpotifyError {
    fn from(_err: std::sync::mpsc::RecvError) -> SpotifyError {
        SpotifyError::Recv
    }
}

pub struct SpotifyServer {
    client: Arc<Mutex<Spotify>>,
    channel: Receiver<SpotifyCmd>,
}

impl SpotifyServer {
    const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

    pub fn new(client: Arc<Mutex<Spotify>>, channel: Receiver<SpotifyCmd>) -> SpotifyServer {
        SpotifyServer { client, channel }
    }

    pub fn spawn(self) -> JoinHandle<Result<(), SpotifyError>> {
        tokio::spawn(self.run().inspect_err(|error| {
            error!("spotify thread error: {:?}", error);
        }))
    }

    pub async fn run(self) -> Result<(), SpotifyError> {
        loop {
            let msg = self.channel.recv()?;
            tokio::time::timeout(
                Self::REQUEST_TIMEOUT,
                Self::handle(self.client.clone(), msg),
            )
            .unwrap_or_else(|_| {
                error!("spotify request timeout");
                Ok(())
            })
            .await?;
        }
    }

    async fn handle(client: Arc<Mutex<Spotify>>, msg: SpotifyCmd) -> Result<(), SpotifyError> {
        use SpotifyCmd::*;
        debug!("serving message: {:?}", msg);

        match msg {
            SetupClient { tx, id, secret } => {
                let url = client.lock().await.setup_client(id, secret).await;
                tx.send(url)?;
            }
            StartPlayback => {
                let _ = client.lock().await.start_playback().await;
            }
            SeekTrack { pos } => {
                let _ = client.lock().await.seek_track(pos).await;
            }
            PausePlayback => {
                let _ = client.lock().await.pause_playback().await;
            }
            PlayNextTrack => {
                let _ = client.lock().await.play_next_track().await;
            }
            PlayPrevTrack => {
                let _ = client.lock().await.play_prev_track().await;
            }
            GetPlaybackState { tx } => {
                let state = client.lock().await.get_playback_state().await;
                tx.send(state)?;
            }
            GetArtistAlbums {
                tx,
                limit,
                offset,
                uri,
            } => {
                let reply = client
                    .lock()
                    .await
                    .get_artist_albums(&uri, offset, limit)
                    .await;
                tx.send(reply)?;
            }
            GetAlbumTracks {
                tx,
                limit,
                offset,
                uri,
            } => {
                let tracks = client
                    .lock()
                    .await
                    .get_album_tracks(&uri, offset, limit)
                    .await;
                tx.send(tracks)?;
            }
            GetPlaylistTracks {
                tx,
                limit,
                offset,
                uri,
            } => {
                let tracks = client
                    .lock()
                    .await
                    .get_playlist_tracks(&uri, offset, limit)
                    .await;
                tx.send(tracks)?;
            }
            GetQueueTracks { tx } => {
                let reply = client.lock().await.get_queue_tracks().await;
                tx.send(reply)?;
            }
            GetAuthorizeUrl { tx } => {
                let url = client.lock().await.get_authorize_url();
                tx.send(url)?;
            }
            UseDevice { id } => {
                let _ = client.lock().await.use_device(id).await;
            }
            AuthorizeUser { tx, code } => {
                let reply = client.lock().await.authorize_user(code).await;
                tx.send(reply)?;
            }
            RefreshUserToken => {
                let result = client.lock().await.refresh_user_token().await;
                info!("refresh access token result: {:?}", result);
            }
            GetMyShows { tx, offset, limit } => {
                let result = client.lock().await.get_my_shows(offset, limit).await;
                tx.send(result)?;
            }
            GetShowEpisodes {
                tx,
                uri,
                offset,
                limit,
            } => {
                let result = client
                    .lock()
                    .await
                    .get_show_episodes(&uri, offset, limit)
                    .await;
                tx.send(result)?;
            }
            GetMyArtists { tx, cursor, limit } => {
                let artists = client.lock().await.get_my_artists(cursor, limit).await;
                tx.send(artists)?;
            }
            GetMyAlbums { tx, offset, limit } => {
                let albums = client.lock().await.get_my_albums(offset, limit).await;
                tx.send(albums)?;
            }
            GetMyPlaylists { tx, offset, limit } => {
                let playlists = client.lock().await.get_my_playlists(offset, limit).await;
                tx.send(playlists)?;
            }
            GetMyTracks { tx, offset, limit } => {
                let tracks = client.lock().await.get_my_tracks(offset, limit).await;
                tx.send(tracks)?;
            }
            PlayTracks { uris } => {
                let _ = client.lock().await.play_tracks(uris).await;
            }
            PlayContext { uri, start_uri } => {
                let _ = client.lock().await.play_context(uri, start_uri).await;
            }
            GetTracksFeatures { tx, uris } => {
                let features = client.lock().await.get_tracks_features(&uris).await;
                tx.send(features)?;
            }
            GetMyDevices { tx } => {
                let devices = client.lock().await.get_my_devices().await;
                tx.send(devices)?;
            }
            SetVolume { value } => {
                let _ = client.lock().await.set_volume(value).await;
            }
            SetShuffle { state } => {
                let _ = client.lock().await.set_shuffle(state).await;
            }
            SetRepeatMode { mode } => {
                let _ = client.lock().await.set_repeat_mode(mode).await;
            }
            GetAlbum { tx, uri } => {
                let reply = client.lock().await.get_album(&uri).await;
                tx.send(reply)?;
            }
            GetPlaylist { tx, uri } => {
                let reply = client.lock().await.get_playlist(&uri).await;
                tx.send(reply)?;
            }
            GetArtist { tx, uri } => {
                let reply = client.lock().await.get_artist(&uri).await;
                tx.send(reply)?;
            }
            GetShow { tx, uri } => {
                let reply = client.lock().await.get_show(&uri).await;
                tx.send(reply)?;
            }
            GetRecentTracks { tx, limit } => {
                let reply = client.lock().await.get_recent_tracks(limit).await;
                tx.send(reply)?;
            }
            EnqueueTracks { uris } => {
                let _ = client.lock().await.enqueue_tracks(uris).await;
            }
            DequeueTracks { uris } => {
                let _ = client.lock().await.dequeue_tracks(&uris).await;
            }
            AddMyTracks { uris } => {
                let _ = client.lock().await.add_my_tracks(&uris).await;
            }
            RemoveMyTracks { uris } => {
                let _ = client.lock().await.remove_my_tracks(&uris).await;
            }
            GetCategories { tx, offset, limit } => {
                let reply = client.lock().await.get_categories(offset, limit).await;
                tx.send(reply)?;
            }
            GetCategoryPlaylists {
                tx,
                category_id,
                offset,
                limit,
            } => {
                let reply = client
                    .lock()
                    .await
                    .get_category_playlists(&category_id, offset, limit)
                    .await;
                tx.send(reply)?;
            }
            GetFeaturedPlaylists { tx, offset, limit } => {
                let reply = client
                    .lock()
                    .await
                    .get_featured_playlists(offset, limit)
                    .await;
                tx.send(reply)?;
            }
            GetNewReleases { tx, offset, limit } => {
                let reply = client.lock().await.get_new_releases(offset, limit).await;
                tx.send(reply)?;
            }
            GetMyTopArtists { tx, offset, limit } => {
                let reply = client.lock().await.get_my_top_artists(offset, limit).await;
                tx.send(reply)?;
            }
            GetMyTopTracks { tx, offset, limit } => {
                let reply = client.lock().await.get_my_top_tracks(offset, limit).await;
                tx.send(reply)?;
            }
            GetArtistTopTracks { tx, uri } => {
                let reply = client.lock().await.get_artist_top_tracks(&uri).await;
                tx.send(reply)?;
            }
            GetRecommendedTracks {
                tx,
                seed_artists,
                seed_genres,
                seed_tracks,
                tunables,
                limit,
            } => {
                let reply = client
                    .lock()
                    .await
                    .get_recommended_tracks(seed_genres, seed_artists, seed_tracks, tunables, limit)
                    .await;
                tx.send(reply)?;
            }
        }
        Ok(())
    }
}

impl Spotify {
    pub async fn new(id: String, secret: String, cache_path: PathBuf) -> Self {
        Spotify {
            client: Self::create_client(id, secret, cache_path.clone()).await,
            queue: VecDeque::new(),
            cache_path,
        }
    }

    async fn get_recent_tracks(&self, limit: u32) -> ClientResult<Vec<PlayHistory>> {
        self.client
            .current_user_recently_played(limit)
            .await
            .map(|page| page.items)
    }

    async fn get_my_devices(&self) -> ClientResult<Vec<Device>> {
        self.client.device().await.map(|reply| reply.devices)
    }

    async fn use_device(&self, id: String) -> ClientResult<()> {
        self.client.transfer_playback(&id, false).await
    }

    async fn play_tracks(&self, uris: Vec<String>) -> ClientResult<()> {
        if uris.is_empty() {
            return Ok(());
        }

        self.client
            .start_playback(None, None, Some(uris), None, None)
            .await
    }

    async fn play_context(&self, uri: String, start_uri: Option<String>) -> ClientResult<()> {
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

    async fn get_tracks_features(&self, uris: &[String]) -> ClientResult<Vec<AudioFeatures>> {
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

    async fn get_my_tracks(&self, offset: u32, limit: u32) -> ClientResult<Page<SavedTrack>> {
        self.client.current_user_saved_tracks(limit, offset).await
    }

    async fn get_my_playlists(
        &self,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedPlaylist>> {
        self.client.current_user_playlists(limit, offset).await
    }

    async fn get_my_albums(&self, offset: u32, limit: u32) -> ClientResult<Page<SavedAlbum>> {
        self.client.current_user_saved_albums(limit, offset).await
    }

    async fn setup_client(&mut self, id: String, secret: String) -> ClientResult<String> {
        self.client = Self::create_client(id, secret, self.cache_path.clone()).await;
        self.client.get_authorize_url(false)
    }

    async fn create_client(
        id: String,
        secret: String,
        cache_path: PathBuf,
    ) -> rspotify::client::Spotify {
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
            ]))
            .redirect_uri("http://localhost:8888/callback")
            .build()
            .unwrap();

        let creds: rspotify::oauth2::Credentials = rspotify::oauth2::CredentialsBuilder::default()
            .id(id)
            .secret(secret)
            .build()
            .unwrap();

        let mut client: rspotify::client::Spotify = rspotify::client::SpotifyBuilder::default()
            .oauth(oauth)
            .credentials(creds)
            .cache_path(cache_path)
            .build()
            .unwrap();

        client.token = client.read_token_cache().await;
        client
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

    async fn get_playback_state(&self) -> ClientResult<Option<CurrentPlaybackContext>> {
        self.client
            .current_playback(
                None,
                Some(vec![AdditionalType::Track, AdditionalType::Episode]),
            )
            .await
    }

    async fn start_playback(&self) -> ClientResult<()> {
        self.client
            .start_playback(None, None, None, None, None)
            .await
    }

    async fn pause_playback(&self) -> ClientResult<()> {
        self.client.pause_playback(None).await
    }

    async fn play_next_track(&self) -> ClientResult<()> {
        self.client.next_track(None).await
    }

    async fn play_prev_track(&self) -> ClientResult<()> {
        self.client.previous_track(None).await
    }

    async fn get_playlist_tracks(
        &self,
        uri: &str,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<PlaylistTrack>> {
        self.client
            .playlist_tracks(uri, None, limit, offset, None)
            .await
    }

    async fn get_artist_albums(
        &self,
        uri: &str,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedAlbum>> {
        self.client
            .artist_albums(uri, None, None, Some(limit), Some(offset))
            .await
    }

    async fn get_album_tracks(
        &self,
        uri: &str,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedTrack>> {
        self.client.album_track(uri, limit, offset).await
    }

    async fn get_my_artists(
        &self,
        cursor: Option<String>,
        limit: u32,
    ) -> ClientResult<CursorBasedPage<FullArtist>> {
        self.client
            .current_user_followed_artists(limit, cursor)
            .await
            .map(|artists| artists.artists)
    }

    async fn seek_track(&self, pos: u32) -> ClientResult<()> {
        self.client.seek_track(pos, None).await
    }

    async fn set_volume(&self, value: u8) -> ClientResult<()> {
        self.client.volume(value, None).await
    }

    async fn set_shuffle(&self, value: bool) -> ClientResult<()> {
        self.client.shuffle(value, None).await
    }

    async fn set_repeat_mode(&self, mode: RepeatState) -> ClientResult<()> {
        self.client.repeat(mode, None).await
    }

    async fn refresh_user_token(&mut self) -> ClientResult<()> {
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

    async fn get_album(&self, uri: &str) -> ClientResult<FullAlbum> {
        self.client.album(uri).await
    }

    async fn get_artist(&self, uri: &str) -> ClientResult<FullArtist> {
        self.client.artist(uri).await
    }

    async fn get_playlist(&self, uri: &str) -> ClientResult<FullPlaylist> {
        self.client.playlist(uri, None, None).await
    }

    pub fn set_redirect_uri<'a>(&mut self, url: impl Into<Cow<'a, str>>) {
        if let Some(ref mut oauth) = self.client.oauth {
            oauth.redirect_uri = url.into().into_owned();
        }
    }

    async fn get_my_shows(&self, offset: u32, limit: u32) -> ClientResult<Page<Show>> {
        self.client.get_saved_show(limit, offset).await
    }

    async fn get_show(&self, uri: &str) -> ClientResult<FullShow> {
        if let Some(id) = Self::get_id(uri) {
            self.client.get_a_show(id.to_owned(), None).await
        } else {
            Err(ClientError::Request("Invalid show URI".into()))
        }
    }

    async fn get_show_episodes(
        &self,
        uri: &str,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedEpisode>> {
        if let Some(id) = Self::get_id(uri) {
            self.client
                .get_shows_episodes(id.to_owned(), limit, offset, None)
                .await
        } else {
            Err(ClientError::Request("Invalid show URI".into()))
        }
    }

    async fn dequeue_tracks(&mut self, uris: &[String]) -> ClientResult<()> {
        self.queue.retain(|uri| !uris.contains(uri));
        Ok(())
    }

    async fn enqueue_tracks(&mut self, uris: Vec<String>) -> ClientResult<()> {
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

    async fn get_queue_tracks(&self) -> ClientResult<Vec<FullTrack>> {
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

    async fn add_my_tracks(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .current_user_saved_tracks_add(uris.iter().map(Deref::deref))
            .await
    }

    async fn remove_my_tracks(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .current_user_saved_tracks_delete(uris.iter().map(Deref::deref))
            .await
    }

    async fn get_categories(&self, offset: u32, limit: u32) -> ClientResult<Page<Category>> {
        self.client
            .categories(None, None, limit, offset)
            .await
            .map(|page| page.categories)
    }

    async fn get_category_playlists(
        &self,
        category_id: &str,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedPlaylist>> {
        self.client
            .category_playlists(category_id, None, limit, offset)
            .await
            .map(|reply| reply.playlists)
    }

    async fn get_featured_playlists(
        &self,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedPlaylist>> {
        self.client
            .featured_playlists(None, None, None, limit, offset)
            .await
            .map(|page| page.playlists)
    }

    async fn get_new_releases(
        &self,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedAlbum>> {
        self.client
            .new_releases(None, limit, offset)
            .await
            .map(|page| page.albums)
    }

    async fn get_my_top_tracks(&self, offset: u32, limit: u32) -> ClientResult<Page<FullTrack>> {
        self.client
            .current_user_top_tracks(limit, offset, TimeRange::MediumTerm)
            .await
    }

    async fn get_my_top_artists(&self, offset: u32, limit: u32) -> ClientResult<Page<FullArtist>> {
        self.client
            .current_user_top_artists(limit, offset, TimeRange::MediumTerm)
            .await
    }

    async fn get_artist_top_tracks(&self, uri: &str) -> ClientResult<Vec<FullTrack>> {
        self.client
            .artist_top_tracks(uri, None)
            .await
            .map(|reply| reply.tracks)
    }

    async fn get_recommended_tracks(
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

    fn get_id(uri: &str) -> Option<&str> {
        uri.split(':').last()
    }
}
