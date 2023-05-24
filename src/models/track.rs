use crate::{
    models::{
        common::constants::*, Empty, HasDuration, HasImages, HasName, Merge, MissingColumns, RowLike, ToFull, ToSimple,
        Wrapper,
    },
    services::store::StorageModel,
};
use chrono::{DateTime, Utc};
use gdk_pixbuf::Pixbuf;
use glib::{IsA, StaticType, Type};
use gtk::prelude::GtkListStoreExtManual;
use itertools::Itertools;
use rspotify::model::{
    FullTrack, Image, PlayHistory, PlayableItem, PlaylistItem, SavedTrack, SimplifiedAlbum, SimplifiedArtist, SimplifiedTrack,
    Type as ModelType,
};
use std::{collections::HashMap, time::SystemTime};
use crate::models::HasId;

pub mod constants {
    use crate::models::{COL_ITEM_NAME, COL_ITEM_THUMB, COL_ITEM_ID};
    pub const COL_TRACK_THUMB: u32 = COL_ITEM_THUMB;
    pub const COL_TRACK_ID: u32 = COL_ITEM_ID;
    pub const COL_TRACK_NAME: u32 = COL_ITEM_NAME;
    pub const COL_TRACK_ARTISTS: u32 = 3;
    pub const COL_TRACK_NUMBER: u32 = 4;
    pub const COL_TRACK_ALBUM: u32 = 5;
    pub const COL_TRACK_CANT_PLAY: u32 = 6;
    pub const COL_TRACK_DURATION: u32 = 7;
    pub const COL_TRACK_DURATION_MS: u32 = 8;
    pub const COL_TRACK_BPM: u32 = 9;
    pub const COL_TRACK_TIMELINE: u32 = 10;
    pub const COL_TRACK_RELEASE_DATE: u32 = 11;
    pub const COL_TRACK_DESCRIPTION: u32 = 12;
    pub const COL_TRACK_ALBUM_ID: u32 = 13;
    pub const COL_TRACK_ARTIST_ID: u32 = 14;
    pub const COL_TRACK_RATE: u32 = 15;
    pub const COL_TRACK_SAVED: u32 = 16;
}
pub use self::constants::*;

impl Merge for FullTrack {
    fn merge(self, other: FullTrack) -> Self {
        FullTrack {
            album: self.album.merge(other.album),
            artists: self.artists.merge(other.artists),
            available_markets: self.available_markets.merge(other.available_markets),
            disc_number: self.disc_number.merge(other.disc_number),
            duration: self.duration,
            explicit: self.explicit || other.explicit,
            external_ids: self.external_ids.merge(other.external_ids),
            external_urls: self.external_urls.merge(other.external_urls),
            href: self.href.or(other.href),
            id: self.id.or(other.id),
            is_local: self.is_local || other.is_local,
            is_playable: self.is_playable.or(other.is_playable),
            linked_from: self.linked_from.or(other.linked_from),
            restrictions: self.restrictions.or(other.restrictions),
            name: self.name.merge(other.name),
            popularity: self.popularity.merge(other.popularity),
            preview_url: self.preview_url.or(other.preview_url),
            track_number: self.track_number.merge(other.track_number),
        }
    }
}

impl ToSimple for FullTrack {
    type Simple = SimplifiedTrack;

    fn to_simple(&self) -> Self::Simple {
        SimplifiedTrack {
            artists: self.artists.clone(),
            available_markets: Some(self.available_markets.clone()),
            disc_number: self.disc_number,
            duration: self.duration,
            explicit: self.explicit,
            external_urls: self.external_urls.clone(),
            href: self.href.clone(),
            id: self.id.clone(),
            is_local: self.is_local,
            is_playable: None,
            linked_from: None,
            restrictions: None,
            name: self.name.clone(),
            preview_url: self.preview_url.clone(),
            track_number: self.track_number,
        }
    }

    fn into_simple(self) -> Self::Simple {
        SimplifiedTrack {
            artists: self.artists,
            available_markets: Some(self.available_markets),
            disc_number: self.disc_number,
            duration: self.duration,
            explicit: self.explicit,
            external_urls: self.external_urls,
            href: self.href,
            id: self.id,
            is_local: self.is_local,
            is_playable: None,
            linked_from: None,
            restrictions: None,
            name: self.name,
            preview_url: self.preview_url,
            track_number: self.track_number,
        }
    }
}

pub trait TrackLike: HasId + HasDuration + HasImages + HasName {
    fn description(&self) -> Option<&str> { None }

    fn artists(&self) -> &[SimplifiedArtist] { &[] }

    fn number(&self) -> u32 { 0 }

    fn album(&self) -> Option<&SimplifiedAlbum> { None }

    fn is_playable(&self) -> bool { true }

    fn rate(&self) -> u32;

    fn release_date(&self) -> Option<&str> { self.album().and_then(|album| album.release_date.as_deref()) }
}

impl<T: TrackLike> RowLike for T {
    fn content_types() -> Vec<Type> {
        vec![
            Pixbuf::static_type(), // thumb
            String::static_type(), // track id
            String::static_type(), // name
            String::static_type(), // artists
            u32::static_type(),    // number
            String::static_type(), // album
            bool::static_type(),   // is playable
            String::static_type(), // formatted duration
            u32::static_type(),    // duration in ms
            f32::static_type(),    // bpm
            String::static_type(), // duration from start
            String::static_type(), // release date
            String::static_type(), // description
            String::static_type(), // album uri
            String::static_type(), // first artist uri
            u32::static_type(),    // rate/popularity
            bool::static_type(),   // saved in library
        ]
    }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter {
        store.insert_with_values(
            None,
            &[
                COL_TRACK_ID,
                COL_TRACK_NAME,
                COL_TRACK_ARTISTS,
                COL_TRACK_ALBUM,
                COL_TRACK_CANT_PLAY,
                COL_TRACK_DURATION,
                COL_TRACK_DURATION_MS,
                COL_TRACK_RELEASE_DATE,
                COL_TRACK_DESCRIPTION,
                COL_TRACK_ALBUM_ID,
                COL_TRACK_ARTIST_ID,
                COL_TRACK_RATE,
            ],
            &[
                &self.id(),
                &self.name(),
                &self.artists().iter().map(|artist| &artist.name).join(", "),
                &self.album().map(|album| &*album.name),
                &!self.is_playable(),
                &crate::utils::humanize_time(self.duration()),
                &self.duration(),
                &self.release_date(),
                &self.description(),
                &self.album().map(|album| album.id()),
                &self.artists().iter().next().and_then(|artist| artist.id.as_deref()),
                &self.rate(),
            ],
        )
    }
}

impl TrackLike for PlayHistory {
    fn artists(&self) -> &[SimplifiedArtist] { self.track.artists() }

    fn number(&self) -> u32 { self.track.number() }

    fn album(&self) -> Option<&SimplifiedAlbum> { self.track.album() }

    fn is_playable(&self) -> bool { self.track.is_playable() }

    fn rate(&self) -> u32 { self.track.popularity }

    fn release_date(&self) -> Option<&str> { self.track.release_date() }
}

impl HasId for PlayHistory {
    fn id(&self) -> &str { self.track.id() }
}

impl HasName for PlayHistory {
    fn name(&self) -> &str { self.track.name() }
}

impl HasDuration for PlayHistory {
    fn duration(&self) -> u32 { self.track.duration() }
}

impl HasImages for PlayHistory {
    fn images(&self) -> &[Image] { self.album().map(|album| &*album.images).unwrap_or(&[]) }
}

impl MissingColumns for PlayHistory {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_TRACK_DESCRIPTION]
    }
}

impl TrackLike for PlaylistItem {
    fn artists(&self) -> &[SimplifiedArtist] { self.track.as_ref().map(PlayableItem::artists).unwrap_or(&[]) }

    fn number(&self) -> u32 { self.track.as_ref().map(PlayableItem::number).unwrap_or(0) }

    fn album(&self) -> Option<&SimplifiedAlbum> { self.track.as_ref().and_then(PlayableItem::album) }

    fn is_playable(&self) -> bool { self.track.as_ref().map(PlayableItem::is_playable).unwrap_or(false) }

    fn rate(&self) -> u32 { self.track.as_ref().map_or(0, PlayableItem::rate) }

    fn release_date(&self) -> Option<&str> { self.track.as_ref().and_then(PlayableItem::release_date) }
}

impl HasId for PlaylistItem {
    fn id(&self) -> &str { self.track.as_ref().map(PlayableItem::id).unwrap_or("") }
}

impl HasName for PlaylistItem {
    fn name(&self) -> &str { self.track.as_ref().map(PlayableItem::name).unwrap_or("") }
}

impl Wrapper for PlaylistItem {
    type For = PlayableItem;

    fn unwrap(self) -> Self::For { self.track.unwrap() }

    fn wrap(track: Self::For) -> Self {
        PlaylistItem {
            added_at: None,
            added_by: None,
            is_local: false,
            track: Some(track),
        }
    }
}

impl HasDuration for PlaylistItem {
    fn duration(&self) -> u32 { self.track.as_ref().map_or(0, |track| track.duration()) }

    fn duration_exact(&self) -> bool { self.track.is_some() }
}

impl HasImages for PlaylistItem {
    fn images(&self) -> &[Image] { self.album().map(|album| &*album.images).unwrap_or(&[]) }
}

impl MissingColumns for PlaylistItem {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_TRACK_DESCRIPTION]
    }
}

impl TrackLike for FullTrack {
    fn artists(&self) -> &[SimplifiedArtist] { &self.artists }

    fn number(&self) -> u32 { self.track_number }

    fn album(&self) -> Option<&SimplifiedAlbum> { Some(&self.album) }

    fn is_playable(&self) -> bool { self.is_playable.unwrap_or(true) }

    fn rate(&self) -> u32 { self.popularity }

    fn release_date(&self) -> Option<&str> { self.album.release_date.as_deref() }
}

impl HasId for FullTrack {
    fn id(&self) -> &str { self.id.as_deref().unwrap_or("") }
}

impl HasName for FullTrack {
    fn name(&self) -> &str { &self.name }
}

impl HasDuration for FullTrack {
    fn duration(&self) -> u32 { self.duration.as_millis() }
}

impl HasImages for FullTrack {
    fn images(&self) -> &[Image] { self.album().map(|album| &*album.images).unwrap_or(&[]) }
}

impl MissingColumns for FullTrack {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_TRACK_DESCRIPTION]
    }
}

impl TrackLike for SimplifiedTrack {
    fn artists(&self) -> &[SimplifiedArtist] { &self.artists }

    fn number(&self) -> u32 { self.track_number }

    fn rate(&self) -> u32 { 0 }

    fn is_playable(&self) -> bool { self.is_playable.unwrap_or(true) }
}

impl HasId for SimplifiedTrack {
    fn id(&self) -> &str { self.id.as_deref().unwrap_or("") }
}

impl HasName for SimplifiedTrack {
    fn name(&self) -> &str { &self.name }
}

impl HasDuration for SimplifiedTrack {
    fn duration(&self) -> u32 { self.duration_ms }
}

impl HasImages for SimplifiedTrack {
    fn images(&self) -> &[Image] { self.album().map(|album| &*album.images).unwrap_or(&[]) }
}

impl MissingColumns for SimplifiedTrack {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[
            COL_TRACK_ALBUM,
            COL_TRACK_THUMB,
            COL_TRACK_RELEASE_DATE,
            COL_TRACK_DESCRIPTION,
            COL_TRACK_RATE,
        ]
    }
}

impl ToFull for SimplifiedTrack {
    type Full = FullTrack;

    fn to_full(&self) -> Self::Full {
        FullTrack {
            album: SimplifiedAlbum::empty(),
            artists: self.artists.clone(),
            available_markets: self.available_markets.clone().unwrap_or_else(Vec::new),
            disc_number: self.disc_number,
            duration_ms: self.duration_ms,
            explicit: self.explicit,
            external_ids: HashMap::new(),
            external_urls: self.external_urls.clone(),
            href: self.href.clone(),
            id: self.id.clone(),
            is_local: self.is_local,
            is_playable: None,
            linked_from: None,
            restrictions: None,
            name: self.name.clone(),
            popularity: 0,
            preview_url: self.preview_url.clone(),
            track_number: self.track_number,
            _type: ModelType::Track,
            uri: self.uri.clone(),
        }
    }
}

impl TrackLike for SavedTrack {
    fn artists(&self) -> &[SimplifiedArtist] { self.track.artists() }

    fn number(&self) -> u32 { self.track.number() }

    fn album(&self) -> Option<&SimplifiedAlbum> { self.track.album() }

    fn is_playable(&self) -> bool { self.track.is_playable() }

    fn rate(&self) -> u32 { self.track.popularity }

    fn release_date(&self) -> Option<&str> { self.track.release_date() }
}

impl HasId for SavedTrack {
    fn id(&self) -> &str { self.track.id() }
}

impl HasName for SavedTrack {
    fn name(&self) -> &str { self.track.name() }
}

impl Wrapper for SavedTrack {
    type For = FullTrack;

    fn unwrap(self) -> Self::For { self.track }

    fn wrap(track: Self::For) -> Self {
        SavedTrack {
            added_at: DateTime::<Utc>::from(SystemTime::now()),
            track,
        }
    }
}

impl HasDuration for SavedTrack {
    fn duration(&self) -> u32 { self.track.duration_ms }
}

impl HasImages for SavedTrack {
    fn images(&self) -> &[Image] { self.album().map(|album| &*album.images).unwrap_or(&[]) }
}

impl MissingColumns for SavedTrack {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_TRACK_DESCRIPTION]
    }
}

impl HasDuration for PlayableItem {
    fn duration(&self) -> u32 {
        match self {
            PlayableItem::Track(track) => track.duration.as_millis() as u32,
            PlayableItem::Episode(episode) => episode.duration.as_millis() as u32,
        }
    }
}

impl MissingColumns for PlayableItem {}

impl HasImages for PlayableItem {
    fn images(&self) -> &[Image] {
        match self {
            PlayableItem::Track(track) => track.images(),
            PlayableItem::Episode(episode) => episode.images(),
        }
    }
}

impl HasName for PlayableItem {
    fn name(&self) -> &str {
        match self {
            PlayableItem::Track(track) => &track.name,
            PlayableItem::Episode(episode) => &episode.name,
        }
    }
}

macro_rules! impl_track_like_for_playing_item {
    ($($method:ident -> $tpe:ty),+) => {
        impl TrackLike for PlayableItem {
            $(fn $method(&self) -> $tpe {
                match self {
                    PlayableItem::Track(track) => track.$method(),
                    PlayableItem::Episode(episode) => episode.$method(),
                }
            })+
        }
    }
}

impl_track_like_for_playing_item! {
    artists -> &[SimplifiedArtist], number -> u32,
    album -> Option<&SimplifiedAlbum>, is_playable -> bool,
    release_date -> Option<&str>,
    description -> Option<&str>,
    rate -> u32
}

impl HasId for PlayableItem {
    fn id(&self) -> &str {
        match self {
            Self::Episode(episode) => episode.id(),
            Self::Track(track) => track.id(),
        }
    }
}

impl StorageModel for FullTrack {
    const TREE_NAME: &'static str = "tracks";

    fn key(&self) -> &str { self.id() }
}
