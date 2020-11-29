use crate::models::common::*;
use gdk_pixbuf::Pixbuf;
use glib::{IsA, StaticType, Type};
use gtk::prelude::GtkListStoreExtManual;
use rspotify::model::{FullPlaylist, Image, SimplifiedPlaylist, Type as ModelType};
use serde_json::Value;
use std::collections::HashMap;

pub const COL_PLAYLIST_THUMB: u32 = COL_ITEM_THUMB;
pub const COL_PLAYLIST_URI: u32 = COL_ITEM_URI;
pub const COL_PLAYLIST_NAME: u32 = COL_ITEM_NAME;
pub const COL_PLAYLIST_TOTAL_TRACKS: u32 = 3;
pub const COL_PLAYLIST_DURATION: u32 = 4;
pub const COL_PLAYLIST_DESCRIPTION: u32 = 5;
pub const COL_PLAYLIST_PUBLISHER: u32 = 6;

pub trait PlaylistLike: HasDuration + HasImages + HasUri + HasName {
    fn id(&self) -> &str;
    fn description(&self) -> &str;
    fn publisher(&self) -> &str;

    fn total_tracks(&self) -> u32 {
        0
    }

    fn insert_into_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter {
        store.insert_with_values(
            None,
            &[
                COL_PLAYLIST_URI,
                COL_PLAYLIST_NAME,
                COL_PLAYLIST_TOTAL_TRACKS,
                COL_PLAYLIST_DURATION,
                COL_PLAYLIST_DESCRIPTION,
                COL_PLAYLIST_PUBLISHER,
            ],
            &[
                &self.uri(),
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
            String::static_type(), // uri
            String::static_type(), // name
            u32::static_type(),    // total tracks
            u32::static_type(),    // duration
            String::static_type(), // description
            String::static_type(), // publisher
        ]
    }
}

impl PlaylistLike for SimplifiedPlaylist {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> &str {
        ""
    }

    fn publisher(&self) -> &str {
        self.owner.display_name.as_deref().unwrap_or(&self.owner.id)
    }

    fn total_tracks(&self) -> u32 {
        self.tracks
            .get("total")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as u32
    }
}

impl HasUri for SimplifiedPlaylist {
    fn uri(&self) -> &str {
        &self.uri
    }
}

impl HasName for SimplifiedPlaylist {
    fn name(&self) -> &str {
        &self.name
    }
}

impl HasDuration for SimplifiedPlaylist {
    fn duration_exact(&self) -> bool {
        false
    }
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
    fn images(&self) -> &[Image] {
        &self.images
    }
}

impl RowLike for SimplifiedPlaylist {
    fn content_types() -> Vec<Type> {
        Self::store_content_types()
    }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter {
        self.insert_into_store(store)
    }
}

impl PlaylistLike for FullPlaylist {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn publisher(&self) -> &str {
        self.owner.display_name.as_deref().unwrap_or(&self.owner.id)
    }

    fn total_tracks(&self) -> u32 {
        self.tracks.total
    }
}

impl HasUri for FullPlaylist {
    fn uri(&self) -> &str {
        &self.uri
    }
}

impl HasName for FullPlaylist {
    fn name(&self) -> &str {
        &self.name
    }
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
            tracks: {
                let mut map = HashMap::new();
                map.insert("href".to_owned(), Value::String(String::new()));
                map.insert("total".to_owned(), Value::Number(self.tracks.total.into()));
                map
            },
            _type: ModelType::Playlist,
            uri: self.uri.clone(),
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
            tracks: {
                let mut map = HashMap::new();
                map.insert("href".to_owned(), Value::String(String::new()));
                map.insert("total".to_owned(), Value::Number(self.tracks.total.into()));
                map
            },
            _type: ModelType::Playlist,
            uri: self.uri,
        }
    }
}

impl HasDuration for FullPlaylist {
    fn duration(&self) -> u32 {
        self.tracks
            .items
            .iter()
            .flat_map(|track| track.track.as_ref())
            .map(|track| track.duration_ms)
            .sum()
    }

    fn duration_exact(&self) -> bool {
        self.tracks.total as usize == self.tracks.items.len()
    }
}

impl HasImages for FullPlaylist {
    fn images(&self) -> &[Image] {
        &self.images
    }
}

impl RowLike for FullPlaylist {
    fn content_types() -> Vec<Type> {
        Self::store_content_types()
    }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter {
        self.insert_into_store(store)
    }
}
