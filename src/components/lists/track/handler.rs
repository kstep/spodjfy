use crate::components::lists::{
    ContainerMsg, GetSelectedRows, MessageHandler, TrackList, TrackMsg,
};
use crate::loaders::ContainerLoader;
use crate::models::common::*;
use crate::models::page::*;
use crate::models::track::*;
use crate::services::SpotifyRef;
use crate::utils::Spawn;
use async_trait::async_trait;
use glib::{Continue, ToValue};
use gtk::{
    prelude::GtkListStoreExtManual, ProgressBarExt, TreeModelExt, TreeSelectionExt, TreeViewExt,
};
use relm::EventStream;
use rspotify::client::ClientError;

pub struct TrackMsgHandler;

impl<Loader> MessageHandler<TrackList<Loader>, TrackMsg<Loader>> for TrackMsgHandler
where
    Loader: ContainerLoader + 'static,
    Loader::Page: PageLike<Loader::Item>, // + Send,
    //<Loader::Page as PageLike<Loader::Item>>::Offset: Send,
    Loader::Item: RowLike + HasImages + TrackLike + HasDuration + MissingColumns,
    Loader::ParentId: Clone + Send + PlayTracksContext,
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
                this.spawn_args(
                    (uris, iters),
                    async move |pool,
                                (stream, spotify): (EventStream<_>, SpotifyRef),
                                (uris, iters)| {
                        let (saved, feats) = pool
                            .spawn(async move {
                                let spotify = spotify.read().await;
                                let saved = spotify.are_my_tracks(&uris).await?;
                                let feats = spotify.get_tracks_features(&uris).await?;
                                Ok::<_, ClientError>((saved, feats))
                            })
                            .await??;
                        stream.emit(NewTracksInfo(feats, iters.clone()));
                        stream.emit(NewTracksSaved(saved, iters.clone()));
                        Ok(())
                    },
                );
            }
            NewTracksSaved(saved, iters) => {
                let store = &this.model.store;
                for (idx, pos) in iters.iter().enumerate() {
                    store.set_value(pos, COL_TRACK_SAVED, &saved[idx].to_value());
                }
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
                this.spawn_args(
                    this.get_selected_tracks_uris(),
                    async move |pool, spotify: SpotifyRef, uris| {
                        pool.spawn(async move {
                            let mut spotify = spotify.write().await;
                            spotify.enqueue_tracks(uris).await
                        })
                        .await??;
                        Ok(())
                    },
                );
            }
            AddChosenTracks => {}
            SaveChosenTracks => {
                this.spawn_args(
                    this.get_selected_tracks_uris(),
                    async move |pool, spotify: SpotifyRef, uris| {
                        pool.spawn(async move { spotify.write().await.add_my_tracks(&uris).await })
                            .await??;
                        Ok(())
                    },
                );
            }
            UnsaveChosenTracks => {
                this.spawn_args(
                    this.get_selected_tracks_uris(),
                    async move |pool, spotify: SpotifyRef, uris| {
                        pool.spawn(
                            async move { spotify.write().await.remove_my_tracks(&uris).await },
                        )
                        .await??;
                        Ok(())
                    },
                );
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
                    this.spawn_args(
                        (loader.parent_id().clone(), uris),
                        async move |pool,
                                    (stream, spotify): (EventStream<_>, SpotifyRef),
                                    (play_ctx, uris)| {
                            pool.spawn(play_ctx.play_tracks(spotify, uris)).await??;
                            stream.emit(PlayingNewTrack);
                            Ok(())
                        },
                    );
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

#[async_trait]
pub trait PlayTracksContext {
    async fn play_tracks(self, spotify: SpotifyRef, uris: Vec<String>) -> Result<(), ClientError>;
}

#[async_trait]
impl PlayTracksContext for () {
    #[allow(clippy::unit_arg)]
    async fn play_tracks(self, spotify: SpotifyRef, uris: Vec<String>) -> Result<(), ClientError> {
        spotify.read().await.play_tracks(uris).await
    }
}

/*
#[async_trait]
impl<K, V> PlayTracksContext for Map<K, V> {
    async fn play_tracks(
        self,
        spotify: SpotifyRef,
        uris: Vec<String>,
    ) {
        spotify.play_tracks(uris).await;
    }
}
 */

#[async_trait]
impl PlayTracksContext for String {
    async fn play_tracks(self, spotify: SpotifyRef, uris: Vec<String>) -> Result<(), ClientError> {
        let start_uri = if self.starts_with("spotify:album:")
            || self.starts_with("spotify:playlist:")
            || self.starts_with("spotify:show:")
        {
            uris.first().cloned()
        } else {
            None
        };

        spotify.read().await.play_context(self, start_uri).await
    }
}
