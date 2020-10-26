use relm::Widget;
use spodjfy::components::win::{Settings, Win};
use std::io::Read;
use std::sync::{Arc, RwLock};

fn main() {
    let settings: Settings = directories::ProjectDirs::from("me", "kstep", "spodjfy")
        .and_then(|dirs| std::fs::File::open(dirs.config_dir().join("settings.toml")).ok())
        .and_then(|mut file| {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).ok().map(|_| buf)
        })
        .and_then(|data| toml::from_slice(&data).ok())
        .unwrap_or_default();

    /*
    let mut oauth = rspotify::blocking::oauth2::SpotifyOAuth::default()
        .client_id(&settings.client_id)
        .client_secret(&settings.client_secret)
        .redirect_uri("http://localhost:8888/callback")
        .build();

    rspotify::blocking::util::request_token(&mut oauth);
    let creds = rspotify::blocking::oauth2::SpotifyClientCredentials {
        client_id: settings.client_id.clone(),
        client_secret: settings.client_secret.clone(),
        token_info: {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            input = input.trim_end().to_owned();
            rspotify::blocking::util::process_token(&mut oauth, &mut input)
        }
    };
    let client = rspotify::blocking::client::Spotify::default()
        .client_credentials_manager(creds)
        .build();
    println!("{:?}", client.current_user_saved_albums(100, 0));
    */

    Win::run(Arc::new(RwLock::new(settings))).unwrap();
}
