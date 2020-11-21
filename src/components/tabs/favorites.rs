use crate::components::lists::{ContainerMsg, TrackList, TrackMsg};
use crate::components::tabs::MusicTabMsg;
use crate::loaders::SavedTracksLoader as SavedLoader;
use crate::servers::spotify::SpotifyProxy;
use relm::Widget;
use relm_derive::widget;
use std::sync::Arc;

pub struct FavoritesModel {
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for FavoritesTab {
    fn model(spotify: Arc<SpotifyProxy>) -> FavoritesModel {
        FavoritesModel { spotify }
    }

    fn update(&mut self, event: MusicTabMsg) {
        use MusicTabMsg::*;
        match event {
            ShowTab => {
                self.tracks.emit(ContainerMsg::Load(()).into());
            }
            GoToTrack(uri) => {
                self.tracks.emit(ContainerMsg::Load(()).into());
                self.tracks.emit(TrackMsg::GoToTrack(uri));
            }
            _ => {}
        }
    }

    view! {
        #[name="tracks"]
        TrackList::<SavedLoader>(self.model.spotify.clone())
    }
}
