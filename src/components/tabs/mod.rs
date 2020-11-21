pub mod albums;
pub mod artists;
pub mod categories;
pub mod devices;
pub mod favorites;
pub mod featured;
pub mod new_releases;
pub mod playlists;
pub mod queue;
pub mod recent;
pub mod search;
pub mod settings;
pub mod shows;
pub mod top_artists;
pub mod top_tracks;

use relm_derive::Msg;

#[derive(Msg)]
pub enum MusicTabMsg {
    ShowTab,
    OpenContainer(u8, String, String),
    GoToTrack(String),
    PlaybackUpdate,
}
