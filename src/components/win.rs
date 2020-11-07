use gtk::{
    self, CssProviderExt, GtkWindowExt, Inhibit, OverlayExt, PanedExt, SearchBarExt, StackExt,
    StackSidebarExt, WidgetExt,
};
use relm::{Relm, Widget};
use relm_derive::{widget, Msg};
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;

use crate::components::notifier::{Notifier, NotifierMsg};
use crate::components::spotify::{SpotifyCmd, SpotifyProxy};
use crate::components::tabs::albums::{AlbumsMsg, AlbumsTab};
use crate::components::tabs::artists::{ArtistsMsg, ArtistsTab};
use crate::components::tabs::devices::{DevicesMsg, DevicesTab};
use crate::components::tabs::favorites::{FavoritesMsg, FavoritesTab};
use crate::components::tabs::now_playing::{NowPlayingMsg, NowPlayingTab};
use crate::components::tabs::playlists::{PlaylistsMsg, PlaylistsTab};
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
    pub settings: Settings,
    pub spotify: Arc<SpotifyProxy>,
    pub spotify_errors: relm::EventStream<rspotify::client::ClientError>,

    pub screen: gdk::Screen,
    pub style: gtk::CssProvider,
    pub stream: relm::EventStream<Msg>,
    pub notifier: relm::Component<Notifier>,
}

#[derive(Msg)]
pub enum Msg {
    SearchStart(gdk::EventKey),
    ChangeTab(Option<glib::GString>),
    GoToSettings,
    Quit,
}

pub struct Params {
    pub settings: Settings,
    pub spotify: SpotifyProxy,
    pub spotify_errors: relm::EventStream<rspotify::client::ClientError>,
}

#[widget]
impl Widget for Win {
    fn model(relm: &Relm<Self>, params: Params) -> State {
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

                #now_playing_tab button.link#track_name_label,
                #now_playing_tab label#context_name_label {
                    padding: 0;
                    font-weight: bold;
                }
                #now_playing_tab button.link#track_name_label {
                    font-size: 32px;
                }
                #now_playing_tab label#context_name_label {
                    font-size: 24px;
                }

                #now_playing_tab label#track_album_label,
                #now_playing_tab label#context_genres_label {
                    font-style: italic;
                }
                "#,
            )
            .expect("Invalid CSS styles");

        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &style,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let stream = relm.stream().clone();
        State {
            settings: params.settings,
            spotify: Arc::new(params.spotify),
            spotify_errors: params.spotify_errors,
            notifier: relm::create_component::<Notifier>(()),
            screen,
            style,
            stream,
        }
    }

    fn update(&mut self, event: Msg) {
        use Msg::*;
        match event {
            Quit => gtk::main_quit(),
            SearchStart(ref event) => {
                self.searchbar.handle_event(event);
            }
            GoToSettings => {
                self.stack.set_visible_child(self.settings_tab.widget());
            }
            ChangeTab(widget_name) => match widget_name.as_deref() {
                Some("now_playing_tab") => {
                    self.now_playing_tab.emit(NowPlayingMsg::ShowTab);
                }
                Some("settings_tab") => {
                    self.settings_tab.emit(SettingsMsg::ShowTab);
                }
                Some("albums_tab") => {
                    self.albums_tab.emit(AlbumsMsg::ShowTab);
                }
                Some("artists_tab") => {
                    self.artists_tab.emit(ArtistsMsg::ShowTab);
                }
                Some("playlists_tab") => {
                    self.playlists_tab.emit(PlaylistsMsg::ShowTab);
                }
                Some("devices_tab") => {
                    self.devices_tab.emit(DevicesMsg::ShowTab);
                }
                Some("favorites_tab") => {
                    self.favorites_tab.emit(FavoritesMsg::ShowTab);
                }
                _ => {}
            },
        }
    }

    view! {
        #[name="window"]
        gtk::Window {
            icon_name: Some("multimedia-player"),

            #[name="overlay"]
            gtk::Overlay {
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
                            NowPlayingTab(self.model.spotify.clone()) {
                               widget_name: "now_playing_tab",
                               child: {
                                   name: Some("now_playing_tab"),
                                   title: Some("\u{25B6} Now playing")
                               },
                            },

                            #[name="favorites_tab"]
                            FavoritesTab(self.model.spotify.clone()) {
                                widget_name: "favorites_tab",
                                child: {
                                    name: Some("favorites_tab"),
                                    title: Some("\u{1F31F} Favorites"),
                                }
                            },

                            #[name="playlists_tab"]
                            PlaylistsTab(self.model.spotify.clone()) {
                                widget_name: "playlists_tab",
                                child: {
                                    name: Some("playlists_tab"),
                                    title: Some("\u{1F4C1} Playlists"),
                                }
                            },

                            #[name="artists_tab"]
                            ArtistsTab(self.model.spotify.clone()) {
                                widget_name: "artists_tab",
                                child: {
                                    name: Some("artists_tab"),
                                    title: Some("\u{1F935} Artists"),
                                }
                            },

                            #[name="albums_tab"]
                            AlbumsTab(self.model.spotify.clone()) {
                                widget_name: "albums_tab",
                                child: {
                                    name: Some("albums_tab"),
                                    title: Some("\u{1F4BF} Albums"),
                                }
                            },

                            #[name="devices_tab"]
                            DevicesTab(self.model.spotify.clone()) {
                                widget_name: "devices_tab",
                                child: {
                                    name: Some("devices_tab"),
                                    title: Some("\u{1F39B} Devices"),
                                },
                            },

                            #[name="settings_tab"]
                            SettingsTab((self.model.settings.clone(), self.model.spotify.clone())) {
                                widget_name: "settings_tab",
                                child: {
                                    name: Some("settings_tab"),
                                    title: Some("\u{2699} Settings"),
                                },
                            },

                            property_visible_child_name_notify(stack) => Msg::ChangeTab(stack.get_visible_child_name()),
                        }
                    },

                },
            },

            delete_event(_, _) => (Msg::Quit, Inhibit(false)),
            //key_press_event(_, event) => (Msg::SearchStart(event.clone()), Inhibit(false)),
        }
    }

    fn init_view(&mut self) {
        self.sidebar.set_stack(&self.stack);
        self.searchbar.connect_entry(&self.searchentry);
        self.overlay.add_overlay(self.model.notifier.widget());

        let stream = self.model.stream.clone();
        let notifier = self.model.notifier.stream().clone();
        let spotify = self.model.spotify.clone();
        self.model.spotify_errors.observe(move |err| {
            use rspotify::client::ClientError::*;
            match err {
                InvalidAuth(msg) => {
                    notifier.emit(NotifierMsg::Notify {
                        title: "Error!".to_owned(),
                        body: format!("Authentication error: {}. Check credentials in <Settings> and click <Authorize> to fix", msg),
                        timeout_ms: 5000,
                    });
                    spotify.tell(SpotifyCmd::RefreshUserToken);
                    stream.emit(Msg::GoToSettings);
                },
                Unauthorized => {
                    notifier.emit(NotifierMsg::Notify {
                        title: "Error!".to_owned(),
                        body: "Authorization error. Check credentials in <Settings> and click <Authorize> to fix".to_owned(),
                        timeout_ms: 5000
                    });
                    stream.emit(Msg::GoToSettings);
                },
                _ => (),
                /*
                err => notifier.emit(NotifierMsg::Notify {
                    title: "Error!".to_owned(),
                    body: err.to_string(),
                    timeout_ms: 5000
                }),
                 */
            }
        });

        self.now_playing_tab.emit(NowPlayingMsg::ShowTab);
    }
}
