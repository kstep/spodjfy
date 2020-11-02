use crate::components::spotify::SpotifyCmd;
use gdk_pixbuf::InterpType;
use glib::StaticType;
use gtk::prelude::*;
use gtk::IconViewExt;
use relm::{Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::album::SavedAlbum;
use rspotify::model::page::Page;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use tokio::sync::oneshot::{channel, error::TryRecvError, Receiver};

#[derive(Msg)]
pub enum AlbumsMsg {
    Load,
    TryRecv(Receiver<Option<Page<SavedAlbum>>>),
}

const THUMB_SIZE: i32 = 256;

pub struct AlbumsModel {
    relm: Relm<AlbumsTab>,
    spotify_tx: Arc<Sender<SpotifyCmd>>,
    store: gtk::ListStore,
}

#[widget]
impl Widget for AlbumsTab {
    fn model(relm: &Relm<Self>, spotify_tx: Arc<Sender<SpotifyCmd>>) -> AlbumsModel {
        let store = gtk::ListStore::new(&[
            gdk_pixbuf::Pixbuf::static_type(),
            String::static_type(),
            String::static_type(),
        ]);
        AlbumsModel {
            relm: relm.clone(),
            spotify_tx,
            store,
        }
    }

    fn update(&mut self, event: AlbumsMsg) {
        use AlbumsMsg::*;
        match event {
            Load => {
                let (tx, rx) = channel::<Option<Page<SavedAlbum>>>();
                self.model
                    .spotify_tx
                    .send(SpotifyCmd::GetAlbums {
                        tx,
                        limit: 50,
                        offset: 0,
                    })
                    .unwrap();
                self.model.relm.stream().emit(TryRecv(rx));
            }
            TryRecv(mut rx) => match rx.try_recv() {
                Err(TryRecvError::Empty) => self.model.relm.stream().emit(TryRecv(rx)),
                Err(TryRecvError::Closed) => (),
                Ok(Some(albums)) => {
                    let store = &self.model.store;
                    store.clear();
                    for album in &albums.items {
                        let image = album
                            .album
                            .images
                            .iter()
                            .max_by_key(|img| img.width.unwrap_or(0))
                            .and_then(|img| crate::utils::pixbuf_from_url(&img.url).ok())
                            .and_then(|pb| {
                                pb.scale_simple(THUMB_SIZE, THUMB_SIZE, InterpType::Nearest)
                            });

                        store.insert_with_values(
                            None,
                            &[0, 1, 2],
                            &[&image, &album.album.name, &album.album.release_date],
                        );
                    }
                }
                Ok(None) => {
                    self.model.store.clear();
                }
            },
        }
    }

    view! {
        gtk::ScrolledWindow {
            #[name="albums_view"]
            /*
            gtk::TreeView {
                model: Some(&__relm_model.store)),
            }
             */
            gtk::IconView {
                pixbuf_column: 0,
                text_column: 1,
                model: Some(&__relm_model.store),
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
