use crate::components::spotify::SpotifyProxy;
use crate::components::track_list::{TrackList, TrackListMsg};
use relm::Widget;
use relm_derive::{widget, Msg};
use rspotify::model::playlist::PlaylistTrack;
use rspotify::senum::Type;
use std::sync::Arc;

#[derive(Msg)]
pub enum NowPlayingMsg {
    ShowTab,
    LoadTracks(Type, String),
    GoToTrack(String),
}

pub struct NowPlayingModel {
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for NowPlayingTab {
    fn model(spotify: Arc<SpotifyProxy>) -> NowPlayingModel {
        NowPlayingModel { spotify }
    }

    fn update(&mut self, event: NowPlayingMsg) {
        use NowPlayingMsg::*;
        match event {
            ShowTab => {}
            GoToTrack(track_uri) => {
                self.tracks_view.emit(TrackListMsg::GoToTrack(track_uri));
            }
            LoadTracks(kind, uri) => {
                match kind {
                    Type::Playlist => self.tracks_view.emit(TrackListMsg::Reset(uri, true)),
                    // TODO: sources for other context types:
                    Type::Album => (),
                    Type::Artist => (),
                    Type::User => (),
                    Type::Show => (),
                    _ => (),
                }
            }
        }
    }

    view! {
        gtk::Box(gtk::Orientation::Vertical, 10) {
            // TODO: make an universal component out of this window
            #[name="tracks_view"]
            TrackList::<PlaylistTrack>(self.model.spotify.clone()),
        }
    }
}
