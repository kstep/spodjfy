use crate::loaders::common::ContainerLoader;
use crate::Spotify;
use async_trait::async_trait;
use rspotify::client::ClientResult;
use rspotify::model::{Page, Show, SimplifiedPlaylist};
use std::ops::Deref;

const NAME: &str = "playlists";

#[derive(Clone, Copy)]
pub struct FeaturedLoader(usize);

#[async_trait]
impl ContainerLoader for FeaturedLoader {
    type ParentId = ();
    type Item = SimplifiedPlaylist;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;
    const NAME: &'static str = "featured playlists";

    fn new(_id: Self::ParentId) -> Self {
        FeaturedLoader(rand::random())
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    async fn load_page(
        self,
        spotify: impl Deref<Target = Spotify> + Send + 'static,
        offset: u32,
    ) -> ClientResult<Self::Page> {
        spotify
            .get_featured_playlists(offset, Self::PAGE_LIMIT)
            .await
    }

    fn epoch(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct SavedLoader(usize);

#[async_trait]
impl ContainerLoader for SavedLoader {
    type ParentId = ();
    type Item = SimplifiedPlaylist;
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
        spotify.get_my_playlists(offset, Self::PAGE_LIMIT).await
    }

    fn epoch(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct ShowsLoader(usize);

#[async_trait]
impl ContainerLoader for ShowsLoader {
    type ParentId = ();
    type Item = Show;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;
    const NAME: &'static str = "shows";

    fn new(_id: Self::ParentId) -> Self {
        ShowsLoader(rand::random())
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    async fn load_page(
        self,
        spotify: impl Deref<Target = Spotify> + Send + 'static,
        offset: u32,
    ) -> ClientResult<Self::Page> {
        spotify.get_my_shows(offset, Self::PAGE_LIMIT).await
    }

    fn epoch(&self) -> usize {
        self.0
    }
}

#[derive(Clone)]
pub struct CategoryLoader {
    id: String,
}

#[async_trait]
impl ContainerLoader for CategoryLoader {
    type ParentId = String;
    type Item = SimplifiedPlaylist;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;
    const NAME: &'static str = NAME;

    fn new(id: Self::ParentId) -> Self {
        CategoryLoader { id }
    }

    fn parent_id(&self) -> &Self::ParentId {
        &self.id
    }

    async fn load_page(
        self,
        spotify: impl Deref<Target = Spotify> + Send + 'static,
        offset: u32,
    ) -> ClientResult<Self::Page> {
        spotify
            .get_category_playlists(&self.id, offset, Self::PAGE_LIMIT)
            .await
    }
}

pub struct UserLoader {
    user_id: String,
}

#[async_trait]
impl ContainerLoader for UserLoader {
    type ParentId = String;
    type Item = SimplifiedPlaylist;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;
    const NAME: &'static str = "user playlists";

    fn new(user_id: Self::ParentId) -> Self {
        UserLoader { user_id }
    }

    fn parent_id(&self) -> &Self::ParentId {
        &self.user_id
    }

    async fn load_page(
        self,
        spotify: impl Deref<Target = Spotify> + Send + 'static,
        offset: u32,
    ) -> ClientResult<Self::Page> {
        spotify
            .get_user_playlists(&self.user_id, offset, Self::PAGE_LIMIT)
            .await
    }
}
