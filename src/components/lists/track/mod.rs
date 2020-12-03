pub mod handler;
pub mod item_view;
pub mod message;

use crate::{
    components::lists::{ContainerList, GetSelectedRows},
    loaders::ContainerLoader,
    models::track::*,
};
use gtk::TreeModelExt;
use handler::TrackMsgHandler;
use item_view::TrackView;

pub use message::TrackMsg;

pub type TrackList<Loader> = ContainerList<Loader, TrackView, TrackMsgHandler, TrackMsg<Loader>>;

impl<Loader: ContainerLoader> TrackList<Loader> {
    fn get_selected_tracks_uris(&self) -> Vec<String> {
        let (rows, model) = self.items_view.get_selected_rows();

        rows.into_iter()
            .filter_map(|path| model.get_iter(&path))
            .filter_map(|pos| model.get_value(&pos, COL_TRACK_URI as i32).get::<String>().ok().flatten())
            .collect::<Vec<_>>()
    }
}
