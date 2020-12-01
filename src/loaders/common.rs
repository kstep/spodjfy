use crate::models::PageLike;
use crate::servers::spotify::SpotifyCmd;

pub trait ContainerLoader {
    type ParentId;
    type Item;
    type Page: PageLike<Self::Item>;
    const PAGE_LIMIT: u32;
    const NAME: &'static str = "items";

    fn new(id: Self::ParentId) -> Self;
    fn parent_id(&self) -> &Self::ParentId;
    fn load_page(
        self,
        tx: ResultSender<Self::Page>,
        offset: <<Self as ContainerLoader>::Page as PageLike<Self::Item>>::Offset,
    ) -> SpotifyCmd;
    fn epoch(&self) -> usize {
        self as *const _ as *const () as usize
    }
}
