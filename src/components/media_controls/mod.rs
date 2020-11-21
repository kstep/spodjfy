//! # Media controls component
//!
//! A component to show media controls, currently playing track and context information,
//! controls and track media playback state.
//!
//! Parameters:
//!   - `Arc<SpotifyProxy>` - a reference to spotify client proxy
//!   - `Arc<RwLock<Settings>` - a reference to application settings
//!
//! Usage:
//!
//! ```
//!# use std::sync::{Arc, RwLock, mpsc::channel};
//!# use crate::spodjfy::{servers::spotify::SpotifyProxy, config::Config};
//!# macro_rules! view { ($body:tt*) => {} }
//! let (tx, rx) = channel();
//! let spotify = Arc::new(SpotifyProxy::new(tx));
//! let settings = Arc::new(RwLock::new(Config::new().load_settings()));
//!
//! view! {
//!     MediaControls(spotify.clone(), settings.clone())
//! }
//! ```
mod play_context;

use self::play_context::PlayContext;
use crate::config::Settings;
use crate::loaders::track::TrackLike;
use crate::servers::spotify::{SpotifyCmd, SpotifyProxy};
use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{ButtonBoxExt, GridExt, ImageExt, RangeExt, RevealerExt, ScaleExt, WidgetExt};
use itertools::Itertools;
use notify_rust::Notification;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::context::Context;
use rspotify::model::device::Device;
use rspotify::model::show::FullEpisode;
use rspotify::model::track::FullTrack;
use rspotify::model::{CurrentPlaybackContext, PlayingItem, RepeatState, Type};
use std::sync::{Arc, RwLock};

/// Media controls component messages
#[derive(Msg)]
pub enum MediaControlsMsg {
    Reload,
    LoadState,
    NewState(Box<Option<CurrentPlaybackContext>>),
    LoadDevices,
    NewDevices(Vec<Device>),
    UseDevice(Option<String>),
    LoadCover(String, bool),
    NewCover(Pixbuf, bool),
    Play,
    Pause,
    PrevTrack,
    NextTrack,
    LoadContext(Type, String),
    NewContext(Box<PlayContext>),
    Tick(u32),
    SeekTrack(u32),
    SetVolume(u8),
    SetShuffle(bool),
    ToggleRepeatMode,
    ClickTrackUri(Option<String>),
    GoToTrack(Type, String, Option<(String, String)>),
    ShowInfo(bool),
}

#[doc(hidden)]
pub struct MediaControlsModel {
    stream: EventStream<MediaControlsMsg>,
    devices: gtk::ListStore,
    spotify: Arc<SpotifyProxy>,
    state: Option<CurrentPlaybackContext>,
    context: Option<PlayContext>,
    track_cover: Option<Pixbuf>,
    context_cover: Option<Pixbuf>,
    settings: Arc<RwLock<Settings>>,
}

#[doc(hidden)]
const TRACK_COVER_SIZE: i32 = 256;
#[doc(hidden)]
const CONTEXT_COVER_SIZE: i32 = 128;

#[widget]
impl Widget for MediaControls {
    fn model(
        relm: &Relm<Self>,
        (spotify, settings): (Arc<SpotifyProxy>, Arc<RwLock<Settings>>),
    ) -> MediaControlsModel {
        let stream = relm.stream().clone();

        let _update_timer = {
            let stream = stream.clone();
            let mut counter = 0;
            glib::timeout_add_seconds_local(1, move || {
                counter = (counter + 1) % 10;
                if counter == 0 {
                    stream.emit(MediaControlsMsg::LoadState);
                } else {
                    stream.emit(MediaControlsMsg::Tick(1));
                }
                Continue(true)
            })
        };

        let devices = gtk::ListStore::new(&[String::static_type(), String::static_type()]);

        MediaControlsModel {
            stream,
            spotify,
            devices,
            settings,
            state: None,
            context: None,
            track_cover: None,
            context_cover: None,
        }
    }

    fn update(&mut self, event: MediaControlsMsg) {
        use MediaControlsMsg::*;
        match event {
            Reload => {
                self.model.stream.emit(LoadDevices);
                self.model.stream.emit(LoadState);
            }
            LoadDevices => {
                self.model
                    .spotify
                    .ask(
                        self.model.stream.clone(),
                        |tx| SpotifyCmd::GetMyDevices { tx },
                        NewDevices,
                    )
                    .unwrap();
            }
            NewDevices(devices) => {
                let store = &self.model.devices;
                store.clear();
                for device in devices {
                    store.insert_with_values(None, &[0, 1], &[&device.id, &device.name]);
                }
            }
            UseDevice(device_id) => {
                if let Some(id) = device_id {
                    if let Some(state) = self.model.state.as_mut() {
                        state.device.id = id.clone();
                    }
                    self.model
                        .spotify
                        .tell(SpotifyCmd::UseDevice { id })
                        .unwrap();
                }
            }
            LoadState => {
                self.model
                    .spotify
                    .ask(
                        self.model.stream.clone(),
                        move |tx| SpotifyCmd::GetPlaybackState { tx },
                        |reply| NewState(Box::new(reply)),
                    )
                    .unwrap();
            }
            NewState(state) => {
                let old_state: Option<&CurrentPlaybackContext> = self.model.state.as_ref();
                let old_track_uri = match old_state {
                    Some(CurrentPlaybackContext {
                        item: Some(PlayingItem::Track(FullTrack { uri: track_uri, .. })),
                        ..
                    })
                    | Some(CurrentPlaybackContext {
                        item: Some(PlayingItem::Episode(FullEpisode { uri: track_uri, .. })),
                        ..
                    }) => track_uri.as_str(),
                    _ => "",
                };
                let old_context_uri = match old_state {
                    Some(CurrentPlaybackContext {
                        context:
                            Some(Context {
                                uri: context_uri, ..
                            }),
                        ..
                    }) => context_uri.as_str(),
                    _ => "",
                };

                if let Some(item) = state.as_ref().as_ref().and_then(|s| s.item.as_ref()) {
                    let (cover_url, duration_ms, track_uri) = match item {
                        PlayingItem::Track(track) => (
                            crate::loaders::image::find_best_thumb(
                                track.album.images.iter(),
                                TRACK_COVER_SIZE,
                            ),
                            track.duration_ms,
                            &*track.uri,
                        ),
                        PlayingItem::Episode(episode) => (
                            crate::loaders::image::find_best_thumb(
                                episode.images.iter(),
                                TRACK_COVER_SIZE,
                            ),
                            episode.duration_ms,
                            &*episode.uri,
                        ),
                    };

                    if track_uri != old_track_uri {
                        self.model.track_cover = None;

                        if let Some(url) = cover_url {
                            self.model.stream.emit(LoadCover(url.to_owned(), true));
                        }

                        self.track_seek_bar.set_range(0.0, duration_ms as f64);

                        let item = state.as_ref().as_ref().unwrap().item.as_ref().unwrap();

                        if self.model.settings.read().unwrap().show_notifications {
                            let _ = Notification::new()
                                .summary(item.name())
                                .body(&format!(
                                    "\u{1F935} {}\n\u{1F4BF} {}",
                                    item.artists()
                                        .iter()
                                        .next()
                                        .map(|a| &*a.name)
                                        .unwrap_or("<Unknown Artist>"),
                                    item.album().map(|a| &*a.name).unwrap_or("<No Album>"),
                                ))
                                .show();
                        }
                    }
                }

                if let Some(ctx) = state.as_ref().as_ref().and_then(|s| s.context.as_ref()) {
                    if ctx.uri != old_context_uri {
                        self.model
                            .stream
                            .emit(LoadContext(ctx._type, ctx.uri.clone()));
                    }
                } else {
                    self.model.context = None;
                    self.model.context_cover = None;
                }

                self.model.state = *state;
            }
            LoadCover(url, is_for_track) => {
                let pixbuf = crate::loaders::image::pixbuf_from_url(
                    &url,
                    if is_for_track {
                        TRACK_COVER_SIZE
                    } else {
                        CONTEXT_COVER_SIZE
                    },
                );
                if let Ok(cover) = pixbuf {
                    self.model.stream.emit(NewCover(cover, is_for_track));
                }
            }
            NewCover(cover, is_for_track) => {
                if is_for_track {
                    self.model.track_cover = Some(cover);
                } else {
                    self.model.context_cover = Some(cover);
                }
            }
            SeekTrack(pos) => {
                self.model
                    .spotify
                    .tell(SpotifyCmd::SeekTrack { pos })
                    .unwrap();

                if let Some(CurrentPlaybackContext {
                    progress_ms: Some(ref mut progress),
                    ..
                }) = self.model.state
                {
                    *progress = pos;
                }
                //self.model.stream.emit(LoadState);
            }
            SetVolume(value) => {
                if let Some(state) = self.model.state.as_mut() {
                    state.device.volume_percent = value as u32;
                }
                self.model
                    .spotify
                    .tell(SpotifyCmd::SetVolume { value })
                    .unwrap();
            }
            SetShuffle(state) => {
                if let Some(st) = self.model.state.as_mut() {
                    st.shuffle_state = state;
                }
                self.model
                    .spotify
                    .tell(SpotifyCmd::SetShuffle { state })
                    .unwrap();
            }
            ToggleRepeatMode => {
                let mode = match self
                    .model
                    .state
                    .as_ref()
                    .map(|s| s.repeat_state)
                    .unwrap_or(RepeatState::Off)
                {
                    RepeatState::Off => RepeatState::Context,
                    RepeatState::Context => RepeatState::Track,
                    RepeatState::Track => RepeatState::Off,
                };

                if let Some(state) = self.model.state.as_mut() {
                    state.repeat_state = mode;
                }
                self.repeat_btn.set_active(mode != RepeatState::Off);
                self.repeat_btn.set_image(Some(&gtk::Image::from_icon_name(
                    Some(if mode == RepeatState::Track {
                        "media-playlist-repeat-song"
                    } else {
                        "media-playlist-repeat"
                    }),
                    gtk::IconSize::LargeToolbar,
                )));
                self.model
                    .spotify
                    .tell(SpotifyCmd::SetRepeatMode { mode })
                    .unwrap();
            }
            ShowInfo(state) => {
                self.state_info.set_reveal_child(state);
            }
            Play => {
                self.model.spotify.tell(SpotifyCmd::StartPlayback).unwrap();
                self.model.stream.emit(LoadState);
            }
            Pause => {
                self.model.spotify.tell(SpotifyCmd::PausePlayback).unwrap();
                self.model.stream.emit(LoadState);
            }
            NextTrack => {
                self.model.spotify.tell(SpotifyCmd::PlayNextTrack).unwrap();
                self.model.stream.emit(LoadState);
            }
            PrevTrack => {
                self.model.spotify.tell(SpotifyCmd::PlayPrevTrack).unwrap();
                self.model.stream.emit(LoadState);
            }
            LoadContext(kind, uri) => {
                let stream = &self.model.stream;

                match kind {
                    Type::Playlist => {
                        self.model
                            .spotify
                            .ask(
                                stream.clone(),
                                |tx| SpotifyCmd::GetPlaylist { tx, uri },
                                |reply| NewContext(Box::new(PlayContext::Playlist(reply))),
                            )
                            .unwrap();
                    }
                    Type::Album => {
                        self.model
                            .spotify
                            .ask(
                                stream.clone(),
                                |tx| SpotifyCmd::GetAlbum { tx, uri },
                                |reply| NewContext(Box::new(PlayContext::Album(reply))),
                            )
                            .unwrap();
                    }
                    Type::Artist => {
                        self.model
                            .spotify
                            .ask(
                                stream.clone(),
                                |tx| SpotifyCmd::GetArtist { tx, uri },
                                |reply| NewContext(Box::new(PlayContext::Artist(reply))),
                            )
                            .unwrap();
                    }
                    Type::Show => {
                        self.model
                            .spotify
                            .ask(
                                stream.clone(),
                                |tx| SpotifyCmd::GetShow { tx, uri },
                                |reply| NewContext(Box::new(PlayContext::Show(reply))),
                            )
                            .unwrap();
                    }
                    _ => {
                        self.model.context = None;
                        self.model.context_cover = None;
                    }
                };
            }
            NewContext(context) => {
                let images = context.images();
                if let Some(cover_url) =
                    crate::loaders::image::find_best_thumb(images, CONTEXT_COVER_SIZE)
                {
                    self.model
                        .stream
                        .emit(LoadCover(cover_url.to_owned(), false));
                }
                self.model.context = Some(*context);
            }
            Tick(timeout) => {
                if let Some(CurrentPlaybackContext {
                    is_playing: true,
                    progress_ms: Some(ref mut progress),
                    ..
                }) = self.model.state
                {
                    *progress += timeout * 1000;
                    self.track_seek_bar.set_value(*progress as f64);
                }
            }
            ClickTrackUri(Some(uri)) => {
                let (kind, context_info) = self
                    .model
                    .context
                    .as_ref()
                    .map(|ctx| {
                        (
                            ctx.kind(),
                            Some((ctx.uri().to_owned(), ctx.name().to_owned())),
                        )
                    })
                    .unwrap_or((Type::Track, None));

                self.model
                    .stream
                    .emit(MediaControlsMsg::GoToTrack(kind, uri, context_info));
            }
            ClickTrackUri(None) => {}
            GoToTrack(_, _, _) => {}
        }
    }

    view! {
        gtk::Box(gtk::Orientation::Vertical, 10) {
            #[name="state_info"]
            gtk::Revealer {
                gtk::Grid {
                    column_homogeneous: true,
                    column_spacing: 10,
                    margin_top: 15,
                    margin_start: 15,
                    margin_end: 15,

                    gtk::Box(gtk::Orientation::Horizontal, 10) {
                        cell: { left_attach: 0, top_attach: 0, width: 2, },
                        #[name="track_cover_image"]
                        gtk::Image {
                            valign: gtk::Align::Start,
                            from_pixbuf: self.model.track_cover.as_ref()
                        },
                        #[name="track_infobox"]
                        gtk::Box(gtk::Orientation::Vertical, 10) {
                            #[name="track_name_label"]
                            gtk::LinkButton {
                                widget_name: "track_name_label",
                                uri: self.model.state.as_ref().and_then(|s| s.item.as_ref()).map(|it| it.uri()).unwrap_or(""),

                                activate_link(btn) => (MediaControlsMsg::ClickTrackUri(btn.get_uri().map(|u| u.into())), Inhibit(true)),

                                gtk::Label {
                                    halign: gtk::Align::Start,
                                    xalign: 0.0,
                                    line_wrap: true,
                                    ellipsize: pango::EllipsizeMode::End,
                                    lines: 2,
                                    text: self.model.state.as_ref().and_then(|s| s.item.as_ref()).map(|it| it.name()).unwrap_or("<Nothing>"),
                                }
                            },
                            #[name="track_artists_label"]
                            gtk::Label {
                                halign: gtk::Align::Start,
                                selectable: true,
                                text: self.model.state.as_ref().and_then(|s| s.item.as_ref()).map(|it| match it {
                                    PlayingItem::Track(track) => track.artists.iter().map(|artist| &artist.name).join(", "),
                                    PlayingItem::Episode(episode) => episode.show.publisher.clone(),
                                }).as_deref().unwrap_or("<Unknown Artist>")
                            },
                            #[name="track_album_label"]
                            gtk::Label {
                                halign: gtk::Align::Start,
                                widget_name: "track_album_label",
                                selectable: true,
                                text: self.model.state.as_ref().and_then(|s| s.item.as_ref()).map(|it| match it {
                                    PlayingItem::Track(track) => &*track.album.name,
                                    PlayingItem::Episode(episode) => &*episode.show.name,
                                }).unwrap_or("")
                            },
                            //gtk::ScrolledWindow {
                                #[name="track_description_label"]
                                gtk::Label {
                                    halign: gtk::Align::Start,
                                    selectable: true,
                                    line_wrap: true,
                                    xalign: 0.0,
                                    text: self.model.state.as_ref()
                                        .and_then(|s| s.item.as_ref())
                                        .and_then(|it| it.description())
                                        .unwrap_or("")
                                },
                            //},
                        },
                    },

                    gtk::Box(gtk::Orientation::Horizontal, 10) {
                        cell: { left_attach: 2, top_attach: 0, width: 1, },

                        #[name="context_cover_image"]
                        gtk::Image {
                            valign: gtk::Align::Start,
                            from_pixbuf: self.model.context_cover.as_ref(),
                        },
                        #[name="context_infobox"]
                        gtk::Box(gtk::Orientation::Vertical, 10) {
                            gtk::Box(gtk::Orientation::Horizontal, 5) {
                                gtk::Label {
                                    halign: gtk::Align::Start,
                                    valign: gtk::Align::Start,
                                    text: match self.model.context {
                                            Some(PlayContext::Album(_)) => "\u{1F4BF}",
                                            Some(PlayContext::Playlist(_)) => "\u{1F4C1}",
                                            Some(PlayContext::Artist(_)) => "\u{1F935}",
                                            Some(PlayContext::Show(_)) => "\u{1F399}",
                                            None => "",
                                        }
                                },
                                #[name="context_name_label"]
                                gtk::Label {
                                    halign: gtk::Align::Start,
                                    widget_name: "context_name_label",
                                    selectable: true,
                                    line_wrap: true,
                                    xalign: 0.0,
                                    text: self.model.context.as_ref().map(|c| c.name()).unwrap_or(""),
                                },
                            },
                            #[name="context_artists_label"]
                            gtk::Label {
                                halign: gtk::Align::Start,
                                selectable: true,
                                text: self.model.context.as_ref()
                                    .and_then(|ctx| ctx.artists())
                                    .as_deref()
                                    .unwrap_or("")
                            },
                            #[name="context_tracks_number_label"]
                            gtk::Label {
                                halign: gtk::Align::Start,
                                selectable: true,
                                text: self.model.context.as_ref()
                                    .map(|c| match (c.tracks_number(), c.duration()) {
                                        (0, None) => String::new(),
                                        (0, Some(d)) => format!("Duration: {}", crate::utils::humanize_time(d)),
                                        (n, None) => format!("Tracks: {}", n),
                                        (n, Some(d)) => format!("Tracks: {}, duration: {}", n, crate::utils::humanize_time(d)),
                                    })
                                    .as_deref()
                                    .unwrap_or("")
                            },
                            //gtk::ScrolledWindow {
                                #[name="context_description_label"]
                                gtk::Label {
                                    halign: gtk::Align::Start,
                                    line_wrap: true,
                                    selectable: true,
                                    xalign: 0.0,
                                    text: self.model.context.as_ref()
                                        .map(|c| c.description())
                                        .unwrap_or("")
                                },
                            //},
                            #[name="context_genres_label"]
                            gtk::Label {
                                halign: gtk::Align::Start,
                                selectable: true,
                                text: self.model.context.as_ref()
                                    .and_then(|c| c.genres())
                                    .map(|gs| gs.iter().join(", "))
                                    .as_deref()
                                    .unwrap_or("")
                            },
                        }
                    },
                },
            },

            gtk::Box(gtk::Orientation::Horizontal, 10) {
                #[name="track_seek_bar"]
                gtk::Scale(gtk::Orientation::Horizontal, Some(&gtk::Adjustment::new(0.0, 0.0, 0.0, 1000.0, 1000.0, 1000.0))) {
                    margin_start: 10,
                    hexpand: true,
                    valign: gtk::Align::Center,
                    value_pos: gtk::PositionType::Left,
                    value: self.model.state.as_ref().and_then(|s| s.progress_ms).unwrap_or(0) as f64,

                    change_value(_, _, pos) => (MediaControlsMsg::SeekTrack(pos as u32), Inhibit(false)),
                    format_value(seek, value) => return {
                        let value = value as u32;
                        let duration = seek.get_adjustment().get_upper() as u32;
                        format!(
                            "{} / -{}",
                            crate::utils::humanize_time(value),
                            crate::utils::humanize_time(duration - value)
                        )
                    },
                },
                gtk::Label {
                    margin_end: 10,
                    text: self.model.state.as_ref().and_then(|s| s.item.as_ref()).map(|it| match it {
                        PlayingItem::Track(track) => track.duration_ms,
                        PlayingItem::Episode(episode) => episode.duration_ms,
                    }).map(crate::utils::humanize_time).as_deref().unwrap_or("??:??")
                },
            },
            gtk::Box(gtk::Orientation::Horizontal, 5) {
                halign: gtk::Align::Center,

                #[name="buttons"]
                gtk::ButtonBox(gtk::Orientation::Horizontal) {
                    layout: gtk::ButtonBoxStyle::Center,
                    hexpand: false,

                    #[name="show_info_btn"]
                    gtk::ToggleButton {
                        child: { non_homogeneous: true },
                        tooltip_text: Some("Show info"),
                        image: Some(&gtk::Image::from_icon_name(Some("go-down-symbolic"), gtk::IconSize::LargeToolbar)),
                        toggled(btn) => MediaControlsMsg::ShowInfo(btn.get_active()),
                    },
                    #[name="prev_track_btn"]
                    gtk::Button {
                        tooltip_text: Some("Previous track"),
                        image: Some(&gtk::Image::from_icon_name(Some("media-skip-backward"), gtk::IconSize::LargeToolbar)),
                        clicked(_) => MediaControlsMsg::PrevTrack,
                    },
                    #[name="pause_btn"]
                    gtk::Button {
                        tooltip_text: Some("Pause"),
                        image: Some(&gtk::Image::from_icon_name(Some("media-playback-pause"), gtk::IconSize::LargeToolbar)),
                        clicked(_) => MediaControlsMsg::Pause,
                    },
                    #[name="play_btn"]
                    gtk::Button {
                        widget_name: "play_btn",
                        tooltip_text: Some("Play"),
                        image: Some(&gtk::Image::from_icon_name(Some("media-playback-start"), gtk::IconSize::LargeToolbar)),
                        child: { non_homogeneous: true },
                        clicked(_) => MediaControlsMsg::Play,
                    },
                    #[name="next_track_btn"]
                    gtk::Button {
                        tooltip_text: Some("Next track"),
                        image: Some(&gtk::Image::from_icon_name(Some("media-skip-forward"), gtk::IconSize::LargeToolbar)),
                        clicked(_) => MediaControlsMsg::NextTrack,
                    },

                    #[name="shuffle_btn"]
                    gtk::ToggleButton {
                        child: { non_homogeneous: true },
                        tooltip_text: Some("Shuffle"),
                        image: Some(&gtk::Image::from_icon_name(Some("media-playlist-shuffle"), gtk::IconSize::LargeToolbar)),
                        active: self.model.state.as_ref().map(|s| s.shuffle_state).unwrap_or(false),
                        toggled(btn) => MediaControlsMsg::SetShuffle(btn.get_active()),
                    },
                    #[name="repeat_btn"]
                    gtk::ToggleButton {
                        child: { non_homogeneous: true },
                        tooltip_text: Some("Repeat mode"),
                        image: Some(&gtk::Image::from_icon_name(
                            self.model.state.as_ref()
                                .map(|s| {
                                    if s.repeat_state == RepeatState::Track {
                                        "media-playlist-repeat-song"
                                    } else {
                                        "media-playlist-repeat"
                                    }
                                }).or_else(|| Some("media-playlist-repeat")),
                            gtk::IconSize::LargeToolbar)),
                        active: self.model.state.as_ref().map(|s| s.repeat_state != RepeatState::Off).unwrap_or(false),
                        toggled(_) => MediaControlsMsg::ToggleRepeatMode,
                    },
                },

                gtk::Scale(gtk::Orientation::Horizontal, Some(&gtk::Adjustment::new(0.0, 0.0, 101.0, 1.0, 1.0, 1.0))) {
                    tooltip_text: Some("Volume"),
                    digits: 0,
                    value: self.model.state.as_ref().map(|s| s.device.volume_percent as f64).unwrap_or(0.0),
                    property_width_request: 200,
                    valign: gtk::Align::Center,
                    vexpand: false,

                    change_value(_, _, pos) => (MediaControlsMsg::SetVolume(pos as u8), Inhibit(false)),
                },

                #[name="device_selector"]
                gtk::ComboBox {
                    tooltip_text: Some("Current device"),
                    halign: gtk::Align::Start,
                    valign: gtk::Align::Center,
                    vexpand: false,
                    model: Some(&self.model.devices),
                    active_id: self.model.state.as_ref().map(|s| &*s.device.id),
                    id_column: 0,
                    entry_text_column: 1,

                    changed(combo) => MediaControlsMsg::UseDevice(combo.get_active_id().map(|id| id.into())),
                },
            },
        }
    }

    fn init_view(&mut self) {
        let stream = self.model.stream.clone();

        let cell = gtk::CellRendererText::new();
        self.device_selector.pack_start(&cell, true);
        self.device_selector.add_attribute(&cell, "text", 1 as i32);

        self.buttons.get_style_context().add_class("linked");

        stream.emit(MediaControlsMsg::Reload);
    }
}