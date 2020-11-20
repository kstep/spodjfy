use crate::components::lists::common::ContainerListMsg;
use crate::components::lists::track::{TrackList, TrackMsg};
use crate::loaders::track::SavedLoader;
use crate::servers::spotify::SpotifyProxy;
use relm::Widget;
use relm_derive::{widget, Msg};
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
                self.tracks.emit(ContainerListMsg::Reset((), true));
            }
            FavoritesMsg::GoToTrack(uri) => {
                self.tracks
                    .emit(ContainerListMsg::Custom(TrackMsg::GoToTrack(uri)));
            }
        }
    }

    view! {
        #[name="tracks"]
        TrackList::<SavedLoader>(self.model.spotify.clone())
    }
}
