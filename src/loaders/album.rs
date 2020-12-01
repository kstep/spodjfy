use crate::loaders::ContainerLoader;
use crate::servers::spotify::SpotifyCmd;
use rspotify::model::{Page, SavedAlbum, SimplifiedAlbum};

const NAME: &str = "albums";

#[derive(Clone, Copy)]
pub struct SavedLoader(usize);

impl ContainerLoader for SavedLoader {
    type ParentId = ();
    type Item = SavedAlbum;
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
        SpotifyCmd::GetMyAlbums {
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
pub struct NewReleasesLoader(usize);

impl ContainerLoader for NewReleasesLoader {
    type ParentId = ();
    type Item = SimplifiedAlbum;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;
    const NAME: &'static str = "new releases";

    fn new(_id: Self::ParentId) -> Self {
        NewReleasesLoader(rand::random())
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetNewReleases {
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
pub struct ArtistLoader {
    uri: String,
}
impl ContainerLoader for ArtistLoader {
    type ParentId = String;
    type Item = SimplifiedAlbum;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;
    const NAME: &'static str = "artist's albums";

    fn new(uri: Self::ParentId) -> Self {
        ArtistLoader { uri }
    }

    fn parent_id(&self) -> &Self::ParentId {
        &self.uri
    }

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetArtistAlbums {
            tx,
            offset,
            uri: self.parent_id().clone(),
            limit: Self::PAGE_LIMIT,
        }
    }
}
