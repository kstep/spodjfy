use crate::components::track_list::{TrackList, TrackListMsg};
use crate::loaders::track::RecentLoader;
use crate::servers::spotify::SpotifyProxy;
use glib::IsA;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::Arc;

#[derive(Msg)]
pub enum RecentMsg {
    ShowTab,
    GoToTrack(String),
}

pub struct RecentModel {
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for RecentTab {
    fn model(spotify: Arc<SpotifyProxy>) -> RecentModel {
        RecentModel { spotify }
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
        TrackList::<RecentLoader>(self.model.spotify.clone()),
    }

    fn on_add<W: IsA<gtk::Widget> + IsA<glib::Object>>(&self, _parent: W) {
        //self.recent_view.emit(TrackListMsg::Reset((), true));
    }
}
