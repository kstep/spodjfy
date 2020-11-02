use relm::Widget;
use spodjfy::components::spotify::{Spotify, SpotifyCmd};
use spodjfy::components::win::{Params, Settings, Win};
use std::io::Read;
use std::sync::{Arc, RwLock};

#[tokio::main]
async fn main() {
    let (tx, rx) = std::sync::mpsc::channel::<SpotifyCmd>();

    let settings: Settings = directories::ProjectDirs::from("me", "kstep", "spodjfy")
        .and_then(|dirs| std::fs::File::open(dirs.config_dir().join("settings.toml")).ok())
        .and_then(|mut file| {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).ok().map(|_| buf)
        })
        .and_then(|data| toml::from_slice(&data).ok())
        .unwrap_or_default();

    std::thread::spawn(move || {
        let mut rt = tokio::runtime::Builder::new()
            .threaded_scheduler()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(Spotify::new().run(rx));
    });

    tx.send(SpotifyCmd::SetupClient {
        id: settings.client_id.clone(),
        secret: settings.client_secret.clone(),
        force: true,
    })
    .unwrap();

    Win::run(Params {
        settings: Arc::new(RwLock::new(settings)),
        spotify_tx: tx,
    })
    .unwrap();
}
