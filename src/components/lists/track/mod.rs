pub mod handler;
pub mod item_view;
pub mod message;

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

pub use message::TrackMsg;

pub type TrackList<Loader> = ContainerList<Loader, TrackView, TrackMsgHandler, TrackMsg<Loader>>;

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
