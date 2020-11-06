use crate::components::spotify::SpotifyProxy;
use crate::components::track_list::{TrackList, TrackListMsg};
use relm::Widget;
use relm_derive::{widget, Msg};
use rspotify::model::track::SavedTrack;
use std::sync::Arc;

#[derive(Msg)]
pub enum FavoritesMsg {
    ShowTab,
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
        }
    }

    view! {
        #[name="tracks"]
        TrackList::<SavedTrack>(self.model.spotify.clone())
    }
}
