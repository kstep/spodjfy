use crate::components::lists::{ContainerMsg, TrackList, TrackMsg};
use crate::components::tabs::MusicTabMsg;
use crate::loaders::QueueLoader;
use crate::servers::spotify::SpotifyProxy;
use relm::Widget;
use relm_derive::widget;
use std::sync::Arc;

pub struct QueueModel {
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for QueueTab {
    fn model(spotify: Arc<SpotifyProxy>) -> QueueModel {
        QueueModel { spotify }
    }

    fn update(&mut self, event: MusicTabMsg) {
        use MusicTabMsg::*;
        match event {
            ShowTab => {
                self.tracks.emit(ContainerMsg::Load(()).into());
            }
            GoToTrack(uri) => {
                self.tracks.emit(TrackMsg::GoToTrack(uri));
            }
            _ => {}
        }
    }

    view! {
        #[name="tracks"]
        TrackList::<QueueLoader>(self.model.spotify.clone())
    }
}
