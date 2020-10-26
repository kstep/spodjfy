use gtk::{
    self, CssProviderExt, Inhibit, PanedExt, SearchBarExt, StackExt, StackSidebarExt, WidgetExt,
};
use relm::Widget;
use relm_derive::{widget, Msg};
use serde_derive::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use crate::components::tabs::settings::{SettingsMsg, SettingsTab};

#[derive(Clone, Deserialize, Serialize)]
pub struct Settings {
    pub client_id: String,
    pub client_secret: String,
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
    pub settings: Arc<RwLock<Settings>>,

    pub screen: gdk::Screen,
    pub style: gtk::CssProvider,
}

#[derive(Msg)]
pub enum Msg {
    SearchStart(gdk::EventKey),
    ChangeTab(Option<glib::GString>),
    Quit,
}

#[widget]
impl Widget for Win {
    fn model(settings: Arc<RwLock<Settings>>) -> State {
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
            settings,
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
            }
            ChangeTab(widget_name) => {
                match widget_name.as_deref() {
                    Some("settings_tab") => {
                        self.settings_tab.emit(SettingsMsg::Show);
                    }
                    Some("albums_tab") => {
                        let settings = self.model.settings.read().unwrap();
                        let oauth = rspotify::blocking::oauth2::SpotifyOAuth::default();
                        let creds = rspotify::blocking::oauth2::SpotifyClientCredentials {
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
                    }
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
