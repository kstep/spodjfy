use gtk::{
    self, ButtonExt, CssProviderExt, EntryExt, GridExt, Inhibit, LabelExt, PanedExt, SearchBarExt,
    StackExt, StackSidebarExt, WidgetExt, FrameExt,
};
use relm::Widget;
use relm_derive::{widget, Msg};
use serde_derive::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::sync::{RwLock, Arc};

#[derive(Clone, Deserialize, Serialize)]
pub struct Settings {
    client_id: String,
    client_secret: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
        }
    }
}

pub struct State {
    settings: Arc<RwLock<Settings>>,

    screen: gdk::Screen,
    style: gtk::CssProvider,
}

#[derive(Msg)]
pub enum Msg {
    SearchStart(gdk::EventKey),
    ChangeTab(Option<glib::GString>),
    Quit,
}

#[derive(Msg)]
pub enum SettingsMsg {
    Show,
    Save,
}

#[widget]
impl Widget for SettingsTab {
    fn model(settings: Arc<RwLock<Settings>>) -> Arc<RwLock<Settings>> {
        settings
    }

    fn update(&mut self, event: SettingsMsg) {
        use SettingsMsg::*;
        match event {
            Show => {
                let settings = self.model.read().unwrap();
                self.client_id_entry.set_text(&*settings.client_id);
                self.client_secret_entry.set_text(&*settings.client_secret);
            },
            Save => {
                {
                    let mut settings = self.model.write().unwrap();
                    settings.client_id = self.client_id_entry.get_text().into();
                    settings.client_secret = self.client_secret_entry.get_text().into();
                }

                directories::ProjectDirs::from("me", "kstep", "spodjfy")
                    .and_then(|dirs| {
                        std::fs::File::create(dirs.config_dir().join("settings.toml")).ok()
                    })
                    .and_then(|mut conf_file| {
                        toml::to_vec(&*self.model.read().unwrap())
                            .ok()
                            .and_then(|data| conf_file.write_all(&data).ok())
                    })
                    .expect("Error saving settings");
            }
        }
    }

    view! {
        gtk::Frame {
            label: Some("Credentials"),
            gtk::Grid {
                column_homogeneous: true,
                margin_top: 50,
                margin_bottom: 50,
                margin_start: 50,
                margin_end: 50,
                row_spacing: 5,
                column_spacing: 5,

                #[name="client_id_label"]
                gtk::Label {
                    text_with_mnemonic: "Client _ID",
                    halign: gtk::Align::End,
                    cell: {
                        left_attach: 0,
                        top_attach: 0,
                    }
                },
                #[name="client_id_entry"]
                gtk::Entry {
                    text: &*__relm_model.read().unwrap().client_id,
                    cell: {
                        left_attach: 1,
                        top_attach: 0,
                    }
                },

                #[name="client_secret_label"]
                gtk::Label {
                    text_with_mnemonic: "Client _Secret",
                    halign: gtk::Align::End,
                    cell: {
                        left_attach: 0,
                        top_attach: 1,
                    }
                },
                #[name="client_secret_entry"]
                gtk::Entry {
                    text: &*__relm_model.read().unwrap().client_secret,
                    cell: {
                        left_attach: 1,
                        top_attach: 1,
                    }
                },

                gtk::Button {
                    hexpand: false,
                    halign: gtk::Align::End,
                    label: "Save",
                    cell: {
                        left_attach: 1,
                        top_attach: 2,
                    },

                    clicked(_) => SettingsMsg::Save,
                }
            },
        }
    }

    fn init_view(&mut self) {
        self.client_id_label
            .set_mnemonic_widget(Some(&self.client_id_entry));
        self.client_secret_label
            .set_mnemonic_widget(Some(&self.client_secret_entry));
    }
}

pub struct Params {
    settings: Arc<RwLock<Settings>>,
}

impl Params {
    fn new(settings: Settings) -> Self {
        Self {
            settings: Arc::new(RwLock::new(settings)),
        }
    }
}

#[widget]
impl Widget for Win {
    fn model(params: Params) -> State {
        let style = gtk::CssProvider::new();
        let screen = gdk::Screen::get_default().unwrap();

        style
            .load_from_data(
                br#"
                window {
                    font-family: "Noto Sans";
                    font-size: 18px;
                }
                stacksidebar {
                    font-family: "Noto Color Emoji";
                }
                "#,
            )
            .expect("Invalid CSS styles");

        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &style,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        State {
            settings: params.settings,
            screen,
            style,
        }
    }

    fn update(&mut self, event: Msg) {
        use Msg::*;
        match event {
            Quit => gtk::main_quit(),
            SearchStart(ref event) => {
                self.searchbar.handle_event(event);
            },
            ChangeTab(widget_name) => {
                match widget_name.as_deref() {
                    Some("settings_tab") => {
                        self.settings_tab.emit(SettingsMsg::Show);
                    },
                    Some("albums_tab") => {
                        let settings = self.model.settings.read().unwrap();
                        let oauth = rspotify::blocking::oauth2::SpotifyOAuth::default();
                        let mut creds = rspotify::blocking::oauth2::SpotifyClientCredentials {
                            client_id: settings.client_id.clone(),
                            client_secret: settings.client_secret.clone(),
                            token_info: None,
                        };
                        let token = creds.get_access_token();
                        println!("{:?}", creds);
                        println!("{:?}", token);
                        let spotify = rspotify::blocking::client::Spotify::default()
                            .client_credentials_manager(creds)
                            .build();

                        let albums = spotify.current_user_saved_albums(100, 0);
                        println!("{:?}", albums);

                        //if let Ok(page) = rt.block_on(albums) {
                        //    println!("{:?}", page);
                        //}

                    },
                    _ => (),
                }
            }
        }
    }

    view! {
        #[name="window"]
        gtk::Window {
            gtk::Box(gtk::Orientation::Vertical, 1) {
                #[name="searchbar"]
                gtk::SearchBar {
                    gtk::Box(gtk::Orientation::Horizontal, 0) {
                        //gtk::MenuButton {},
                        #[name="searchentry"]
                        gtk::SearchEntry {
                            hexpand: true,
                        },
                    },
                },
                gtk::Paned(gtk::Orientation::Horizontal) {
                    #[name="sidebar"]
                    gtk::StackSidebar {
                        child: { shrink: false },
                        property_width_request: 300,
                        vexpand: true,
                    },
                    #[name="stack"]
                    gtk::Stack {
                        vexpand: true,
                        hexpand: true,
                        transition_type: gtk::StackTransitionType::SlideUpDown,

                        #[name="now_playing_tab"]
                        gtk::Label(Some("Now playing")) {
                           child: { title: Some("\u{25B6} Now playing") },
                        },

                        #[name="favorites_tab"]
                        gtk::Label(Some("Favorites")) {
                           child: { title: Some("\u{1F31F} Favorites") },
                        },

                        #[name="playlists_tab"]
                        gtk::Label(Some("Playlists")) {
                           child: { title: Some("\u{1F4C1} Playlists") },
                        },

                        #[name="artists_tab"]
                        gtk::Label(Some("Artists")) {
                           child: { title: Some("\u{1F935} Artists") },
                        },

                        #[name="albums_tab"]
                        gtk::Label(Some("Albums")) {
                            child: {
                                name: Some("albums_tab"),
                                title: Some("\u{1F4BF} Albums"),
                            },
                        },

                        #[name="genres_tab"]
                        gtk::Label(Some("Genres")) {
                           child: { title: Some("\u{1F3B7} Genres") },
                        },

                        #[name="tracks_tab"]
                        gtk::Label(Some("Tracks")) {
                           child: { title: Some("\u{1F3B5} Tracks") },
                        },

                        #[name="settings_tab"]
                        SettingsTab(__relm_model.settings.clone()) {
                           child: {
                               name: Some("settings_tab"),
                               title: Some("\u{2699} Settings"),
                           },
                        },

                        property_visible_child_name_notify(stack) => Msg::ChangeTab(stack.get_visible_child_name()),
                    }
                },
            },
            delete_event(_, _) => (Msg::Quit, Inhibit(false)),
            //key_press_event(_, event) => (Msg::SearchStart(event.clone()), Inhibit(false)),
        }
    }

    fn init_view(&mut self) {
        self.sidebar.set_stack(&self.stack);
        self.searchbar.connect_entry(&self.searchentry);
    }
}

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

    let params = Params::new(settings);
    Win::run(params).unwrap();
}
