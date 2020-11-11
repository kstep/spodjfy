use crate::components::track_list::{TrackList, TrackListMsg};
use crate::spotify::SpotifyProxy;
use relm::Widget;
use relm_derive::{widget, Msg};
use rspotify::model::track::SavedTrack;
use std::sync::Arc;

#[derive(Msg)]
pub enum FavoritesMsg {
    ShowTab,
    GoToTrack(String),
}

pub struct FavoritesModel {
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for FavoritesTab {
    fn model(spotify: Arc<SpotifyProxy>) -> FavoritesModel {
        FavoritesModel { spotify }
    }

    fn update(&mut self, event: FavoritesMsg) {
        match event {
            FavoritesMsg::ShowTab => {
                self.tracks.emit(TrackListMsg::Reload);
            }
            FavoritesMsg::GoToTrack(uri) => {
                self.tracks.emit(TrackListMsg::GoToTrack(uri));
            }
        }
    }

    view! {
        #[name="tracks"]
        TrackList::<SavedTrack>(self.model.spotify.clone())
    }
}
