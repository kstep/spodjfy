use crate::components::lists::common::ContainerListMsg;
use crate::components::lists::track::{TrackList, TrackMsg};
use crate::loaders::track::MyTopTracksLoader;
use crate::servers::spotify::SpotifyProxy;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::Arc;

#[derive(Msg)]
pub enum TopTracksMsg {
    ShowTab,
    GoToTrack(String),
}

pub struct TopTracksModel {
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for TopTracksTab {
    fn model(spotify: Arc<SpotifyProxy>) -> TopTracksModel {
        TopTracksModel { spotify }
    }

    fn update(&mut self, event: TopTracksMsg) {
        match event {
            TopTracksMsg::ShowTab => {
                self.tracks.emit(ContainerListMsg::Reset((), true));
            }
            TopTracksMsg::GoToTrack(uri) => {
                self.tracks
                    .emit(ContainerListMsg::Custom(TrackMsg::GoToTrack(uri)));
            }
        }
    }

    view! {
        #[name="tracks"]
        TrackList::<MyTopTracksLoader>(self.model.spotify.clone())
    }
}
