use crate::loaders::{
    ContainerLoader, HasDuration, HasImages, MissingColumns, RowLike, COL_ITEM_NAME,
    COL_ITEM_THUMB, COL_ITEM_URI,
};
use crate::servers::spotify::{ResultSender, SpotifyCmd};
use gdk_pixbuf::Pixbuf;
use glib::{IsA, StaticType, Type};
use gtk::prelude::GtkListStoreExtManual;
use itertools::Itertools;
use rspotify::model::{CursorBasedPage, FullArtist, Image, Page, SimplifiedArtist};

const NAME: &str = "artists";

pub trait ArtistLike: HasDuration + HasImages {
    fn id(&self) -> &str;
    fn uri(&self) -> &str;
    fn name(&self) -> &str;
    fn rate(&self) -> u32;

    fn genres(&self) -> &[String] {
        &[]
    }
}

impl ArtistLike for SimplifiedArtist {
    fn id(&self) -> &str {
        self.id.as_deref().unwrap_or("")
    }

    fn uri(&self) -> &str {
        self.uri.as_deref().unwrap_or("")
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn rate(&self) -> u32 {
        0
    }
}

impl HasDuration for SimplifiedArtist {
    fn duration_exact(&self) -> bool {
        false
    }
}

impl HasImages for SimplifiedArtist {
    fn images(&self) -> &[Image] {
        &[]
    }
}

impl MissingColumns for SimplifiedArtist {
    fn missing_columns() -> &'static [u32] {
        &[COL_ARTIST_THUMB, COL_ARTIST_GENRES, COL_ARTIST_RATE]
    }
}

impl RowLike for SimplifiedArtist {
    fn content_types() -> Vec<Type> {
        FullArtist::content_types()
    }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter {
        store.insert_with_values(
            None,
            &[COL_ARTIST_URI, COL_ARTIST_NAME],
            &[&self.uri, &self.name],
        )
    }
}

impl ArtistLike for FullArtist {
    fn id(&self) -> &str {
        &self.id
    }

    fn uri(&self) -> &str {
        &self.uri
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn genres(&self) -> &[String] {
        &self.genres
    }

    fn rate(&self) -> u32 {
        self.popularity
    }
}

impl HasDuration for FullArtist {
    fn duration_exact(&self) -> bool {
        false
    }
}

impl HasImages for FullArtist {
    fn images(&self) -> &[Image] {
        &self.images
    }
}

impl RowLike for FullArtist {
    fn content_types() -> Vec<Type> {
        vec![
            Pixbuf::static_type(), // thumb
            String::static_type(), // uri
            String::static_type(), // name
            String::static_type(), // genres
            u32::static_type(),    // rate (a.k.a. popularity)
        ]
    }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter {
        store.insert_with_values(
            None,
            &[
                COL_ARTIST_URI,
                COL_ARTIST_NAME,
                COL_ARTIST_GENRES,
                COL_ARTIST_RATE,
            ],
            &[
                &self.uri,
                &self.name,
                &self.genres.iter().join(", "),
                &self.popularity,
            ],
        )
    }
}

impl MissingColumns for FullArtist {}

pub const COL_ARTIST_THUMB: u32 = COL_ITEM_THUMB;
pub const COL_ARTIST_URI: u32 = COL_ITEM_URI;
pub const COL_ARTIST_NAME: u32 = COL_ITEM_NAME;
pub const COL_ARTIST_GENRES: u32 = 3;
pub const COL_ARTIST_RATE: u32 = 4;

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
    const NAME: &'static str = NAME;

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
