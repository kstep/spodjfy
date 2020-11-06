use crate::scopes::Scope::{self, *};
use gtk::{BoxExt, DialogExt, EntryExt, GtkWindowExt, WidgetExt};
use rspotify::client::{ClientError, ClientResult, Spotify as Client};
use rspotify::model::album::SavedAlbum;
use rspotify::model::audio::AudioFeatures;
use rspotify::model::context::CurrentlyPlaybackContext;
use rspotify::model::device::Device;
use rspotify::model::page::Page;
use rspotify::model::playlist::{PlaylistTrack, SimplifiedPlaylist};
use rspotify::model::track::{SavedTrack, SimplifiedTrack};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

pub struct SpotifyProxy {
    spotify_tx: Sender<SpotifyCmd>,
    errors_stream: relm::EventStream<ClientError>,
}

impl SpotifyProxy {
    pub fn new(spotify_tx: Sender<SpotifyCmd>) -> (SpotifyProxy, relm::EventStream<ClientError>) {
        let errors_stream = relm::EventStream::new();
        (
            SpotifyProxy {
                spotify_tx,
                errors_stream: errors_stream.clone(),
            },
            errors_stream,
        )
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
    PausePlayback,
    StartPlayback,
    PlayPrevTrack,
    PlayNextTrack,
    GetAlbums {
        tx: ResultSender<Page<SavedAlbum>>,
        offset: u32,
        limit: u32,
    },
    GetPlaylists {
        tx: ResultSender<Page<SimplifiedPlaylist>>,
        offset: u32,
        limit: u32,
    },
    GetFavoriteTracks {
        tx: ResultSender<Page<SavedTrack>>,
        offset: u32,
        limit: u32,
    },
    PlayTracks {
        context_uri: Option<String>,
        uris: Vec<String>,
    },
    GetTracksFeatures {
        tx: ResultSender<Vec<AudioFeatures>>,
        uris: Vec<String>,
    },
    GetDevices {
        tx: ResultSender<Vec<Device>>,
    },
    UseDevice {
        id: String,
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

    async fn get_devices(&self) -> ClientResult<Vec<Device>> {
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
                    self.authorize_user(code).await;
                }
                GetAlbums { tx, offset, limit } => {
                    let albums = self.get_albums(offset, limit).await;
                    tx.send(albums).unwrap();
                }
                GetPlaylists { tx, offset, limit } => {
                    let playlists = self.get_playlists(offset, limit).await;
                    tx.send(playlists).unwrap();
                }
                GetFavoriteTracks { tx, offset, limit } => {
                    let tracks = self.get_favorite_tracks(offset, limit).await;
                    tx.send(tracks).unwrap();
                }
                PlayTracks { context_uri, uris } => {
                    self.play_tracks(context_uri, uris).await;
                }
                GetTracksFeatures { tx, uris } => {
                    let features = self.get_tracks_features(uris).await;
                    tx.send(features).unwrap();
                }
                GetDevices { tx } => {
                    let devices = self.get_devices().await;
                    tx.send(devices).unwrap();
                }
            }
        }
    }

    async fn use_device(&self, id: String) {
        let _ = self.client.transfer_playback(&id, true).await;
    }

    async fn play_tracks(&self, context_uri: Option<String>, uris: Vec<String>) {
        let _ = self
            .client
            .start_playback(None, context_uri, Some(uris), None, None)
            .await;
    }

    async fn get_tracks_features(&self, uris: Vec<String>) -> ClientResult<Vec<AudioFeatures>> {
        self.client.audios_features(&uris).await.map(|payload| {
            payload
                .map(|features| features.audio_features)
                .unwrap_or_else(Vec::new)
        })
    }

    async fn get_favorite_tracks(&self, offset: u32, limit: u32) -> ClientResult<Page<SavedTrack>> {
        self.client.current_user_saved_tracks(limit, offset).await
    }

    async fn get_playlists(
        &self,
        offset: u32,
        limit: u32,
    ) -> ClientResult<Page<SimplifiedPlaylist>> {
        self.client.current_user_playlists(limit, offset).await
    }

    async fn get_albums(&self, offset: u32, limit: u32) -> ClientResult<Page<SavedAlbum>> {
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

    async fn seek_track(&self, pos: u32) -> ClientResult<()> {
        self.client.seek_track(pos, None).await
    }
}
