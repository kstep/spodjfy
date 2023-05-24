use crate::models::common::*;
use gdk_pixbuf::Pixbuf;
use glib::{IsA, StaticType, Type};
use gtk::prelude::GtkListStoreExtManual;
use rspotify::model::{Followers, FullPlaylist, Image, Page, PlaylistTracksRef, PublicUser, SimplifiedPlaylist, Type as ModelType};
use serde_json::Value;
use std::collections::HashMap;

pub mod constants {
    use crate::models::{COL_ITEM_NAME, COL_ITEM_THUMB, COL_ITEM_ID};
    pub const COL_PLAYLIST_THUMB: u32 = COL_ITEM_THUMB;
    pub const COL_PLAYLIST_ID: u32 = COL_ITEM_ID;
    pub const COL_PLAYLIST_NAME: u32 = COL_ITEM_NAME;
    pub const COL_PLAYLIST_TOTAL_TRACKS: u32 = 3;
    pub const COL_PLAYLIST_DURATION: u32 = 4;
    pub const COL_PLAYLIST_DESCRIPTION: u32 = 5;
    pub const COL_PLAYLIST_PUBLISHER: u32 = 6;
}
pub use self::constants::*;

pub trait PlaylistLike: HasId + HasDuration + HasImages + HasName {
    fn description(&self) -> &str;
    fn publisher(&self) -> &str;
    fn total_tracks(&self) -> u32 { 0 }

    fn insert_into_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter {
        store.insert_with_values(
            None,
            &[
                COL_PLAYLIST_ID,
                COL_PLAYLIST_NAME,
                COL_PLAYLIST_TOTAL_TRACKS,
                COL_PLAYLIST_DURATION,
                COL_PLAYLIST_DESCRIPTION,
                COL_PLAYLIST_PUBLISHER,
            ],
            &[
                &self.id(),
                &self.name(),
                &self.total_tracks(),
                &self.duration(),
                &self.description(),
                &self.publisher(),
            ],
        )
    }

    fn store_content_types() -> Vec<Type> {
        vec![
            Pixbuf::static_type(), // thumb
            String::static_type(), // id
            String::static_type(), // name
            u32::static_type(),    // total tracks
            u32::static_type(),    // duration
            String::static_type(), // description
            String::static_type(), // publisher
        ]
    }
}

impl PlaylistLike for SimplifiedPlaylist {
    fn description(&self) -> &str { "" }

    fn publisher(&self) -> &str { self.owner.display_name.as_deref().unwrap_or(self.owner.id.as_ref()) }

    fn total_tracks(&self) -> u32 { self.tracks.total }
}

impl HasId for SimplifiedPlaylist {
    fn id(&self) -> &str { self.id.as_ref() }
}

impl HasName for SimplifiedPlaylist {
    fn name(&self) -> &str { &self.name }
}

impl HasDuration for SimplifiedPlaylist {
    fn duration_exact(&self) -> bool { false }
}

impl MissingColumns for SimplifiedPlaylist {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_PLAYLIST_DURATION, COL_PLAYLIST_DESCRIPTION]
    }
}

impl HasImages for SimplifiedPlaylist {
    fn images(&self) -> &[Image] { &self.images }
}

impl RowLike for SimplifiedPlaylist {
    fn content_types() -> Vec<Type> { Self::store_content_types() }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter { self.insert_into_store(store) }
}

impl PlaylistLike for FullPlaylist {
    fn description(&self) -> &str { self.description.as_deref().unwrap_or("") }

    fn publisher(&self) -> &str { self.owner.display_name.as_deref().unwrap_or(self.owner.id.as_ref()) }

    fn total_tracks(&self) -> u32 { self.tracks.total }
}

impl HasId for FullPlaylist {
    fn id(&self) -> &str { self.id.as_ref() }
}

impl HasName for FullPlaylist {
    fn name(&self) -> &str { &self.name }
}

impl ToSimple for FullPlaylist {
    type Simple = SimplifiedPlaylist;

    fn to_simple(&self) -> Self::Simple {
        SimplifiedPlaylist {
            collaborative: self.collaborative,
            external_urls: self.external_urls.clone(),
            href: self.href.clone(),
            id: self.id.clone(),
            images: self.images.clone(),
            name: self.name.clone(),
            owner: self.owner.clone(),
            public: self.public,
            snapshot_id: self.snapshot_id.clone(),
            tracks: PlaylistTracksRef {
                href: String::new(),
                total: self.tracks.total,
            },
        }
    }

    fn into_simple(self) -> Self::Simple {
        SimplifiedPlaylist {
            collaborative: self.collaborative,
            external_urls: self.external_urls,
            href: self.href,
            id: self.id,
            images: self.images,
            name: self.name,
            owner: self.owner,
            public: self.public,
            snapshot_id: self.snapshot_id,
            tracks: PlaylistTracksRef {
                href: String::new(),
                total: self.tracks.total,
            },
        }
    }
}

impl HasDuration for FullPlaylist {
    fn duration(&self) -> u32 {
        self.tracks
            .items
            .iter()
            .flat_map(|track| track.track.as_ref())
            .map(|track| track.duration())
            .sum()
    }

    fn duration_exact(&self) -> bool { self.tracks.total as usize == self.tracks.items.len() }
}

impl HasImages for FullPlaylist {
    fn images(&self) -> &[Image] { &self.images }
}

impl RowLike for FullPlaylist {
    fn content_types() -> Vec<Type> { Self::store_content_types() }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter { self.insert_into_store(store) }
}

impl ToFull for SimplifiedPlaylist {
    type Full = FullPlaylist;

    fn to_full(&self) -> Self::Full {
        FullPlaylist {
            collaborative: self.collaborative,
            description: None,
            external_urls: self.external_urls.clone(),
            followers: Followers { total: 0 },
            href: self.href.clone(),
            id: self.id.clone(),
            images: self.images.clone(),
            name: self.name.clone(),
            owner: self.owner.clone(),
            public: self.public,
            snapshot_id: self.snapshot_id.clone(),
            tracks: Page::empty(),
        }
    }

    fn into_full(self) -> Self::Full {
        FullPlaylist {
            collaborative: self.collaborative,
            description: None,
            external_urls: self.external_urls,
            followers: Followers { total: 0 },
            href: self.href,
            id: self.id,
            images: self.images,
            name: self.name,
            owner: self.owner,
            public: self.public,
            snapshot_id: self.snapshot_id,
            tracks: Page::empty(),
        }
    }
}

impl Merge for FullPlaylist {
    fn merge(self, other: Self) -> Self {
        FullPlaylist {
            collaborative: self.collaborative || other.collaborative,
            description: self.description.merge(other.description),
            external_urls: self.external_urls.merge(other.external_urls),
            followers: Followers {
                total: self.followers.total.merge(other.followers.total),
            },
            href: self.href.merge(other.href),
            id: self.id,
            images: self.images.merge(other.images),
            name: self.name.merge(other.name),
            owner: PublicUser {
                display_name: self.owner.display_name.merge(other.owner.display_name),
                external_urls: self.owner.external_urls.merge(other.owner.external_urls),
                followers: self.owner.followers.merge(other.owner.followers),
                href: self.owner.href.merge(other.owner.href),
                id: self.owner.id,
                images: self.owner.images.merge(other.owner.images),
            },
            public: self.public.merge(other.public),
            snapshot_id: self.snapshot_id.merge(other.snapshot_id),
            tracks: self.tracks.merge(other.tracks),
        }
    }
}
