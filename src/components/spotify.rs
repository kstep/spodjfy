use crate::scopes::Scope::{self, *};
use relm::Sender;
use rspotify::client::Spotify as Client;
use rspotify::model::album::SavedAlbum;
use rspotify::model::audio::AudioFeatures;
use rspotify::model::page::Page;
use rspotify::model::playlist::SimplifiedPlaylist;
use rspotify::model::track::SavedTrack;
use std::sync::mpsc::Receiver;

pub enum SpotifyCmd {
    SetupClient {
        id: String,
        secret: String,
        force: bool,
    },
    GetAlbums {
        tx: Sender<Option<Page<SavedAlbum>>>,
        offset: u32,
        limit: u32,
    },
    GetPlaylists {
        tx: Sender<Option<Page<SimplifiedPlaylist>>>,
        offset: u32,
        limit: u32,
    },
    GetFavoriteTracks {
        tx: Sender<Option<Page<SavedTrack>>>,
        offset: u32,
        limit: u32,
    },
    PlayTracks {
        uris: Vec<String>,
    },
    GetTracksFeatures {
        tx: Sender<Option<Vec<AudioFeatures>>>,
        uris: Vec<String>,
    },
}

pub struct Spotify {
    client: Option<Client>,
}

impl Spotify {
    pub fn new() -> Self {
        Spotify { client: None }
    }

    pub async fn run(&mut self, channel: Receiver<SpotifyCmd>) {
        use SpotifyCmd::*;
        while let Ok(msg) = channel.recv() {
            match msg {
                SetupClient { id, secret, force } => self.setup_client(id, secret, force).await,
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
            .build()
            .unwrap();

        println!("{:?}", client);
        client.prompt_for_user_token().await.unwrap();
        println!("{:?}", client.current_user().await);

        self.client.replace(client);
    }
}
