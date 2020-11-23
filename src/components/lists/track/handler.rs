use crate::components::lists::{
    ContainerMsg, GetSelectedRows, MessageHandler, TrackList, TrackMsg,
};
use crate::loaders::track::*;
use crate::loaders::{ContainerLoader, HasDuration, HasImages, MissingColumns, PageLike, RowLike};
use crate::servers::spotify::SpotifyCmd;
use glib::{Continue, ToValue};
use gtk::{
    prelude::GtkListStoreExtManual, ProgressBarExt, TreeModelExt, TreeSelectionExt, TreeViewExt,
};
use serde_json::Map;

pub struct TrackMsgHandler;

impl<Loader> MessageHandler<TrackList<Loader>, TrackMsg<Loader>> for TrackMsgHandler
where
    Loader: ContainerLoader + 'static,
    Loader::Page: PageLike<Loader::Item>,
    Loader::Item: RowLike + HasImages + TrackLike + HasDuration + MissingColumns,
    Loader::ParentId: Clone + PlayContextCmd,
    ContainerMsg<Loader>: Into<TrackMsg<Loader>>,
{
    fn handle(this: &mut TrackList<Loader>, message: TrackMsg<Loader>) -> Option<TrackMsg<Loader>> {
        use crate::components::lists::track::message::TrackMsg::*;

        match message {
            Parent(ContainerMsg::NewPage(page, epoch)) => {
                use ContainerMsg::{LoadPage, LoadThumb};

                if epoch != this.current_epoch() {
                    return None;
                }

                let stream = &this.stream;
                let store = &this.model.store;
                let tracks = page.items();
                let offset = page.num_offset();

                this.progress_bar
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
                            &crate::utils::humanize_time(this.model.total_duration + page_duration),
                        ],
                    );

                    let image = this.model.image_loader.find_best_thumb(track.images());

                    if let Some(url) = image {
                        stream.emit(LoadThumb(url.to_owned(), pos.clone()).into());
                    }

                    uris.push(track.uri().to_owned());
                    iters.push(pos);
                    page_duration += track.duration();
                }

                this.model.total_duration += page_duration;

                if !Loader::Item::missing_columns().contains(&COL_TRACK_BPM) {
                    stream.emit(LoadTracksInfo(uris, iters));
                }

                if let Some(next_offset) = page.next_offset() {
                    stream.emit(LoadPage(next_offset, epoch).into());
                } else {
                    this.model.total_items = page.total();
                    this.finish_load();
                }
            }
            event @ Parent(_) => {
                return Some(event);
            }
            LoadTracksInfo(uris, iters) => {
                this.model
                    .spotify
                    .ask(
                        this.stream.clone(),
                        |tx| SpotifyCmd::GetTracksFeatures { tx, uris },
                        move |feats| NewTracksInfo(feats, iters.clone()),
                    )
                    .unwrap();
            }
            NewTracksInfo(info, iters) => {
                let store = &this.model.store;
                for (idx, pos) in iters.iter().enumerate() {
                    store.set(pos, &[COL_TRACK_BPM], &[&info[idx].tempo]);
                }
            }
            GoToTrack(track_id) => {
                let store = &this.model.store;
                let found = if let Some(pos) = store.get_iter_first() {
                    loop {
                        if let Ok(Some(uri)) =
                            store.get_value(&pos, COL_TRACK_URI as i32).get::<&str>()
                        {
                            if uri == track_id {
                                let select = this.items_view.get_selection();
                                select.unselect_all();
                                select.select_iter(&pos);

                                this.items_view.scroll_to_cell(
                                    store.get_path(&pos).as_ref(),
                                    None::<&gtk::TreeViewColumn>,
                                    false,
                                    0.0,
                                    0.0,
                                );

                                break true;
                            }
                        }
                        if !store.iter_next(&pos) {
                            break false;
                        }
                    }
                } else {
                    false
                };

                // If the track was not found in the list, and the list is still loading,
                // try looking for it a little later
                if !found && this.model.is_loading {
                    let stream = this.stream.clone();
                    glib::timeout_add_local(500, move || {
                        stream.emit(GoToTrack(track_id.clone()));
                        Continue(false)
                    });
                }
            }
            PlayChosenTracks => {
                let uris = this.get_selected_tracks_uris();
                this.stream.emit(PlayTracks(uris));
            }

            EnqueueChosenTracks => {
                let uris = this.get_selected_tracks_uris();
                this.model
                    .spotify
                    .tell(SpotifyCmd::EnqueueTracks { uris })
                    .unwrap();
            }
            AddChosenTracks => {}
            SaveChosenTracks => {
                let uris = this.get_selected_tracks_uris();
                this.model
                    .spotify
                    .tell(SpotifyCmd::AddMyTracks { uris })
                    .unwrap();
            }
            UnsaveChosenTracks => {
                let uris = this.get_selected_tracks_uris();
                this.model
                    .spotify
                    .tell(SpotifyCmd::RemoveMyTracks { uris })
                    .unwrap();
            }
            RecommendTracks => {}
            GoToChosenTrackAlbum => {
                let (rows, model) = this.items_view.get_selected_rows();
                if let Some(pos) = rows
                    .into_iter()
                    .filter_map(|path| model.get_iter(&path))
                    .next()
                {
                    let album_uri = model
                        .get_value(&pos, COL_TRACK_ALBUM_URI as i32)
                        .get::<String>()
                        .ok()
                        .flatten();
                    let album_name = model
                        .get_value(&pos, COL_TRACK_ALBUM as i32)
                        .get::<String>()
                        .ok()
                        .flatten();

                    if let (Some(uri), Some(name)) = (album_uri, album_name) {
                        this.stream.emit(GoToAlbum(uri, name));
                    }
                }
            }
            GoToChosenTrackArtist => {
                let (rows, model) = this.items_view.get_selected_rows();
                if let Some(pos) = rows
                    .into_iter()
                    .filter_map(|path| model.get_iter(&path))
                    .next()
                {
                    let artist_uri = model
                        .get_value(&pos, COL_TRACK_ARTIST_URI as i32)
                        .get::<String>()
                        .ok()
                        .flatten();
                    let artist_name = model
                        .get_value(&pos, COL_TRACK_ARTISTS as i32)
                        .get::<String>()
                        .ok()
                        .flatten();

                    if let (Some(uri), Some(name)) = (artist_uri, artist_name) {
                        this.stream.emit(GoToArtist(uri, name));
                    }
                }
            }
            GoToAlbum(_, _) => {}
            GoToArtist(_, _) => {}
            PlayTracks(uris) => {
                if let Some(ref loader) = this.model.items_loader {
                    this.model
                        .spotify
                        .tell(loader.parent_id().clone().play_tracks_cmd(uris))
                        .unwrap();
                    this.stream.emit(PlayingNewTrack);
                }
            }
            PlayingNewTrack => {}
            NewBpm(path, bpm) => {
                let store = &this.model.store;
                if let Some(iter) = store.get_iter(&path) {
                    store.set_value(&iter, COL_TRACK_BPM, &bpm.to_value());
                }
            }
        }
        None
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
        let start_uri = if self.starts_with("spotify:album:")
            || self.starts_with("spotify:playlist:")
            || self.starts_with("spotify:show:")
        {
            uris.first().cloned()
        } else {
            None
        };

        SpotifyCmd::PlayContext {
            uri: self,
            start_uri,
        }
    }
}
