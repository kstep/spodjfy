use crate::components::spotify::SpotifyCmd;
use gdk_pixbuf::InterpType;
use glib::StaticType;
use gtk::prelude::*;
use gtk::IconViewExt;
use relm::{Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::page::Page;
use rspotify::model::playlist::SimplifiedPlaylist;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use tokio::sync::oneshot::{channel, error::TryRecvError, Receiver};

const THUMB_SIZE: i32 = 256;

#[derive(Msg)]
pub enum PlaylistsMsg {
    Load,
    TryRecv(Receiver<Option<Page<SimplifiedPlaylist>>>),
}

pub struct PlaylistsModel {
    relm: Relm<PlaylistsTab>,
    spotify_tx: Arc<Sender<SpotifyCmd>>,
    store: gtk::ListStore,
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
        }
    }

    fn update(&mut self, event: PlaylistsMsg) {
        use PlaylistsMsg::*;
        match event {
            Load => {
                let (tx, rx) = channel::<Option<Page<SimplifiedPlaylist>>>();
                self.model
                    .spotify_tx
                    .send(SpotifyCmd::GetPlaylists {
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
                Ok(Some(playlists)) => {
                    let store = &self.model.store;

                    store.clear();
                    for playlist in &playlists.items {
                        let image = playlist
                            .images
                            .iter()
                            .max_by_key(|img| img.width.unwrap_or(0))
                            .and_then(|img| crate::utils::pixbuf_from_url(&img.url).ok())
                            .and_then(|pb| {
                                pb.scale_simple(THUMB_SIZE, THUMB_SIZE, InterpType::Nearest)
                            });

                        store.insert_with_values(None, &[0, 1], &[&image, &playlist.name]);
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
