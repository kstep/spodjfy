use crate::components::spotify::SpotifyCmd;
use gdk_pixbuf::InterpType;
use glib::StaticType;
use gtk::prelude::*;
use gtk::IconViewExt;
use itertools::Itertools;
use relm::{Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::page::Page;
use rspotify::model::playlist::SimplifiedPlaylist;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use tokio::sync::oneshot::{channel, error::TryRecvError, Receiver};

const THUMB_SIZE: i32 = 256;
const PAGE_LIMIT: u32 = 10;

#[derive(Msg)]
pub enum PlaylistsMsg {
    ShowTab,
    LoadPage(u32),
    TryRecvPage(Receiver<Option<Page<SimplifiedPlaylist>>>),
    NewPage(Vec<SimplifiedPlaylist>),
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
            ShowTab => {
                self.model.store.clear();
                self.model.relm.stream().emit(LoadPage(0))
            }
            LoadPage(offset) => {
                let (tx, rx) = channel::<Option<Page<SimplifiedPlaylist>>>();
                self.model
                    .spotify_tx
                    .send(SpotifyCmd::GetPlaylists {
                        tx,
                        limit: PAGE_LIMIT,
                        offset,
                    })
                    .unwrap();
                self.model.relm.stream().emit(TryRecvPage(rx));
            }
            TryRecvPage(mut rx) => match rx.try_recv() {
                Err(TryRecvError::Empty) => self.model.relm.stream().emit(TryRecvPage(rx)),
                Err(TryRecvError::Closed) => (),
                Ok(Some(page)) => {
                    let stream = self.model.relm.stream();
                    for chunk in &page.items.into_iter().chunks(5) {
                        stream.emit(NewPage(chunk.collect::<Vec<_>>()))
                    }
                    if page.next.is_some() {
                        stream.emit(LoadPage(page.offset + PAGE_LIMIT));
                    }
                }
                Ok(None) => {
                    self.model.store.clear();
                }
            },
            NewPage(playlists) => {
                let store = &self.model.store;
                for playlist in &playlists {
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
