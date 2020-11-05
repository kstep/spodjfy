use crate::components::spotify::{SpotifyCmd, SpotifyProxy};
use crate::utils::ImageLoader;
use glib::StaticType;
use gtk::prelude::*;
use gtk::IconViewExt;
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
}

const THUMB_SIZE: i32 = 256;
const PAGE_LIMIT: u32 = 10;

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
                self.model.spotify.ask(
                    self.model.stream.clone(),
                    move |tx| SpotifyCmd::GetAlbums {
                        tx,
                        limit: PAGE_LIMIT,
                        offset,
                    },
                    NewPage,
                );
            }
            NewPage(page) => {
                let stream = &self.model.stream;
                let store = &self.model.store;
                let albums = page.items;
                for album in albums {
                    let pos = store.insert_with_values(
                        None,
                        &[1, 2],
                        &[&album.album.name, &album.album.release_date],
                    );

                    let image =
                        crate::utils::find_best_thumb(album.album.images.into_iter(), THUMB_SIZE);
                    if let Some(url) = image {
                        stream.emit(LoadThumb(url, pos));
                    }
                }

                if page.next.is_some() {
                    stream.emit(LoadPage(page.offset + PAGE_LIMIT));
                }
            }
            LoadThumb(url, pos) => {
                let stream = Fragile::new(self.model.stream.clone());
                let pos = Fragile::new(pos);
                self.model.image_loader.load_from_url(url, move |loaded| {
                    if let Ok(pb) = loaded {
                        stream.into_inner().emit(NewThumb(pb, pos.into_inner()));
                    }
                });
            }
            NewThumb(thumb, pos) => {
                self.model.store.set_value(&pos, 0, &thumb.to_value());
            }
        }
    }

    view! {
        gtk::ScrolledWindow {
            #[name="albums_view"]
            /*
            gtk::TreeView {
                model: Some(&self.model.store)),
            }
             */
            gtk::IconView {
                pixbuf_column: 0,
                text_column: 1,
                model: Some(&self.model.store),
            }
        }
    }

    fn init_view(&mut self) {
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
