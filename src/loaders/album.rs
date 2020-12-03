use crate::{
    loaders::ContainerLoader,
    services::api::{AlbumsStorageApi, ThreadSafe},
    utils::AsyncCell,
};
use async_trait::async_trait;
use rspotify::{
    client::ClientResult,
    model::{Page, SavedAlbum, SimplifiedAlbum},
};

const NAME: &str = "albums";

#[derive(Clone, Copy)]
pub struct SavedLoader(usize);

#[async_trait]
impl<Client> ContainerLoader<Client> for SavedLoader
where
    Client: AlbumsStorageApi + ThreadSafe,
{
    type Item = SavedAlbum;
    type Page = Page<Self::Item>;
    type ParentId = ();

    const NAME: &'static str = NAME;

    fn new(_id: Self::ParentId) -> Self { SavedLoader(rand::random()) }

    fn parent_id(&self) -> &Self::ParentId { &() }

    async fn load_page(self, spotify: AsyncCell<Client>, offset: u32) -> ClientResult<Self::Page> {
        spotify.read().await.get_my_albums(offset, 20).await
    }

    fn epoch(&self) -> usize { self.0 }
}

#[derive(Clone, Copy)]
pub struct NewReleasesLoader(usize);

#[async_trait]
impl<Client> ContainerLoader<Client> for NewReleasesLoader
where
    Client: AlbumsStorageApi + ThreadSafe,
{
    type Item = SimplifiedAlbum;
    type Page = Page<Self::Item>;
    type ParentId = ();

    const NAME: &'static str = "new releases";

    fn new(_id: Self::ParentId) -> Self { NewReleasesLoader(rand::random()) }

    fn parent_id(&self) -> &Self::ParentId { &() }

    async fn load_page(self, spotify: AsyncCell<Client>, offset: u32) -> ClientResult<Self::Page> {
        spotify.read().await.get_new_releases(offset, 20).await
    }

    fn epoch(&self) -> usize { self.0 }
}

#[derive(Clone)]
pub struct ArtistLoader {
    uri: String,
}

#[async_trait]
impl<Client> ContainerLoader<Client> for ArtistLoader
where
    Client: AlbumsStorageApi + ThreadSafe,
{
    type Item = SimplifiedAlbum;
    type Page = Page<Self::Item>;
    type ParentId = String;

    const NAME: &'static str = "artist's albums";

    fn new(uri: Self::ParentId) -> Self { ArtistLoader { uri } }

    fn parent_id(&self) -> &Self::ParentId { &self.uri }

    async fn load_page(self, spotify: AsyncCell<Client>, offset: u32) -> ClientResult<Self::Page> {
        spotify.read().await.get_artist_albums(&self.uri, offset, 20).await
    }
}
