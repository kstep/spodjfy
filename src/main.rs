use relm::Widget;
use spodjfy::{Config, LoginService, Params, RefreshTokenService, Spotify, Win};
use std::sync::Arc;
use tokio::sync::RwLock;

fn main() {
    env_logger::init();

    let config = Config::new();
    let settings = config.load_settings();
    let spotify_cache_path = config.spotify_token_file();

    let (client_id, client_secret) = (settings.client_id.clone(), settings.client_secret.clone());

    let mut runtime = tokio::runtime::Builder::new()
        .threaded_scheduler()
        .max_threads(100)
        .enable_all()
        .build()
        .unwrap();

    let mut spotify = Spotify::new(client_id, client_secret, spotify_cache_path);

    runtime.block_on(async {
        spotify.load_token_from_cache().await;
    });

    let spotify = Arc::new(RwLock::new(spotify));

    LoginService::new(spotify.clone()).spawn(&runtime);
    RefreshTokenService::new(spotify.clone()).spawn(&runtime);

    let pool = runtime.handle().clone();

    Win::run(Params { pool, settings, spotify }).unwrap();
}
