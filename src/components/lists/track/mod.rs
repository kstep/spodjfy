pub mod item_view;

use crate::components::lists::common::{
    ContainerList, ContainerListModel, ContainerListMsg, GetSelectedRows,
};
use crate::loaders::common::{ContainerLoader, HasImages, MissingColumns};
use crate::loaders::paged::{PageLike, RowLike};
use crate::loaders::track::*;
use crate::servers::spotify::{SpotifyCmd, SpotifyProxy};
use gtk::prelude::*;
use gtk::{TreeModelExt, TreeViewExt};
use item_view::TrackView;
use relm::{Relm, Update, Widget, EventStream};
use rspotify::model::audio::AudioFeatures;
use serde_json::Map;
use std::sync::Arc;

pub trait PlayContextCmd {
    fn play_tracks_cmd(self, uris: Vec<String>) -> SpotifyCmd;
}

#[derive(Clone, Debug)]
pub enum TrackMsg {
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

pub struct TrackList<Loader>(ContainerList<Loader, TrackView>)
where
    Loader: ContainerLoader,
    Loader::Item: MissingColumns + RowLike + HasImages,
    Loader::ParentId: PartialEq;

impl<Loader> TrackList<Loader>
where
    Loader: ContainerLoader,
    Loader::Item: MissingColumns + RowLike + HasImages,
    Loader::ParentId: PartialEq,
{
    fn get_selected_tracks_uris(&self) -> Vec<String> {
        let (rows, model) = self.0.items_view.get_selected_rows();
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

impl<Loader> Update for TrackList<Loader>
where
    Loader: ContainerLoader,
    Loader::Item: Clone + HasImages + RowLike + MissingColumns + TrackLike,
    Loader::Page: Clone,
    Loader::ParentId: PartialEq + PlayContextCmd,
{
    type Model = ContainerListModel<Loader, TrackMsg>;
    type ModelParam = Arc<SpotifyProxy>;
    type Msg = ContainerListMsg<Loader, TrackMsg>;

    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> Self::Model {
        ContainerListModel::from_row::<Loader::Item>(relm.stream().clone(), spotify)
    }

    fn update(&mut self, event: Self::Msg) {
        use ContainerListMsg::Custom;
        match event {
            Custom(event) => {
                use TrackMsg::*;

                match event {
                    LoadTracksInfo(uris, iters) => {
                        self.0
                            .model
                            .spotify
                            .ask(
                                self.0.model.stream.clone(),
                                |tx| SpotifyCmd::GetTracksFeatures { tx, uris },
                                move |feats| Custom(NewTracksInfo(feats, iters.clone())),
                            )
                            .unwrap();
                    }
                    NewTracksInfo(info, iters) => {
                        let store = &self.0.model.store;
                        for (idx, pos) in iters.iter().enumerate() {
                            store.set(pos, &[COL_TRACK_BPM], &[&info[idx].tempo]);
                        }
                    }
                    GoToTrack(track_id) if self.0.model.is_loading => {
                        let stream = self.0.model.stream.clone();
                        glib::timeout_add_local(500, move || {
                            stream.emit(Custom(GoToTrack(track_id.clone())));
                            Continue(false)
                        });
                    }
                    GoToTrack(track_id) => {
                        let store = &self.0.model.store;
                        if let Some(pos) = store.get_iter_first() {
                            loop {
                                if let Ok(Some(uri)) =
                                    store.get_value(&pos, COL_TRACK_URI as i32).get::<&str>()
                                {
                                    if uri == track_id {
                                        let select = self.0.items_view.get_selection();
                                        select.unselect_all();
                                        select.select_iter(&pos);

                                        self.0.items_view.scroll_to_cell(
                                            store.get_path(&pos).as_ref(),
                                            None::<&gtk::TreeViewColumn>,
                                            false,
                                            0.0,
                                            0.0,
                                        );

                                        break;
                                    }
                                }
                                if !store.iter_next(&pos) {
                                    break;
                                }
                            }
                        }
                    }
                    PlayChosenTracks => {
                        let uris = self.get_selected_tracks_uris();
                        self.0.model.stream.emit(Custom(PlayTracks(uris)));
                    }

                    EnqueueChosenTracks => {
                        let uris = self.get_selected_tracks_uris();
                        self.0
                            .model
                            .spotify
                            .tell(SpotifyCmd::EnqueueTracks { uris })
                            .unwrap();
                    }
                    AddChosenTracks => {}
                    SaveChosenTracks => {
                        let uris = self.get_selected_tracks_uris();
                        self.0
                            .model
                            .spotify
                            .tell(SpotifyCmd::AddMyTracks { uris })
                            .unwrap();
                    }
                    UnsaveChosenTracks => {
                        let uris = self.get_selected_tracks_uris();
                        self.0
                            .model
                            .spotify
                            .tell(SpotifyCmd::RemoveMyTracks { uris })
                            .unwrap();
                    }
                    RecommendTracks => {}
                    GoToChosenTrackAlbum => {
                        let (rows, model) = self.0.items_view.get_selected_rows();
                        if let Some(pos) = rows.into_iter()
                            .filter_map(|path| model.get_iter(&path))
                            .next() {
                            let album_uri = model.get_value(&pos, COL_TRACK_ALBUM_URI as i32).get::<String>().ok().flatten();
                            let album_name = model.get_value(&pos, COL_TRACK_ALBUM as i32).get::<String>().ok().flatten();

                            if let (Some(uri), Some(name)) = (album_uri, album_name) {
                                self.0.model.stream.emit(Custom(GoToAlbum(uri, name)));
                            }
                        }
                    }
                    GoToChosenTrackArtist => {
                        let (rows, model) = self.0.items_view.get_selected_rows();
                        if let Some(pos) = rows.into_iter()
                            .filter_map(|path| model.get_iter(&path))
                            .next() {
                            let artist_uri = model.get_value(&pos, COL_TRACK_ARTIST_URI as i32).get::<String>().ok().flatten();
                            let artist_name = model.get_value(&pos, COL_TRACK_ARTISTS as i32).get::<String>().ok().flatten();

                            if let (Some(uri), Some(name)) = (artist_uri, artist_name) {
                                self.0.model.stream.emit(Custom(GoToArtist(uri, name)));
                            }
                        }
                    }
                    GoToAlbum(_, _) => {}
                    GoToArtist(_, _) => {}
                    PlayTracks(uris) => {
                        if let Some(ref loader) = self.0.model.items_loader {
                            self.0
                                .model
                                .spotify
                                .tell(loader.parent_id().clone().play_tracks_cmd(uris))
                                .unwrap();
                            self.0.model.stream.emit(Custom(PlayingNewTrack));
                        }
                    }
                    PlayingNewTrack => {}
                    NewBpm(path, bpm) => {
                        let store = &self.0.model.store;
                        if let Some(iter) = store.get_iter(&path) {
                            store.set_value(&iter, COL_TRACK_BPM, &bpm.to_value());
                        }
                    }
                }
            }
            ContainerListMsg::NewPage(page, epoch) => {
                if epoch != self.0.current_epoch() {
                    return;
                }

                use ContainerListMsg::{LoadPage, LoadThumb};
                use TrackMsg::*;

                let stream = &self.0.model.stream;
                let store = &self.0.model.store;
                let tracks = page.items();
                let offset = page.num_offset();

                self.0
                    .progress_bar
                    .set_fraction((offset as f64 + tracks.len() as f64) / page.total() as f64);

                let mut uris = Vec::with_capacity(tracks.len());
                let mut iters = Vec::with_capacity(tracks.len());

                let mut page_duration = 0;
                for (idx, track) in tracks.iter().enumerate() {
                    let pos = track.append_to_store(store);
                    store.set(
                        &pos,
                        &[COL_TRACK_NUMBER, COL_TRACK_TIMELINE],
                        &[
                            &(idx as u32 + offset + 1),
                            &crate::utils::humanize_time(
                                self.0.model.total_duration + page_duration,
                            ),
                        ],
                    );

                    let image = self.0.model.image_loader.find_best_thumb(track.images());

                    if let Some(url) = image {
                        stream.emit(LoadThumb(url.to_owned(), pos.clone()));
                    }

                    uris.push(track.uri().to_owned());
                    iters.push(pos);
                    page_duration += track.duration();
                }

                self.0.model.total_duration += page_duration;

                if !Loader::Item::missing_columns().contains(&COL_TRACK_BPM) {
                    stream.emit(Custom(LoadTracksInfo(uris, iters)));
                }

                if let Some(next_offset) = page.next_offset() {
                    stream.emit(LoadPage(next_offset, epoch));
                } else {
                    drop(stream);
                    self.0.model.total_items = page.total();
                    self.0.finish_load();
                }
            }
            other => {
                self.0.update(other);
            }
        }
    }
}

impl<Loader> Widget for TrackList<Loader>
where
    Loader: ContainerLoader,
    Loader::Item: Clone + MissingColumns + RowLike + HasImages + TrackLike,
    Loader::Page: Clone,
    Loader::ParentId: PartialEq + PlayContextCmd,
{
    type Root = gtk::Box;

    fn root(&self) -> Self::Root {
        self.0.root()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let inner_relm = unsafe { std::mem::transmute(relm) };
        TrackList(ContainerList::<Loader, TrackView>::view(inner_relm, model))
    }
}
