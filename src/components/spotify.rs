use crate::scopes::Scope::{self, *};
use gtk::{BoxExt, DialogExt, EntryExt, GtkWindowExt, WidgetExt};
use rspotify::client::Spotify as Client;
use rspotify::model::album::SavedAlbum;
use rspotify::model::audio::AudioFeatures;
use rspotify::model::device::Device;
use rspotify::model::page::Page;
use rspotify::model::playlist::SimplifiedPlaylist;
use rspotify::model::track::SavedTrack;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

pub struct SpotifyProxy(Sender<SpotifyCmd>);

impl SpotifyProxy {
    pub fn new(tx: Sender<SpotifyCmd>) -> SpotifyProxy {
        SpotifyProxy(tx)
    }

    pub fn tell(&self, cmd: SpotifyCmd) {
        self.0.send(cmd).unwrap();
    }
    pub fn ask<T, F, R, M>(&self, stream: relm::EventStream<M>, make_command: F, convert_output: R)
    where
        F: FnOnce(relm::Sender<Option<T>>) -> SpotifyCmd + 'static,
        R: Fn(T) -> M + 'static,
        M: 'static,
    {
        let (_, tx) = relm::Channel::<Option<T>>::new(move |reply| {
            if let Some(out) = reply {
                stream.emit(convert_output(out));
            }
        });
        self.0.send(make_command(tx)).unwrap();
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

pub enum SpotifyCmd {
    SetupClient {
        id: String,
        secret: String,
        force: bool,
    },
    OpenAuthorizeUrl,
    AuthorizeUser {
        code: String,
    },
    GetAlbums {
        tx: relm::Sender<Option<Page<SavedAlbum>>>,
        offset: u32,
        limit: u32,
    },
    GetPlaylists {
        tx: relm::Sender<Option<Page<SimplifiedPlaylist>>>,
        offset: u32,
        limit: u32,
    },
    GetFavoriteTracks {
        tx: relm::Sender<Option<Page<SavedTrack>>>,
        offset: u32,
        limit: u32,
    },
    PlayTracks {
        uris: Vec<String>,
    },
    GetTracksFeatures {
        tx: relm::Sender<Option<Vec<AudioFeatures>>>,
        uris: Vec<String>,
    },
    GetDevices {
        tx: relm::Sender<Option<Vec<Device>>>,
    },
}

pub struct Spotify {
    cache_path: PathBuf,
    client: Option<Client>,
}

impl Spotify {
    pub fn new(cache_path: PathBuf) -> Self {
        Spotify {
            client: None,
            cache_path,
        }
    }

    async fn get_devices(&self) -> Option<Vec<Device>> {
        if let Some(ref client) = self.client {
            client.device().await.ok().map(|reply| reply.devices)
        } else {
            None
        }
    }

    pub async fn run(&mut self, channel: Receiver<SpotifyCmd>) {
        use SpotifyCmd::*;
        while let Ok(msg) = channel.recv() {
            match msg {
                SetupClient { id, secret, force } => self.setup_client(id, secret, force).await,
                OpenAuthorizeUrl => {
                    self.open_authorize_url();
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
                PlayTracks { uris } => {
                    self.play_tracks(uris).await;
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

    async fn play_tracks(&self, uris: Vec<String>) {
        if let Some(ref client) = self.client {
            let _ = client
                .start_playback(None, None, Some(uris), None, None)
                .await;
        }
    }

    async fn get_tracks_features(&self, uris: Vec<String>) -> Option<Vec<AudioFeatures>> {
        if let Some(ref client) = self.client {
            client
                .audios_features(&uris)
                .await
                .ok()
                .and_then(|payload| payload.map(|features| features.audio_features))
        } else {
            None
        }
    }

    async fn get_favorite_tracks(&self, offset: u32, limit: u32) -> Option<Page<SavedTrack>> {
        if let Some(ref client) = self.client {
            client.current_user_saved_tracks(limit, offset).await.ok()
        } else {
            None
        }
    }

    async fn get_playlists(&self, offset: u32, limit: u32) -> Option<Page<SimplifiedPlaylist>> {
        if let Some(ref client) = self.client {
            client.current_user_playlists(limit, offset).await.ok()
        } else {
            None
        }
    }

    async fn get_albums(&self, offset: u32, limit: u32) -> Option<Page<SavedAlbum>> {
        if let Some(ref client) = self.client {
            client.current_user_saved_albums(limit, offset).await.ok()
        } else {
            None
        }
    }

    async fn setup_client(&mut self, id: String, secret: String, force: bool) {
        if !force && self.client.is_some() {
            return;
        }

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
            .id(&id)
            .secret(&secret)
            .build()
            .unwrap();

        let mut client: rspotify::client::Spotify = rspotify::client::SpotifyBuilder::default()
            .oauth(oauth)
            .credentials(creds)
            .cache_path(self.cache_path.clone())
            .build()
            .unwrap();

        client.token = client.read_token_cache().await;

        self.client.replace(client);
    }

    async fn authorize_user(&mut self, code: String) -> bool {
        if let Some(ref mut client) = self.client {
            if code.starts_with("http") {
                if let Some(code) = client.parse_response_code(&code) {
                    client.request_user_token(&code).await.is_ok()
                } else {
                    false
                }
            } else {
                client.request_user_token(&code).await.is_ok()
            }
        } else {
            false
        }
    }

    pub fn open_authorize_url(&self) {
        if let Some(ref client) = self.client {
            if let Ok(url) = client.get_authorize_url(false) {
                webbrowser::open(&url).unwrap();
            }
        }
    }
}
