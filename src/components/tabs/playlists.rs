use crate::components::spotify::SpotifyCmd;
use crate::utils::ImageLoader;
use glib::StaticType;
use gtk::prelude::*;
use gtk::IconViewExt;
use relm::vendor::fragile::Fragile;
use relm::{Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::page::Page;
use rspotify::model::playlist::SimplifiedPlaylist;
use std::sync::mpsc::Sender;
use std::sync::Arc;

const THUMB_SIZE: i32 = 256;
const PAGE_LIMIT: u32 = 10;

#[derive(Msg)]
pub enum PlaylistsMsg {
    ShowTab,
    LoadPage(u32),
    NewPage(Page<SimplifiedPlaylist>),
    LoadThumb(String, gtk::TreeIter),
    NewThumb(gdk_pixbuf::Pixbuf, gtk::TreeIter),
}

pub struct PlaylistsModel {
    relm: Relm<PlaylistsTab>,
    spotify_tx: Arc<Sender<SpotifyCmd>>,
    store: gtk::ListStore,
    image_loader: ImageLoader,
}

#[widget]
impl Widget for PlaylistsTab {
    fn model(relm: &Relm<Self>, spotify_tx: Arc<Sender<SpotifyCmd>>) -> PlaylistsModel {
        let store =
            gtk::ListStore::new(&[gdk_pixbuf::Pixbuf::static_type(), String::static_type()]);
        PlaylistsModel {
            relm: relm.clone(),
            spotify_tx,
            store,
            image_loader: ImageLoader::new_with_resize(THUMB_SIZE),
        }
    }

    fn update(&mut self, event: PlaylistsMsg) {
        use PlaylistsMsg::*;
        match event {
            ShowTab => {
                self.model.store.clear();
                self.model.relm.stream().emit(LoadPage(0))
            }
            LoadPage(offset) => {
                let stream = self.model.relm.stream().clone();
                let (_, tx) =
                    relm::Channel::<Option<Page<SimplifiedPlaylist>>>::new(move |reply| {
                        if let Some(page) = reply {
                            stream.emit(NewPage(page));
                        }
                    });
                self.model
                    .spotify_tx
                    .send(SpotifyCmd::GetPlaylists {
                        tx,
                        limit: PAGE_LIMIT,
                        offset,
                    })
                    .unwrap();
            }
            NewPage(page) => {
                let stream = self.model.relm.stream();
                let store = &self.model.store;
                let playlists = page.items;
                for playlist in playlists {
                    let pos = store.insert_with_values(None, &[1], &[&playlist.name]);

                    let image = crate::utils::find_best_thumb(playlist.images, THUMB_SIZE);
                    if let Some(url) = image {
                        stream.emit(LoadThumb(url, pos));
                    }
                }
                if page.next.is_some() {
                    stream.emit(LoadPage(page.offset + PAGE_LIMIT));
                }
            }
            LoadThumb(url, pos) => {
                let stream = Fragile::new(self.model.relm.stream().clone());
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
            #[name="playlists_view"]
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
}
