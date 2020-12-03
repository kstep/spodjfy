use crate::{
    loaders::common::ContainerLoader,
    services::spotify::{PlaylistsStorageApi, ShowsStorageApi, ThreadSafe},
    utils::AsyncCell,
};
use async_trait::async_trait;
use rspotify::{
    client::ClientResult,
    model::{Page, Show, SimplifiedPlaylist},
};

const NAME: &str = "playlists";

#[derive(Clone, Copy)]
pub struct FeaturedLoader(usize);

#[async_trait]
impl<Client> ContainerLoader<Client> for FeaturedLoader
where
    Client: PlaylistsStorageApi + ThreadSafe,
{
    type Item = SimplifiedPlaylist;
    type Page = Page<Self::Item>;
    type ParentId = ();

    const NAME: &'static str = "featured playlists";

    fn new(_id: Self::ParentId) -> Self { FeaturedLoader(rand::random()) }

    fn parent_id(&self) -> &Self::ParentId { &() }

    async fn load_page(self, spotify: AsyncCell<Client>, offset: u32) -> ClientResult<Self::Page> {
        spotify.read().await.get_featured_playlists(offset, 20).await
    }

    fn epoch(&self) -> usize { self.0 }
}

#[derive(Clone, Copy)]
pub struct SavedLoader(usize);

#[async_trait]
impl<Client> ContainerLoader<Client> for SavedLoader
where
    Client: PlaylistsStorageApi + ThreadSafe,
{
    type Item = SimplifiedPlaylist;
    type Page = Page<Self::Item>;
    type ParentId = ();

    const NAME: &'static str = NAME;

    fn new(_id: Self::ParentId) -> Self { SavedLoader(rand::random()) }

    fn parent_id(&self) -> &Self::ParentId { &() }

    async fn load_page(self, spotify: AsyncCell<Client>, offset: u32) -> ClientResult<Self::Page> {
        spotify.read().await.get_my_playlists(offset, 20).await
    }

    fn epoch(&self) -> usize { self.0 }
}

#[derive(Clone, Copy)]
pub struct ShowsLoader(usize);

#[async_trait]
impl<Client> ContainerLoader<Client> for ShowsLoader
where
    Client: ShowsStorageApi + ThreadSafe,
{
    type Item = Show;
    type Page = Page<Self::Item>;
    type ParentId = ();

    const NAME: &'static str = "shows";

    fn new(_id: Self::ParentId) -> Self { ShowsLoader(rand::random()) }

    fn parent_id(&self) -> &Self::ParentId { &() }

    async fn load_page(self, spotify: AsyncCell<Client>, offset: u32) -> ClientResult<Self::Page> {
        spotify.read().await.get_my_shows(offset, 20).await
    }

    fn epoch(&self) -> usize { self.0 }
}

#[derive(Clone)]
pub struct CategoryLoader {
    id: String,
}

#[async_trait]
impl<Client> ContainerLoader<Client> for CategoryLoader
where
    Client: PlaylistsStorageApi + ThreadSafe,
{
    type Item = SimplifiedPlaylist;
    type Page = Page<Self::Item>;
    type ParentId = String;

    const NAME: &'static str = NAME;

    fn new(id: Self::ParentId) -> Self { CategoryLoader { id } }

    fn parent_id(&self) -> &Self::ParentId { &self.id }

    async fn load_page(self, spotify: AsyncCell<Client>, offset: u32) -> ClientResult<Self::Page> {
        spotify.read().await.get_category_playlists(&self.id, offset, 20).await
    }
}

pub struct UserLoader {
    user_id: String,
}

#[async_trait]
impl<Client> ContainerLoader<Client> for UserLoader
where
    Client: PlaylistsStorageApi + ThreadSafe,
{
    type Item = SimplifiedPlaylist;
    type Page = Page<Self::Item>;
    type ParentId = String;

    const NAME: &'static str = "user playlists";

    fn new(user_id: Self::ParentId) -> Self { UserLoader { user_id } }

    fn parent_id(&self) -> &Self::ParentId { &self.user_id }

    async fn load_page(self, spotify: AsyncCell<Client>, offset: u32) -> ClientResult<Self::Page> {
        spotify.read().await.get_user_playlists(&self.user_id, offset, 20).await
    }
}
