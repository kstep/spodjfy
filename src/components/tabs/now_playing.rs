use crate::components::spotify::{SpotifyCmd, SpotifyProxy};
use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{
    CellRendererPixbuf, ImageExt, RangeExt, ScaleExt, TreeView, TreeViewColumn,
    TreeViewColumnBuilder, TreeViewExt, WidgetExt,
};
use itertools::Itertools;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::context::{Context, CurrentlyPlaybackContext};
use rspotify::model::page::Page;
use rspotify::model::show::FullEpisode;
use rspotify::model::track::FullTrack;
use rspotify::model::PlayingItem;
use rspotify::senum::Type;
use std::sync::Arc;

#[derive(Msg)]
pub enum NowPlayingMsg {
    ShowTab,
    LoadState,
    NewState(Option<CurrentlyPlaybackContext>),
    LoadCover(String),
    NewCover(Pixbuf),
    Click(gdk::EventButton),
    Play,
    Pause,
    PrevTrack,
    NextTrack,
    LoadTracks(Type, String),
    NewTracks(Page<FullTrack>),
    Tick(u32),
    SeekTrack(u32),
}

pub struct NowPlayingModel {
    stream: EventStream<NowPlayingMsg>,
    spotify: Arc<SpotifyProxy>,
    state: Option<CurrentlyPlaybackContext>,
    cover: Option<Pixbuf>,
    tracks_store: gtk::ListStore,
    update_timer: glib::SourceId,
}

const PAGE_LIMIT: u32 = 20;
const THUMB_SIZE: i32 = 32;
const COVER_SIZE: i32 = 256;

const COL_TRACK_ID: u32 = 0;
const COL_TRACK_THUMB: u32 = 1;
const COL_TRACK_NAME: u32 = 2;
const COL_TRACK_ARTISTS: u32 = 3;
const COL_TRACK_NUMBER: u32 = 4;
const COL_TRACK_ALBUM: u32 = 5;
const COL_TRACK_CAN_PLAY: u32 = 6;
const COL_TRACK_DURATION: u32 = 7;
const COL_TRACK_DURATION_MS: u32 = 8;
const COL_TRACK_URI: u32 = 9;
const COL_TRACK_BPM: u32 = 10;

#[widget]
impl Widget for NowPlayingTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> NowPlayingModel {
        let stream = relm.stream().clone();
        let tracks_store = gtk::ListStore::new(&[
            String::static_type(),
            Pixbuf::static_type(),
            String::static_type(),
            String::static_type(),
            u32::static_type(),
            String::static_type(),
            bool::static_type(),
            String::static_type(),
            u32::static_type(),
            String::static_type(),
            f32::static_type(),
        ]);
        let update_timer = {
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
        NowPlayingModel {
            stream,
            spotify,
            state: None,
            cover: None,
            tracks_store,
            update_timer,
        }
    }

    fn update(&mut self, event: NowPlayingMsg) {
        use NowPlayingMsg::*;
        match event {
            ShowTab => {
                self.model.tracks_store.clear();
                self.model.stream.emit(LoadState);
            }
            LoadState => {
                self.model.spotify.ask(
                    self.model.stream.clone(),
                    move |tx| SpotifyCmd::GetPlaybackState { tx },
                    NewState,
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

                if let Some(item) = state.as_ref().and_then(|s| s.item.as_ref()) {
                    let (cover_url, duration_ms, track_uri) = match item {
                        PlayingItem::Track(track) => (
                            crate::utils::find_best_thumb(track.album.images.iter(), COVER_SIZE),
                            track.duration_ms,
                            &*track.uri,
                        ),
                        PlayingItem::Episode(episode) => (
                            crate::utils::find_best_thumb(episode.images.iter(), COVER_SIZE),
                            episode.duration_ms,
                            &*episode.uri,
                        ),
                    };

                    if track_uri != old_track_uri {
                        self.model.cover = None;

                        if let Some(url) = cover_url {
                            self.model.stream.emit(LoadCover(url.to_owned()));
                        }

                        self.track_seek_bar.set_range(0.0, duration_ms as f64);
                    }
                }

                if let Some(ctx) = state.as_ref().and_then(|s| s.context.as_ref()) {
                    if &ctx.uri != old_context_uri {
                        self.model.tracks_store.clear();
                        self.model
                            .stream
                            .emit(LoadTracks(ctx._type, ctx.uri.clone()));
                    }
                }

                self.model.state = state;
            }
            LoadCover(url) => {
                let pixbuf = crate::utils::pixbuf_from_url(&url, COVER_SIZE);
                if let Ok(cover) = pixbuf {
                    self.model.stream.emit(NewCover(cover));
                }
            }
            NewCover(cover) => {
                self.model.cover = Some(cover);
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
                println!("loading tracks for {:?} {}", kind, uri);
                match kind {
                    Type::Playlist => self.model.spotify.ask(
                        self.model.stream.clone(),
                        |tx| SpotifyCmd::GetPlaylistTracks {
                            tx,
                            uri,
                            offset: 0,
                            limit: PAGE_LIMIT,
                        },
                        NewTracks,
                    ),
                    _ => (), // TODO: sources for other context types
                }
            }
            Tick(timeout) => {
                // FIXME: it's a hack to make #[widget] insert view bindings here
                let mut state = self.model.state.take();
                if let Some(CurrentlyPlaybackContext {
                    is_playing: true,
                    progress_ms: Some(ref mut progress),
                    ..
                }) = state
                {
                    *progress += timeout * 1000;
                }
                self.model.state = state;
            }
            NewTracks(page) => {
                let stream = &self.model.stream;
                let store: &gtk::ListStore = &self.model.tracks_store;
                let tracks = page.items;

                //let mut uris = Vec::with_capacity(tracks.len());
                //let mut iters = Vec::with_capacity(tracks.len());

                for track in tracks {
                    let pos = store.insert_with_values(
                        None,
                        &[
                            COL_TRACK_ID,
                            COL_TRACK_NAME,
                            COL_TRACK_ARTISTS,
                            COL_TRACK_NUMBER,
                            COL_TRACK_ALBUM,
                            COL_TRACK_CAN_PLAY,
                            COL_TRACK_DURATION,
                            COL_TRACK_DURATION_MS,
                            COL_TRACK_URI,
                        ],
                        &[
                            &track.id,
                            &track.name,
                            &track.artists.iter().map(|artist| &artist.name).join(", "),
                            &track.track_number,
                            &track.album.name,
                            &track.is_playable.unwrap_or(false),
                            &crate::utils::humanize_time(track.duration_ms),
                            &track.duration_ms,
                            &track.uri,
                        ],
                    );

                    //let image = crate::utils::find_best_thumb(&track.album.images, THUMB_SIZE);

                    //if let Some(url) = image {
                    //    stream.emit(LoadThumb(url.to_owned(), pos.clone()));
                    //}

                    //uris.push(track.uri);
                    //iters.push(pos);
                }
            }
            Click(_) => {}
        }
    }

    view! {
        gtk::Box(gtk::Orientation::Vertical, 10) {
            gtk::Box(gtk::Orientation::Horizontal, 10) {
                halign: gtk::Align::Center,
                margin_top: 15,
                #[name="track_cover_image"]
                gtk::Image {
                    from_pixbuf: self.model.cover.as_ref()
                },
                gtk::Box(gtk::Orientation::Vertical, 10) {
                    halign: gtk::Align::Start,
                    #[name="track_name_label"]
                    gtk::Label {
                        widget_name: "track_name_label",
                        halign: gtk::Align::Start,
                        hexpand: true,
                        text: self.model.state.as_ref().and_then(|s| s.item.as_ref()).and_then(|it| match it {
                            PlayingItem::Track(track) => Some(&*track.name),
                            _ => None
                        }).unwrap_or("<Nothing>")
                    },
                    #[name="track_artists_label"]
                    gtk::Label {
                        halign: gtk::Align::Start,
                        text: self.model.state.as_ref().and_then(|s| s.item.as_ref()).and_then(|it| match it {
                            PlayingItem::Track(track) => Some(track.artists.iter().map(|artist| &artist.name).join(", ")),
                            _ => None
                        }).as_deref().unwrap_or("<Unknown Artist>")
                    },
                    #[name="track_album_label"]
                    gtk::Label {
                        widget_name: "track_album_label",
                        halign: gtk::Align::Start,
                        text: self.model.state.as_ref().and_then(|s| s.item.as_ref()).and_then(|it| match it {
                            PlayingItem::Track(track) => Some(&*track.album.name),
                            _ => None
                        }).unwrap_or("")
                    },
                    #[name="current_device_label"]
                    gtk::Label {
                        widget_name: "current_device_label",
                        halign: gtk::Align::Start,
                        text: self.model.state.as_ref().map(|s| &*s.device.name).unwrap_or("")
                    },
                },
            },
            gtk::Box(gtk::Orientation::Horizontal, 10) {
                #[name="track_seek_bar"]
                gtk::Scale(gtk::Orientation::Horizontal, Some(&gtk::Adjustment::new(0.0, 0.0, 300000.0, 1000.0, 5000.0, 5000.0))) {
                    margin_start: 10,
                    hexpand: true,
                    valign: gtk::Align::Center,
                    value_pos: gtk::PositionType::Left,
                    value: self.model.state.as_ref().and_then(|s| s.progress_ms).unwrap_or(0) as f64,

                    change_value(_, _, pos) => (NowPlayingMsg::SeekTrack(pos as u32), Inhibit(false))
                },
                gtk::Label {
                    margin_end: 10,
                    text: self.model.state.as_ref().and_then(|s| s.item.as_ref()).and_then(|it| match it {
                        PlayingItem::Track(track) => Some(track.duration_ms),
                        _ => None
                    }).map(crate::utils::humanize_time).as_deref().unwrap_or("??:??")
                },
            },
            gtk::Box(gtk::Orientation::Horizontal, 5) {
                halign: gtk::Align::Center,
                gtk::Button {
                    label: "« Prev",
                    clicked(_) => NowPlayingMsg::PrevTrack,
                },
                gtk::Button {
                    label: "Play",
                    clicked(_) => NowPlayingMsg::Play,
                },
                gtk::Button {
                    label: "Pause",
                    clicked(_) => NowPlayingMsg::Pause,
                },
                gtk::Button {
                    label: "Next »",
                    clicked(_) => NowPlayingMsg::NextTrack,
                },
            },
            // TODO: make an universal component out of this window
            gtk::ScrolledWindow {
                vexpand: true,
                #[name="tracks_view"]
                gtk::TreeView {
                    model: Some(&self.model.tracks_store),
                },
            },
        }
    }

    fn init_view(&mut self) {
        let tree: &TreeView = &self.tracks_view;
        tree.get_selection().set_mode(gtk::SelectionMode::Multiple);

        let base_column = TreeViewColumnBuilder::new()
            .resizable(true)
            .reorderable(true)
            .expand(true);

        tree.append_column(&{
            let icon_cell = CellRendererPixbuf::new();
            //icon_cell.set_property_icon_name(Some("audio-x-generic-symbolic"));

            let column = TreeViewColumn::new();
            column.pack_start(&icon_cell, true);
            column.add_attribute(&icon_cell, "pixbuf", COL_TRACK_THUMB as i32);
            column
        });

        tree.append_column(&{
            let text_cell = gtk::CellRendererText::new();
            let column = base_column
                .clone()
                .title("Title")
                .sort_column_id(COL_TRACK_NAME as i32)
                .build();
            column.pack_start(&text_cell, true);
            column.add_attribute(&text_cell, "text", COL_TRACK_NAME as i32);
            column
        });

        tree.append_column(&{
            let text_cell = gtk::CellRendererText::new();
            text_cell.set_alignment(1.0, 0.5);
            let column = base_column
                .clone()
                .title("Duration")
                .sort_column_id(COL_TRACK_DURATION_MS as i32)
                .build();
            column.pack_start(&text_cell, true);
            column.add_attribute(&text_cell, "text", COL_TRACK_DURATION as i32);
            column
        });

        tree.append_column(&{
            let text_cell = gtk::CellRendererText::new();
            text_cell.set_alignment(1.0, 0.5);
            let column = base_column
                .clone()
                .title("BPM")
                .sort_column_id(COL_TRACK_BPM as i32)
                .build();
            <TreeViewColumn as TreeViewColumnExt>::set_cell_data_func(
                &column,
                &text_cell,
                Some(Box::new(|_layout, cell, model, iter| {
                    let bpm: f32 = model
                        .get_value(iter, COL_TRACK_BPM as i32)
                        .get()
                        .ok()
                        .flatten()
                        .unwrap_or(0.0);
                    let _ = cell.set_property("text", &format!("{:.0}", bpm));
                })),
            );
            column.pack_start(&text_cell, true);
            column.add_attribute(&text_cell, "text", COL_TRACK_BPM as i32);
            column
        });

        tree.append_column(&{
            let text_cell = gtk::CellRendererText::new();
            let column = base_column
                .clone()
                .title("Artists")
                .sort_column_id(COL_TRACK_ARTISTS as i32)
                .build();
            column.pack_start(&text_cell, true);
            column.add_attribute(&text_cell, "text", COL_TRACK_ARTISTS as i32);
            column
        });

        tree.append_column(&{
            let text_cell = gtk::CellRendererText::new();
            let column = base_column
                .clone()
                .title("Album")
                .sort_column_id(COL_TRACK_ALBUM as i32)
                .build();
            column.pack_start(&text_cell, true);
            column.add_attribute(&text_cell, "text", COL_TRACK_ALBUM as i32);
            column
        });

        self.track_seek_bar
            .connect_format_value(|_, value| crate::utils::humanize_time(value as u32));
    }
}
