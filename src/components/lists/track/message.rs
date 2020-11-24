use crate::components::lists::common::ContainerMsg;
use crate::loaders::ContainerLoader;
use relm_derive::Msg;
use rspotify::model::AudioFeatures;
use std::convert::TryFrom;

#[derive(Msg)]
pub enum TrackMsg<Loader: ContainerLoader> {
    Parent(ContainerMsg<Loader>),

    PlayTracks(Vec<String>),
    PlayingNewTrack,

    LoadTracksInfo(Vec<String>, Vec<gtk::TreeIter>),
    NewTracksInfo(Vec<AudioFeatures>, Vec<gtk::TreeIter>),
    NewTracksSaved(Vec<bool>, Vec<gtk::TreeIter>),
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
