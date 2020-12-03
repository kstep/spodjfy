use crate::services::{api::TracksStorageApi, storage::store::Collection};
use async_trait::async_trait;
use rspotify::{
    client::ClientResult,
    model::{AudioAnalysis, AudioFeatures, FullTrack, Id, Page, PlayHistory, PlaylistItem, SavedTrack, SimplifiedTrack, Type},
};

pub struct TracksStorage {
    tracks_coll: Collection<FullTrack>,
    features_coll: Collection<AudioFeatures>,
    analysis_coll: Collection<AudioAnalysis>,
}

#[async_trait]
impl TracksStorageApi for TracksStorage {
    async fn get_track(&self, uri: &str) -> ClientResult<FullTrack> {
        let id = Id::from_id_or_uri(Type::Track, uri)?;
        let track = self.tracks_coll.get(id.id()).unwrap().unwrap();
        Ok(track)
    }

    async fn get_tracks(&self, uris: &[String]) -> ClientResult<Vec<FullTrack>> {
        uris.iter()
            .map(|uri| {
                let id = Id::from_id_or_uri(Type::Track, &*uri)?;
                Ok(self.tracks_coll.get(id.id()).unwrap().unwrap())
            })
            .collect()
    }

    async fn get_track_analysis(&self, uri: &str) -> ClientResult<AudioAnalysis> {
        let id = Id::from_id_or_uri(Type::Track, uri)?;
        let analysis = self.analysis_coll.get(id.id()).unwrap().unwrap();
        Ok(analysis)
    }

    async fn get_tracks_features(&self, uris: &[String]) -> ClientResult<Vec<AudioFeatures>> {
        uris.iter()
            .map(|uri| {
                let id = Id::from_id_or_uri(Type::Track, &*uri)?;
                Ok(self.features_coll.get(id.id()).unwrap().unwrap())
            })
            .collect()
    }

    async fn get_my_tracks(&self, offset: u32, limit: u32) -> ClientResult<Page<SavedTrack>> { unimplemented!() }

    async fn get_my_top_tracks(&self, offset: u32, limit: u32) -> ClientResult<Page<FullTrack>> { unimplemented!() }

    async fn get_recent_tracks(&self, limit: u32) -> ClientResult<Vec<PlayHistory>> { unimplemented!() }

    async fn get_playlist_tracks(&self, uri: &str, offset: u32, limit: u32) -> ClientResult<Page<PlaylistItem>> {
        unimplemented!()
    }

    async fn get_album_tracks(&self, uri: &str, offset: u32, limit: u32) -> ClientResult<Page<SimplifiedTrack>> {
        unimplemented!()
    }

    async fn get_artist_top_tracks(&self, uri: &str) -> ClientResult<Vec<FullTrack>> { unimplemented!() }

    async fn add_my_tracks(&self, uris: &[String]) -> ClientResult<()> { unimplemented!() }

    async fn remove_my_tracks(&self, uris: &[String]) -> ClientResult<()> { unimplemented!() }

    async fn are_my_tracks(&self, uris: &[String]) -> ClientResult<Vec<bool>> { unimplemented!() }
}
