use futures::join;
use relm::Widget;
use spodjfy::components::win::{Params, Settings, Win};
use spodjfy::servers::login::LoginServer;
use spodjfy::servers::spotify::{Spotify, SpotifyCmd, SpotifyProxy, SpotifyServer};
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    env_logger::init();

    let (tx, rx) = std::sync::mpsc::channel::<SpotifyCmd>();

    let dirs = directories::ProjectDirs::from("me", "kstep", "spodjfy");

    let settings: Settings = dirs
        .as_ref()
        .and_then(|dirs| std::fs::File::open(dirs.config_dir().join("settings.toml")).ok())
        .and_then(|mut file| {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).ok().map(|_| buf)
        })
        .and_then(|data| toml::from_slice(&data).ok())
        .unwrap_or_default();

    let spotify_cache_path = dirs
        .as_ref()
        .map(|dirs| dirs.cache_dir().join("token.json"))
        .unwrap_or_else(|| PathBuf::from(rspotify::client::DEFAULT_CACHE_PATH));

    let (client_id, client_secret) = (settings.client_id.clone(), settings.client_secret.clone());

    std::thread::spawn(move || {
        let mut rt = tokio::runtime::Builder::new()
            .threaded_scheduler()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let client = Arc::new(Mutex::new(
                Spotify::new(client_id, client_secret, spotify_cache_path).await,
            ));

            let _ = join! {
                LoginServer::new(client.clone()).spawn(),
                SpotifyServer::new(client, rx).spawn(),
            };
        });
    });

    let (spotify, spotify_errors) = SpotifyProxy::new(tx);
    Win::run(Params {
        settings,
        spotify,
        spotify_errors,
    })
    .unwrap();
}
