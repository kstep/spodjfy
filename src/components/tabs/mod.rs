pub mod albums;
pub mod artists;
pub mod categories;
pub mod devices;
pub mod featured;
pub mod new_releases;
pub mod playlists;
pub mod queue;
pub mod recent;
pub mod search;
pub mod settings;
pub mod shows;
pub mod tracks;

use crate::{components::lists::TrackMsg, loaders::ContainerLoader, services::SpotifyRef};
use relm_derive::Msg;
use rspotify::model::Type;
use tokio::runtime::Handle;

#[derive(Msg)]
pub enum MusicTabMsg {
    ShowTab,
    OpenContainer(u8, String, String),
    GoToTrack(String),
    GoTo(Type, String, String),
    PlaybackUpdate,
}

pub struct TracksObserver {
    upstream: relm::EventStream<MusicTabMsg>,
}

pub struct MusicTabModel {
    pool: Handle,
    spotify: SpotifyRef,
}

pub type MusicTabParams = (Handle, SpotifyRef);

impl MusicTabModel {
    fn from_params((pool, spotify): MusicTabParams) -> Self { Self { pool, spotify } }
}

impl TracksObserver {
    pub fn new(upstream: &relm::EventStream<MusicTabMsg>) -> Self {
        Self {
            upstream: upstream.clone(),
        }
    }
}

impl<Loader: ContainerLoader> FnOnce<(&TrackMsg<Loader>,)> for TracksObserver {
    type Output = ();

    extern "rust-call" fn call_once(self, args: (&TrackMsg<Loader>,)) -> Self::Output { self.call(args) }
}

impl<Loader: ContainerLoader> FnMut<(&TrackMsg<Loader>,)> for TracksObserver {
    extern "rust-call" fn call_mut(&mut self, args: (&TrackMsg<Loader>,)) -> Self::Output { self.call(args) }
}

impl<Loader: ContainerLoader> Fn<(&TrackMsg<Loader>,)> for TracksObserver {
    extern "rust-call" fn call(&self, args: (&TrackMsg<Loader>,)) -> Self::Output {
        let (msg,) = args;

        match msg {
            TrackMsg::PlayingNewTrack => self.upstream.emit(MusicTabMsg::PlaybackUpdate),
            TrackMsg::GoToArtist(uri, name) => self.upstream.emit(MusicTabMsg::GoTo(Type::Artist, uri.clone(), name.clone())),
            TrackMsg::GoToAlbum(uri, name) => self.upstream.emit(MusicTabMsg::GoTo(Type::Album, uri.clone(), name.clone())),
            _ => {}
        }
    }
}
