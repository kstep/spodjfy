use crate::components::track_list::{TrackList, TrackListMsg};
use crate::loaders::image::ImageLoader;
use crate::loaders::track::AlbumLoader;
use crate::servers::spotify::{SpotifyCmd, SpotifyProxy};
use glib::StaticType;
use gtk::prelude::*;
use gtk::IconViewExt;
use relm::vendor::fragile::Fragile;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::album::SimplifiedAlbum;
use rspotify::model::artist::FullArtist;
use rspotify::model::page::Page;
use std::sync::Arc;

#[derive(Msg)]
pub enum TopArtistsMsg {
    ShowTab,
    LoadPage(u32),
    NewPage(Page<FullArtist>),
    LoadThumb(String, gtk::TreeIter, bool),
    NewThumb(gdk_pixbuf::Pixbuf, gtk::TreeIter, bool),
    OpenArtist(Option<(String, String)>),
    OpenAlbum(Option<(String, String)>),
    LoadAlbumsPage(String, u32),
    NewAlbumsPage(String, Page<SimplifiedAlbum>),
}

const THUMB_SIZE: i32 = 256;
const PAGE_LIMIT: u32 = 10;

const COL_ARTIST_THUMB: u32 = 0;
const COL_ARTIST_NAME: u32 = 1;
const COL_ARTIST_URI: u32 = 2;

const COL_ALBUM_THUMB: u32 = 0;
const COL_ALBUM_NAME: u32 = 1;
const COL_ALBUM_URI: u32 = 2;

pub struct TopArtistsModel {
    stream: EventStream<TopArtistsMsg>,
    spotify: Arc<SpotifyProxy>,
    artists_store: gtk::ListStore,
    albums_store: gtk::ListStore,
    image_loader: ImageLoader,
}

#[widget]
impl Widget for TopArtistsTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> TopArtistsModel {
        let artists_store = gtk::ListStore::new(&[
            gdk_pixbuf::Pixbuf::static_type(),
            String::static_type(),
            String::static_type(),
        ]);
        let albums_store = gtk::ListStore::new(&[
            gdk_pixbuf::Pixbuf::static_type(),
            String::static_type(),
            String::static_type(),
        ]);
        let stream = relm.stream().clone();
        TopArtistsModel {
            stream,
            spotify,
            artists_store,
            albums_store,
            image_loader: ImageLoader::new_with_resize(THUMB_SIZE),
        }
    }

    fn update(&mut self, event: TopArtistsMsg) {
        use TopArtistsMsg::*;
        match event {
            ShowTab => {
                self.model.artists_store.clear();
                self.model.stream.emit(LoadPage(0))
            }
            LoadPage(offset) => {
                self.model
                    .spotify
                    .ask(
                        self.model.stream.clone(),
                        move |tx| SpotifyCmd::GetMyTopArtists {
                            tx,
                            offset,
                            limit: PAGE_LIMIT,
                        },
                        NewPage,
                    )
                    .unwrap();
            }
            NewPage(page) => {
                let stream = &self.model.stream;
                let store = &self.model.artists_store;
                let artists = page.items;
                for artist in artists {
                    let pos = store.insert_with_values(
                        None,
                        &[COL_ARTIST_NAME, COL_ARTIST_URI],
                        &[&artist.name, &artist.uri],
                    );

                    let image = crate::loaders::image::find_best_thumb(&artist.images, THUMB_SIZE);
                    if let Some(url) = image {
                        stream.emit(LoadThumb(url.to_owned(), pos, false));
                    }
                }

                if page.next.is_some() {
                    stream.emit(LoadPage(page.offset + PAGE_LIMIT));
                }
            }
            LoadThumb(url, pos, is_album) => {
                let stream = Fragile::new(self.model.stream.clone());
                let pos = Fragile::new(pos);
                self.model.image_loader.load_from_url(&url, move |loaded| {
                    if let Ok(Some(pb)) = loaded {
                        stream
                            .into_inner()
                            .emit(NewThumb(pb, pos.into_inner(), is_album));
                    }
                });
            }
            NewThumb(thumb, pos, is_album) => {
                if is_album {
                    self.model
                        .albums_store
                        .set_value(&pos, COL_ALBUM_THUMB, &thumb.to_value());
                } else {
                    self.model
                        .artists_store
                        .set_value(&pos, COL_ARTIST_THUMB, &thumb.to_value());
                }
            }
            OpenArtist(Some((uri, name))) => {
                /*
                let icon_view: &gtk::IconView = &self.artists_view;
                let store: &gtk::ListStore = &self.model.store;
                if let Some((Some(_uri), Some(_name))) = icon_view
                    .get_selected_items()
                    .first()
                    .and_then(|path| store.get_iter(path))
                    .map(|iter| {
                        (
                            store
                                .get_value(&iter, COL_ARTIST_URI as i32)
                                .get::<String>()
                                .ok()
                                .flatten(),
                            store
                                .get_value(&iter, COL_ARTIST_NAME as i32)
                                .get::<String>()
                                .ok()
                                .flatten(),
                        )
                    })

                 */

                let albums_tab = &self.albums_tab;
                self.stack.set_child_title(albums_tab, Some(&name));
                self.stack.set_visible_child(albums_tab);
                self.model.albums_store.clear();
                self.model.stream.emit(LoadAlbumsPage(uri, 0));
            }
            OpenArtist(None) => {}
            OpenAlbum(Some((uri, name))) => {
                self.tracks_view.emit(TrackListMsg::Reset(uri, true));

                let tracks_tab = self.tracks_view.widget();
                self.stack.set_child_title(tracks_tab, Some(&name));
                self.stack.set_visible_child(tracks_tab);
            }
            OpenAlbum(None) => {}

            LoadAlbumsPage(uri, offset) => {
                let artist_uri = uri.clone();
                self.model
                    .spotify
                    .ask(
                        self.model.stream.clone(),
                        move |tx| SpotifyCmd::GetArtistAlbums {
                            tx,
                            uri,
                            offset,
                            limit: PAGE_LIMIT,
                        },
                        move |albums| NewAlbumsPage(artist_uri.clone(), albums),
                    )
                    .unwrap();
            }
            NewAlbumsPage(artist_uri, page) => {
                let stream = &self.model.stream;
                let store = &self.model.albums_store;
                let albums = page.items;
                for album in albums {
                    let pos = store.insert_with_values(
                        None,
                        &[COL_ALBUM_NAME, COL_ALBUM_URI],
                        &[&album.name, &album.uri],
                    );

                    let image = crate::loaders::image::find_best_thumb(&album.images, THUMB_SIZE);
                    if let Some(url) = image {
                        stream.emit(LoadThumb(url.to_owned(), pos, true));
                    }
                }

                if page.next.is_some() {
                    stream.emit(LoadAlbumsPage(artist_uri, page.offset + PAGE_LIMIT));
                }
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

                #[name="artists_tab"]
                gtk::ScrolledWindow {
                    child: {
                        title: Some("Top artists"),
                    },

                    #[name="artists_view"]
                    /*
                    gtk::TreeView {
                        model: Some(&self.model.store)),
                    }
                     */
                    gtk::IconView {
                        item_width: THUMB_SIZE,
                        pixbuf_column: COL_ARTIST_THUMB as i32,
                        text_column: COL_ARTIST_NAME as i32,
                        model: Some(&self.model.artists_store),

                        item_activated(view, path) => TopArtistsMsg::OpenArtist(
                            view.get_model().and_then(|model| {
                                model.get_iter(path).and_then(|pos|
                                    model.get_value(&pos, COL_ARTIST_URI as i32).get::<String>().ok().flatten()
                                        .zip(model.get_value(&pos, COL_ARTIST_NAME as i32).get::<String>().ok().flatten()))
                            })),
                    }
                },

                #[name="albums_tab"]
                gtk::ScrolledWindow {
                    #[name="albums_view"]
                    /*
                    gtk::TreeView {
                        model: Some(&self.model.store)),
                    }
                     */
                    gtk::IconView {
                        item_width: THUMB_SIZE,
                        pixbuf_column: COL_ALBUM_THUMB as i32,
                        text_column: COL_ALBUM_NAME as i32,
                        model: Some(&self.model.albums_store),

                        item_activated(view, path) => TopArtistsMsg::OpenAlbum(
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
        /*
        let tree: &TreeView = &self.artists_view;

        let text_cell = gtk::CellRendererText::new();
        let image_cell = gtk::CellRendererPixbuf::new();

        tree.append_column(&{
            let column = TreeViewColumnBuilder::new()
                .expand(true)
                .build();
            column.pack_start(&image_cell, true);
            column.add_attribute(&image_cell, "pixbuf", 0);
            column
        });

        tree.append_column(&{
            let column = TreeViewColumnBuilder::new()
                .title("Title")
                .expand(true)
                .sort_column_id(1)
                .build();
            column.pack_start(&text_cell, true);
            column.add_attribute(&text_cell, "text", 1);
            column
        });

        tree.append_column(&{
            let column = TreeViewColumnBuilder::new()
                .title("Release date")
                .expand(true)
                .sort_column_id(2)
                .build();
            column.pack_start(&text_cell, true);
            column.add_attribute(&text_cell, "text", 2);
            column
        });
         */
    }
}
