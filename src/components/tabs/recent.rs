use crate::components::track_list::{TrackList, TrackListMsg};
use crate::servers::spotify::SpotifyProxy;
use glib::IsA;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::playing::PlayHistory;
use std::sync::Arc;

#[derive(Msg)]
pub enum RecentMsg {
    ShowTab,
    GoToTrack(String),
}

pub struct RecentModel {
    spotify: Arc<SpotifyProxy>,
    stream: EventStream<RecentMsg>,
}

#[widget]
impl Widget for RecentTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> RecentModel {
        RecentModel {
            spotify,
            stream: relm.stream().clone(),
        }
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
        TrackList::<PlayHistory, Vec<PlayHistory>>(self.model.spotify.clone()),
    }

    fn on_add<W: IsA<gtk::Widget> + IsA<glib::Object>>(&self, _parent: W) {
        self.model.stream.emit(RecentMsg::ShowTab);
    }
}
