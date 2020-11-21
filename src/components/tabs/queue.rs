use crate::components::lists::{ContainerMsg, TrackList, TrackMsg};
use crate::loaders::QueueLoader;
use crate::servers::spotify::SpotifyProxy;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::Arc;

#[derive(Msg)]
pub enum QueueMsg {
    ShowTab,
    GoToTrack(String),
}

pub struct QueueModel {
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for QueueTab {
    fn model(spotify: Arc<SpotifyProxy>) -> QueueModel {
        QueueModel { spotify }
    }

    fn update(&mut self, event: QueueMsg) {
        match event {
            QueueMsg::ShowTab => {
                self.tracks.emit(ContainerMsg::Reset((), true).into());
            }
            QueueMsg::GoToTrack(uri) => {
                self.tracks.emit(TrackMsg::GoToTrack(uri));
            }
        }
    }

    view! {
        #[name="tracks"]
        TrackList::<QueueLoader>(self.model.spotify.clone())
    }
}
