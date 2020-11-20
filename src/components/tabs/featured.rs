use crate::components::lists::common::ContainerListMsg;
use crate::components::lists::playlist::PlaylistList;
use crate::components::lists::track::{TrackList, TrackListMsg};
use crate::loaders::playlist::FeaturedLoader;
use crate::loaders::track::PlaylistLoader;
use crate::servers::spotify::SpotifyProxy;
use gtk::prelude::*;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use std::sync::Arc;

#[derive(Msg)]
pub enum FeaturedMsg {
    ShowTab,
    OpenPlaylist(String, String),
    GoToTrack(String),
}

pub struct FeaturedModel {
    stream: EventStream<FeaturedMsg>,
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for FeaturedTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> FeaturedModel {
        let stream = relm.stream().clone();
        FeaturedModel { stream, spotify }
    }

    fn update(&mut self, event: FeaturedMsg) {
        use FeaturedMsg::*;
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
                PlaylistList::<FeaturedLoader>(self.model.spotify.clone()) {
                    child: { title: Some("Featured") },
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
            ContainerListMsg::ActivateItem(uri, name) => {
                stream.emit(FeaturedMsg::OpenPlaylist(uri.clone(), name.clone()));
            }
            _ => {}
        });
    }
}
