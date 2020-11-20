use crate::loaders::common::ContainerLoader;
use crate::servers::spotify::{ResultSender, SpotifyCmd};
use glib::IsA;
use gtk::prelude::GtkListStoreExtManual;
use rspotify::model::{
    FullPlaylist, FullShow, Image, Page, Show, SimplifiedPlaylist, SimplifiedShow,
};

pub const COL_PLAYLIST_THUMB: u32 = 0;
pub const COL_PLAYLIST_URI: u32 = 1;
pub const COL_PLAYLIST_NAME: u32 = 2;
pub const COL_PLAYLIST_TOTAL_TRACKS: u32 = 3;
pub const COL_PLAYLIST_DURATION: u32 = 4;
pub const COL_PLAYLIST_DESCRIPTION: u32 = 5;
pub const COL_PLAYLIST_PUBLISHER: u32 = 6;

pub trait PlaylistLike {
    fn id(&self) -> &str;
    fn uri(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn publisher(&self) -> &str;

    fn images(&self) -> &[Image];
    fn total_tracks(&self) -> u32 {
        0
    }
    fn duration(&self) -> u32 {
        0
    }
    fn duration_exact(&self) -> bool {
        false
    }
    fn unavailable_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[]
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
}

impl PlaylistLike for SimplifiedPlaylist {
    fn id(&self) -> &str {
        &self.id
    }

    fn uri(&self) -> &str {
        &self.uri
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        ""
    }

    fn publisher(&self) -> &str {
        self.owner.display_name.as_deref().unwrap_or(&self.owner.id)
    }

    fn images(&self) -> &[Image] {
        &self.images
    }

    fn total_tracks(&self) -> u32 {
        self.tracks
            .get("total")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as u32
    }

    fn unavailable_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_PLAYLIST_DURATION, COL_PLAYLIST_DESCRIPTION]
    }
}

impl PlaylistLike for FullPlaylist {
    fn id(&self) -> &str {
        &self.id
    }

    fn uri(&self) -> &str {
        &self.uri
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn publisher(&self) -> &str {
        self.owner.display_name.as_deref().unwrap_or(&self.owner.id)
    }

    fn images(&self) -> &[Image] {
        &self.images
    }

    fn total_tracks(&self) -> u32 {
        self.tracks.total
    }

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

impl PlaylistLike for FullShow {
    fn id(&self) -> &str {
        &self.id
    }

    fn uri(&self) -> &str {
        &self.uri
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn publisher(&self) -> &str {
        &self.publisher
    }

    fn images(&self) -> &[Image] {
        &self.images
    }

    fn total_tracks(&self) -> u32 {
        self.episodes.total
    }

    fn duration(&self) -> u32 {
        self.episodes
            .items
            .iter()
            .map(|episode| episode.duration_ms)
            .sum()
    }

    fn duration_exact(&self) -> bool {
        self.episodes.items.len() == self.episodes.total as usize
    }
}

impl PlaylistLike for SimplifiedShow {
    fn id(&self) -> &str {
        &self.id
    }

    fn uri(&self) -> &str {
        &self.uri
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn publisher(&self) -> &str {
        &self.publisher
    }

    fn images(&self) -> &[Image] {
        &self.images
    }

    fn unavailable_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_PLAYLIST_TOTAL_TRACKS, COL_PLAYLIST_DURATION]
    }
}

impl PlaylistLike for Show {
    fn id(&self) -> &str {
        self.show.id()
    }

    fn uri(&self) -> &str {
        self.show.uri()
    }

    fn name(&self) -> &str {
        self.show.name()
    }

    fn description(&self) -> &str {
        self.show.description()
    }

    fn publisher(&self) -> &str {
        self.show.publisher()
    }

    fn images(&self) -> &[Image] {
        self.show.images()
    }

    fn unavailable_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_PLAYLIST_TOTAL_TRACKS, COL_PLAYLIST_DURATION]
    }
}

#[derive(Clone, Copy)]
pub struct FeaturedLoader;
impl ContainerLoader for FeaturedLoader {
    type ParentId = ();
    type Item = SimplifiedPlaylist;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;

    fn new(_id: Self::ParentId) -> Self {
        FeaturedLoader
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetFeaturedPlaylists {
            tx,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}

#[derive(Clone, Copy)]
pub struct SavedLoader;
impl ContainerLoader for SavedLoader {
    type ParentId = ();
    type Item = SimplifiedPlaylist;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;

    fn new(_id: Self::ParentId) -> Self {
        SavedLoader
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetMyPlaylists {
            tx,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}

#[derive(Clone, Copy)]
pub struct ShowsLoader;
impl ContainerLoader for ShowsLoader {
    type ParentId = ();
    type Item = Show;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;

    fn new(_id: Self::ParentId) -> Self {
        ShowsLoader
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetMyShows {
            tx,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}

#[derive(Clone)]
pub struct CategoryLoader {
    id: String,
}
impl ContainerLoader for CategoryLoader {
    type ParentId = String;
    type Item = SimplifiedPlaylist;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;

    fn new(id: Self::ParentId) -> Self {
        CategoryLoader { id }
    }

    fn parent_id(&self) -> &Self::ParentId {
        &self.id
    }

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetCategoryPlaylists {
            tx,
            category_id: self.parent_id().clone(),
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}
