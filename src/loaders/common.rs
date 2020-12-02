use crate::models::PageLike;
use crate::services::SpotifyRef;
use async_trait::async_trait;
use rspotify::client::ClientResult;

#[async_trait]
pub trait ContainerLoader {
    type ParentId;
    type Item;
    type Page: PageLike<Self::Item>;
    const PAGE_LIMIT: u32;
    const NAME: &'static str = "items";

    fn new(id: Self::ParentId) -> Self;
    fn parent_id(&self) -> &Self::ParentId;
    async fn load_page(
        self,
        spotify: SpotifyRef,
        offset: <<Self as ContainerLoader>::Page as PageLike<Self::Item>>::Offset,
    ) -> ClientResult<Self::Page>;
    fn epoch(&self) -> usize {
        self as *const _ as *const () as usize
    }
}
