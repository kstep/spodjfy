use crate::loaders::paged::PageLike;
use crate::servers::spotify::{ResultSender, SpotifyCmd};
use rspotify::model::Image;

pub const COL_ITEM_THUMB: u32 = 0;
pub const COL_ITEM_URI: u32 = 1;
pub const COL_ITEM_NAME: u32 = 2;

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

pub trait HasImages {
    fn images(&self) -> &[Image];
}

pub trait MissingColumns {
    fn missing_columns() -> &'static [u32] {
        &[]
    }
}
