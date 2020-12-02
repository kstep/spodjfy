use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Error, ErrorKind, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

const SETTINGS_FILE: &str = "settings.toml";
const SPOTIFY_TOKEN_FILE: &str = "token.json";
const THUMB_CACHE_DIR: &str = "thumbs";

#[derive(Clone, Deserialize, Serialize)]
pub struct Settings {
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub show_notifications: bool,
}

pub type SettingsRef = Arc<RwLock<Settings>>;

impl Default for Settings {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            show_notifications: true,
        }
    }
}

pub struct Config {
    dirs: ProjectDirs,
}

impl Default for Config {
    fn default() -> Config {
        Config::new()
    }
}

impl Config {
    pub fn new() -> Config {
        let dirs = ProjectDirs::from("me", "kstep", "spodjfy").unwrap();
        if !dirs.config_dir().exists() {
            std::fs::create_dir_all(dirs.config_dir()).unwrap();
        }
        if !dirs.cache_dir().exists() {
            std::fs::create_dir_all(dirs.cache_dir()).unwrap();
        }
        Config { dirs }
    }

    pub fn config_file(&self) -> PathBuf {
        self.dirs.config_dir().join(SETTINGS_FILE)
    }

    pub fn spotify_token_file(&self) -> PathBuf {
        self.dirs.cache_dir().join(SPOTIFY_TOKEN_FILE)
    }

    pub fn load_settings(&self) -> Settings {
        self.try_load_settings()
            .map_err(|error| {
                error!("failed to read settings file: {:?}", error);
            })
            .unwrap_or_default()
    }

    fn try_load_settings(&self) -> Result<Settings, Error> {
        let mut file = File::open(self.config_file())?;
        let mut buf = Vec::with_capacity(256);
        file.read_to_end(&mut buf)?;
        toml::from_slice(&buf).map_err(|error| Error::new(ErrorKind::InvalidData, error))
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), Error> {
        let data = toml::to_vec(settings)
            .map_err(|error| std::io::Error::new(ErrorKind::InvalidData, error))?;
        let mut file = File::create(self.config_file())?;
        file.write_all(&data)?;
        file.flush()?;
        Ok(())
    }

    pub fn thumb_cache_dir(&self) -> PathBuf {
        let dir = self.dirs.cache_dir().join(THUMB_CACHE_DIR);
        if !dir.exists() {
            std::fs::create_dir_all(&dir).unwrap();
        }
        dir
    }
}
