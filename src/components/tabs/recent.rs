use crate::components::track_list::{TrackList, TrackListMsg};
use crate::servers::spotify::SpotifyProxy;
use relm::Widget;
use relm_derive::{widget, Msg};
use rspotify::model::playing::PlayHistory;
use std::sync::Arc;

#[derive(Msg)]
pub enum RecentMsg {
    ShowTab,
    GoToTrack(String),
}

#[widget]
impl Widget for RecentTab {
    fn model(spotify: Arc<SpotifyProxy>) -> Arc<SpotifyProxy> {
        spotify
    }

    fn update(&mut self, event: RecentMsg) {
        use RecentMsg::*;
        match event {
            ShowTab => {
                self.recent_view.emit(TrackListMsg::Reset((), true));
            }
            GoToTrack(uri) => {
                self.recent_view.emit(TrackListMsg::GoToTrack(uri));
            }
        }
    }

    view! {
        #[name="recent_view"]
        TrackList::<PlayHistory, Vec<PlayHistory>>(self.model.clone()),
    }
}
