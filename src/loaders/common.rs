use crate::{models::PageLike, utils::AsyncCell, Spotify};
use async_trait::async_trait;
use rspotify::ClientResult;

#[async_trait]

pub trait ContainerLoader<Client = Spotify> {
    type Item;
    type Page: PageLike<Self::Item>;
    type ParentId;
    const NAME: &'static str = "items";

    fn new(id: Self::ParentId) -> Self;
    fn parent_id(&self) -> &Self::ParentId;

    async fn load_page(
        self,
        spotify: AsyncCell<Client>,
        offset: <<Self as ContainerLoader<Client>>::Page as PageLike<Self::Item>>::Offset,
    ) -> ClientResult<Self::Page>;

    fn epoch(&self) -> usize { self as *const _ as *const () as usize }
}
