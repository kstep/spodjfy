use crate::loaders::ContainerLoader;
use crate::Spotify;
use async_trait::async_trait;
use rspotify::client::ClientResult;
use rspotify::model::{Page, SavedAlbum, SimplifiedAlbum};
use std::ops::Deref;

const NAME: &str = "albums";

#[derive(Clone, Copy)]
pub struct SavedLoader(usize);

#[async_trait]
impl ContainerLoader for SavedLoader {
    type ParentId = ();
    type Item = SavedAlbum;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;
    const NAME: &'static str = NAME;

    fn new(_id: Self::ParentId) -> Self {
        SavedLoader(rand::random())
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    async fn load_page(
        self,
        spotify: impl Deref<Target = Spotify> + Send + 'static,
        offset: u32,
    ) -> ClientResult<Self::Page> {
        spotify.get_my_albums(offset, Self::PAGE_LIMIT).await
    }

    fn epoch(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct NewReleasesLoader(usize);

#[async_trait]
impl ContainerLoader for NewReleasesLoader {
    type ParentId = ();
    type Item = SimplifiedAlbum;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;
    const NAME: &'static str = "new releases";

    fn new(_id: Self::ParentId) -> Self {
        NewReleasesLoader(rand::random())
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    async fn load_page(
        self,
        spotify: impl Deref<Target = Spotify> + Send + 'static,
        offset: u32,
    ) -> ClientResult<Self::Page> {
        spotify.get_new_releases(offset, Self::PAGE_LIMIT).await
    }

    fn epoch(&self) -> usize {
        self.0
    }
}

#[derive(Clone)]
pub struct ArtistLoader {
    uri: String,
}

#[async_trait]
impl ContainerLoader for ArtistLoader {
    type ParentId = String;
    type Item = SimplifiedAlbum;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;
    const NAME: &'static str = "artist's albums";

    fn new(uri: Self::ParentId) -> Self {
        ArtistLoader { uri }
    }

    fn parent_id(&self) -> &Self::ParentId {
        &self.uri
    }

    async fn load_page(
        self,
        spotify: impl Deref<Target = Spotify> + Send + 'static,
        offset: u32,
    ) -> ClientResult<Self::Page> {
        spotify
            .get_artist_albums(&self.uri, offset, Self::PAGE_LIMIT)
            .await
    }
}
