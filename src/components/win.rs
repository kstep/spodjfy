use gtk::{self, CssProviderExt, GtkWindowExt, Inhibit, OverlayExt, PanedExt, SettingsExt, StackExt, StackSidebarExt, WidgetExt};
use relm::{Channel, Relm, Widget};
use relm_derive::{widget, Msg};
use std::sync::{Arc, RwLock};

use crate::{
    components::{
        media_controls::{MediaControls, MediaControlsMsg},
        notifier::{Notifier, NotifierMsg},
        tabs::{
            albums::AlbumsTab,
            artists::ArtistsTab,
            categories::CategoriesTab,
            devices::{DevicesMsg, DevicesTab},
            featured::FeaturedTab,
            new_releases::NewReleasesTab,
            playlists::PlaylistsTab,
            queue::QueueTab,
            recent::RecentTab,
            search::{SearchMsg, SearchTab},
            settings::{SettingsMsg, SettingsTab},
            shows::ShowsTab,
            tracks::TracksTab,
            MusicTabMsg,
        },
    },
    config::{Settings, SettingsRef},
    observe,
    services::spotify::SpotifyRef,
    AppEvent,
};
use rspotify::model::Type;
use tokio::runtime::Handle;

pub struct State {
    pub settings: SettingsRef,
    pub spotify: SpotifyRef,
    pub pool: Handle,

    pub screen: gdk::Screen,
    pub style: gtk::CssProvider,
    pub stream: relm::EventStream<Msg>,
    pub notifier: relm::Component<Notifier>,
}

#[derive(Msg)]
pub enum Msg {
    ChangeTab(Option<glib::GString>),
    GoToTab(Tab),
    Quit,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Tab {
    Search,
    RecentlyPlayed,
    Queue,
    Tracks,
    Playlists,
    Artists,
    Albums,
    Shows,
    Categories,
    Featured,
    NewReleases,
    Devices,
    Settings,
}

pub struct Params {
    pub pool: Handle,
    pub settings: Settings,
    pub spotify: SpotifyRef,
}

#[widget]
impl Widget for Win {
    view! {
        #[name="window"]
        gtk::Window {
            icon_name: Some("multimedia-player"),

            #[name="overlay"]
            gtk::Overlay {
                gtk::Box(gtk::Orientation::Vertical, 1) {
                    gtk::Paned(gtk::Orientation::Horizontal) {
                        #[name="sidebar"]
                        gtk::StackSidebar {
                            child: { shrink: false },
                            property_width_request: 300,
                            vexpand: true,
                        },
                        gtk::Box(gtk::Orientation::Vertical, 1) {
                            #[name="media_controls"]
                            MediaControls((self.model.pool.clone(), self.model.spotify.clone(), self.model.settings.clone())) {
                                widget_name: "media_controls",
                            },

                            #[name="stack"]
                            gtk::Stack {
                                vexpand: true,
                                hexpand: true,
                                transition_type: gtk::StackTransitionType::SlideUpDown,

                                #[name="search_tab"]
                                SearchTab((self.model.pool.clone(), self.model.spotify.clone())) {
                                    widget_name: "search_tab",
                                    child: {
                                        name: Some("search_tab"),
                                        title: Some("\u{1F50D} Search")
                                    },
                                },

                                #[name="recent_tab"]
                                RecentTab((self.model.pool.clone(), self.model.spotify.clone())) {
                                    widget_name: "recent_tab",
                                    child: {
                                        name: Some("recent_tab"),
                                        title: Some("\u{23F3} Recently played"),
                                    }
                                },

                                #[name="queue_tab"]
                                QueueTab((self.model.pool.clone(), self.model.spotify.clone())) {
                                    widget_name: "queue_tab",
                                    child: {
                                        name: Some("queue_tab"),
                                        title: Some("\u{25B6} Queue"),
                                    }
                                },

                                #[name="tracks_tab"]
                                TracksTab((self.model.pool.clone(), self.model.spotify.clone())) {
                                    widget_name: "tracks_tab",
                                    child: {
                                        name: Some("tracks_tab"),
                                        title: Some("\u{1F3B5} Tracks"),
                                    }
                                },

                                #[name="playlists_tab"]
                                PlaylistsTab((self.model.pool.clone(), self.model.spotify.clone())) {
                                    widget_name: "playlists_tab",
                                    child: {
                                        name: Some("playlists_tab"),
                                        title: Some("\u{1F4C1} Playlists"),
                                    }
                                },

                                #[name="artists_tab"]
                                ArtistsTab((self.model.pool.clone(), self.model.spotify.clone())) {
                                    widget_name: "artists_tab",
                                    child: {
                                        name: Some("artists_tab"),
                                        title: Some("\u{1F935} Artists"),
                                    }
                                },

                                #[name="albums_tab"]
                                AlbumsTab((self.model.pool.clone(), self.model.spotify.clone())) {
                                    widget_name: "albums_tab",
                                    child: {
                                        name: Some("albums_tab"),
                                        title: Some("\u{1F4BF} Albums"),
                                    }
                                },

                                #[name="shows_tab"]
                                ShowsTab((self.model.pool.clone(), self.model.spotify.clone())) {
                                    widget_name: "shows_tab",
                                    child: {
                                        name: Some("shows_tab"),
                                        title: Some("\u{1F399} Shows"),
                                    }
                                },

                                #[name="categories_tab"]
                                CategoriesTab((self.model.pool.clone(), self.model.spotify.clone())) {
                                    widget_name: "categories_tab",
                                    child: {
                                        name: Some("categories_tab"),
                                        title: Some("\u{1F4D2} Categories")
                                    }
                                },

                                #[name="featured_tab"]
                                FeaturedTab((self.model.pool.clone(), self.model.spotify.clone())) {
                                    widget_name: "featured_tab",
                                    child: {
                                        name: Some("featured_tab"),
                                        title: Some("\u{1F525} Featured")
                                    }
                                },

                                #[name="new_releases_tab"]
                                NewReleasesTab((self.model.pool.clone(), self.model.spotify.clone())) {
                                    widget_name: "new_releases_tab",
                                    child: {
                                        name: Some("new_releases_tab"),
                                        title: Some("\u{1F4C5} New releases")
                                    }
                                },

                                #[name="devices_tab"]
                                DevicesTab((self.model.pool.clone(), self.model.spotify.clone())) {
                                    widget_name: "devices_tab",
                                    child: {
                                        name: Some("devices_tab"),
                                        title: Some("\u{1F39B} Devices"),
                                    },
                                },

                                #[name="settings_tab"]
                                SettingsTab((self.model.pool.clone(), self.model.spotify.clone(), self.model.settings.clone())) {
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

        }
    }

    fn model(relm: &Relm<Self>, params: Params) -> State {
        let style = gtk::CssProvider::new();
        let screen = gdk::Screen::get_default().unwrap();

        if let Some(settings) = gtk::Settings::get_for_screen(&screen).or_else(gtk::Settings::get_default) {
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

        gtk::StyleContext::add_provider_for_screen(&screen, &style, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

        let stream = relm.stream().clone();

        State {
            settings: Arc::new(RwLock::new(params.settings)),
            spotify: params.spotify,
            pool: params.pool,
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
            GoToTab(tab) => {
                self.stack.set_visible_child(match tab {
                    Tab::Search => self.search_tab.widget(),
                    Tab::Tracks => self.tracks_tab.widget(),
                    Tab::Albums => self.albums_tab.widget(),
                    Tab::Playlists => self.playlists_tab.widget(),
                    Tab::Artists => self.artists_tab.widget(),
                    Tab::Shows => self.shows_tab.widget(),
                    Tab::Categories => self.categories_tab.widget(),
                    Tab::Settings => self.settings_tab.widget(),
                    _ => self.search_tab.widget(),
                });
            }
            ChangeTab(widget_name) => match widget_name.as_deref() {
                Some("recent_tab") => {
                    self.recent_tab.emit(MusicTabMsg::ShowTab);
                }
                Some("queue_tab") => {
                    self.queue_tab.emit(MusicTabMsg::ShowTab);
                }
                Some("settings_tab") => {
                    self.settings_tab.emit(SettingsMsg::ShowTab);
                }
                Some("albums_tab") => {
                    self.albums_tab.emit(MusicTabMsg::ShowTab);
                }
                Some("artists_tab") => {
                    self.artists_tab.emit(MusicTabMsg::ShowTab);
                }
                Some("playlists_tab") => {
                    self.playlists_tab.emit(MusicTabMsg::ShowTab);
                }
                Some("devices_tab") => {
                    self.devices_tab.emit(DevicesMsg::ShowTab);
                }
                Some("tracks_tab") => {
                    self.tracks_tab.emit(MusicTabMsg::ShowTab);
                }
                Some("shows_tab") => {
                    self.shows_tab.emit(MusicTabMsg::ShowTab);
                }
                Some("new_releases_tab") => {
                    self.new_releases_tab.emit(MusicTabMsg::ShowTab);
                }
                Some("featured_tab") => {
                    self.featured_tab.emit(MusicTabMsg::ShowTab);
                }
                Some("categories_tab") => {
                    self.categories_tab.emit(MusicTabMsg::ShowTab);
                }
                Some("search_tab") => {
                    self.search_tab.emit(SearchMsg::ShowTab);
                }
                _ => {}
            },
        }
    }

    fn init_view(&mut self) {
        let notifier = self.model.notifier.widget();

        self.sidebar.set_stack(&self.stack);
        self.overlay.add_overlay(notifier);
        self.overlay.set_overlay_pass_through(notifier, true);
    }

    fn subscriptions(&mut self, relm: &Relm<Self>) {
        let stream = relm.stream().clone();
        let notifier = self.model.notifier.stream().clone();

        let (_, notifier_tx) = Channel::new(move |msg| {
            notifier.emit(msg);
        });

        let (_, stream_tx) = Channel::new(move |msg| {
            stream.emit(msg);
        });

        observe(&self.model.pool, move |event| match event {
            AppEvent::SpotifyAuthError(msg) => {
                let _ = notifier_tx.send(NotifierMsg::Notify {
                    message: format!(
                        "Authentication error: {}. Check credentials in <Settings> and click <Open authorization URL> to fix",
                        msg
                    ),
                    kind: gtk::MessageType::Error,
                    timeout_ms: 5000,
                });

                let _ = stream_tx.send(Msg::GoToTab(Tab::Settings));
            }
            AppEvent::SpotifyError(msg) => {
                let _ = notifier_tx.send(NotifierMsg::Notify {
                    message: msg,
                    kind: gtk::MessageType::Warning,
                    timeout_ms: 5000,
                });
            }
        });

        /*
        let spotify = self.model.spotify.clone();
        self.model.spotify_errors.observe(move |err| {
            match err {
                InvalidAuth(msg) => {
                    notifier.emit(NotifierMsg::Notify {
                        message: format!("Authentication error: {}. Check credentials in <Settings> and click <Open authorization URL> to fix", msg),
                        kind: gtk::MessageType::Error,
                        timeout_ms: 5000,
                    });
                    spotify.tell(SpotifyCmd::RefreshUserToken).unwrap();
                    stream.emit(Msg::GoToTab(Tab::Settings));
                },
                Unauthorized => {
                    notifier.emit(NotifierMsg::Notify {
                        message: "Authorization error. Check credentials in <Settings> and click <Authorize> to fix".to_owned(),
                        kind: gtk::MessageType::Error,
                        timeout_ms: 5000
                    });
                    stream.emit(Msg::GoToTab(Tab::Settings));
                },
                err => notifier.emit(NotifierMsg::Notify {
                    message: err.to_string(),
                    kind: gtk::MessageType::Warning,
                    timeout_ms: 5000
                }),
            }
        });
        */

        let artists_stream = self.artists_tab.stream().clone();
        let albums_stream = self.albums_tab.stream().clone();
        let playlists_stream = self.playlists_tab.stream().clone();
        let tracks_stream = self.tracks_tab.stream().clone();
        let shows_stream = self.shows_tab.stream().clone();
        let stream = self.model.stream.clone();

        self.media_controls.stream().observe(move |msg| {
            if let MediaControlsMsg::GoToTrack(kind, uri, context_info) = msg {
                let (tab, tab_stream) = match kind {
                    Type::Album => (Tab::Albums, &albums_stream),
                    Type::Playlist => (Tab::Playlists, &playlists_stream),
                    Type::Show => (Tab::Shows, &shows_stream),
                    Type::Artist => (Tab::Artists, &artists_stream),
                    Type::Track => (Tab::Tracks, &tracks_stream),
                    _ => return,
                };

                stream.emit(Msg::GoToTab(tab));

                let uri = uri.clone();
                let context_info = context_info.clone();

                if let Some((uri, name)) = context_info {
                    tab_stream.emit(MusicTabMsg::OpenContainer(0, uri, name));
                }

                tab_stream.emit(MusicTabMsg::GoToTrack(uri));
            }
        });

        macro_rules! connect_playback_update {
            ($media_controls:ident => ($($tab:ident),+)) => {{
                $(
                let media_controls_stream = self.$media_controls.stream().clone();
                let artists_stream = self.artists_tab.stream().clone();
                let albums_stream = self.albums_tab.stream().clone();
                let stream = self.model.stream.clone();
                self.$tab.stream().observe(move |msg| {
                    match msg {
                        MusicTabMsg::PlaybackUpdate => {
                            media_controls_stream.emit(MediaControlsMsg::LoadState);
                        }
                        MusicTabMsg::GoToArtist(uri, name) => {
                            artists_stream.emit(MusicTabMsg::OpenContainer(0, uri.clone(), name.clone()));
                            stream.emit(Msg::GoToTab(Tab::Artists));
                        }
                        MusicTabMsg::GoToAlbum(uri, name) => {
                            albums_stream.emit(MusicTabMsg::OpenContainer(0, uri.clone(), name.clone()));
                            stream.emit(Msg::GoToTab(Tab::Albums));
                        }
                        _ => {}
                    }
                });
                )+
            }}
        }

        connect_playback_update!(media_controls => (
            albums_tab, artists_tab, categories_tab, tracks_tab,
            featured_tab, new_releases_tab, queue_tab, recent_tab, shows_tab,
            playlists_tab
        ));
    }
}
