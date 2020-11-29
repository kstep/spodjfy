use crate::scopes::Scope::{self, *};
use crate::servers::{Proxy, ResultSender};
use derivative::Derivative;
use futures_util::TryFutureExt;
use relm::EventStream;
use rspotify::client::{ClientError, ClientResult, Spotify as Client};
use rspotify::model::album::{FullAlbum, SavedAlbum, SimplifiedAlbum};
use rspotify::model::artist::FullArtist;
use rspotify::model::audio::AudioFeatures;
use rspotify::model::category::Category;
use rspotify::model::context::CurrentPlaybackContext;
use rspotify::model::device::Device;
use rspotify::model::page::{CursorBasedPage, Page};
use rspotify::model::playing::PlayHistory;
use rspotify::model::show::{FullShow, Show, SimplifiedEpisode};
use rspotify::model::track::{FullTrack, SavedTrack, SimplifiedTrack};
use rspotify::model::{
    offset, AdditionalType, FullPlaylist, PlaylistItem, PrivateUser, PublicUser, RepeatState,
    SimplifiedPlaylist, TimeRange, Type,
};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::collections::VecDeque;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, SendError, Sender};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

const DEFAULT_REFRESH_TOKEN_TIMEOUT: u64 = 20 * 60;

#[derive(Clone)]
pub struct SpotifyProxy {
    tx: Sender<SpotifyCmd>,
    errors_stream: relm::EventStream<ClientError>,
}

impl Proxy for SpotifyProxy {
    type Command = SpotifyCmd;
    type Error = ClientError;
    fn tell(&self, cmd: Self::Command) -> Result<(), SendError<Self::Command>> {
        self.tx.send(cmd)
    }

    fn errors_stream(&self) -> EventStream<Self::Error> {
        self.errors_stream.clone()
    }
}

impl SpotifyProxy {
    pub fn new() -> (
        SpotifyProxy,
        Receiver<SpotifyCmd>,
        relm::EventStream<ClientError>,
    ) {
        let (tx, rx) = channel();
        let errors_stream = relm::EventStream::new();
        (
            SpotifyProxy {
                tx,
                errors_stream: errors_stream.clone(),
            },
            rx,
            errors_stream,
        )
    }
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub enum SpotifyCmd {
    SetupClient {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<String>,
        id: String,
        secret: String,
    },
    GetAuthorizeUrl {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<String>,
    },
    AuthorizeUser {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<()>,
        code: String,
    },
    RefreshUserToken,
    PausePlayback,
    StartPlayback,
    PlayPrevTrack,
    PlayNextTrack,
    GetMyProfile {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<PrivateUser>,
    },
    GetUserProfile {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<PublicUser>,
        uri: String,
    },
    GetMyShows {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Page<Show>>,
        offset: u32,
        limit: u32,
    },
    GetShowEpisodes {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Page<SimplifiedEpisode>>,
        limit: u32,
        offset: u32,
        uri: String,
    },
    GetMyArtists {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<CursorBasedPage<FullArtist>>,
        cursor: Option<String>,
        limit: u32,
    },
    GetMyAlbums {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Page<SavedAlbum>>,
        offset: u32,
        limit: u32,
    },
    GetMyPlaylists {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Page<SimplifiedPlaylist>>,
        offset: u32,
        limit: u32,
    },
    GetUserPlaylists {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Page<SimplifiedPlaylist>>,
        user_id: String,
        offset: u32,
        limit: u32,
    },
    GetMyTracks {
        #[derivative(Debug = "ignore")]
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
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Vec<AudioFeatures>>,
        uris: Vec<String>,
    },
    GetMyDevices {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Vec<Device>>,
    },
    UseDevice {
        id: String,
        play: bool,
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
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Option<CurrentPlaybackContext>>,
    },
    GetPlaylistItems {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Page<PlaylistItem>>,
        limit: u32,
        offset: u32,
        uri: String,
    },
    GetAlbumTracks {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Page<SimplifiedTrack>>,
        limit: u32,
        offset: u32,
        uri: String,
    },
    SeekTrack {
        pos: u32,
    },
    GetPlaylist {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<FullPlaylist>,
        uri: String,
    },
    GetAlbum {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<FullAlbum>,
        uri: String,
    },
    GetArtist {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<FullArtist>,
        uri: String,
    },
    GetArtistAlbums {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Page<SimplifiedAlbum>>,
        uri: String,
        offset: u32,
        limit: u32,
    },
    GetShow {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<FullShow>,
        uri: String,
    },
    GetRecentTracks {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Vec<PlayHistory>>,
        limit: u32,
    },
    EnqueueTracks {
        uris: Vec<String>,
    },
    DequeueTracks {
        uris: Vec<String>,
    },
    AddToMyLibrary {
        kind: Type,
        uris: Vec<String>,
    },
    RemoveFromMyLibrary {
        kind: Type,
        uris: Vec<String>,
    },
    AreInMyLibrary {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Vec<bool>>,
        kind: Type,
        uris: Vec<String>,
    },
    GetQueueTracks {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Vec<FullTrack>>,
    },
    GetCategories {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Page<Category>>,
        offset: u32,
        limit: u32,
    },
    GetCategoryPlaylists {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Page<SimplifiedPlaylist>>,
        category_id: String,
        offset: u32,
        limit: u32,
    },
    GetFeaturedPlaylists {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Page<SimplifiedPlaylist>>,
        offset: u32,
        limit: u32,
    },
    GetNewReleases {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Page<SimplifiedAlbum>>,
        offset: u32,
        limit: u32,
    },
    GetMyTopTracks {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Page<FullTrack>>,
        offset: u32,
        limit: u32,
    },
    GetMyTopArtists {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Page<FullArtist>>,
        offset: u32,
        limit: u32,
    },
    GetArtistTopTracks {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Vec<FullTrack>>,
        uri: String,
    },
    GetArtistRelatedArtists {
        #[derivative(Debug = "ignore")]
        tx: ResultSender<Vec<FullArtist>>,
        uri: String,
    },
    GetRecommendedTracks {
        #[derivative(Debug = "ignore")]
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

pub struct SpotifyServer {
    client: Arc<Mutex<Spotify>>,
    channel: Receiver<SpotifyCmd>,
}

impl SpotifyServer {
    pub fn new(client: Arc<Mutex<Spotify>>, channel: Receiver<SpotifyCmd>) -> SpotifyServer {
        SpotifyServer { client, channel }
    }

    pub async fn spawn(self) -> JoinHandle<Result<(), ProxyError>> {
        tokio::spawn(
            Self::refresh_token(self.client.clone()).inspect_err(|error| {
                error!("spotify refresh token thread error: {:?}", error);
            }),
        );
        tokio::spawn(self.run().inspect_err(|error| {
            error!("spotify main thread error: {:?}", error);
        }))
    }

    async fn refresh_token(client: Arc<Mutex<Spotify>>) -> Result<(), ProxyError> {
        let mut refresh_token_timer =
            tokio::time::interval(Duration::from_secs(DEFAULT_REFRESH_TOKEN_TIMEOUT));
        loop {
            refresh_token_timer.tick().await;
            info!("refresh access token");
            client
                .lock()
                .await
                .refresh_user_token()
                .await
                .map_err(|error| {
                    error!("refresh access token thread stopped: {:?}", error);
                    ProxyError::Upstream
                })?;
        }
    }

    async fn run(self) -> Result<(), ProxyError> {
        loop {
            let msg = self.channel.recv()?;

            loop {
                let msg = msg.clone();
                let reply = Self::handle(self.client.clone(), msg).await;
                if let Err(ProxyError::Backpressure(timeout)) = reply {
                    let timeout = timeout + 1;
                    warn!(
                        "spotify rate limit exceeded, waiting {} secs to resend",
                        timeout
                    );
                    tokio::time::delay_for(Duration::from_secs(timeout as u64)).await;
                } else {
                    reply?;
                    break;
                }
            }
        }
    }

    fn handle_upstream_errors<T>(reply: ClientResult<T>) -> Result<ClientResult<T>, ProxyError> {
        match reply {
            Err(ClientError::RateLimited(timeout)) => {
                Err(ProxyError::Backpressure(timeout.unwrap_or(4)))
            }
            reply => Ok(reply),
        }
    }

    async fn handle(client: Arc<Mutex<Spotify>>, msg: SpotifyCmd) -> Result<(), ProxyError> {
        use SpotifyCmd::*;
        debug!("serving message: {:?}", msg);

        match msg {
            SetupClient { tx, id, secret } => {
                let url = Self::handle_upstream_errors(
                    client.lock().await.setup_client(id, secret).await,
                )?;
                tx.send(url)?;
            }
            StartPlayback => {
                let _ = Self::handle_upstream_errors(client.lock().await.start_playback().await)?;
            }
            SeekTrack { pos } => {
                let _ = Self::handle_upstream_errors(client.lock().await.seek_track(pos).await)?;
            }
            PausePlayback => {
                let _ = Self::handle_upstream_errors(client.lock().await.pause_playback().await)?;
            }
            PlayNextTrack => {
                let _ = Self::handle_upstream_errors(client.lock().await.play_next_track().await)?;
            }
            PlayPrevTrack => {
                let _ = Self::handle_upstream_errors(client.lock().await.play_prev_track().await)?;
            }
            GetPlaybackState { tx } => {
                let state =
                    Self::handle_upstream_errors(client.lock().await.get_playback_state().await)?;
                tx.send(state)?;
            }
            GetArtistAlbums {
                tx,
                limit,
                offset,
                uri,
            } => {
                let reply = Self::handle_upstream_errors(
                    client
                        .lock()
                        .await
                        .get_artist_albums(&uri, offset, limit)
                        .await,
                )?;
                tx.send(reply)?;
            }
            GetAlbumTracks {
                tx,
                limit,
                offset,
                uri,
            } => {
                let tracks = Self::handle_upstream_errors(
                    client
                        .lock()
                        .await
                        .get_album_tracks(&uri, offset, limit)
                        .await,
                )?;
                tx.send(tracks)?;
            }
            GetPlaylistItems {
                tx,
                limit,
                offset,
                uri,
            } => {
                let tracks = Self::handle_upstream_errors(
                    client
                        .lock()
                        .await
                        .get_playlist_tracks(&uri, offset, limit)
                        .await,
                )?;
                tx.send(tracks)?;
            }
            GetQueueTracks { tx } => {
                let reply =
                    Self::handle_upstream_errors(client.lock().await.get_queue_tracks().await)?;
                tx.send(reply)?;
            }
            GetAuthorizeUrl { tx } => {
                let url = Self::handle_upstream_errors(client.lock().await.get_authorize_url())?;
                tx.send(url)?;
            }
            UseDevice { id, play } => {
                let _ =
                    Self::handle_upstream_errors(client.lock().await.use_device(id, play).await)?;
            }
            AuthorizeUser { tx, code } => {
                let reply =
                    Self::handle_upstream_errors(client.lock().await.authorize_user(code).await)?;
                tx.send(reply)?;
            }
            RefreshUserToken => {
                let reply =
                    Self::handle_upstream_errors(client.lock().await.refresh_user_token().await)?;
                info!("refresh access token result: {:?}", reply);
            }
            GetMyProfile { tx } => {
                let reply =
                    Self::handle_upstream_errors(client.lock().await.get_my_profile().await)?;
                tx.send(reply)?;
            }
            GetUserProfile { tx, uri } => {
                let reply =
                    Self::handle_upstream_errors(client.lock().await.get_user_profile(&uri).await)?;
                tx.send(reply)?;
            }
            GetMyShows { tx, offset, limit } => {
                let reply = Self::handle_upstream_errors(
                    client.lock().await.get_my_shows(offset, limit).await,
                )?;
                tx.send(reply)?;
            }
            GetShowEpisodes {
                tx,
                uri,
                offset,
                limit,
            } => {
                let result = Self::handle_upstream_errors(
                    client
                        .lock()
                        .await
                        .get_show_episodes(&uri, offset, limit)
                        .await,
                )?;
                tx.send(result)?;
            }
            GetMyArtists { tx, cursor, limit } => {
                let artists = Self::handle_upstream_errors(
                    client.lock().await.get_my_artists(cursor, limit).await,
                )?;
                tx.send(artists)?;
            }
            GetMyAlbums { tx, offset, limit } => {
                let albums = Self::handle_upstream_errors(
                    client.lock().await.get_my_albums(offset, limit).await,
                )?;
                tx.send(albums)?;
            }
            GetMyPlaylists { tx, offset, limit } => {
                let playlists = Self::handle_upstream_errors(
                    client.lock().await.get_my_playlists(offset, limit).await,
                )?;
                tx.send(playlists)?;
            }
            GetUserPlaylists {
                tx,
                user_id,
                offset,
                limit,
            } => {
                let playlists = Self::handle_upstream_errors(
                    client
                        .lock()
                        .await
                        .get_user_playlists(&user_id, offset, limit)
                        .await,
                )?;
                tx.send(playlists)?;
            }
            GetMyTracks { tx, offset, limit } => {
                let tracks = Self::handle_upstream_errors(
                    client.lock().await.get_my_tracks(offset, limit).await,
                )?;
                tx.send(tracks)?;
            }
            PlayTracks { uris } => {
                let _ = Self::handle_upstream_errors(client.lock().await.play_tracks(uris).await)?;
            }
            PlayContext { uri, start_uri } => {
                let _ = Self::handle_upstream_errors(
                    client.lock().await.play_context(uri, start_uri).await,
                )?;
            }
            GetTracksFeatures { tx, uris } => {
                let features = Self::handle_upstream_errors(
                    client.lock().await.get_tracks_features(&uris).await,
                )?;
                tx.send(features)?;
            }
            GetMyDevices { tx } => {
                let devices =
                    Self::handle_upstream_errors(client.lock().await.get_my_devices().await)?;
                tx.send(devices)?;
            }
            SetVolume { value } => {
                let _ = Self::handle_upstream_errors(client.lock().await.set_volume(value).await)?;
            }
            SetShuffle { state } => {
                let _ = Self::handle_upstream_errors(client.lock().await.set_shuffle(state).await)?;
            }
            SetRepeatMode { mode } => {
                let _ =
                    Self::handle_upstream_errors(client.lock().await.set_repeat_mode(mode).await)?;
            }
            GetAlbum { tx, uri } => {
                let reply =
                    Self::handle_upstream_errors(client.lock().await.get_album(&uri).await)?;
                tx.send(reply)?;
            }
            GetPlaylist { tx, uri } => {
                let reply =
                    Self::handle_upstream_errors(client.lock().await.get_playlist(&uri).await)?;
                tx.send(reply)?;
            }
            GetArtist { tx, uri } => {
                let reply =
                    Self::handle_upstream_errors(client.lock().await.get_artist(&uri).await)?;
                tx.send(reply)?;
            }
            GetShow { tx, uri } => {
                let reply = Self::handle_upstream_errors(client.lock().await.get_show(&uri).await)?;
                tx.send(reply)?;
            }
            GetRecentTracks { tx, limit } => {
                let reply = Self::handle_upstream_errors(
                    client.lock().await.get_recent_tracks(limit).await,
                )?;
                tx.send(reply)?;
            }
            EnqueueTracks { uris } => {
                let _ =
                    Self::handle_upstream_errors(client.lock().await.enqueue_tracks(uris).await)?;
            }
            DequeueTracks { uris } => {
                let _ =
                    Self::handle_upstream_errors(client.lock().await.dequeue_tracks(&uris).await)?;
            }
            AddToMyLibrary { kind, uris } => {
                let client = client.lock().await;
                let _ = Self::handle_upstream_errors(match kind {
                    Type::Artist => client.add_my_artists(&uris).await,
                    Type::User => client.add_my_users(&uris).await,
                    Type::Show => client.add_my_shows(&uris).await,
                    Type::Album => client.add_my_albums(&uris).await,
                    Type::Playlist => client.add_my_playlists(&uris, true).await,
                    Type::Track => client.add_my_tracks(&uris).await,
                    Type::Episode => Ok(()),
                })?;
            }
            RemoveFromMyLibrary { kind, uris } => {
                let client = client.lock().await;
                let _ = Self::handle_upstream_errors(match kind {
                    Type::Artist => client.remove_my_artists(&uris).await,
                    Type::User => client.remove_my_users(&uris).await,
                    Type::Show => client.remove_my_shows(&uris).await,
                    Type::Album => client.remove_my_albums(&uris).await,
                    Type::Playlist => client.remove_my_playlists(&uris).await,
                    Type::Track => client.remove_my_tracks(&uris).await,
                    Type::Episode => Ok(()),
                })?;
            }
            AreInMyLibrary { tx, kind, uris } => {
                let client = client.lock().await;
                let reply = Self::handle_upstream_errors(match kind {
                    Type::Artist => client.are_my_artists(&uris).await,
                    Type::User => client.are_my_users(&uris).await,
                    Type::Show => client.are_my_shows(&uris).await,
                    Type::Album => client.are_my_albums(&uris).await,
                    Type::Playlist => client.are_my_playlists(&uris).await,
                    Type::Track => client.are_my_tracks(&uris).await,
                    Type::Episode => Ok(std::iter::repeat(false).take(uris.len()).collect()),
                })?;
                tx.send(reply)?;
            }
            GetCategories { tx, offset, limit } => {
                let reply = Self::handle_upstream_errors(
                    client.lock().await.get_categories(offset, limit).await,
                )?;
                tx.send(reply)?;
            }
            GetCategoryPlaylists {
                tx,
                category_id,
                offset,
                limit,
            } => {
                let reply = Self::handle_upstream_errors(
                    client
                        .lock()
                        .await
                        .get_category_playlists(&category_id, offset, limit)
                        .await,
                )?;
                tx.send(reply)?;
            }
            GetFeaturedPlaylists { tx, offset, limit } => {
                let reply = Self::handle_upstream_errors(
                    client
                        .lock()
                        .await
                        .get_featured_playlists(offset, limit)
                        .await,
                )?;
                tx.send(reply)?;
            }
            GetNewReleases { tx, offset, limit } => {
                let reply = Self::handle_upstream_errors(
                    client.lock().await.get_new_releases(offset, limit).await,
                )?;
                tx.send(reply)?;
            }
            GetMyTopArtists { tx, offset, limit } => {
                let reply = Self::handle_upstream_errors(
                    client.lock().await.get_my_top_artists(offset, limit).await,
                )?;
                tx.send(reply)?;
            }
            GetMyTopTracks { tx, offset, limit } => {
                let reply = Self::handle_upstream_errors(
                    client.lock().await.get_my_top_tracks(offset, limit).await,
                )?;
                tx.send(reply)?;
            }
            GetArtistTopTracks { tx, uri } => {
                let reply = Self::handle_upstream_errors(
                    client.lock().await.get_artist_top_tracks(&uri).await,
                )?;
                tx.send(reply)?;
            }
            GetArtistRelatedArtists { tx, uri } => {
                let reply = Self::handle_upstream_errors(
                    client.lock().await.get_artist_related_artists(&uri).await,
                )?;
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
                let reply = Self::handle_upstream_errors(
                    client
                        .lock()
                        .await
                        .get_recommended_tracks(
                            seed_genres,
                            seed_artists,
                            seed_tracks,
                            tunables,
                            limit,
                        )
                        .await,
                )?;
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

    async fn use_device(&self, id: String, play: bool) -> ClientResult<()> {
        self.client.transfer_playback(&id, play).await
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

    async fn get_user_playlists(
        &self,
        user_id: &str,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedPlaylist>> {
        self.client.user_playlists(user_id, limit, offset).await
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
    ) -> ClientResult<Page<PlaylistItem>> {
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

    async fn get_my_profile(&self) -> ClientResult<PrivateUser> {
        self.client.current_user().await
    }

    async fn get_user_profile(&self, user_id: &str) -> ClientResult<PublicUser> {
        self.client.user(user_id).await
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

    async fn add_my_albums(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .current_user_saved_albums_add(uris.iter().map(Deref::deref))
            .await
    }

    async fn remove_my_albums(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .current_user_saved_albums_delete(uris.iter().map(Deref::deref))
            .await
    }

    async fn add_my_artists(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .user_follow_artists(uris.iter().map(Deref::deref))
            .await
    }

    async fn remove_my_artists(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .user_unfollow_artists(uris.iter().map(Deref::deref))
            .await
    }

    async fn add_my_shows(&self, uris: &[String]) -> ClientResult<()> {
        self.client.save_shows(uris.iter().map(Deref::deref)).await
    }

    async fn remove_my_shows(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .remove_users_saved_shows(uris.iter().map(Deref::deref), None)
            .await
    }

    async fn add_my_playlists(&self, uris: &[String], public: bool) -> ClientResult<()> {
        futures::future::try_join_all(
            uris.iter()
                .map(|uri| self.client.playlist_follow(&uri, public)),
        )
        .await
        .map(|_| ())
    }

    async fn remove_my_playlists(&self, uris: &[String]) -> ClientResult<()> {
        futures::future::try_join_all(uris.iter().map(|uri| self.client.playlist_unfollow(&uri)))
            .await
            .map(|_| ())
    }

    async fn add_my_users(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .user_follow_users(uris.iter().map(Deref::deref))
            .await
    }

    async fn remove_my_users(&self, uris: &[String]) -> ClientResult<()> {
        self.client
            .user_unfollow_users(uris.iter().map(Deref::deref))
            .await
    }

    async fn are_my_albums(&self, uris: &[String]) -> ClientResult<Vec<bool>> {
        self.client
            .current_user_saved_albums_contains(uris.iter().map(Deref::deref))
            .await
    }

    async fn are_my_artists(&self, uris: &[String]) -> ClientResult<Vec<bool>> {
        self.client
            .user_artist_check_follow(uris.iter().map(Deref::deref))
            .await
    }

    async fn are_my_shows(&self, uris: &[String]) -> ClientResult<Vec<bool>> {
        self.client
            .check_users_saved_shows(uris.iter().map(Deref::deref))
            .await
    }

    async fn are_my_playlists(&self, uris: &[String]) -> ClientResult<Vec<bool>> {
        // TODO: dummy implementation
        Ok(std::iter::repeat(true).take(uris.len()).collect())
    }

    async fn are_my_users(&self, uris: &[String]) -> ClientResult<Vec<bool>> {
        // TODO: dummy implementation
        Ok(std::iter::repeat(false).take(uris.len()).collect())
    }

    async fn are_my_tracks(&self, uris: &[String]) -> ClientResult<Vec<bool>> {
        self.client
            .current_user_saved_tracks_contains(uris.iter().map(Deref::deref))
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

    async fn get_artist_related_artists(&self, uri: &str) -> ClientResult<Vec<FullArtist>> {
        self.client
            .artist_related_artists(uri)
            .await
            .map(|reply| reply.artists)
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
