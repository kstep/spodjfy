use crate::components::lists::{ContainerMsg, TrackList, TrackMsg};
use crate::loaders::SavedTracksLoader as SavedLoader;
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
                self.tracks.emit(ContainerMsg::Reload.into());
            }
            FavoritesMsg::GoToTrack(uri) => {
                self.tracks.emit(ContainerMsg::Load(()).into());
                self.tracks.emit(TrackMsg::GoToTrack(uri));
            }
        }
    }

    view! {
        #[name="tracks"]
        TrackList::<SavedLoader>(self.model.spotify.clone())
    }
}
