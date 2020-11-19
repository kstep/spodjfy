use crate::components::lists::common::ContainerListMsg;
use crate::components::lists::playlist::PlaylistList;
use crate::components::lists::track::{TrackList, TrackListMsg};
use crate::loaders::playlist::SavedLoader;
use crate::loaders::track::PlaylistLoader;
use crate::servers::spotify::SpotifyProxy;
use gtk::prelude::*;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use std::sync::Arc;

#[derive(Msg)]
pub enum PlaylistsMsg {
    ShowTab,
    OpenPlaylist(String, String),
    GoToTrack(String),
}

pub struct PlaylistsModel {
    stream: EventStream<PlaylistsMsg>,
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for PlaylistsTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> PlaylistsModel {
        let stream = relm.stream().clone();
        PlaylistsModel { stream, spotify }
    }

    fn update(&mut self, event: PlaylistsMsg) {
        use PlaylistsMsg::*;
        match event {
            ShowTab => {
                self.playlists_view.emit(ContainerListMsg::Reset((), true));
            }
            OpenPlaylist(uri, name) => {
                self.tracks_view.emit(TrackListMsg::Load(uri));

                let tracks_tab = self.tracks_view.widget();
                self.stack.set_child_title(tracks_tab, Some(&name));
                self.stack.set_visible_child(tracks_tab);
            }
            GoToTrack(uri) => {
                self.tracks_view.emit(TrackListMsg::GoToTrack(uri));
            }
        }
    }

    view! {
        gtk::Box(gtk::Orientation::Vertical, 1) {
            #[name="breadcrumb"]
            gtk::StackSwitcher {},
            #[name="stack"]
            gtk::Stack {
                vexpand: true,

                #[name="playlists_view"]
                PlaylistList::<SavedLoader>(self.model.spotify.clone()) {
                    child: { title: Some("Playlists") },
                },

                #[name="tracks_view"]
                TrackList::<PlaylistLoader>(self.model.spotify.clone()),
            },
        }
    }

    fn init_view(&mut self) {
        self.breadcrumb.set_stack(Some(&self.stack));

        let stream = self.model.stream.clone();
        self.playlists_view.stream().observe(move |msg| match msg {
            ContainerListMsg::OpenItem(uri, name) => {
                stream.emit(PlaylistsMsg::OpenPlaylist(uri.clone(), name.clone()));
            }
            _ => {}
        });
    }
}
