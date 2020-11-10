use crate::components::spotify::{SpotifyCmd, SpotifyProxy};
use crate::components::track_list::{TrackList, TrackListMsg};
use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{ButtonBoxExt, ImageExt, RangeExt, ScaleExt, WidgetExt};
use itertools::Itertools;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::album::FullAlbum;
use rspotify::model::artist::FullArtist;
use rspotify::model::context::{Context, CurrentlyPlaybackContext};
use rspotify::model::device::Device;
use rspotify::model::image::Image;
use rspotify::model::playlist::{FullPlaylist, PlaylistTrack};
use rspotify::model::show::FullEpisode;
use rspotify::model::track::FullTrack;
use rspotify::model::PlayingItem;
use rspotify::senum::{RepeatState, Type};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum PlayContext {
    Album(FullAlbum),
    Playlist(FullPlaylist),
    Artist(FullArtist),
}

impl PlayContext {
    fn name(&self) -> &str {
        match self {
            PlayContext::Album(ctx) => &*ctx.name,
            PlayContext::Artist(ctx) => &*ctx.name,
            PlayContext::Playlist(ctx) => &*ctx.name,
        }
    }

    fn genres(&self) -> Option<&Vec<String>> {
        match self {
            PlayContext::Album(ctx) => Some(&ctx.genres),
            PlayContext::Artist(ctx) => Some(&ctx.genres),
            PlayContext::Playlist(_) => None,
        }
    }

    fn images(&self) -> &Vec<Image> {
        match self {
            PlayContext::Album(ctx) => &ctx.images,
            PlayContext::Artist(ctx) => &ctx.images,
            PlayContext::Playlist(ctx) => &ctx.images,
        }
    }

    fn tracks_number(&self) -> u32 {
        match self {
            PlayContext::Album(ctx) => ctx.tracks.total,
            PlayContext::Artist(_) => 0,
            PlayContext::Playlist(ctx) => ctx.tracks.total,
        }
    }
}

#[derive(Msg)]
pub enum NowPlayingMsg {
    ShowTab,
    LoadState,
    NewState(Box<Option<CurrentlyPlaybackContext>>),
    NewDevices(Vec<Device>),
    UseDevice(Option<String>),
    LoadCover(String, bool),
    NewCover(Pixbuf, bool),
    Play,
    Pause,
    PrevTrack,
    NextTrack,
    LoadTracks(Type, String),
    LoadContext(Type, String),
    NewContext(Box<PlayContext>),
    Tick(u32),
    SeekTrack(u32),
    SetVolume(u8),
    SetShuffle(bool),
    ToggleRepeatMode,
    GoToTrack(Option<String>),
}

pub struct NowPlayingModel {
    stream: EventStream<NowPlayingMsg>,
    devices: gtk::ListStore,
    spotify: Arc<SpotifyProxy>,
    state: Option<CurrentlyPlaybackContext>,
    context: Option<PlayContext>,
    track_cover: Option<Pixbuf>,
    context_cover: Option<Pixbuf>,
}

const TRACK_COVER_SIZE: i32 = 256;
const CONTEXT_COVER_SIZE: i32 = 128;

#[widget]
impl Widget for NowPlayingTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> NowPlayingModel {
        let stream = relm.stream().clone();

        let _update_timer = {
            let stream = stream.clone();
            let mut counter = 0;
            glib::timeout_add_seconds_local(1, move || {
                counter = (counter + 1) % 10;
                if counter == 0 {
                    stream.emit(NowPlayingMsg::LoadState);
                } else {
                    stream.emit(NowPlayingMsg::Tick(1));
                }
                Continue(true)
            })
        };

        let devices = gtk::ListStore::new(&[String::static_type(), String::static_type()]);

        NowPlayingModel {
            stream,
            spotify,
            devices,
            state: None,
            context: None,
            track_cover: None,
            context_cover: None,
        }
    }

    fn update(&mut self, event: NowPlayingMsg) {
        use NowPlayingMsg::*;
        match event {
            ShowTab => {
                self.model.stream.emit(LoadState);
                self.model.spotify.ask(
                    self.model.stream.clone(),
                    |tx| SpotifyCmd::GetMyDevices { tx },
                    NewDevices,
                )
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
                        if state.device.id != id {
                            state.device.id = id.clone();
                            self.model.spotify.tell(SpotifyCmd::UseDevice { id });
                        }
                    }
                }
            }
            LoadState => {
                self.model.spotify.ask(
                    self.model.stream.clone(),
                    move |tx| SpotifyCmd::GetPlaybackState { tx },
                    |reply| NewState(Box::new(reply)),
                );
            }
            NewState(state) => {
                let old_state: Option<&CurrentlyPlaybackContext> = self.model.state.as_ref();
                let old_track_uri = match old_state {
                    Some(CurrentlyPlaybackContext {
                        item: Some(PlayingItem::Track(FullTrack { uri: track_uri, .. })),
                        ..
                    })
                    | Some(CurrentlyPlaybackContext {
                        item: Some(PlayingItem::Episode(FullEpisode { uri: track_uri, .. })),
                        ..
                    }) => track_uri.as_str(),
                    _ => "",
                };
                let old_context_uri = match old_state {
                    Some(CurrentlyPlaybackContext {
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
                            crate::utils::find_best_thumb(
                                track.album.images.iter(),
                                TRACK_COVER_SIZE,
                            ),
                            track.duration_ms,
                            &*track.uri,
                        ),
                        PlayingItem::Episode(episode) => (
                            crate::utils::find_best_thumb(episode.images.iter(), TRACK_COVER_SIZE),
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
                    }
                }

                if let Some(ctx) = state.as_ref().as_ref().and_then(|s| s.context.as_ref()) {
                    if ctx.uri != old_context_uri {
                        self.model
                            .stream
                            .emit(LoadContext(ctx._type, ctx.uri.clone()));
                    }
                }

                self.model.state = *state;
            }
            LoadCover(url, is_for_track) => {
                let pixbuf = crate::utils::pixbuf_from_url(
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
                self.model.spotify.tell(SpotifyCmd::SeekTrack { pos });

                if let Some(CurrentlyPlaybackContext {
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
                self.model.spotify.tell(SpotifyCmd::SetVolume { value });
            }
            SetShuffle(state) => {
                if let Some(st) = self.model.state.as_mut() {
                    st.shuffle_state = state;
                }
                self.model.spotify.tell(SpotifyCmd::SetShuffle { state });
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
                self.model.spotify.tell(SpotifyCmd::SetRepeatMode { mode });
            }
            GoToTrack(track_uri) => {
                if let Some(track_uri) = track_uri {
                    self.tracks_view.emit(TrackListMsg::GoToTrack(track_uri));
                }
            }
            Play => {
                self.model.spotify.tell(SpotifyCmd::StartPlayback);
                self.model.stream.emit(LoadState);
            }
            Pause => {
                self.model.spotify.tell(SpotifyCmd::PausePlayback);
                self.model.stream.emit(LoadState);
            }
            NextTrack => {
                self.model.spotify.tell(SpotifyCmd::PlayNextTrack);
                self.model.stream.emit(LoadState);
            }
            PrevTrack => {
                self.model.spotify.tell(SpotifyCmd::PlayPrevTrack);
                self.model.stream.emit(LoadState);
            }
            LoadTracks(kind, uri) => {
                match kind {
                    Type::Playlist => self.tracks_view.emit(TrackListMsg::Reset(uri, true)),
                    // TODO: sources for other context types:
                    Type::Album => (),
                    Type::Artist => (),
                    Type::User => (),
                    Type::Show => (),
                    _ => (),
                }
            }
            LoadContext(kind, uri) => {
                let stream = &self.model.stream;

                {
                    let uri = uri.clone();
                    match kind {
                        Type::Playlist => self.model.spotify.ask(
                            stream.clone(),
                            |tx| SpotifyCmd::GetPlaylist { tx, uri },
                            |reply| NewContext(Box::new(PlayContext::Playlist(reply))),
                        ),
                        Type::Album => self.model.spotify.ask(
                            stream.clone(),
                            |tx| SpotifyCmd::GetAlbum { tx, uri },
                            |reply| NewContext(Box::new(PlayContext::Album(reply))),
                        ),
                        Type::Artist => self.model.spotify.ask(
                            stream.clone(),
                            |tx| SpotifyCmd::GetArtist { tx, uri },
                            |reply| NewContext(Box::new(PlayContext::Artist(reply))),
                        ),
                        _ => {
                            self.model.context = None;
                        }
                    };
                }

                stream.emit(LoadTracks(kind, uri));
            }
            NewContext(context) => {
                let images = context.images();
                if let Some(cover_url) = crate::utils::find_best_thumb(images, CONTEXT_COVER_SIZE) {
                    self.model
                        .stream
                        .emit(LoadCover(cover_url.to_owned(), false));
                }
                self.model.context = Some(*context);
            }
            Tick(timeout) => {
                if let Some(CurrentlyPlaybackContext {
                    is_playing: true,
                    progress_ms: Some(ref mut progress),
                    ..
                }) = self.model.state
                {
                    *progress += timeout * 1000;
                    self.track_seek_bar.set_value(*progress as f64);
                }
            }
        }
    }

    view! {
        gtk::Box(gtk::Orientation::Vertical, 10) {
            gtk::Box(gtk::Orientation::Horizontal, 10) {
                halign: gtk::Align::Start,
                margin_top: 15,
                margin_start: 15,
                #[name="track_cover_image"]
                gtk::Image {
                    from_pixbuf: self.model.track_cover.as_ref()
                },
                #[name="track_infobox"]
                gtk::Box(gtk::Orientation::Vertical, 10) {
                    halign: gtk::Align::Start,
                    #[name="track_name_label"]
                    gtk::LinkButton {
                        widget_name: "track_name_label",
                        halign: gtk::Align::Start,
                        hexpand: true,
                        label: self.model.state.as_ref().and_then(|s| s.item.as_ref()).map(|it| match it {
                            PlayingItem::Track(track) => &*track.name,
                            PlayingItem::Episode(episode) => &*episode.name,
                        }).unwrap_or("<Nothing>"),
                        uri: self.model.state.as_ref().and_then(|s| s.item.as_ref()).map(|it| match it {
                            PlayingItem::Track(track) => &*track.uri,
                            PlayingItem::Episode(episode) => &*episode.uri,
                        }).unwrap_or(""),

                        activate_link(btn) => (NowPlayingMsg::GoToTrack(btn.get_uri().map(|u| u.into())), Inhibit(true)),
                    },
                    #[name="track_artists_label"]
                    gtk::Label {
                        halign: gtk::Align::Start,
                        text: self.model.state.as_ref().and_then(|s| s.item.as_ref()).map(|it| match it {
                            PlayingItem::Track(track) => track.artists.iter().map(|artist| &artist.name).join(", "),
                            PlayingItem::Episode(episode) => episode.show.publisher.clone(),
                        }).as_deref().unwrap_or("<Unknown Artist>")
                    },
                    #[name="track_album_label"]
                    gtk::Label {
                        widget_name: "track_album_label",
                        halign: gtk::Align::Start,
                        text: self.model.state.as_ref().and_then(|s| s.item.as_ref()).map(|it| match it {
                            PlayingItem::Track(track) => &*track.album.name,
                            PlayingItem::Episode(episode) => &*episode.show.name,
                        }).unwrap_or("")
                    },
                },
                #[name="context_cover_image"]
                gtk::Image {
                    valign: gtk::Align::Start,
                    halign: gtk::Align::End,
                    from_pixbuf: self.model.context_cover.as_ref(),
                },
                #[name="context_infobox"]
                gtk::Box(gtk::Orientation::Vertical, 10) {
                    halign: gtk::Align::End,

                    gtk::Box(gtk::Orientation::Horizontal, 5) {
                        gtk::Image {
                            from_pixbuf: gtk::IconTheme::new()
                                .load_icon(match self.model.context {
                                    Some(PlayContext::Album(_)) => "media-optical",
                                    Some(PlayContext::Playlist(_)) => "folder-music",
                                    Some(PlayContext::Artist(_)) => "emblem-music",
                                    None => "emblem-music",
                                }, 24, gtk::IconLookupFlags::empty())
                                .ok()
                                .flatten()
                                .as_ref(),
                        },
                        #[name="context_name_label"]
                        gtk::Label {
                            widget_name: "context_name_label",
                            line_wrap: true,
                            property_width_request: 200,
                            halign: gtk::Align::Start,
                            text: self.model.context.as_ref().map(|c| c.name()).unwrap_or(""),

                        },
                    },
                    #[name="context_tracks_number_label"]
                    gtk::Label {
                        halign: gtk::Align::Start,
                        text: self.model.context.as_ref()
                            .map(|c| match c.tracks_number() {
                                0 => String::new(),
                                n => format!("Tracks: {}", n),
                            })
                            .as_deref()
                            .unwrap_or("")
                    },
                    #[name="context_genres_label"]
                    gtk::Label {
                        halign: gtk::Align::Start,
                        text: self.model.context.as_ref()
                            .and_then(|c| c.genres())
                            .map(|gs| gs.iter().join(", "))
                            .as_deref()
                            .unwrap_or("")
                    },
                }
            },
            gtk::Box(gtk::Orientation::Horizontal, 10) {
                #[name="track_seek_bar"]
                gtk::Scale(gtk::Orientation::Horizontal, Some(&gtk::Adjustment::new(0.0, 0.0, 300000.0, 1000.0, 1000.0, 1000.0))) {
                    margin_start: 10,
                    hexpand: true,
                    valign: gtk::Align::Center,
                    value_pos: gtk::PositionType::Left,
                    value: self.model.state.as_ref().and_then(|s| s.progress_ms).unwrap_or(0) as f64,

                    change_value(_, _, pos) => (NowPlayingMsg::SeekTrack(pos as u32), Inhibit(false))
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
                    #[name="prev_track_btn"]
                    gtk::Button {
                        tooltip_text: Some("Previous track"),
                        image: Some(&gtk::Image::from_icon_name(Some("media-skip-backward"), gtk::IconSize::LargeToolbar)),
                        clicked(_) => NowPlayingMsg::PrevTrack,
                    },
                    #[name="pause_btn"]
                    gtk::Button {
                        tooltip_text: Some("Pause"),
                        image: Some(&gtk::Image::from_icon_name(Some("media-playback-pause"), gtk::IconSize::LargeToolbar)),
                        clicked(_) => NowPlayingMsg::Pause,
                    },
                    #[name="play_btn"]
                    gtk::Button {
                        widget_name: "play_btn",
                        tooltip_text: Some("Play"),
                        image: Some(&gtk::Image::from_icon_name(Some("media-playback-start"), gtk::IconSize::LargeToolbar)),
                        child: {
                            non_homogeneous: true,
                        },
                        clicked(_) => NowPlayingMsg::Play,
                    },
                    #[name="next_track_btn"]
                    gtk::Button {
                        tooltip_text: Some("Next track"),
                        image: Some(&gtk::Image::from_icon_name(Some("media-skip-forward"), gtk::IconSize::LargeToolbar)),
                        clicked(_) => NowPlayingMsg::NextTrack,
                    },

                    #[name="shuffle_btn"]
                    gtk::ToggleButton {
                        tooltip_text: Some("Shuffle"),
                        image: Some(&gtk::Image::from_icon_name(Some("media-playlist-shuffle"), gtk::IconSize::LargeToolbar)),
                        active: self.model.state.as_ref().map(|s| s.shuffle_state).unwrap_or(false),
                        toggled(btn) => NowPlayingMsg::SetShuffle(btn.get_active()),
                    },
                    #[name="repeat_btn"]
                    gtk::ToggleButton {
                        tooltip_text: Some("Repeat mode"),
                        image: Some(&gtk::Image::from_icon_name(
                            self.model.state.as_ref()
                                .map(|s| {
                                    if s.repeat_state == RepeatState::Track {
                                        "media-playlist-repeat-song"
                                    } else {
                                        "media-playlist-repeat"
                                    }
                                }),
                            gtk::IconSize::LargeToolbar)),
                        active: self.model.state.as_ref().map(|s| s.repeat_state != RepeatState::Off).unwrap_or(false),
                        toggled(_) => NowPlayingMsg::ToggleRepeatMode,
                    },
                },

                gtk::Scale(gtk::Orientation::Horizontal, Some(&gtk::Adjustment::new(0.0, 0.0, 101.0, 1.0, 1.0, 1.0))) {
                    tooltip_text: Some("Volume"),
                    digits: 0,
                    value: self.model.state.as_ref().map(|s| s.device.volume_percent as f64).unwrap_or(0.0),
                    property_width_request: 200,
                    valign: gtk::Align::Center,
                    vexpand: false,

                    change_value(_, _, pos) => (NowPlayingMsg::SetVolume(pos as u8), Inhibit(false)),
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

                    changed(combo) => NowPlayingMsg::UseDevice(combo.get_active_id().map(|id| id.into())),
                },
            },
            // TODO: make an universal component out of this window
            #[name="tracks_view"]
            TrackList::<PlaylistTrack>(self.model.spotify.clone()),
        }
    }

    fn init_view(&mut self) {
        let stream = self.model.stream.clone();

        self.track_seek_bar.connect_format_value(|seek, value| {
            let value = value as u32;
            let duration = seek.get_adjustment().get_upper() as u32;
            format!(
                "{} / -{}",
                crate::utils::humanize_time(value),
                crate::utils::humanize_time(duration - value)
            )
        });
        self.tracks_view.stream().observe(move |msg| {
            if let TrackListMsg::PlayingNewTrack = msg {
                stream.emit(NowPlayingMsg::LoadState);
            }
        });

        let cell = gtk::CellRendererText::new();
        self.device_selector.pack_start(&cell, true);
        self.device_selector.add_attribute(&cell, "text", 1 as i32);

        self.buttons.get_style_context().add_class("linked");
    }
}
