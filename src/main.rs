use futures::join;
use relm::Widget;
use spodjfy::{Config, LoginServer, Params, Spotify, SpotifyProxy, SpotifyServer, Win};
use std::sync::Arc;
use tokio::sync::RwLock;

fn main() {
    env_logger::init();

    let config = Config::new();
    let settings = config.load_settings();
    let spotify_cache_path = config.spotify_token_file();

    let (client_id, client_secret) = (settings.client_id.clone(), settings.client_secret.clone());

    let rt = tokio::runtime::Builder::new()
        .threaded_scheduler()
        .enable_all()
        .build()
        .unwrap();

    let (spotify, rx, spotify_errors) = SpotifyProxy::new(&rt);

    rt.spawn(async {
        let client = Arc::new(RwLock::new(
            Spotify::new(client_id, client_secret, spotify_cache_path).await,
        ));

        let _ = join! {
            LoginServer::new(client.clone()).spawn(),
            SpotifyServer::new(client, rx).spawn(),
        };
    });

    Win::run(Params {
        settings,
        spotify,
        spotify_errors,
    })
    .unwrap();
}
