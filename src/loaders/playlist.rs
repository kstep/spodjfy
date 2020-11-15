use crate::loaders::paged::PageLike;
use crate::servers::spotify::{ResultSender, SpotifyCmd};
use rspotify::model::{FullPlaylist, Image, Page, SimplifiedPlaylist};

pub const COL_PLAYLIST_THUMB: u32 = 0;
pub const COL_PLAYLIST_URI: u32 = 1;
pub const COL_PLAYLIST_NAME: u32 = 2;
pub const COL_PLAYLIST_TOTAL_TRACKS: u32 = 3;
pub const COL_PLAYLIST_DURATION: u32 = 4;

pub trait PlaylistLike {
    fn id(&self) -> &str;
    fn uri(&self) -> &str;
    fn name(&self) -> &str;

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
        &[COL_PLAYLIST_DURATION]
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

pub trait PlaylistsLoader: Clone + 'static {
    type ParentId;
    type Playlist: PlaylistLike;
    type Page: PageLike<Self::Playlist>;
    const PAGE_LIMIT: u32;
    fn new(id: Self::ParentId) -> Self;
    fn parent_id(&self) -> Self::ParentId;
    fn load_page(
        self,
        tx: ResultSender<Self::Page>,
        offset: <<Self as PlaylistsLoader>::Page as PageLike<Self::Playlist>>::Offset,
    ) -> SpotifyCmd;
    fn uuid(&self) -> usize {
        self as *const _ as *const () as usize
    }
}

#[derive(Clone, Copy)]
pub struct FeaturedLoader;
impl PlaylistsLoader for FeaturedLoader {
    type ParentId = ();
    type Playlist = SimplifiedPlaylist;
    type Page = Page<Self::Playlist>;
    const PAGE_LIMIT: u32 = 20;

    fn new(_id: Self::ParentId) -> Self {
        FeaturedLoader
    }

    fn parent_id(&self) -> Self::ParentId {}

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
impl PlaylistsLoader for SavedLoader {
    type ParentId = ();
    type Playlist = SimplifiedPlaylist;
    type Page = Page<Self::Playlist>;
    const PAGE_LIMIT: u32 = 20;

    fn new(_id: Self::ParentId) -> Self {
        SavedLoader
    }

    fn parent_id(&self) -> Self::ParentId {}

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetMyPlaylists {
            tx,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}
