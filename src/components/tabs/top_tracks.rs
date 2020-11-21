use crate::components::lists::{ContainerMsg, TrackList, TrackMsg};
use crate::components::tabs::MusicTabMsg;
use crate::loaders::MyTopTracksLoader;
use crate::servers::spotify::SpotifyProxy;
use relm::{Relm, Widget};
use relm_derive::widget;
use std::sync::Arc;

pub struct TopTracksModel {
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for TopTracksTab {
    fn model(spotify: Arc<SpotifyProxy>) -> TopTracksModel {
        TopTracksModel { spotify }
    }

    fn update(&mut self, event: MusicTabMsg) {
        use MusicTabMsg::*;
        match event {
            ShowTab => {
                self.tracks_view.emit(ContainerMsg::Load(()).into());
            }
            GoToTrack(uri) => {
                self.tracks_view.emit(TrackMsg::GoToTrack(uri));
            }
            _ => {}
        }
    }

    view! {
        #[name="tracks_view"]
        TrackList::<MyTopTracksLoader>(self.model.spotify.clone())
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
