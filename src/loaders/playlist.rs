use crate::loaders::common::ContainerLoader;
use crate::servers::{ResultSender, SpotifyCmd};
use rspotify::model::{Page, Show, SimplifiedPlaylist};

const NAME: &str = "playlists";

#[derive(Clone, Copy)]
pub struct FeaturedLoader(usize);

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

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetFeaturedPlaylists {
            tx,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }

    fn epoch(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct SavedLoader(usize);

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

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetMyPlaylists {
            tx,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }

    fn epoch(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct ShowsLoader(usize);

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

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetMyShows {
            tx,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }

    fn epoch(&self) -> usize {
        self.0
    }
}

#[derive(Clone)]
pub struct CategoryLoader {
    id: String,
}

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

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetCategoryPlaylists {
            tx,
            category_id: self.parent_id().clone(),
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}

pub struct UserLoader {
    user_id: String,
}

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

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetUserPlaylists {
            tx,
            user_id: self.user_id,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}
