use crate::components::lists::{ContainerMsg, TrackList, TrackMsg};
use crate::components::tabs::MusicTabMsg;
use crate::loaders::{MyTopTracksLoader, SavedTracksLoader as SavedLoader};
use crate::servers::spotify::SpotifyProxy;
use gtk::prelude::*;
use relm::{Relm, Widget};
use relm_derive::widget;
use std::sync::Arc;

pub struct FavoritesModel {
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for TracksTab {
    fn model(spotify: Arc<SpotifyProxy>) -> FavoritesModel {
        FavoritesModel { spotify }
    }

    fn update(&mut self, event: MusicTabMsg) {
        use MusicTabMsg::*;
        match event {
            ShowTab => {
                self.saved_tracks_view.emit(ContainerMsg::Load(()).into());
                self.top_tracks_view.emit(ContainerMsg::Load(()).into());
            }
            GoToTrack(uri) => {
                self.saved_tracks_view.emit(ContainerMsg::Load(()).into());
                self.saved_tracks_view.emit(TrackMsg::GoToTrack(uri));
            }
            _ => {}
        }
    }

    view! {
        gtk::Box(gtk::Orientation::Vertical, 1) {
            #[name="breadcrumb"]
            gtk::StackSwitcher {},

            #[name="stack"]
            gtk::Stack {
                vexpand: true,

                #[name="saved_tracks_view"]
                TrackList::<SavedLoader>(self.model.spotify.clone()) {
                    child: {
                        title: Some("Saved Tracks"),
                    }
                },
                #[name="top_tracks_view"]
                TrackList::<MyTopTracksLoader>(self.model.spotify.clone()) {
                    child: {
                        title: Some("Top Tracks"),
                    }
                },
            },
        },
    }

    fn init_view(&mut self) {
        self.breadcrumb.set_stack(Some(&self.stack));
    }

    fn subscriptions(&mut self, relm: &Relm<Self>) {
        let stream = relm.stream().clone();
        self.saved_tracks_view.stream().observe(move |msg| {
            if let TrackMsg::PlayingNewTrack = msg {
                stream.emit(MusicTabMsg::PlaybackUpdate);
            }
        });

        let stream = relm.stream().clone();
        self.top_tracks_view.stream().observe(move |msg| {
            if let TrackMsg::PlayingNewTrack = msg {
                stream.emit(MusicTabMsg::PlaybackUpdate);
            }
        });
    }
}
