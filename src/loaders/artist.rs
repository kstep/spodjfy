use crate::loaders::ContainerLoader;
use crate::services::SpotifyRef;
use async_trait::async_trait;
use rspotify::client::ClientResult;
use rspotify::model::{CursorBasedPage, FullArtist, Page};

const NAME: &str = "artists";

#[derive(Clone, Copy)]
pub struct SavedLoader(usize);

#[async_trait]
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

    async fn load_page(self, spotify: SpotifyRef, cursor: String) -> ClientResult<Self::Page> {
        let cursor = if cursor.is_empty() {
            None
        } else {
            Some(cursor)
        };
        spotify
            .read()
            .await
            .get_my_artists(cursor, Self::PAGE_LIMIT)
            .await
    }

    fn epoch(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct MyTopArtistsLoader(usize);

#[async_trait]
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

    async fn load_page(self, spotify: SpotifyRef, offset: u32) -> ClientResult<Self::Page> {
        spotify
            .read()
            .await
            .get_my_top_artists(offset, Self::PAGE_LIMIT)
            .await
    }

    fn epoch(&self) -> usize {
        self.0
    }
}

#[derive(Clone)]
pub struct RelatedArtistsLoader {
    artist_id: String,
}

#[async_trait]
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

    #[allow(clippy::unit_arg)]
    async fn load_page(self, spotify: SpotifyRef, _offset: ()) -> ClientResult<Self::Page> {
        spotify
            .read()
            .await
            .get_artist_related_artists(&self.artist_id)
            .await
    }
}
