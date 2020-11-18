use crate::loaders::paged::PageLike;
use crate::servers::spotify::{ResultSender, SpotifyCmd};
use rspotify::model::{
    AlbumType, FullAlbum, Image, Page, SavedAlbum, SimplifiedAlbum, SimplifiedArtist,
};

pub trait AlbumsLoader: Clone + 'static {
    type ParentId: Clone;
    type Album: AlbumLike;
    type Page: PageLike<Self::Album>;
    const PAGE_LIMIT: u32;
    fn new(id: Self::ParentId) -> Self;
    fn parent_id(&self) -> &Self::ParentId;
    fn load_page(
        self,
        tx: ResultSender<Self::Page>,
        offset: <<Self as AlbumsLoader>::Page as PageLike<Self::Album>>::Offset,
    ) -> SpotifyCmd;
    fn uuid(&self) -> usize {
        self as *const _ as *const () as usize
    }
}

#[derive(Clone, Copy)]
pub struct SavedLoader;
impl AlbumsLoader for SavedLoader {
    type ParentId = ();
    type Album = SavedAlbum;
    type Page = Page<Self::Album>;
    const PAGE_LIMIT: u32 = 20;

    fn new(_id: Self::ParentId) -> Self {
        SavedLoader
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetMyAlbums {
            tx,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}

#[derive(Clone, Copy)]
pub struct NewReleasesLoader;
impl AlbumsLoader for NewReleasesLoader {
    type ParentId = ();
    type Album = SimplifiedAlbum;
    type Page = Page<Self::Album>;
    const PAGE_LIMIT: u32 = 20;

    fn new(_id: Self::ParentId) -> Self {
        NewReleasesLoader
    }

    fn parent_id(&self) -> &Self::ParentId {
        &()
    }

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetNewReleases {
            tx,
            offset,
            limit: Self::PAGE_LIMIT,
        }
    }
}

#[derive(Clone)]
pub struct ArtistLoader {
    uri: String,
}
impl AlbumsLoader for ArtistLoader {
    type ParentId = String;
    type Album = SimplifiedAlbum;
    type Page = Page<Self::Album>;
    const PAGE_LIMIT: u32 = 20;

    fn new(uri: Self::ParentId) -> Self {
        ArtistLoader { uri }
    }

    fn parent_id(&self) -> &Self::ParentId {
        &self.uri
    }

    fn load_page(self, tx: ResultSender<Self::Page>, offset: u32) -> SpotifyCmd {
        SpotifyCmd::GetArtistAlbums {
            tx,
            offset,
            uri: self.parent_id().clone(),
            limit: Self::PAGE_LIMIT,
        }
    }
}

pub trait AlbumLike {
    fn uri(&self) -> &str;
    fn name(&self) -> &str;
    fn release_date(&self) -> &str;
    fn total_tracks(&self) -> u32 {
        0
    }
    fn artists(&self) -> &[SimplifiedArtist];
    fn genres(&self) -> &[String] {
        &[]
    }
    fn kind(&self) -> AlbumType;
    fn duration(&self) -> u32 {
        0
    }
    fn duration_exact(&self) -> bool {
        false
    }
    fn images(&self) -> &[Image];

    fn unavailable_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[]
    }
}

impl AlbumLike for FullAlbum {
    fn uri(&self) -> &str {
        &self.uri
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn release_date(&self) -> &str {
        &self.release_date
    }

    fn total_tracks(&self) -> u32 {
        self.tracks.total
    }

    fn artists(&self) -> &[SimplifiedArtist] {
        &self.artists
    }

    fn genres(&self) -> &[String] {
        &self.genres
    }

    fn kind(&self) -> AlbumType {
        self.album_type
    }

    fn duration(&self) -> u32 {
        self.tracks
            .items
            .iter()
            .map(|track| track.duration_ms)
            .sum()
    }

    fn duration_exact(&self) -> bool {
        self.tracks.total as usize == self.tracks.items.len()
    }

    fn images(&self) -> &[Image] {
        &self.images
    }
}

impl AlbumLike for SimplifiedAlbum {
    fn uri(&self) -> &str {
        self.uri.as_deref().unwrap_or("")
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn release_date(&self) -> &str {
        self.release_date.as_deref().unwrap_or("")
    }

    fn artists(&self) -> &[SimplifiedArtist] {
        &self.artists
    }

    fn kind(&self) -> AlbumType {
        self.album_type
            .as_ref()
            .map_or(AlbumType::Album, |tpe| match &**tpe {
                "single" => AlbumType::Single,
                "appears_on" => AlbumType::AppearsOn,
                "compilation" => AlbumType::Compilation,
                _ => AlbumType::Album,
            })
    }

    fn images(&self) -> &[Image] {
        &self.images
    }

    fn unavailable_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_ALBUM_DURATION, COL_ALBUM_TOTAL_TRACKS, COL_ALBUM_GENRES]
    }
}

impl AlbumLike for SavedAlbum {
    fn uri(&self) -> &str {
        self.album.uri()
    }

    fn name(&self) -> &str {
        self.album.name()
    }

    fn release_date(&self) -> &str {
        self.album.release_date()
    }

    fn total_tracks(&self) -> u32 {
        self.album.total_tracks()
    }

    fn artists(&self) -> &[SimplifiedArtist] {
        self.album.artists()
    }

    fn genres(&self) -> &[String] {
        self.album.genres()
    }

    fn kind(&self) -> AlbumType {
        self.album.kind()
    }

    fn duration(&self) -> u32 {
        self.album.duration()
    }

    fn duration_exact(&self) -> bool {
        self.album.duration_exact()
    }

    fn images(&self) -> &[Image] {
        self.album.images()
    }
}

pub const COL_ALBUM_THUMB: u32 = 0;
pub const COL_ALBUM_URI: u32 = 1;
pub const COL_ALBUM_NAME: u32 = 2;
pub const COL_ALBUM_RELEASE_DATE: u32 = 3;
pub const COL_ALBUM_TOTAL_TRACKS: u32 = 4;
pub const COL_ALBUM_ARTISTS: u32 = 5;
pub const COL_ALBUM_GENRES: u32 = 6;
pub const COL_ALBUM_TYPE: u32 = 7;
pub const COL_ALBUM_DURATION: u32 = 8;
