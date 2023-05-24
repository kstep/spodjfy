use crate::{
    loaders::ContainerLoader,
    services::api::{ArtistsStorageApi, ThreadSafe},
    utils::AsyncCell,
};
use async_trait::async_trait;
use rspotify::{
    ClientResult,
    model::{CursorBasedPage, FullArtist, Page},
};
use rspotify::model::ArtistId;

const NAME: &str = "artists";

#[derive(Clone, Copy)]
pub struct SavedLoader(usize);

#[async_trait]
impl<Client> ContainerLoader<Client> for SavedLoader
where
    Client: ArtistsStorageApi + ThreadSafe,
{
    type Item = FullArtist;
    type Page = CursorBasedPage<Self::Item>;
    type ParentId = ();

    const NAME: &'static str = NAME;

    fn new(_id: Self::ParentId) -> Self { SavedLoader(rand::random()) }

    fn parent_id(&self) -> &Self::ParentId { &() }

    async fn load_page(self, spotify: AsyncCell<Client>, cursor: String) -> ClientResult<Self::Page> {
        let cursor = if cursor.is_empty() { None } else { Some(cursor) };
        spotify.read().await.get_my_artists(cursor, 20).await
    }

    fn epoch(&self) -> usize { self.0 }
}

#[derive(Clone, Copy)]
pub struct MyTopArtistsLoader(usize);

#[async_trait]
impl<Client> ContainerLoader<Client> for MyTopArtistsLoader
where
    Client: ArtistsStorageApi + ThreadSafe,
{
    type Item = FullArtist;
    type Page = Page<Self::Item>;
    type ParentId = ();

    const NAME: &'static str = "top artists";

    fn new(_id: Self::ParentId) -> Self { MyTopArtistsLoader(rand::random()) }

    fn parent_id(&self) -> &Self::ParentId { &() }

    async fn load_page(self, spotify: AsyncCell<Client>, offset: u32) -> ClientResult<Self::Page> {
        spotify.read().await.get_my_top_artists(offset, 20).await
    }

    fn epoch(&self) -> usize { self.0 }
}

#[derive(Clone)]
pub struct RelatedArtistsLoader {
    id: ArtistId,
}

#[async_trait]
impl<Client> ContainerLoader<Client> for RelatedArtistsLoader
where
    Client: ArtistsStorageApi + ThreadSafe,
{
    type Item = FullArtist;
    type Page = Vec<Self::Item>;
    type ParentId = ArtistId;

    const NAME: &'static str = "related artists";

    fn new(id: Self::ParentId) -> Self { RelatedArtistsLoader { id } }

    fn parent_id(&self) -> &Self::ParentId { &self.id }

    #[allow(clippy::unit_arg)]
    async fn load_page(self, spotify: AsyncCell<Client>, _offset: ()) -> ClientResult<Self::Page> {
        spotify.read().await.get_artist_related_artists(&self.id).await
    }
}
