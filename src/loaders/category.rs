use crate::loaders::ContainerLoader;
use crate::servers::spotify::SpotifyCmd;
use crate::servers::ResultSender;
use rspotify::model::{Category, Page};
#[derive(Clone, Copy)]
pub struct CategoriesLoader(usize);

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

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetCategories {
            tx,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }

    fn epoch(&self) -> usize {
        self.0
    }
}
