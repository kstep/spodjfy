use crate::loaders::ContainerLoader;
use crate::Spotify;
use async_trait::async_trait;
use rspotify::client::ClientResult;
use rspotify::model::{Category, Page};
use std::ops::Deref;

#[derive(Clone, Copy)]
pub struct CategoriesLoader(usize);

#[async_trait]
impl ContainerLoader for CategoriesLoader {
    type ParentId = ();
    type Item = Category;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;
    const NAME: &'static str = "categories";

    fn new(_id: Self::ParentId) -> Self {
        CategoriesLoader(rand::random())
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    async fn load_page(
        self,
        spotify: impl Deref<Target = Spotify> + Send + 'static,
        offset: u32,
    ) -> ClientResult<Self::Page> {
        spotify.get_categories(offset, Self::PAGE_LIMIT).await
    }

    fn epoch(&self) -> usize {
        self.0
    }
}
