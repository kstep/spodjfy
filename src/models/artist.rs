use crate::models::common::*;
use gdk_pixbuf::Pixbuf;
use glib::{IsA, StaticType, Type};
use gtk::prelude::GtkListStoreExtManual;
use itertools::Itertools;
use rspotify::model::{Followers, FullArtist, Image, SimplifiedArtist, Type as ModelType};

pub const COL_ARTIST_THUMB: u32 = COL_ITEM_THUMB;

pub const COL_ARTIST_URI: u32 = COL_ITEM_URI;

pub const COL_ARTIST_NAME: u32 = COL_ITEM_NAME;

pub const COL_ARTIST_GENRES: u32 = 3;

pub const COL_ARTIST_RATE: u32 = 4;

pub const COL_ARTIST_FOLLOWERS: u32 = 5;

pub trait ArtistLike: HasDuration + HasImages + HasUri + HasName {
    fn id(&self) -> &str;

    fn rate(&self) -> u32;

    fn followers(&self) -> u32;

    fn genres(&self) -> &[String] { &[] }
}

impl ArtistLike for SimplifiedArtist {
    fn id(&self) -> &str { self.id.as_deref().unwrap_or("") }

    fn rate(&self) -> u32 { 0 }

    fn followers(&self) -> u32 { 0 }
}

impl HasUri for SimplifiedArtist {
    fn uri(&self) -> &str { self.uri.as_deref().unwrap_or("") }
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
        store.insert_with_values(None, &[COL_ARTIST_URI, COL_ARTIST_NAME], &[&self.uri, &self.name])
    }
}

impl ArtistLike for FullArtist {
    fn id(&self) -> &str { &self.id }

    fn rate(&self) -> u32 { self.popularity }

    fn followers(&self) -> u32 { self.followers.total }

    fn genres(&self) -> &[String] { &self.genres }
}

impl HasUri for FullArtist {
    fn uri(&self) -> &str { &self.uri }
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
                COL_ARTIST_URI,
                COL_ARTIST_NAME,
                COL_ARTIST_GENRES,
                COL_ARTIST_RATE,
                COL_ARTIST_FOLLOWERS,
            ],
            &[
                &self.uri,
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
            _type: ModelType::Artist,
            uri: self.uri.clone().unwrap_or_else(String::new),
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
            _type: ModelType::Artist,
            uri: self.uri.unwrap_or_else(String::new),
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
            _type: ModelType::Artist,
            uri: Some(self.uri.clone()),
        }
    }

    fn into_simple(self) -> Self::Simple {
        SimplifiedArtist {
            external_urls: self.external_urls,
            href: Some(self.href),
            id: Some(self.id),
            name: self.name,
            _type: ModelType::Artist,
            uri: Some(self.uri),
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
            id: self.id.merge(other.id),
            images: self.images.merge(other.images),
            name: self.name.merge(other.name),
            popularity: self.popularity.merge(other.popularity),
            _type: ModelType::Artist,
            uri: self.uri.merge(other.uri),
        }
    }
}
