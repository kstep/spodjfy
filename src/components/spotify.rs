use rspotify::client::Spotify as Client;
use rspotify::model::album::SavedAlbum;
use rspotify::model::page::Page;
use rspotify::model::playlist::SimplifiedPlaylist;
use std::sync::mpsc::Receiver;
use tokio::sync::oneshot::Sender;

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
            }
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
            .scope(
                r#"user-follow-read user-read-recently-played user-read-playback-state
               user-read-playback-position user-top-read user-library-read
               user-modify-playback-state user-read-currently-playing
               playlist-read-private playlist-read-collaborative"#,
            )
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
