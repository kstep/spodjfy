use crate::{
    loaders::ContainerLoader,
    services::api::{PlaylistsStorageApi, ThreadSafe},
    utils::AsyncCell,
};
use async_trait::async_trait;
use rspotify::{
    ClientResult,
    model::{Category, Page},
};

#[derive(Clone, Copy)]
pub struct CategoriesLoader(usize);

#[async_trait]
impl<Client> ContainerLoader<Client> for CategoriesLoader
where
    Client: PlaylistsStorageApi + ThreadSafe,
{
    type Item = Category;
    type Page = Page<Self::Item>;
    type ParentId = ();

    const NAME: &'static str = "categories";

    fn new(_id: Self::ParentId) -> Self { CategoriesLoader(rand::random()) }

    fn parent_id(&self) -> &Self::ParentId { &() }

    async fn load_page(self, spotify: AsyncCell<Client>, offset: u32) -> ClientResult<Self::Page> {
        spotify.read().await.get_categories(offset, 20).await
    }

    fn epoch(&self) -> usize { self.0 }
}
