use crate::components::track_list::{TrackList, TrackListMsg};
use crate::loaders::image::ImageLoader;
use crate::loaders::track::AlbumLoader;
use crate::servers::spotify::{SpotifyCmd, SpotifyProxy};
use glib::{Cast, StaticType};
use gtk::prelude::*;
use gtk::{CellRendererExt, CellRendererTextExt, TreeModelExt, TreeViewExt};
use itertools::Itertools;
use relm::vendor::fragile::Fragile;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::album::SavedAlbum;
use rspotify::model::page::Page;
use std::sync::Arc;

#[derive(Msg)]
pub enum AlbumsMsg {
    ShowTab,
    LoadPage(u32),
    NewPage(Page<SavedAlbum>),
    LoadThumb(String, gtk::TreeIter),
    NewThumb(gdk_pixbuf::Pixbuf, gtk::TreeIter),
    OpenChosenAlbum,
    OpenAlbum(Option<(String, String)>),
    GoToTrack(String),
}

const THUMB_SIZE: i32 = 48;
const PAGE_LIMIT: u32 = 10;

const COL_ALBUM_THUMB: u32 = 0;
const COL_ALBUM_URI: u32 = 1;
const COL_ALBUM_NAME: u32 = 2;
const COL_ALBUM_RELEASE_DATE: u32 = 3;
const COL_ALBUM_TOTAL_TRACKS: u32 = 4;
const COL_ALBUM_ARTISTS: u32 = 5;
const COL_ALBUM_GENRES: u32 = 6;
const COL_ALBUM_TYPE: u32 = 7;
const COL_ALBUM_DURATION: u32 = 8;

pub struct AlbumsModel {
    stream: EventStream<AlbumsMsg>,
    spotify: Arc<SpotifyProxy>,
    store: gtk::ListStore,
    image_loader: ImageLoader,
}

#[widget]
impl Widget for AlbumsTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> AlbumsModel {
        let store = gtk::ListStore::new(&[
            gdk_pixbuf::Pixbuf::static_type(), // thumb
            String::static_type(),             // uri
            String::static_type(),             // name
            String::static_type(),             // release date
            u32::static_type(),                // total tracks
            String::static_type(),             // artists
            String::static_type(),             // genres
            u8::static_type(),                 // type
            u32::static_type(),                // duration
        ]);
        let stream = relm.stream().clone();
        AlbumsModel {
            stream,
            spotify,
            store,
            image_loader: ImageLoader::new_with_resize(THUMB_SIZE),
        }
    }

    fn update(&mut self, event: AlbumsMsg) {
        use AlbumsMsg::*;
        match event {
            ShowTab => {
                self.model.store.clear();
                self.model.stream.emit(LoadPage(0))
            }
            LoadPage(offset) => {
                self.model
                    .spotify
                    .ask(
                        self.model.stream.clone(),
                        move |tx| SpotifyCmd::GetMyAlbums {
                            tx,
                            limit: PAGE_LIMIT,
                            offset,
                        },
                        NewPage,
                    )
                    .unwrap();
            }
            NewPage(page) => {
                let stream = &self.model.stream;
                let store = &self.model.store;
                let albums = page.items;
                for album in albums {
                    let pos = store.insert_with_values(
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
                        ],
                        &[
                            &album.album.uri,
                            &album.album.name,
                            &album.album.release_date,
                            &album.album.tracks.total,
                            &album
                                .album
                                .artists
                                .iter()
                                .map(|artist| &artist.name)
                                .join(", "),
                            &album.album.genres.iter().join(", "),
                            &(album.album.album_type as u8),
                            &album
                                .album
                                .tracks
                                .items
                                .iter()
                                .map(|track| track.duration_ms)
                                .sum::<u32>(),
                        ],
                    );

                    let image =
                        crate::loaders::image::find_best_thumb(&album.album.images, THUMB_SIZE);
                    if let Some(url) = image {
                        stream.emit(LoadThumb(url.to_owned(), pos));
                    }
                }

                if page.next.is_some() {
                    stream.emit(LoadPage(page.offset + PAGE_LIMIT));
                }
            }
            LoadThumb(url, pos) => {
                let stream = Fragile::new(self.model.stream.clone());
                let pos = Fragile::new(pos);
                self.model.image_loader.load_from_url(&url, move |loaded| {
                    if let Ok(Some(pb)) = loaded {
                        stream.into_inner().emit(NewThumb(pb, pos.into_inner()));
                    }
                });
            }
            NewThumb(thumb, pos) => {
                self.model.store.set_value(&pos, 0, &thumb.to_value());
            }
            OpenChosenAlbum => {
                let select = self.albums_view.get_selection();
                let (rows, model) = select.get_selected_rows();

                self.model.stream.emit(OpenAlbum(
                    rows.first()
                        .and_then(|path| model.get_iter(path))
                        .and_then(|iter| {
                            model
                                .get_value(&iter, COL_ALBUM_URI as i32)
                                .get::<String>()
                                .ok()
                                .flatten()
                                .zip(
                                    model
                                        .get_value(&iter, COL_ALBUM_NAME as i32)
                                        .get::<String>()
                                        .ok()
                                        .flatten(),
                                )
                        }),
                ));
            }
            OpenAlbum(Some((uri, name))) => {
                self.tracks_view.emit(TrackListMsg::Reset(uri, true));

                let album_widget = self.tracks_view.widget();
                self.stack.set_child_title(album_widget, Some(&name));
                self.stack.set_visible_child(album_widget);
            }
            OpenAlbum(None) => {}
            GoToTrack(uri) => {
                self.tracks_view.emit(TrackListMsg::GoToTrack(uri));
            }
        }
    }

    view! {
        gtk::Box(gtk::Orientation::Vertical, 1) {
            #[name="breadcrumb"]
            gtk::StackSwitcher {},

            #[name="stack"]
            gtk::Stack {
                vexpand: true,
                gtk::ScrolledWindow {
                    child: {
                        title: Some("Albums"),
                    },

                    #[name="albums_view"]
                    gtk::TreeView {
                        model: Some(&self.model.store),

                        row_activated(view, path, _) => AlbumsMsg::OpenAlbum(
                            view.get_model().and_then(|model| {
                                model.get_iter(path).and_then(|pos|
                                    model.get_value(&pos, COL_ALBUM_URI as i32).get::<String>().ok().flatten()
                                        .zip(model.get_value(&pos, COL_ALBUM_NAME as i32).get::<String>().ok().flatten()))
                            })),
                    }
                },
                #[name="tracks_view"]
                TrackList::<AlbumLoader>(self.model.spotify.clone()),
            }
        }
    }

    fn init_view(&mut self) {
        self.breadcrumb.set_stack(Some(&self.stack));

        let tree: &gtk::TreeView = &self.albums_view;
        let base_column = gtk::TreeViewColumnBuilder::new()
            .expand(true)
            .resizable(true)
            .reorderable(true);

        tree.append_column(&{
            let cell = gtk::CellRendererPixbuf::new();
            let col = gtk::TreeViewColumn::new();
            col.pack_start(&cell, true);
            col.add_attribute(&cell, "pixbuf", COL_ALBUM_THUMB as i32);
            col
        });

        tree.append_column(&{
            let cell = gtk::CellRendererText::new();
            let col = base_column
                .clone()
                .sort_column_id(COL_ALBUM_TYPE as i32)
                .expand(false)
                .build();
            col.pack_start(&cell, true);
            col.add_attribute(&cell, "text", COL_ALBUM_TYPE as i32);
            gtk::TreeViewColumnExt::set_cell_data_func(
                &col,
                &cell,
                Some(Box::new(move |_col, cell, model, pos| {
                    if let (Some(cell), Ok(Some(value))) = (
                        cell.downcast_ref::<gtk::CellRendererText>(),
                        model.get_value(pos, COL_ALBUM_TYPE as i32).get::<u8>(),
                    ) {
                        cell.set_property_text(Some(match value {
                            0 => "\u{1F4BF}", // album
                            1 => "\u{1F3B5}", // single
                            2 => "\u{1F468}", // appears on
                            3 => "\u{1F4DA}", // compilation
                            _ => "?",
                        }));
                    }
                })),
            );
            col
        });

        tree.append_column(&{
            let cell = gtk::CellRendererText::new();
            let col = base_column
                .clone()
                .title("Title")
                .sort_column_id(COL_ALBUM_NAME as i32)
                .build();
            col.pack_start(&cell, true);
            col.add_attribute(&cell, "text", COL_ALBUM_NAME as i32);
            col
        });

        tree.append_column(&{
            let cell = gtk::CellRendererText::new();
            cell.set_alignment(1.0, 0.5);
            let col = base_column
                .clone()
                .title("Tracks")
                .expand(false)
                .sort_column_id(COL_ALBUM_TOTAL_TRACKS as i32)
                .build();
            col.pack_start(&cell, true);
            col.add_attribute(&cell, "text", COL_ALBUM_TOTAL_TRACKS as i32);
            col
        });

        tree.append_column(&{
            let cell = gtk::CellRendererText::new();
            cell.set_alignment(1.0, 0.5);
            let col = base_column
                .clone()
                .title("Duration")
                .expand(false)
                .sort_column_id(COL_ALBUM_DURATION as i32)
                .build();
            col.pack_start(&cell, true);
            col.add_attribute(&cell, "text", COL_ALBUM_DURATION as i32);
            gtk::TreeViewColumnExt::set_cell_data_func(
                &col,
                &cell,
                Some(Box::new(move |_col, cell, model, pos| {
                    if let (Some(cell), Ok(Some(value))) = (
                        cell.downcast_ref::<gtk::CellRendererText>(),
                        model.get_value(pos, COL_ALBUM_DURATION as i32).get::<u32>(),
                    ) {
                        cell.set_property_text(Some(&crate::utils::humanize_time(value)));
                    }
                })),
            );
            col
        });

        tree.append_column(&{
            let cell = gtk::CellRendererText::new();
            cell.set_alignment(1.0, 0.5);
            let col = base_column
                .clone()
                .title("Released")
                .expand(false)
                .sort_column_id(COL_ALBUM_RELEASE_DATE as i32)
                .build();
            col.pack_start(&cell, true);
            col.add_attribute(&cell, "text", COL_ALBUM_RELEASE_DATE as i32);
            col
        });

        tree.append_column(&{
            let cell = gtk::CellRendererText::new();
            let col = base_column
                .clone()
                .title("Genres")
                .sort_column_id(COL_ALBUM_GENRES as i32)
                .build();
            col.pack_start(&cell, true);
            col.add_attribute(&cell, "text", COL_ALBUM_GENRES as i32);
            col
        });

        tree.append_column(&{
            let cell = gtk::CellRendererText::new();
            let col = base_column
                .clone()
                .title("Artists")
                .sort_column_id(COL_ALBUM_ARTISTS as i32)
                .build();
            col.pack_start(&cell, true);
            col.add_attribute(&cell, "text", COL_ALBUM_ARTISTS as i32);
            col
        });
    }
}
