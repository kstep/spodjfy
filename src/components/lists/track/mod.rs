pub mod handler;
pub mod item_view;

use crate::components::lists::common::{ContainerList, ContainerMsg, GetSelectedRows};
use crate::loaders::common::ContainerLoader;
use crate::loaders::track::*;
use crate::servers::spotify::SpotifyCmd;
use gtk::TreeModelExt;
use handler::TrackMsgHandler;
use item_view::TrackView;
use relm_derive::Msg;
use rspotify::model::audio::AudioFeatures;
use serde_json::Map;
use std::convert::TryFrom;

pub type TrackList<Loader> = ContainerList<Loader, TrackView, TrackMsgHandler, TrackMsg<Loader>>;

#[derive(Msg)]
pub enum TrackMsg<Loader: ContainerLoader> {
    Parent(ContainerMsg<Loader>),

    PlayTracks(Vec<String>),
    PlayingNewTrack,

    LoadTracksInfo(Vec<String>, Vec<gtk::TreeIter>),
    NewTracksInfo(Vec<AudioFeatures>, Vec<gtk::TreeIter>),
    NewBpm(gtk::TreePath, f32),

    PlayChosenTracks,
    GoToTrack(String),
    GoToChosenTrackAlbum,
    GoToAlbum(String, String),
    GoToChosenTrackArtist,
    GoToArtist(String, String),
    EnqueueChosenTracks,
    AddChosenTracks,
    SaveChosenTracks,
    RecommendTracks,
    UnsaveChosenTracks,
}

impl<Loader> From<ContainerMsg<Loader>> for TrackMsg<Loader>
where
    Loader: ContainerLoader,
{
    fn from(msg: ContainerMsg<Loader>) -> Self {
        TrackMsg::Parent(msg)
    }
}
impl<Loader> TryFrom<TrackMsg<Loader>> for ContainerMsg<Loader>
where
    Loader: ContainerLoader,
{
    type Error = ();
    fn try_from(msg: TrackMsg<Loader>) -> Result<Self, Self::Error> {
        match msg {
            TrackMsg::Parent(msg) => Ok(msg),
            _ => Err(()),
        }
    }
}

pub trait PlayContextCmd {
    fn play_tracks_cmd(self, uris: Vec<String>) -> SpotifyCmd;
}

impl PlayContextCmd for () {
    fn play_tracks_cmd(self, uris: Vec<String>) -> SpotifyCmd {
        SpotifyCmd::PlayTracks { uris }
    }
}

impl<K, V> PlayContextCmd for Map<K, V> {
    fn play_tracks_cmd(self, uris: Vec<String>) -> SpotifyCmd {
        SpotifyCmd::PlayTracks { uris }
    }
}

impl PlayContextCmd for String {
    fn play_tracks_cmd(self, uris: Vec<String>) -> SpotifyCmd {
        SpotifyCmd::PlayContext {
            uri: self,
            start_uri: uris.first().cloned(),
        }
    }
}

impl<Loader: ContainerLoader> TrackList<Loader> {
    fn get_selected_tracks_uris(&self) -> Vec<String> {
        let (rows, model) = self.items_view.get_selected_rows();
        rows.into_iter()
            .filter_map(|path| model.get_iter(&path))
            .filter_map(|pos| {
                model
                    .get_value(&pos, COL_TRACK_URI as i32)
                    .get::<String>()
                    .ok()
                    .flatten()
            })
            .collect::<Vec<_>>()
    }
}
