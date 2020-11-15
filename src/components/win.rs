use gtk::{
    self, CssProviderExt, GtkWindowExt, Inhibit, OverlayExt, PanedExt, SearchBarExt, SettingsExt,
    StackExt, StackSidebarExt, WidgetExt,
};
use relm::{Relm, Widget};
use relm_derive::{widget, Msg};
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;

use crate::components::media_controls::{MediaControls, MediaControlsMsg};
use crate::components::notifier::{Notifier, NotifierMsg};
use crate::components::tabs::albums::{AlbumsMsg, AlbumsTab};
use crate::components::tabs::artists::{ArtistsMsg, ArtistsTab};
use crate::components::tabs::categories::{CategoriesMsg, CategoriesTab};
use crate::components::tabs::devices::{DevicesMsg, DevicesTab};
use crate::components::tabs::favorites::{FavoritesMsg, FavoritesTab};
use crate::components::tabs::featured::{FeaturedMsg, FeaturedTab};
use crate::components::tabs::new_releases::{NewReleasesMsg, NewReleasesTab};
use crate::components::tabs::playlists::{PlaylistsMsg, PlaylistsTab};
use crate::components::tabs::queue::{QueueMsg, QueueTab};
use crate::components::tabs::recent::{RecentMsg, RecentTab};
use crate::components::tabs::search::{SearchMsg, SearchTab};
use crate::components::tabs::settings::{SettingsMsg, SettingsTab};
use crate::components::tabs::shows::{ShowsMsg, ShowsTab};
use crate::components::tabs::top_artists::{TopArtistsMsg, TopArtistsTab};
use crate::components::tabs::top_tracks::{TopTracksMsg, TopTracksTab};
use crate::servers::spotify::{SpotifyCmd, SpotifyProxy};
use rspotify::model::Type;

#[derive(Clone, Deserialize, Serialize)]
pub struct Settings {
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub show_notifications: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            show_notifications: true,
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

        if let Some(settings) =
            gtk::Settings::get_for_screen(&screen).or_else(gtk::Settings::get_default)
        {
            settings.set_property_gtk_error_bell(false);
        }

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

                #media_controls button.link#track_name_label,
                #media_controls label#context_name_label {
                    padding: 0;
                    font-weight: bold;
                }
                #media_controls button.link#track_name_label {
                    font-size: 32px;
                }
                media_controls label#context_name_label {
                    font-size: 24px;
                }

                #media_controls label#track_album_label,
                #media_controls label#context_genres_label {
                    font-style: italic;
                }

                #media_controls buttonbox button {
                    min-width: 30px;
                    min-height: 30px;
                }
                #media_controls buttonbox button#play_btn {
                    border-radius: 15px;
                    min-height: 50px;
                    min-width: 80px;
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
                Some("recent_tab") => {
                    self.recent_tab.emit(RecentMsg::ShowTab);
                }
                Some("queue_tab") => {
                    self.queue_tab.emit(QueueMsg::ShowTab);
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
                Some("shows_tab") => {
                    self.shows_tab.emit(ShowsMsg::ShowTab);
                }
                Some("new_releases_tab") => {
                    self.new_releases_tab.emit(NewReleasesMsg::ShowTab);
                }
                Some("featured_tab") => {
                    self.featured_tab.emit(FeaturedMsg::ShowTab);
                }
                Some("categories_tab") => {
                    self.categories_tab.emit(CategoriesMsg::ShowTab);
                }
                Some("top_tracks_tab") => {
                    self.top_tracks_tab.emit(TopTracksMsg::ShowTab);
                }
                Some("top_artists_tab") => {
                    self.top_artists_tab.emit(TopArtistsMsg::ShowTab);
                }
                Some("search_tab") => {
                    self.search_tab.emit(SearchMsg::ShowTab);
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
                        gtk::Box(gtk::Orientation::Vertical, 1) {
                            #[name="media_controls"]
                            MediaControls((self.model.spotify.clone(), self.model.settings.show_notifications)) {
                               widget_name: "media_controls",
                            },

                            #[name="stack"]
                            gtk::Stack {
                                vexpand: true,
                                hexpand: true,
                                transition_type: gtk::StackTransitionType::SlideUpDown,

                                #[name="search_tab"]
                                SearchTab(self.model.spotify.clone()) {
                                //gtk::Label {
                                    widget_name: "search_tab",
                                    child: {
                                        name: Some("search_tab"),
                                        title: Some("\u{1F50D} Search")
                                    },
                                    //text: "Search"
                                },

                                #[name="recent_tab"]
                                RecentTab(self.model.spotify.clone()) {
                                    widget_name: "recent_tab",
                                    child: {
                                        name: Some("recent_tab"),
                                        title: Some("\u{23F3} Recently played"),
                                    }
                                },

                                #[name="queue_tab"]
                                QueueTab(self.model.spotify.clone()) {
                                    widget_name: "queue_tab",
                                    child: {
                                        name: Some("queue_tab"),
                                        title: Some("\u{1F3B5} Queue"),
                                    }
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

                                #[name="shows_tab"]
                                ShowsTab(self.model.spotify.clone()) {
                                    widget_name: "shows_tab",
                                    child: {
                                        name: Some("shows_tab"),
                                        title: Some("\u{1F399} Shows"),
                                    }
                                },

                                #[name="top_tracks_tab"]
                                TopTracksTab(self.model.spotify.clone()) {
                                    widget_name: "top_tracks_tab",
                                    child: {
                                        name: Some("top_tracks_tab"),
                                        title: Some("\u{1F3C5} Top tracks")
                                    }
                                },

                                #[name="top_artists_tab"]
                                TopArtistsTab(self.model.spotify.clone()) {
                                    widget_name: "top_artists_tab",
                                    child: {
                                        name: Some("top_artists_tab"),
                                        title: Some("\u{1F3C5} Top artists")
                                    }
                                },

                                #[name="categories_tab"]
                                CategoriesTab(self.model.spotify.clone()) {
                                    widget_name: "categories_tab",
                                    child: {
                                        name: Some("categories_tab"),
                                        title: Some("\u{1F4D2} Categories")
                                    }
                                },

                                #[name="featured_tab"]
                                FeaturedTab(self.model.spotify.clone()) {
                                    widget_name: "featured_tab",
                                    child: {
                                        name: Some("featured_tab"),
                                        title: Some("\u{1F525} Featured")
                                    }
                                },

                                #[name="new_releases_tab"]
                                NewReleasesTab(self.model.spotify.clone()) {
                                    widget_name: "new_releases_tab",
                                    child: {
                                        name: Some("new_releases_tab"),
                                        title: Some("\u{1F4C5} New releases")
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
                            },
                        },
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
                        body: format!("Authentication error: {}. Check credentials in <Settings> and click <Open authorization URL> to fix", msg),
                        timeout_ms: 5000,
                    });
                    spotify.tell(SpotifyCmd::RefreshUserToken).unwrap();
                    stream.emit(Msg::GoToSettings);
                },
                Unauthorized => {
                    notifier.emit(NotifierMsg::Notify {
                        title: "<b>Error!</b>".to_owned(),
                        body: "Authorization error. Check credentials in <Settings> and click <Authorize> to fix".to_owned(),
                        timeout_ms: 5000
                    });
                    stream.emit(Msg::GoToSettings);
                },
                err => notifier.emit(NotifierMsg::Notify {
                    title: "<b>Error!</b>".to_owned(),
                    body: err.to_string(),
                    timeout_ms: 5000
                }),
            }
        });

        let albums_stream = self.albums_tab.stream().clone();
        let playlists_stream = self.playlists_tab.stream().clone();
        let favorites_stream = self.favorites_tab.stream().clone();
        let shows_stream = self.shows_tab.stream().clone();
        self.media_controls.stream().observe(move |msg| {
            if let MediaControlsMsg::GoToTrack(kind, uri) = msg {
                let uri = uri.clone();
                match kind {
                    Type::Album => albums_stream.emit(AlbumsMsg::GoToTrack(uri)),
                    Type::Playlist => playlists_stream.emit(PlaylistsMsg::GoToTrack(uri)),
                    Type::Track => favorites_stream.emit(FavoritesMsg::GoToTrack(uri)),
                    Type::Show => shows_stream.emit(ShowsMsg::GoToTrack(uri)),
                    _ => (),
                }
            }
        });
    }
}
