use crate::components::lists::{ContainerMsg, TrackList, TrackMsg};
use crate::components::tabs::MusicTabMsg;
use crate::loaders::RecentLoader;
use crate::servers::spotify::SpotifyProxy;
use relm::Widget;
use relm_derive::widget;
use std::sync::Arc;

pub struct RecentModel {
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for RecentTab {
    fn model(spotify: Arc<SpotifyProxy>) -> RecentModel {
        RecentModel { spotify }
    }

    fn update(&mut self, event: MusicTabMsg) {
        use MusicTabMsg::*;
        match event {
            ShowTab => {
                self.recent_view.emit(ContainerMsg::Load(()).into());
            }
            GoToTrack(uri) => {
                self.recent_view.emit(TrackMsg::GoToTrack(uri));
            }
            _ => {}
        }
    }

    view! {
        #[name="recent_view"]
        TrackList::<RecentLoader>(self.model.spotify.clone()),
    }
}
