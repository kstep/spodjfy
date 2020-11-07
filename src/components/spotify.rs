use crate::scopes::Scope::{self, *};
use gtk::{BoxExt, DialogExt, EntryExt, GtkWindowExt, WidgetExt};
use rspotify::client::{ClientError, ClientResult, Spotify as Client};
use rspotify::model::album::SavedAlbum;
use rspotify::model::artist::FullArtist;
use rspotify::model::audio::AudioFeatures;
use rspotify::model::context::CurrentlyPlaybackContext;
use rspotify::model::device::Device;
use rspotify::model::offset;
use rspotify::model::page::{CursorBasedPage, Page};
use rspotify::model::playlist::{PlaylistTrack, SimplifiedPlaylist};
use rspotify::model::track::{SavedTrack, SimplifiedTrack};
use rspotify::senum::RepeatState;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

const DEFAULT_REFRESH_TOKEN_TIMEOUT: u64 = 20 * 60;

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
            Err(err) => errors_stream.emit(err),
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

        return url;
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
        code: String,
    },
    RefreshUserToken,
    PausePlayback,
    StartPlayback,
    PlayPrevTrack,
    PlayNextTrack,
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
}

pub struct Spotify {
    cache_path: PathBuf,
    client: Client,
}

impl Spotify {
    pub async fn new(id: String, secret: String, cache_path: PathBuf) -> Self {
        Spotify {
            client: Self::create_client(id, secret, cache_path.clone()).await,
            cache_path,
        }
    }

    async fn get_my_devices(&self) -> ClientResult<Vec<Device>> {
        self.client.device().await.map(|reply| reply.devices)
    }

    pub async fn run(&mut self, channel: Receiver<SpotifyCmd>) {
        use SpotifyCmd::*;

        while let Ok(msg) = channel.recv() {
            match msg {
                SetupClient { id, secret } => self.setup_client(id, secret).await,
                StartPlayback => {
                    let _ = self.start_playback().await;
                }
                SeekTrack { pos } => {
                    let _ = self.seek_track(pos).await;
                }
                PausePlayback => {
                    let _ = self.pause_playback().await;
                }
                PlayNextTrack => {
                    let _ = self.play_next_track().await;
                }
                PlayPrevTrack => {
                    let _ = self.play_prev_track().await;
                }
                GetPlaybackState { tx } => {
                    let state = self.get_playback_state().await;
                    tx.send(state).unwrap();
                }
                GetAlbumTracks {
                    tx,
                    limit,
                    offset,
                    uri,
                } => {
                    let tracks = self.get_album_tracks(&uri, offset, limit).await;
                    tx.send(tracks).unwrap();
                }
                GetPlaylistTracks {
                    tx,
                    limit,
                    offset,
                    uri,
                } => {
                    let tracks = self.get_playlist_tracks(&uri, offset, limit).await;
                    tx.send(tracks).unwrap();
                }
                OpenAuthorizeUrl => {
                    self.open_authorize_url();
                }
                UseDevice { id } => {
                    self.use_device(id).await;
                }
                AuthorizeUser { code } => {
                    let _ = self.authorize_user(code).await;
                }
                RefreshUserToken => {
                    let result = self.refresh_user_token().await;
                    info!("refresh access token result: {:?}", result);
                }
                GetMyArtists { tx, cursor, limit } => {
                    let artists = self.get_my_artists(cursor, limit).await;
                    tx.send(artists).unwrap();
                }
                GetMyAlbums { tx, offset, limit } => {
                    let albums = self.get_my_albums(offset, limit).await;
                    tx.send(albums).unwrap();
                }
                GetMyPlaylists { tx, offset, limit } => {
                    let playlists = self.get_my_playlists(offset, limit).await;
                    tx.send(playlists).unwrap();
                }
                GetMyTracks { tx, offset, limit } => {
                    let tracks = self.get_my_tracks(offset, limit).await;
                    tx.send(tracks).unwrap();
                }
                PlayTracks { uris } => {
                    self.play_tracks(uris).await;
                }
                PlayContext { uri, start_uri } => {
                    self.play_context(uri, start_uri).await;
                }
                GetTracksFeatures { tx, uris } => {
                    let features = self.get_tracks_features(uris).await;
                    tx.send(features).unwrap();
                }
                GetMyDevices { tx } => {
                    let devices = self.get_my_devices().await;
                    tx.send(devices).unwrap();
                }
                SetVolume { value } => {
                    let _ = self.set_volume(value).await;
                }
                SetShuffle { state } => {
                    let _ = self.set_shuffle(state).await;
                }
                SetRepeatMode { mode } => {
                    let _ = self.set_repeat_mode(mode).await;
                }
            }
        }
    }

    async fn use_device(&self, id: String) {
        let _ = self.client.transfer_playback(&id, true).await;
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
            .scope(Scope::to_string(&[
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

    async fn authorize_user(&mut self, code: String) -> ClientResult<()> {
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
        self.client.current_playback(None, None).await
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
}
