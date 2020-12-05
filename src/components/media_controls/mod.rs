//! # Media controls component
//!
//! A component to show media controls, currently playing track and context
//! information, controls and track media playback state.
//!
//! Parameters:
//!   - `Arc<SpotifyProxy>` - a reference to spotify client proxy
//!   - `Arc<RwLock<Settings>` - a reference to application settings
//!
//! Usage:
//!
//! ```
//! # use std::sync::{Arc, RwLock, mpsc::channel};
//! # use crate::spodjfy::{SpotifyProxy, Config};
//! # macro_rules! view { ($body:tt*) => {} }
//! let (spotify, _rx, _errors_stream) = SpotifyProxy::new();
//! let settings = Arc::new(RwLock::new(Config::new().load_settings()));
//!
//! view! {
//!     MediaControls(Arc::new(spotify.clone()), settings.clone())
//! }
//! ```

mod play_context;

use self::play_context::PlayContext;
use crate::{
    config::SettingsRef,
    loaders::{ImageData, ImageLoader},
    models::{common::*, TrackLike},
    services::{
        api::{
            AlbumsStorageApi, ArtistsStorageApi, LibraryStorageApi, PlaybackControlApi, PlaylistsStorageApi, ShowsStorageApi,
            UsersStorageApi,
        },
        SpotifyRef,
    },
    utils::{Extract, Spawn},
};
use gdk_pixbuf::Pixbuf;
use gtk::{prelude::*, ButtonBoxExt, GridExt, ImageExt, RangeExt, RevealerExt, ScaleExt, WidgetExt};
use itertools::Itertools;
use notify_rust::Notification;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::{
    client::ClientError,
    model::{
        context::Context, device::Device, show::FullEpisode, track::FullTrack, CurrentPlaybackContext, PlayingItem, RepeatState,
        Type,
    },
};
use tokio::runtime::Handle;

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
    NewCover(ImageData, bool),
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
    SaveCurrentTrack(bool),
    SaveCurrentContext(bool),
    IsTrackSaved(bool),
    IsContextSaved(bool),
}

#[doc(hidden)]
pub struct MediaControlsModel {
    pool: Handle,
    stream: EventStream<MediaControlsMsg>,
    devices: gtk::ListStore,
    spotify: SpotifyRef,
    state: Option<CurrentPlaybackContext>,
    play_context: Option<PlayContext>,
    play_context_cover: Option<Pixbuf>,
    play_context_saved: bool,
    track_cover: Option<Pixbuf>,
    track_saved: bool,
    image_loaders: [ImageLoader; 2],
    settings: SettingsRef,
}

#[doc(hidden)]
const TRACK_COVER_SIZE: i32 = 256;

#[doc(hidden)]
const CONTEXT_COVER_SIZE: i32 = 128;

#[widget]
impl Widget for MediaControls {
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
                            gtk::Box(gtk::Orientation::Horizontal, 0) {
                                #[name="track_saved_btn"]
                                gtk::ToggleButton {
                                    tooltip_text: Some("Add to library"),
                                    image: Some(&gtk::Image::from_icon_name(Some("emblem-favorite"), gtk::IconSize::LargeToolbar)),
                                    active: self.model.track_saved,
                                    toggled(btn) => MediaControlsMsg::SaveCurrentTrack(btn.get_active()),
                                },
                                #[name="track_name_label"]
                                gtk::LinkButton {
                                    widget_name: "track_name_label",
                                    uri: self.model.state.as_ref().and_then(|s| s.item.as_ref()).map_or("", |it| it.uri()),

                                    activate_link(btn) => (MediaControlsMsg::ClickTrackUri(btn.get_uri().map(|u| u.into())), Inhibit(true)),

                                    gtk::Label {
                                        halign: gtk::Align::Start,
                                        xalign: 0.0,
                                        line_wrap: true,
                                        ellipsize: pango::EllipsizeMode::End,
                                        lines: 2,
                                        text: self.model.state.as_ref().and_then(|s| s.item.as_ref()).map_or("", |it| it.name()),
                                    }
                                },
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
                                text: self.model.state.as_ref().and_then(|s| s.item.as_ref()).map_or("", |it| match it {
                                    PlayingItem::Track(track) => &*track.album.name,
                                    PlayingItem::Episode(episode) => &*episode.show.name,
                                })
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
                            from_pixbuf: self.model.play_context_cover.as_ref(),
                        },
                        #[name="context_infobox"]
                        gtk::Box(gtk::Orientation::Vertical, 10) {
                            gtk::Box(gtk::Orientation::Horizontal, 5) {
                                #[name="context_saved_btn"]
                                gtk::ToggleButton {
                                    tooltip_text: Some("Add to library"),
                                    image: Some(&gtk::Image::from_icon_name(Some("emblem-favorite"), gtk::IconSize::LargeToolbar)),
                                    active: self.model.play_context_saved,
                                    toggled(btn) => MediaControlsMsg::SaveCurrentContext(btn.get_active()),
                                },
                                gtk::Label {
                                    halign: gtk::Align::Start,
                                    valign: gtk::Align::Center,
                                    text: self.model.play_context.as_ref().map_or("", |ctx| ctx.emoji()),
                                },
                                #[name="context_name_label"]
                                gtk::Label {
                                    halign: gtk::Align::Start,
                                    widget_name: "context_name_label",
                                    selectable: true,
                                    line_wrap: true,
                                    xalign: 0.0,
                                    text: self.model.play_context.as_ref().map_or("", |c| c.name()),
                                },
                            },
                            #[name="context_artists_label"]
                            gtk::Label {
                                halign: gtk::Align::Start,
                                selectable: true,
                                line_wrap: true,
                                text: self.model.play_context.as_ref()
                                    .and_then(|ctx| ctx.artists())
                                    .as_deref()
                                    .unwrap_or("")
                            },
                            #[name="context_genres_label"]
                            gtk::Label {
                                halign: gtk::Align::Start,
                                selectable: true,
                                line_wrap: true,
                                text: self.model.play_context.as_ref()
                                    .and_then(|c| c.genres())
                                    .map(|gs| gs.iter().join(", "))
                                    .as_deref()
                                    .unwrap_or("")
                            },
                            #[name="context_tracks_number_label"]
                            gtk::Label {
                                halign: gtk::Align::Start,
                                selectable: true,
                                text: self.model.play_context.as_ref()
                                    .map(|c| match (c.tracks_number(), c.duration()) {
                                        (0, _) => String::new(),
                                        (n, Ok(d)) => format!("Tracks: {}, duration: {}", n, crate::utils::humanize_time(d)),
                                        (n, Err(d)) => format!("Tracks: {}, duration: around {}", n, crate::utils::humanize_inexact_time(d)),
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
                                    text: self.model.play_context.as_ref()
                                        .map_or("", |c| c.description())
                                },
                            //},
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
                        active: self.model.state.as_ref().map_or(false, |s| s.shuffle_state),
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
                                }).or(Some("media-playlist-repeat")),
                            gtk::IconSize::LargeToolbar)),
                        active: self.model.state.as_ref().map_or(false, |s| s.repeat_state != RepeatState::Off),
                        toggled(_) => MediaControlsMsg::ToggleRepeatMode,
                    },
                },

                gtk::Scale(gtk::Orientation::Horizontal, Some(&gtk::Adjustment::new(0.0, 0.0, 101.0, 1.0, 1.0, 1.0))) {
                    tooltip_text: Some("Volume"),
                    digits: 0,
                    value: self.model.state.as_ref().and_then(|s| s.device.volume_percent).map_or(0.0, |vol| vol as f64),
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
                    active_id: self.model.state.as_ref().and_then(|s| s.device.id.as_deref()),
                    id_column: 0,
                    entry_text_column: 1,

                    changed(combo) => MediaControlsMsg::UseDevice(combo.get_active_id().map(|id| id.into())),
                },
            },
        }
    }

    fn model(relm: &Relm<Self>, (pool, spotify, settings): (Handle, SpotifyRef, SettingsRef)) -> MediaControlsModel {
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
        let track_image_loader = ImageLoader::with_resize(TRACK_COVER_SIZE, false);
        let context_image_loader = ImageLoader::with_resize(CONTEXT_COVER_SIZE, false);

        MediaControlsModel {
            pool,
            stream,
            spotify,
            devices,
            settings,
            image_loaders: [context_image_loader, track_image_loader],
            state: None,
            play_context: None,
            track_saved: false,
            play_context_saved: false,
            track_cover: None,
            play_context_cover: None,
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
                self.spawn(async move |pool, (stream, spotify): (EventStream<_>, SpotifyRef)| {
                    stream.emit(NewDevices(
                        pool.spawn(async move { spotify.read().await.get_my_devices().await })
                            .await??,
                    ));
                    Ok(())
                });
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
                    let play = if let Some(state) = self.model.state.as_mut() {
                        if let Some(ref device_id) = state.device.id {
                            if device_id == &id {
                                return;
                            }
                        }

                        state.device.id = Some(id.clone());
                        state.is_playing
                    } else {
                        false
                    };

                    self.spawn_args(id, async move |pool, spotify: SpotifyRef, id| {
                        Ok(pool
                            .spawn(async move { spotify.read().await.use_device(&id, play).await })
                            .await??)
                    });
                }
            }
            LoadState => {
                self.spawn(async move |pool, (stream, spotify): (EventStream<_>, SpotifyRef)| {
                    stream.emit(NewState(Box::new(
                        pool.spawn(async move { spotify.read().await.get_playback_state().await })
                            .await??,
                    )));

                    Ok(())
                });
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
                        context: Some(Context { uri: context_uri, .. }),
                        ..
                    }) => context_uri.as_str(),
                    _ => "",
                };

                if let Some(item) = state.as_ref().as_ref().and_then(|s| s.item.as_ref()) {
                    let (cover_url, duration_ms, track_uri) = (
                        self.model.image_loaders[1].find_best_thumb(item.images()),
                        item.duration(),
                        item.uri(),
                    );

                    let kind = match item {
                        PlayingItem::Episode(_) => Type::Episode,
                        PlayingItem::Track(_) => Type::Track,
                    };

                    if track_uri != old_track_uri {
                        self.model.track_cover = None;

                        if let Some(url) = cover_url {
                            self.model.stream.emit(LoadCover(url.to_owned(), true));
                        }

                        {
                            let uris = vec![track_uri.to_owned()];

                            self.spawn_args(
                                uris,
                                async move |pool, (stream, spotify): (EventStream<_>, SpotifyRef), uris| {
                                    let reply = pool
                                        .spawn(async move { spotify.read().await.are_in_my_library(kind, &uris).await })
                                        .await??;
                                    stream.emit(IsTrackSaved(reply[0]));
                                    Ok(())
                                },
                            );
                        }

                        self.track_seek_bar.set_range(0.0, duration_ms as f64);

                        let _item = state.as_ref().as_ref().unwrap().item.as_ref().unwrap();

                        if self.model.settings.read().unwrap().show_notifications {
                            let _ = Notification::new()
                                .summary(item.name())
                                .body(&format!(
                                    "\u{1F935} {}\n\u{1F4BF} {}",
                                    item.artists().iter().next().map_or("", |a| &*a.name),
                                    item.album().map_or("", |a| &*a.name),
                                ))
                                .show();
                        }
                    }
                }

                if let Some(ctx) = state.as_ref().as_ref().and_then(|s| s.context.as_ref()) {
                    if ctx.uri != old_context_uri {
                        self.model.stream.emit(LoadContext(ctx._type, ctx.uri.clone()));

                        {
                            let uris = vec![ctx.uri.clone()];

                            let kind = ctx._type;

                            self.spawn_args(
                                uris,
                                async move |pool, (stream, spotify): (EventStream<_>, SpotifyRef), uris| {
                                    let saved = pool
                                        .spawn(async move { spotify.read().await.are_in_my_library(kind, &uris).await })
                                        .await??;
                                    stream.emit(IsContextSaved(saved[0]));
                                    Ok(())
                                },
                            );
                        }
                    }
                } else {
                    if old_state.is_some() {
                        self.model.stream.emit(LoadContext(Type::User, String::new()));
                    }

                    self.model.play_context = None;
                    self.model.play_context_cover = None;
                }

                self.model.state = *state;
            }
            IsTrackSaved(saved) => {
                self.model.track_saved = saved;
            }
            IsContextSaved(saved) => {
                self.model.play_context_saved = saved;
            }
            LoadCover(url, is_for_track) => {
                let loader = self.model.image_loaders[is_for_track as usize].clone();

                self.spawn_args((loader, url), async move |pool, stream: EventStream<_>, (loader, url)| {
                    if let Some(image) = pool.spawn(async move { loader.load_image(&url).await }).await? {
                        stream.emit(NewCover(image, is_for_track));
                    }
                    Ok(())
                });
            }
            NewCover(cover, is_for_track) => {
                if is_for_track {
                    self.model.track_cover = Some(cover.into());
                } else {
                    self.model.play_context_cover = Some(cover.into());
                }
            }
            SeekTrack(pos) => {
                self.spawn(async move |pool, spotify: SpotifyRef| {
                    Ok(pool
                        .spawn(async move { spotify.read().await.seek_track(pos).await })
                        .await??)
                });

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
                    state.device.volume_percent = Some(value as u32);
                }

                self.spawn(async move |pool, spotify: SpotifyRef| {
                    Ok(pool
                        .spawn(async move { spotify.read().await.set_volume(value).await })
                        .await??)
                });
            }
            SetShuffle(state) => {
                if let Some(st) = self.model.state.as_mut() {
                    st.shuffle_state = state;
                }

                self.spawn(async move |pool, spotify: SpotifyRef| {
                    Ok(pool
                        .spawn(async move { spotify.read().await.set_shuffle(state).await })
                        .await??)
                });
            }
            ToggleRepeatMode => {
                let mode = match self.model.state.as_ref().map_or(RepeatState::Off, |s| s.repeat_state) {
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

                self.spawn(async move |pool, spotify: SpotifyRef| {
                    Ok(pool
                        .spawn(async move { spotify.read().await.set_repeat_mode(mode).await })
                        .await??)
                });
            }
            SaveCurrentTrack(save) => {
                if let Some(ref state) = self.model.state {
                    if let Some(ref item) = state.item {
                        let (kind, uri) = match item {
                            PlayingItem::Track(track) => (Type::Track, track.uri.clone()),
                            PlayingItem::Episode(episode) => (Type::Episode, episode.uri.clone()),
                        };

                        let uris = vec![uri];

                        self.spawn_args(uris, async move |pool, spotify: SpotifyRef, uris| {
                            if save {
                                pool.spawn(async move { spotify.read().await.add_to_my_library(kind, &uris).await })
                                    .await??;
                            } else {
                                pool.spawn(async move { spotify.read().await.remove_from_my_library(kind, &uris).await })
                                    .await??;
                            }

                            Ok(())
                        });

                        self.model.track_saved = save;
                    }
                }
            }
            SaveCurrentContext(save) => {
                if let Some(ref context) = self.model.play_context {
                    let kind = context.kind();

                    let uris = vec![context.uri().to_owned()];

                    self.spawn_args(uris, async move |pool, spotify: SpotifyRef, uris| {
                        Ok(pool
                            .spawn(async move {
                                let spotify = spotify.read().await;

                                if save {
                                    spotify.add_to_my_library(kind, &uris).await
                                } else {
                                    spotify.remove_from_my_library(kind, &uris).await
                                }
                            })
                            .await??)
                    });

                    self.model.play_context_saved = save;
                }
            }
            ShowInfo(state) => {
                self.state_info.set_reveal_child(state);
            }
            Play => {
                self.spawn(async move |pool, (stream, spotify): (EventStream<_>, SpotifyRef)| {
                    pool.spawn(async move { spotify.read().await.start_playback().await })
                        .await??;
                    stream.emit(LoadState);
                    Ok(())
                });
            }
            Pause => {
                self.spawn(async move |pool, (stream, spotify): (EventStream<_>, SpotifyRef)| {
                    pool.spawn(async move { spotify.read().await.pause_playback().await })
                        .await??;
                    stream.emit(LoadState);
                    Ok(())
                });
            }
            NextTrack => {
                self.spawn(async move |pool, (stream, spotify): (EventStream<_>, SpotifyRef)| {
                    pool.spawn(async move { spotify.read().await.play_next_track().await })
                        .await??;
                    stream.emit(LoadState);
                    Ok(())
                });
            }
            PrevTrack => {
                self.spawn(async move |pool, (stream, spotify): (EventStream<_>, SpotifyRef)| {
                    pool.spawn(async move { spotify.read().await.play_prev_track().await })
                        .await??;

                    stream.emit(LoadState);

                    Ok(())
                });
            }
            LoadContext(kind, uri) => {
                self.spawn_args(
                    uri,
                    async move |pool, (stream, spotify): (EventStream<_>, SpotifyRef), uri| {
                        let play_context = pool
                            .spawn(async move {
                                let spotify = spotify.read().await;

                                let reply: PlayContext = match kind {
                                    Type::Playlist => spotify.get_playlist(&uri).await?.into(),
                                    Type::Album => spotify.get_album(&uri).await?.into(),
                                    Type::Artist => spotify.get_artist(&uri).await?.into(),
                                    Type::Show => spotify.get_show(&uri).await?.into(),
                                    Type::User if uri.is_empty() => spotify.get_my_profile().await?.into_simple().into(),
                                    Type::User => spotify.get_user_profile(&uri).await?.into(),
                                    _ => unreachable!(),
                                };

                                Ok::<_, ClientError>(Box::new(reply))
                            })
                            .await??;

                        stream.emit(NewContext(play_context));

                        Ok(())
                    },
                );

                self.model.play_context = None;
                self.model.play_context_cover = None;
            }
            NewContext(context) => {
                let images = context.images();

                if let Some(cover_url) = self.model.image_loaders[0].find_best_thumb(images) {
                    self.model.stream.emit(LoadCover(cover_url.to_owned(), false));
                }

                self.model.play_context = Some(*context);
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
                let (kind, context_info) = self.model.play_context.as_ref().map_or((Type::Track, None), |ctx| {
                    (ctx.kind(), Some((ctx.uri().to_owned(), ctx.name().to_owned())))
                });

                self.model.stream.emit(MediaControlsMsg::GoToTrack(kind, uri, context_info));
            }
            ClickTrackUri(None) => {}
            GoToTrack(..) => {}
        }
    }

    fn init_view(&mut self) {
        let stream = self.model.stream.clone();
        let cell = gtk::CellRendererText::new();

        self.device_selector.pack_start(&cell, true);
        self.device_selector.add_attribute(&cell, "text", 1i32);

        self.buttons.get_style_context().add_class("linked");

        stream.emit(MediaControlsMsg::Reload);
    }
}

impl Extract<EventStream<MediaControlsMsg>> for MediaControls {
    fn extract(&self) -> EventStream<MediaControlsMsg> { self.model.stream.clone() }
}

impl Extract<SpotifyRef> for MediaControls {
    fn extract(&self) -> SpotifyRef { self.model.spotify.clone() }
}

impl Spawn for MediaControls {
    fn pool(&self) -> Handle { self.model.pool.clone() }
}
