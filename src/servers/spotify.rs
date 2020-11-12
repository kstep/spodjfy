use crate::scopes::Scope::{self, *};
use futures_util::TryFutureExt;
use gtk::{BoxExt, DialogExt, EntryExt, GtkWindowExt, WidgetExt};
use rspotify::client::{ClientError, ClientResult, Spotify as Client};
use rspotify::model::album::{FullAlbum, SavedAlbum};
use rspotify::model::artist::FullArtist;
use rspotify::model::audio::AudioFeatures;
use rspotify::model::context::CurrentlyPlaybackContext;
use rspotify::model::device::Device;
use rspotify::model::offset;
use rspotify::model::page::{CursorBasedPage, Page};
use rspotify::model::playing::PlayHistory;
use rspotify::model::playlist::{FullPlaylist, PlaylistTrack, SimplifiedPlaylist};
use rspotify::model::show::{FullShow, Show, SimplifiedEpisode};
use rspotify::model::track::{SavedTrack, SimplifiedTrack};
use rspotify::senum::{AdditionalType, RepeatState};
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

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

    async fn refresh_token_thread(spotify_tx: Sender<SpotifyCmd>) {
        let mut refresh_token_timer =
            tokio::time::interval(Duration::from_secs(DEFAULT_REFRESH_TOKEN_TIMEOUT));
        loop {
            refresh_token_timer.tick().await;
            info!("refresh access token");
            spotify_tx.send(SpotifyCmd::RefreshUserToken).unwrap();
        }
    }

    pub fn tell(&self, cmd: SpotifyCmd) {
        self.spotify_tx.send(cmd).unwrap();
    }
    pub fn ask<T, F, R, M>(&self, stream: relm::EventStream<M>, make_command: F, convert_output: R)
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
        self.spotify_tx.send(make_command(tx)).unwrap();
    }

    pub fn get_code_url_from_user() -> Option<String> {
        let dialog = gtk::MessageDialogBuilder::new()
            .title("Authentication")
            .text("Please enter the URL you were redirected to:")
            .message_type(gtk::MessageType::Question)
            .accept_focus(true)
            .modal(true)
            .sensitive(true)
            .buttons(gtk::ButtonsType::OkCancel)
            .build();

        let message_box = dialog.get_content_area();
        let url_entry = gtk::Entry::new();
        message_box.pack_end(&url_entry, true, false, 0);
        dialog.show_all();
        let url = match dialog.run() {
            gtk::ResponseType::Ok => Some(url_entry.get_text().into()),
            gtk::ResponseType::Cancel => None,
            _ => unreachable!(),
        };
        dialog.close();

        url
    }
}

pub type ResultSender<T> = relm::Sender<ClientResult<T>>;

pub enum SpotifyCmd {
    SetupClient {
        id: String,
        secret: String,
    },
    OpenAuthorizeUrl,
    AuthorizeUser {
        tx: ResultSender<()>,
        code: String,
    },
    RefreshUserToken,
    PausePlayback,
    StartPlayback,
    PlayPrevTrack,
    PlayNextTrack,
    GetMyShows {
        tx: ResultSender<Page<Show>>,
        offset: u32,
        limit: u32,
    },
    GetShowEpisodes {
        tx: ResultSender<Page<SimplifiedEpisode>>,
        limit: u32,
        offset: u32,
        uri: String,
    },
    GetMyArtists {
        tx: ResultSender<CursorBasedPage<FullArtist>>,
        cursor: Option<String>,
        limit: u32,
    },
    GetMyAlbums {
        tx: ResultSender<Page<SavedAlbum>>,
        offset: u32,
        limit: u32,
    },
    GetMyPlaylists {
        tx: ResultSender<Page<SimplifiedPlaylist>>,
        offset: u32,
        limit: u32,
    },
    GetMyTracks {
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
        tx: ResultSender<Vec<AudioFeatures>>,
        uris: Vec<String>,
    },
    GetMyDevices {
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
        tx: ResultSender<Option<CurrentlyPlaybackContext>>,
    },
    GetPlaylistTracks {
        tx: ResultSender<Page<PlaylistTrack>>,
        limit: u32,
        offset: u32,
        uri: String,
    },
    GetAlbumTracks {
        tx: ResultSender<Page<SimplifiedTrack>>,
        limit: u32,
        offset: u32,
        uri: String,
    },
    SeekTrack {
        pos: u32,
    },
    GetPlaylist {
        tx: ResultSender<FullPlaylist>,
        uri: String,
    },
    GetAlbum {
        tx: ResultSender<FullAlbum>,
        uri: String,
    },
    GetArtist {
        tx: ResultSender<FullArtist>,
        uri: String,
    },
    GetShow {
        tx: ResultSender<FullShow>,
        uri: String,
    },
    GetRecentTracks {
        tx: ResultSender<Vec<PlayHistory>>,
        limit: u32,
    },
}

pub struct Spotify {
    cache_path: PathBuf,
    client: Client,
}

#[derive(Debug)]
pub enum SpotifyError {
    Recv,
    Send,
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
    pub fn new(client: Arc<Mutex<Spotify>>, channel: Receiver<SpotifyCmd>) -> SpotifyServer {
        SpotifyServer { client, channel }
    }

    pub fn spawn(self) -> JoinHandle<Result<(), SpotifyError>> {
        tokio::spawn(self.run().inspect_err(|error| {
            error!("spotify thread error: {:?}", error);
        }))
    }

    pub async fn run(self) -> Result<(), SpotifyError> {
        use SpotifyCmd::*;

        loop {
            let msg = self.channel.recv()?;
            match msg {
                SetupClient { id, secret } => {
                    self.client.lock().await.setup_client(id, secret).await
                }
                StartPlayback => {
                    let _ = self.client.lock().await.start_playback().await;
                }
                SeekTrack { pos } => {
                    let _ = self.client.lock().await.seek_track(pos).await;
                }
                PausePlayback => {
                    let _ = self.client.lock().await.pause_playback().await;
                }
                PlayNextTrack => {
                    let _ = self.client.lock().await.play_next_track().await;
                }
                PlayPrevTrack => {
                    let _ = self.client.lock().await.play_prev_track().await;
                }
                GetPlaybackState { tx } => {
                    let state = self.client.lock().await.get_playback_state().await;
                    tx.send(state)?;
                }
                GetAlbumTracks {
                    tx,
                    limit,
                    offset,
                    uri,
                } => {
                    let tracks = self
                        .client
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
                    let tracks = self
                        .client
                        .lock()
                        .await
                        .get_playlist_tracks(&uri, offset, limit)
                        .await;
                    tx.send(tracks)?;
                }
                OpenAuthorizeUrl => {
                    self.client.lock().await.open_authorize_url();
                }
                UseDevice { id } => {
                    self.client.lock().await.use_device(id).await;
                }
                AuthorizeUser { tx, code } => {
                    let reply = self.client.lock().await.authorize_user(code).await;
                    tx.send(reply)?;
                }
                RefreshUserToken => {
                    let result = self.client.lock().await.refresh_user_token().await;
                    info!("refresh access token result: {:?}", result);
                }
                GetMyShows { tx, offset, limit } => {
                    let result = self.client.lock().await.get_my_shows(offset, limit).await;
                    tx.send(result)?;
                }
                GetShowEpisodes {
                    tx,
                    uri,
                    offset,
                    limit,
                } => {
                    let result = self
                        .client
                        .lock()
                        .await
                        .get_show_episodes(&uri, offset, limit)
                        .await;
                    tx.send(result)?;
                }
                GetMyArtists { tx, cursor, limit } => {
                    let artists = self.client.lock().await.get_my_artists(cursor, limit).await;
                    tx.send(artists)?;
                }
                GetMyAlbums { tx, offset, limit } => {
                    let albums = self.client.lock().await.get_my_albums(offset, limit).await;
                    tx.send(albums)?;
                }
                GetMyPlaylists { tx, offset, limit } => {
                    let playlists = self
                        .client
                        .lock()
                        .await
                        .get_my_playlists(offset, limit)
                        .await;
                    tx.send(playlists)?;
                }
                GetMyTracks { tx, offset, limit } => {
                    let tracks = self.client.lock().await.get_my_tracks(offset, limit).await;
                    tx.send(tracks)?;
                }
                PlayTracks { uris } => {
                    self.client.lock().await.play_tracks(uris).await;
                }
                PlayContext { uri, start_uri } => {
                    self.client.lock().await.play_context(uri, start_uri).await;
                }
                GetTracksFeatures { tx, uris } => {
                    let features = self.client.lock().await.get_tracks_features(uris).await;
                    tx.send(features)?;
                }
                GetMyDevices { tx } => {
                    let devices = self.client.lock().await.get_my_devices().await;
                    tx.send(devices)?;
                }
                SetVolume { value } => {
                    let _ = self.client.lock().await.set_volume(value).await;
                }
                SetShuffle { state } => {
                    let _ = self.client.lock().await.set_shuffle(state).await;
                }
                SetRepeatMode { mode } => {
                    let _ = self.client.lock().await.set_repeat_mode(mode).await;
                }
                GetAlbum { tx, uri } => {
                    let reply = self.client.lock().await.get_album(&uri).await;
                    tx.send(reply)?;
                }
                GetPlaylist { tx, uri } => {
                    let reply = self.client.lock().await.get_playlist(&uri).await;
                    tx.send(reply)?;
                }
                GetArtist { tx, uri } => {
                    let reply = self.client.lock().await.get_artist(&uri).await;
                    tx.send(reply)?;
                }
                GetShow { tx, uri } => {
                    let reply = self.client.lock().await.get_show(&uri).await;
                    tx.send(reply)?;
                }
                GetRecentTracks { tx, limit } => {
                    let reply = self.client.lock().await.get_recent_tracks(limit).await;
                    tx.send(reply)?;
                }
            }
        }
    }
}

impl Spotify {
    pub async fn new(id: String, secret: String, cache_path: PathBuf) -> Self {
        Spotify {
            client: Self::create_client(id, secret, cache_path.clone()).await,
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

    async fn use_device(&self, id: String) {
        let _ = self.client.transfer_playback(&id, false).await;
    }

    async fn play_tracks(&self, uris: Vec<String>) {
        let _ = self
            .client
            .start_playback(None, None, Some(uris), None, None)
            .await;
    }

    async fn play_context(&self, uri: String, start_uri: Option<String>) {
        let _ = self
            .client
            .start_playback(
                None,
                Some(uri),
                None,
                start_uri.and_then(offset::for_uri),
                None,
            )
            .await;
    }

    async fn get_tracks_features(&self, uris: Vec<String>) -> ClientResult<Vec<AudioFeatures>> {
        self.client.audios_features(&uris).await.map(|payload| {
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

    async fn setup_client(&mut self, id: String, secret: String) {
        self.client = Self::create_client(id, secret, self.cache_path.clone()).await;
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
                Err(ClientError::CLI("Invalid code URL".into()))
            }
        } else {
            self.client.request_user_token(&code).await
        }
    }

    pub fn open_authorize_url(&self) {
        if let Ok(url) = self.client.get_authorize_url(false) {
            webbrowser::open(&url).unwrap();
        }
    }

    async fn get_playback_state(&self) -> ClientResult<Option<CurrentlyPlaybackContext>> {
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
            Err(ClientError::CLI("Invalid show URI".into()))
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
            Err(ClientError::CLI("Invalid show URI".into()))
        }
    }

    fn get_id(uri: &str) -> Option<&str> {
        uri.split(':').last()
    }
}
