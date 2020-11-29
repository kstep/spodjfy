use crate::loaders::ContainerLoader;
use crate::servers::spotify::SpotifyCmd;
use crate::servers::ResultSender;
use rspotify::model::{CursorBasedPage, FullArtist, Page};

const NAME: &str = "artists";

#[derive(Clone, Copy)]
pub struct SavedLoader(usize);

impl ContainerLoader for SavedLoader {
    type ParentId = ();
    type Item = FullArtist;
    type Page = CursorBasedPage<Self::Item>;
    const PAGE_LIMIT: u32 = 20;
    const NAME: &'static str = NAME;

    fn new(_id: Self::ParentId) -> Self {
        SavedLoader(rand::random())
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    fn load_page(self, tx: ResultSender<Self::Page>, cursor: String) -> SpotifyCmd {
        let cursor = if cursor.is_empty() {
            None
        } else {
            Some(cursor)
        };
        SpotifyCmd::GetMyArtists {
            tx,
            cursor,
            limit: Self::PAGE_LIMIT,
        }
    }

    fn epoch(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct MyTopArtistsLoader(usize);

impl ContainerLoader for MyTopArtistsLoader {
    type ParentId = ();
    type Item = FullArtist;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;
    const NAME: &'static str = "top artists";

    fn new(_id: Self::ParentId) -> Self {
        MyTopArtistsLoader(rand::random())
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetMyTopArtists {
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
pub struct RelatedArtistsLoader {
    artist_id: String,
}

impl ContainerLoader for RelatedArtistsLoader {
    type ParentId = String;
    type Item = FullArtist;
    type Page = Vec<Self::Item>;
    const PAGE_LIMIT: u32 = 20;
    const NAME: &'static str = "related artists";

    fn new(artist_id: Self::ParentId) -> Self {
        RelatedArtistsLoader { artist_id }
    }

    fn parent_id(&self) -> &Self::ParentId {
        &self.artist_id
    }

    fn load_page(self, tx: ResultSender<Self::Page>, _offset: ()) -> SpotifyCmd {
        SpotifyCmd::GetArtistRelatedArtists {
            tx,
            uri: self.artist_id,
        }
    }
}
