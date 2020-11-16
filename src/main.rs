use futures::join;
use relm::Widget;
use spodjfy::components::win::{Params, Win};
use spodjfy::config::Config;
use spodjfy::servers::login::LoginServer;
use spodjfy::servers::spotify::{Spotify, SpotifyCmd, SpotifyProxy, SpotifyServer};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    env_logger::init();

    let (tx, rx) = std::sync::mpsc::channel::<SpotifyCmd>();

    let config = Config::new();
    let settings = config.read_settings();
    let spotify_cache_path = config.spotify_token_file();

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
