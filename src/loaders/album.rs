use crate::loaders::common::{
    ContainerLoader, HasImages, MissingColumns, COL_ITEM_NAME, COL_ITEM_THUMB, COL_ITEM_URI,
};
use crate::loaders::paged::RowLike;
use crate::loaders::HasDuration;
use crate::servers::spotify::{ResultSender, SpotifyCmd};
use glib::{IsA, StaticType, Type};
use gtk::prelude::GtkListStoreExtManual;
use itertools::Itertools;
use rspotify::model::{
    AlbumType, FullAlbum, Image, Page, SavedAlbum, SimplifiedAlbum, SimplifiedArtist,
};

const NAME: &str = "albums";

#[derive(Clone, Copy)]
pub struct SavedLoader(usize);

impl ContainerLoader for SavedLoader {
    type ParentId = ();
    type Item = SavedAlbum;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;
    const NAME: &'static str = NAME;

    fn new(_id: Self::ParentId) -> Self {
        SavedLoader(rand::random())
    }

    fn epoch(&self) -> usize {
        self.0
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
pub struct NewReleasesLoader(usize);

impl ContainerLoader for NewReleasesLoader {
    type ParentId = ();
    type Item = SimplifiedAlbum;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;
    const NAME: &'static str = NAME;

    fn new(_id: Self::ParentId) -> Self {
        NewReleasesLoader(rand::random())
    }

    fn epoch(&self) -> usize {
        self.0
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
impl ContainerLoader for ArtistLoader {
    type ParentId = String;
    type Item = SimplifiedAlbum;
    type Page = Page<Self::Item>;
    const PAGE_LIMIT: u32 = 20;
    const NAME: &'static str = NAME;

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

pub trait AlbumLike: HasDuration + HasImages {
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
            gdk_pixbuf::Pixbuf::static_type(), // thumb
            String::static_type(),             // uri
            String::static_type(),             // name
            String::static_type(),             // release date
            u32::static_type(),                // total tracks
            String::static_type(),             // artists
            String::static_type(),             // genres
            u8::static_type(),                 // type
            u32::static_type(),                // duration
            u32::static_type(),                // rate/popularity
        ]
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

    fn rate(&self) -> u32 {
        self.popularity
    }
}

impl HasDuration for FullAlbum {
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
}

impl MissingColumns for FullAlbum {}

impl HasImages for FullAlbum {
    fn images(&self) -> &[Image] {
        &self.images
    }
}

impl RowLike for FullAlbum {
    fn content_types() -> Vec<Type> {
        Self::store_content_types()
    }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter {
        self.insert_into_store(store)
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

    fn rate(&self) -> u32 {
        0
    }
}

impl HasDuration for SimplifiedAlbum {
    fn duration_exact(&self) -> bool {
        false
    }
}

impl MissingColumns for SimplifiedAlbum {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[
            COL_ALBUM_DURATION,
            COL_ALBUM_TOTAL_TRACKS,
            COL_ALBUM_GENRES,
            COL_ALBUM_RATE,
        ]
    }
}

impl HasImages for SimplifiedAlbum {
    fn images(&self) -> &[Image] {
        &self.images
    }
}

impl RowLike for SimplifiedAlbum {
    fn content_types() -> Vec<Type> {
        Self::store_content_types()
    }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter {
        self.insert_into_store(store)
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

    fn rate(&self) -> u32 {
        self.album.popularity
    }
}

impl HasDuration for SavedAlbum {
    fn duration(&self) -> u32 {
        self.album.duration()
    }

    fn duration_exact(&self) -> bool {
        self.album.duration_exact()
    }
}

impl MissingColumns for SavedAlbum {}

impl HasImages for SavedAlbum {
    fn images(&self) -> &[Image] {
        &self.album.images
    }
}

impl RowLike for SavedAlbum {
    fn content_types() -> Vec<Type> {
        Self::store_content_types()
    }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter {
        self.insert_into_store(store)
    }
}

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
