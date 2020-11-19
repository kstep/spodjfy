use crate::loaders::paged::PageLike;
use crate::servers::spotify::{ResultSender, SpotifyCmd};

pub trait ContainerLoader: Clone + 'static {
    type ParentId: Clone;
    type Item;
    type Page: PageLike<Self::Item>;
    const PAGE_LIMIT: u32;
    fn new(id: Self::ParentId) -> Self;
    fn parent_id(&self) -> &Self::ParentId;
    fn load_page(
        self,
        tx: ResultSender<Self::Page>,
        offset: <<Self as ContainerLoader>::Page as PageLike<Self::Item>>::Offset,
    ) -> SpotifyCmd;
    fn uuid(&self) -> usize {
        self as *const _ as *const () as usize
    }
}
