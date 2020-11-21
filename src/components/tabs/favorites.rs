use crate::components::lists::{ContainerMsg, TrackList, TrackMsg};
use crate::components::tabs::MusicTabMsg;
use crate::loaders::SavedTracksLoader as SavedLoader;
use crate::servers::spotify::SpotifyProxy;
use relm::{Relm, Widget};
use relm_derive::widget;
use std::sync::Arc;

pub struct FavoritesModel {
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for FavoritesTab {
    fn model(spotify: Arc<SpotifyProxy>) -> FavoritesModel {
        FavoritesModel { spotify }
    }

    fn update(&mut self, event: MusicTabMsg) {
        use MusicTabMsg::*;
        match event {
            ShowTab => {
                self.tracks_view.emit(ContainerMsg::Load(()).into());
            }
            GoToTrack(uri) => {
                self.tracks_view.emit(ContainerMsg::Load(()).into());
                self.tracks_view.emit(TrackMsg::GoToTrack(uri));
            }
            _ => {}
        }
    }

    view! {
        #[name="tracks_view"]
        TrackList::<SavedLoader>(self.model.spotify.clone())
    }

    fn subscriptions(&mut self, relm: &Relm<Self>) {
        let stream = relm.stream().clone();
        self.tracks_view.stream().observe(move |msg| {
            if let TrackMsg::PlayingNewTrack = msg {
                stream.emit(MusicTabMsg::PlaybackUpdate);
            }
        });
    }
}
