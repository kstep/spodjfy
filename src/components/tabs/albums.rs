use crate::components::track_list::{TrackList, TrackListMsg};
use crate::image_loader::ImageLoader;
use crate::servers::spotify::{SpotifyCmd, SpotifyProxy};
use glib::StaticType;
use gtk::prelude::*;
use gtk::IconViewExt;
use relm::vendor::fragile::Fragile;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::album::SavedAlbum;
use rspotify::model::page::Page;
use rspotify::model::track::SimplifiedTrack;
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

const THUMB_SIZE: i32 = 256;
const PAGE_LIMIT: u32 = 10;

const COL_ALBUM_THUMB: u32 = 0;
const COL_ALBUM_NAME: u32 = 1;
const COL_ALBUM_RELEASE_DATE: u32 = 2;
const COL_ALBUM_URI: u32 = 3;

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
            gdk_pixbuf::Pixbuf::static_type(),
            String::static_type(),
            String::static_type(),
            String::static_type(),
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
                        &[COL_ALBUM_NAME, COL_ALBUM_RELEASE_DATE, COL_ALBUM_URI],
                        &[
                            &album.album.name,
                            &album.album.release_date,
                            &album.album.uri,
                        ],
                    );

                    let image =
                        crate::image_loader::find_best_thumb(&album.album.images, THUMB_SIZE);
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
                let icon_view: &gtk::IconView = &self.albums_view;
                let store: &gtk::ListStore = &self.model.store;
                self.model.stream.emit(OpenAlbum(
                    icon_view
                        .get_selected_items()
                        .first()
                        .and_then(|path| store.get_iter(path))
                        .and_then(|iter| {
                            store
                                .get_value(&iter, COL_ALBUM_URI as i32)
                                .get::<String>()
                                .ok()
                                .flatten()
                                .zip(
                                    store
                                        .get_value(&iter, COL_ALBUM_NAME as i32)
                                        .get::<String>()
                                        .ok()
                                        .flatten(),
                                )
                        }),
                ));
            }
            OpenAlbum(Some((uri, name))) => {
                self.album_view.emit(TrackListMsg::Reset(uri, true));

                let album_widget = self.album_view.widget();
                self.stack.set_child_title(album_widget, Some(&name));
                self.stack.set_visible_child(album_widget);
            }
            OpenAlbum(None) => {}
            GoToTrack(uri) => {
                self.album_view.emit(TrackListMsg::GoToTrack(uri));
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
                    /*
                    gtk::TreeView {
                        model: Some(&self.model.store)),
                    }
                     */
                    gtk::IconView {
                        pixbuf_column: COL_ALBUM_THUMB as i32,
                        text_column: COL_ALBUM_NAME as i32,
                        model: Some(&self.model.store),
                        item_width: THUMB_SIZE,

                        item_activated(view, path) => AlbumsMsg::OpenAlbum(
                            view.get_model().and_then(|model| {
                                model.get_iter(path).and_then(|pos|
                                    model.get_value(&pos, COL_ALBUM_URI as i32).get::<String>().ok().flatten()
                                        .zip(model.get_value(&pos, COL_ALBUM_NAME as i32).get::<String>().ok().flatten()))
                            })),
                    }
                },
                #[name="album_view"]
                TrackList::<SimplifiedTrack>(self.model.spotify.clone()),
            }
        }
    }

    fn init_view(&mut self) {
        self.breadcrumb.set_stack(Some(&self.stack));
        /*
        let tree: &TreeView = &self.albums_view;

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
