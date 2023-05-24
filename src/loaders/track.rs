// TODO: Mode
#![allow(dead_code)]

use crate::{
    loaders::common::ContainerLoader,
    services::api::{EpisodesStorageApi, PlaybackQueueApi, SearchApi, ThreadSafe, TracksStorageApi},
    utils::AsyncCell,
};
use async_trait::async_trait;
use rspotify::{ClientResult, model::*};
use serde_json::{Map, Value};

const NAME: &str = "tracks";

#[derive(Clone, Copy)]
pub struct Seed<Val: Copy> {
    min: Option<Val>,
    max: Option<Val>,
    target: Option<Val>,
}

#[derive(Clone, Copy)]
pub enum Mode {
    Minor = 0,
    Major = 1,
}

#[derive(Clone)]
pub struct RecommendLoader {
    seed_artists: Option<Vec<String>>,
    seed_genres: Option<Vec<String>>,
    seed_tracks: Option<Vec<String>>,
    tunables: Map<String, Value>,
    /*
    accousticness: Option<Seed<f32>>,
    dancability: Option<Seed<f32>>,
    duration_ms: Option<Seed<u32>>,
    energy: Option<Seed<f32>>,
    instrumentalness: Option<Seed<f32>>,
    key: Option<Seed<u8>>,
    liveness: Option<Seed<f32>>,
    loadness: Option<Seed<f32>>,
    mode: Option<Mode>,
    popularity: Option<Seed<u8>>,
    speechness: Option<Seed<f32>>,
    tempo: Option<Seed<f32>>,
    time_signature: Option<Seed<u8>>,
    valence: Option<Seed<f32>>,
     */
}

impl RecommendLoader {
    fn extract_vec_string(params: &mut Map<String, Value>, key: &str, max_items: usize) -> Option<Vec<String>> {
        params.remove(key).and_then(|seed| match seed {
            Value::Array(values) => Some(
                values
                    .into_iter()
                    .take(max_items)
                    .flat_map(|value| match value {
                        Value::String(value) => Some(value),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
    }
}

#[async_trait]
impl<Client> ContainerLoader<Client> for RecommendLoader
where
    Client: SearchApi + ThreadSafe,
{
    type Item = SimplifiedTrack;
    type Page = Vec<Self::Item>;
    type ParentId = Map<String, Value>;

    const NAME: &'static str = "recommended tracks";

    fn new(mut tunables: Self::ParentId) -> Self {
        let seed_artists = Self::extract_vec_string(&mut tunables, "seed_artists", 5);
        let seed_genres = Self::extract_vec_string(&mut tunables, "seed_genres", 5);
        let seed_tracks = Self::extract_vec_string(&mut tunables, "seed_tracks", 5);
        /*
        tunables.retain(|key| {
            matches!(
                &*key,
                "min_accousticness"
                    | "max_acousticness"
                    | "target_acousticness"
                    | "min_danceability"
                    | "max_danceability"
                    | "target_danceability"
                    | "min_duration_ms"
                    | "max_duration_ms"
                    | "target_duration_ms"
                    | "min_energy"
                    | "max_energy"
                    | "target_energy"
                    | "min_instrumentalness"
                    | "max_instrumentalness"
                    | "target_instrumentalness"
                    | "min_key"
                    | "max_key"
                    | "target_key"
                    | "min_liveness"
                    | "max_liveness"
                    | "target_liveness"
                    | "min_loadness"
                    | "max_loudness"
                    | "target_loudness"
                    | "min_mode"
                    | "max_mode"
                    | "target_mode"
                    | "min_popularity"
                    | "max_popularity"
                    | "target_popularity"
                    | "min_speechiness"
                    | "max_speechiness"
                    | "target_speechiness"
                    | "min_tempo"
                    | "max_tempo"
                    | "target_tempo"
                    | "max_time_signature"
                    | "min_time_signature"
                    | "target_time_signature"
                    | "min_valence"
                    | "max_valence"
                    | "target_valence"
            )
        });
         */

        Self {
            seed_artists,
            seed_genres,
            seed_tracks,
            tunables,
        }
    }

    fn parent_id(&self) -> &Self::ParentId {
        &self.tunables
        //let mut params = self.tunables.clone();
        //if let Some(ref seed_artists) = self.seed_artists {
        //    params.insert("seed_artists".into(),
        // Value::from(seed_artists.clone()));
        //}
        //if let Some(ref seed_genres) = self.seed_genres {
        //    params.insert("seed_genres".into(),
        // Value::from(seed_genres.clone()));
        //}
        //if let Some(ref seed_tracks) = self.seed_tracks {
        //    params.insert("seed_tracks".into(),
        // Value::from(seed_tracks.clone()));
        //}
        //params
    }

    #[allow(clippy::unit_arg)]
    async fn load_page(self, spotify: AsyncCell<Client>, _offset: ()) -> ClientResult<Self::Page> {
        let RecommendLoader {
            seed_tracks,
            seed_genres,
            seed_artists,
            tunables,
        } = self;

        spotify
            .read()
            .await
            .get_recommended_tracks(seed_tracks, seed_genres, seed_artists, tunables, 100)
            .await
    }
}

#[derive(Clone, Copy)]
pub struct SavedLoader(usize);

#[async_trait]
impl<Client> ContainerLoader<Client> for SavedLoader
where
    Client: TracksStorageApi + ThreadSafe,
{
    type Item = SavedTrack;
    type Page = Page<Self::Item>;
    type ParentId = ();

    const NAME: &'static str = NAME;

    fn new(_id: Self::ParentId) -> Self { SavedLoader(rand::random()) }

    fn parent_id(&self) -> &Self::ParentId { &() }

    async fn load_page(self, client: AsyncCell<Client>, offset: u32) -> ClientResult<Self::Page> {
        client.read().await.get_my_tracks(offset, 20).await
    }

    fn epoch(&self) -> usize { self.0 }
}

#[derive(Clone, Copy)]
pub struct RecentLoader(usize);

#[async_trait]
impl<Client> ContainerLoader<Client> for RecentLoader
where
    Client: TracksStorageApi + ThreadSafe,
{
    type Item = PlayHistory;
    type Page = Vec<Self::Item>;
    type ParentId = ();

    const NAME: &'static str = "recent tracks";

    fn new(_id: Self::ParentId) -> Self { RecentLoader(rand::random()) }

    fn parent_id(&self) -> &Self::ParentId { &() }

    #[allow(clippy::unit_arg)]
    async fn load_page(self, client: AsyncCell<Client>, _offset: ()) -> ClientResult<Self::Page> {
        client.read().await.get_recent_tracks(50).await
    }

    fn epoch(&self) -> usize { self.0 }
}

#[derive(Clone, Copy)]
pub struct QueueLoader(usize);

#[async_trait]
impl<Client> ContainerLoader<Client> for QueueLoader
where
    Client: PlaybackQueueApi + ThreadSafe,
{
    type Item = FullTrack;
    type Page = Vec<Self::Item>;
    type ParentId = ();

    const NAME: &'static str = NAME;

    fn new(_id: Self::ParentId) -> Self { QueueLoader(rand::random()) }

    fn parent_id(&self) -> &Self::ParentId { &() }

    #[allow(clippy::unit_arg)]
    async fn load_page(self, client: AsyncCell<Client>, _offset: ()) -> ClientResult<Self::Page> {
        client.read().await.get_queue_tracks().await
    }

    fn epoch(&self) -> usize { self.0 }
}

#[derive(Clone)]
pub struct AlbumLoader {
    id: AlbumId,
}

#[async_trait]
impl<Client> ContainerLoader<Client> for AlbumLoader
where
    Client: TracksStorageApi + ThreadSafe,
{
    type Item = SimplifiedTrack;
    type Page = Page<Self::Item>;
    type ParentId = AlbumId;

    const NAME: &'static str = "album tracks";

    fn new(id: Self::ParentId) -> Self { AlbumLoader { id } }

    fn parent_id(&self) -> &Self::ParentId { &self.id }

    async fn load_page(self, spotify: AsyncCell<Client>, offset: u32) -> ClientResult<Self::Page> {
        spotify.read().await.get_album_tracks(&self.id, offset, 10).await
    }
}

#[derive(Clone)]
pub struct PlaylistLoader {
    id: PlaylistId,
}

#[async_trait]
impl<Client> ContainerLoader<Client> for PlaylistLoader
where
    Client: TracksStorageApi + ThreadSafe,
{
    type Item = PlaylistItem;
    type Page = Page<Self::Item>;
    type ParentId = PlaylistId;

    fn new(id: Self::ParentId) -> Self { PlaylistLoader { id } }

    fn parent_id(&self) -> &Self::ParentId { &self.id }

    async fn load_page(self, spotify: AsyncCell<Client>, offset: u32) -> ClientResult<Self::Page> {
        spotify.read().await.get_playlist_tracks(&self.id, offset, 10).await
    }
}

#[derive(Clone, Copy)]
pub struct MyTopTracksLoader(usize);

#[async_trait]
impl<Client> ContainerLoader<Client> for MyTopTracksLoader
where
    Client: TracksStorageApi + ThreadSafe,
{
    type Item = FullTrack;
    type Page = Page<Self::Item>;
    type ParentId = ();

    const NAME: &'static str = "top tracks";

    fn new(_uri: Self::ParentId) -> Self { MyTopTracksLoader(rand::random()) }

    fn parent_id(&self) -> &Self::ParentId { &() }

    async fn load_page(self, spotify: AsyncCell<Client>, offset: u32) -> ClientResult<Self::Page> {
        spotify.read().await.get_my_top_tracks(offset, 20).await
    }

    fn epoch(&self) -> usize { self.0 }
}

#[derive(Clone)]
pub struct ShowLoader {
    id: ShowId,
}

#[async_trait]
impl<Client> ContainerLoader<Client> for ShowLoader
where
    Client: EpisodesStorageApi + ThreadSafe,
{
    type Item = SimplifiedEpisode;
    type Page = Page<Self::Item>;
    type ParentId = ShowId;

    const NAME: &'static str = "episodes";

    fn new(id: Self::ParentId) -> Self { ShowLoader { id } }

    fn parent_id(&self) -> &Self::ParentId { &self.id }

    async fn load_page(self, spotify: AsyncCell<Client>, offset: u32) -> ClientResult<Self::Page> {
        spotify.read().await.get_show_episodes(&self.id, offset, 10).await
    }
}

#[derive(Clone)]
pub struct ArtistTopTracksLoader {
    id: ArtistId,
}

#[async_trait]
impl<Client> ContainerLoader<Client> for ArtistTopTracksLoader
where
    Client: TracksStorageApi + ThreadSafe,
{
    type Item = FullTrack;
    type Page = Vec<Self::Item>;
    type ParentId = ArtistId;

    const NAME: &'static str = "artist's top tracks";

    fn new(id: Self::ParentId) -> Self { ArtistTopTracksLoader { id } }

    fn parent_id(&self) -> &Self::ParentId { &self.id }

    #[allow(clippy::unit_arg)]
    async fn load_page(self, spotify: AsyncCell<Client>, _offset: ()) -> ClientResult<Self::Page> {
        spotify.read().await.get_artist_top_tracks(&self.id).await
    }
}
