use crate::models::common::*;
use chrono::{DateTime, Utc};
use gdk_pixbuf::Pixbuf;
use glib::{IsA, StaticType, Type};
use gtk::prelude::GtkListStoreExtManual;
use itertools::Itertools;
use rspotify::model::{
    AlbumType, DatePrecision, FullAlbum, Image, Page, SavedAlbum, SimplifiedAlbum, SimplifiedArtist, Type as ModelType,
};
use std::{collections::HashMap, time::SystemTime};

pub const COL_ALBUM_THUMB: u32 = COL_ITEM_THUMB;

pub const COL_ALBUM_URI: u32 = COL_ITEM_URI;

pub const COL_ALBUM_NAME: u32 = COL_ITEM_NAME;

pub const COL_ALBUM_RELEASE_DATE: u32 = 3;

pub const COL_ALBUM_TOTAL_TRACKS: u32 = 4;

pub const COL_ALBUM_ARTISTS: u32 = 5;

pub const COL_ALBUM_GENRES: u32 = 6;

pub const COL_ALBUM_TYPE: u32 = 7;

pub const COL_ALBUM_DURATION: u32 = 8;

pub const COL_ALBUM_RATE: u32 = 9;

pub trait AlbumLike: HasDuration + HasImages + HasUri + HasName {
    fn release_date(&self) -> &str;

    fn total_tracks(&self) -> u32 { 0 }

    fn artists(&self) -> &[SimplifiedArtist];

    fn genres(&self) -> &[String] { &[] }

    fn kind(&self) -> AlbumType;

    fn rate(&self) -> u32;

    fn insert_into_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter {
        store.insert_with_values(
            None,
            &[
                COL_ALBUM_URI,
                COL_ALBUM_NAME,
                COL_ALBUM_RELEASE_DATE,
                COL_ALBUM_TOTAL_TRACKS,
                COL_ALBUM_ARTISTS,
                COL_ALBUM_GENRES,
                COL_ALBUM_TYPE,
                COL_ALBUM_DURATION,
                COL_ALBUM_RATE,
            ],
            &[
                &self.uri(),
                &self.name(),
                &self.release_date(),
                &self.total_tracks(),
                &self.artists().iter().map(|artist| &artist.name).join(", "),
                &self.genres().iter().join(", "),
                &(self.kind() as u8),
                &self.duration(),
                &self.rate(),
            ],
        )
    }

    fn store_content_types() -> Vec<Type> {
        vec![
            Pixbuf::static_type(), // thumb
            String::static_type(), // uri
            String::static_type(), // name
            String::static_type(), // release date
            u32::static_type(),    // total tracks
            String::static_type(), // artists
            String::static_type(), // genres
            u8::static_type(),     // type
            u32::static_type(),    // duration
            u32::static_type(),    // rate/popularity
        ]
    }
}

impl AlbumLike for FullAlbum {
    fn release_date(&self) -> &str { &self.release_date }

    fn total_tracks(&self) -> u32 { self.tracks.total }

    fn artists(&self) -> &[SimplifiedArtist] { &self.artists }

    fn genres(&self) -> &[String] { &self.genres }

    fn kind(&self) -> AlbumType { self.album_type }

    fn rate(&self) -> u32 { self.popularity }
}

impl HasName for FullAlbum {
    fn name(&self) -> &str { &self.name }
}

impl HasUri for FullAlbum {
    fn uri(&self) -> &str { &self.uri }
}

impl HasDuration for FullAlbum {
    fn duration(&self) -> u32 { self.tracks.items.iter().map(|track| track.duration_ms).sum() }

    fn duration_exact(&self) -> bool { self.tracks.total as usize == self.tracks.items.len() }
}

impl MissingColumns for FullAlbum {}

impl HasImages for FullAlbum {
    fn images(&self) -> &[Image] { &self.images }
}

impl RowLike for FullAlbum {
    fn content_types() -> Vec<Type> { Self::store_content_types() }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter { self.insert_into_store(store) }
}

impl AlbumLike for SimplifiedAlbum {
    fn release_date(&self) -> &str { self.release_date.as_deref().unwrap_or("") }

    fn artists(&self) -> &[SimplifiedArtist] { &self.artists }

    fn kind(&self) -> AlbumType {
        self.album_type.as_ref().map_or(AlbumType::Album, |tpe| match &**tpe {
            "single" => AlbumType::Single,
            "appears_on" => AlbumType::AppearsOn,
            "compilation" => AlbumType::Compilation,
            _ => AlbumType::Album,
        })
    }

    fn rate(&self) -> u32 { 0 }
}

impl HasUri for SimplifiedAlbum {
    fn uri(&self) -> &str { self.uri.as_deref().unwrap_or("") }
}

impl HasName for SimplifiedAlbum {
    fn name(&self) -> &str { &self.name }
}

impl HasDuration for SimplifiedAlbum {
    fn duration_exact(&self) -> bool { false }
}

impl MissingColumns for SimplifiedAlbum {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_ALBUM_DURATION, COL_ALBUM_TOTAL_TRACKS, COL_ALBUM_GENRES, COL_ALBUM_RATE]
    }
}

impl HasImages for SimplifiedAlbum {
    fn images(&self) -> &[Image] { &self.images }
}

impl RowLike for SimplifiedAlbum {
    fn content_types() -> Vec<Type> { Self::store_content_types() }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter { self.insert_into_store(store) }
}

impl AlbumLike for SavedAlbum {
    fn release_date(&self) -> &str { self.album.release_date() }

    fn total_tracks(&self) -> u32 { self.album.total_tracks() }

    fn artists(&self) -> &[SimplifiedArtist] { self.album.artists() }

    fn genres(&self) -> &[String] { self.album.genres() }

    fn kind(&self) -> AlbumType { self.album.kind() }

    fn rate(&self) -> u32 { self.album.popularity }
}

impl HasName for SavedAlbum {
    fn name(&self) -> &str { self.album.name() }
}

impl HasUri for SavedAlbum {
    fn uri(&self) -> &str { self.album.uri() }
}

impl HasDuration for SavedAlbum {
    fn duration(&self) -> u32 { self.album.duration() }

    fn duration_exact(&self) -> bool { self.album.duration_exact() }
}

impl MissingColumns for SavedAlbum {}

impl HasImages for SavedAlbum {
    fn images(&self) -> &[Image] { &self.album.images }
}

impl RowLike for SavedAlbum {
    fn content_types() -> Vec<Type> { Self::store_content_types() }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter { self.insert_into_store(store) }
}

impl Wrapper for SavedAlbum {
    type For = FullAlbum;

    fn unwrap(self) -> Self::For { self.album }

    fn wrap(album: Self::For) -> Self {
        SavedAlbum {
            added_at: DateTime::<Utc>::from(SystemTime::now()),
            album,
        }
    }
}

impl ToFull for SimplifiedAlbum {
    type Full = FullAlbum;

    fn to_full(&self) -> Self::Full {
        FullAlbum {
            artists: self.artists.clone(),
            album_type: self.album_type.clone().map_or(AlbumType::Album, |tpe| match &*tpe {
                "album" => AlbumType::Album,
                "compilation" => AlbumType::Compilation,
                "appears_on" => AlbumType::AppearsOn,
                "single" => AlbumType::Single,
                _ => unreachable!(),
            }),
            available_markets: self.available_markets.clone(),
            copyrights: Vec::new(),
            external_ids: HashMap::new(),
            external_urls: self.external_urls.clone(),
            genres: Vec::new(),
            href: self.href.clone().unwrap_or_else(String::new),
            id: self.id.clone().unwrap_or_else(String::new),
            images: self.images.clone(),
            name: self.name.clone(),
            popularity: 0,
            release_date: self.release_date.clone().unwrap_or_else(String::new),
            release_date_precision: self
                .release_date_precision
                .clone()
                .map_or(DatePrecision::Year, |prec| match &*prec {
                    "year" => DatePrecision::Year,
                    "month" => DatePrecision::Month,
                    "day" => DatePrecision::Day,
                    _ => unreachable!(),
                }),
            tracks: Page::empty(),
            _type: ModelType::Artist,
            uri: self.uri.clone().unwrap_or_else(String::new),
        }
    }

    fn into_full(self) -> Self::Full {
        FullAlbum {
            artists: self.artists,
            album_type: self.album_type.map_or(AlbumType::Album, |tpe| match &*tpe {
                "single" => AlbumType::Single,
                "appears_on" => AlbumType::AppearsOn,
                "compilation" => AlbumType::Compilation,
                "album" => AlbumType::Album,
                _ => unreachable!(),
            }),
            available_markets: self.available_markets,
            copyrights: Vec::new(),
            external_ids: HashMap::new(),
            external_urls: self.external_urls,
            genres: Vec::new(),
            href: self.href.unwrap_or_else(String::new),
            id: self.id.unwrap_or_else(String::new),
            images: self.images,
            name: self.name,
            popularity: 0,
            release_date: self.release_date.unwrap_or_else(String::new),
            release_date_precision: self
                .release_date_precision
                .map(|prec| match &*prec {
                    "year" => DatePrecision::Year,
                    "month" => DatePrecision::Month,
                    "day" => DatePrecision::Day,
                    _ => unreachable!(),
                })
                .unwrap_or(DatePrecision::Year),
            tracks: Page::empty(),
            _type: ModelType::Artist,
            uri: self.uri.unwrap_or_else(String::new),
        }
    }
}

impl ToSimple for FullAlbum {
    type Simple = SimplifiedAlbum;

    fn to_simple(&self) -> Self::Simple {
        SimplifiedAlbum {
            album_group: None,
            album_type: Some(self.album_type.to_string()),
            artists: self.artists.clone(),
            available_markets: self.available_markets.clone(),
            external_urls: self.external_urls.clone(),
            href: Some(self.href.clone()),
            id: Some(self.id.clone()),
            images: self.images.clone(),
            name: self.name.clone(),
            release_date: Some(self.release_date.clone()),
            release_date_precision: Some(self.release_date_precision.to_string()),
            restrictions: None,
            _type: ModelType::Album,
            uri: Some(self.uri.clone()),
        }
    }

    fn into_simple(self) -> Self::Simple {
        SimplifiedAlbum {
            album_group: None,
            album_type: Some(self.album_type.to_string()),
            artists: self.artists,
            available_markets: self.available_markets,
            external_urls: self.external_urls,
            href: Some(self.href),
            id: Some(self.id),
            images: self.images,
            name: self.name,
            release_date: Some(self.release_date),
            release_date_precision: Some(self.release_date_precision.to_string()),
            restrictions: None,
            _type: ModelType::Album,
            uri: Some(self.uri),
        }
    }
}

impl Merge for FullAlbum {
    fn merge(self, other: Self) -> Self {
        FullAlbum {
            artists: self.artists.merge(other.artists),
            album_type: if self.album_type == AlbumType::Album {
                other.album_type
            } else {
                self.album_type
            },
            available_markets: self.available_markets.merge(other.available_markets),
            copyrights: self.copyrights.merge(other.copyrights),
            external_ids: self.external_ids.merge(other.external_ids),
            external_urls: self.external_urls.merge(other.external_urls),
            genres: self.genres.merge(other.genres),
            href: self.href.merge(other.href),
            id: self.id.merge(other.id),
            images: self.images.merge(other.images),
            name: self.name.merge(other.name),
            popularity: self.popularity.merge(other.popularity),
            release_date: self.release_date.merge(other.release_date),
            release_date_precision: if self.release_date_precision == DatePrecision::Year {
                other.release_date_precision
            } else {
                self.release_date_precision
            },
            tracks: self.tracks.merge(other.tracks),
            _type: ModelType::Album,
            uri: self.uri.merge(other.uri),
        }
    }
}

impl Empty for SimplifiedAlbum {
    fn empty() -> Self {
        SimplifiedAlbum {
            album_group: None,
            album_type: None,
            artists: Vec::new(),
            available_markets: Vec::new(),
            external_urls: Default::default(),
            href: None,
            id: None,
            images: Vec::new(),
            name: String::new(),
            release_date: None,
            release_date_precision: None,
            restrictions: None,
            _type: ModelType::Album,
            uri: None,
        }
    }

    fn is_empty(&self) -> bool { self.name.is_empty() }
}
