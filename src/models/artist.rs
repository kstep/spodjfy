use crate::models::common::*;
use gdk_pixbuf::Pixbuf;
use glib::{IsA, StaticType, Type};
use gtk::prelude::GtkListStoreExtManual;
use itertools::Itertools;
use rspotify::model::{Followers, FullArtist, Image, SimplifiedArtist, Type as ModelType};

pub mod constants {
    use crate::models::{COL_ITEM_NAME, COL_ITEM_THUMB, COL_ITEM_ID};
    pub const COL_ARTIST_THUMB: u32 = COL_ITEM_THUMB;
    pub const COL_ARTIST_ID: u32 = COL_ITEM_ID;
    pub const COL_ARTIST_NAME: u32 = COL_ITEM_NAME;
    pub const COL_ARTIST_GENRES: u32 = 3;
    pub const COL_ARTIST_RATE: u32 = 4;
    pub const COL_ARTIST_FOLLOWERS: u32 = 5;
}
pub use self::constants::*;

pub trait ArtistLike: HasId + HasDuration + HasImages + HasName {
    fn rate(&self) -> u32;
    fn followers(&self) -> u32;
    fn genres(&self) -> &[String] { &[] }
}

impl ArtistLike for SimplifiedArtist {
    fn rate(&self) -> u32 { 0 }

    fn followers(&self) -> u32 { 0 }
}

impl HasId for SimplifiedArtist {
    fn id(&self) -> &str { self.id.as_ref().map(AsRef::as_ref).unwrap_or("") }
}

impl HasName for SimplifiedArtist {
    fn name(&self) -> &str { &self.name }
}

impl HasDuration for SimplifiedArtist {
    fn duration_exact(&self) -> bool { false }
}

impl HasImages for SimplifiedArtist {
    fn images(&self) -> &[Image] { &[] }
}

impl MissingColumns for SimplifiedArtist {
    fn missing_columns() -> &'static [u32] { &[COL_ARTIST_THUMB, COL_ARTIST_GENRES, COL_ARTIST_RATE, COL_ARTIST_FOLLOWERS] }
}

impl RowLike for SimplifiedArtist {
    fn content_types() -> Vec<Type> { FullArtist::content_types() }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter {
        store.insert_with_values(None, &[COL_ARTIST_ID, COL_ARTIST_NAME], &[&self.id(), &self.name])
    }
}

impl ArtistLike for FullArtist {
    fn rate(&self) -> u32 { self.popularity }

    fn followers(&self) -> u32 { self.followers.total }

    fn genres(&self) -> &[String] { &self.genres }
}

impl HasId for FullArtist {
    fn id(&self) -> &str { self.id.as_ref() }
}

impl HasName for FullArtist {
    fn name(&self) -> &str { &self.name }
}

impl HasDuration for FullArtist {
    fn duration_exact(&self) -> bool { false }
}

impl HasImages for FullArtist {
    fn images(&self) -> &[Image] { &self.images }
}

impl RowLike for FullArtist {
    fn content_types() -> Vec<Type> {
        vec![
            Pixbuf::static_type(), // thumb
            String::static_type(), // uri
            String::static_type(), // name
            String::static_type(), // genres
            u32::static_type(),    // rate/popularity
            u64::static_type(),    // followers
        ]
    }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter {
        store.insert_with_values(
            None,
            &[
                COL_ARTIST_ID,
                COL_ARTIST_NAME,
                COL_ARTIST_GENRES,
                COL_ARTIST_RATE,
                COL_ARTIST_FOLLOWERS,
            ],
            &[
                &self.id(),
                &self.name,
                &self.genres.iter().join(", "),
                &self.popularity,
                &self.followers(),
            ],
        )
    }
}

impl MissingColumns for FullArtist {}

impl ToFull for SimplifiedArtist {
    type Full = FullArtist;

    fn to_full(&self) -> Self::Full {
        FullArtist {
            external_urls: self.external_urls.clone(),
            followers: Followers { total: 0 },
            genres: Vec::new(),
            href: self.href.clone().unwrap_or_else(String::new),
            id: self.id.clone().unwrap_or_else(String::new),
            images: Vec::new(),
            name: self.name.clone(),
            popularity: 0,
        }
    }

    fn into_full(self) -> Self::Full {
        FullArtist {
            external_urls: self.external_urls,
            followers: Followers { total: 0 },
            genres: Vec::new(),
            href: self.href.unwrap_or_else(String::new),
            id: self.id.unwrap_or_else(String::new),
            images: Vec::new(),
            name: self.name,
            popularity: 0,
        }
    }
}

impl ToSimple for FullArtist {
    type Simple = SimplifiedArtist;

    fn to_simple(&self) -> Self::Simple {
        SimplifiedArtist {
            external_urls: self.external_urls.clone(),
            href: Some(self.href.clone()),
            id: Some(self.id.clone()),
            name: self.name.clone(),
        }
    }

    fn into_simple(self) -> Self::Simple {
        SimplifiedArtist {
            external_urls: self.external_urls,
            href: Some(self.href),
            id: Some(self.id),
            name: self.name,
        }
    }
}

impl Merge for FullArtist {
    fn merge(self, other: Self) -> Self {
        FullArtist {
            external_urls: self.external_urls.merge(other.external_urls),
            followers: other.followers,
            genres: self.genres.merge(other.genres),
            href: self.href.merge(other.href),
            id: self.id,
            images: self.images.merge(other.images),
            name: self.name.merge(other.name),
            popularity: self.popularity.merge(other.popularity),
        }
    }
}
