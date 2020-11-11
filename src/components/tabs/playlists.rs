use crate::components::track_list::{TrackList, TrackListMsg};
use crate::image_loader::ImageLoader;
use crate::spotify::{SpotifyCmd, SpotifyProxy};
use glib::StaticType;
use gtk::prelude::*;
use gtk::{IconViewExt, TreeModelExt};
use relm::vendor::fragile::Fragile;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::page::Page;
use rspotify::model::playlist::{PlaylistTrack, SimplifiedPlaylist};
use std::sync::Arc;

const THUMB_SIZE: i32 = 256;
const PAGE_LIMIT: u32 = 10;

const COL_PLAYLIST_THUMB: u32 = 0;
const COL_PLAYLIST_NAME: u32 = 1;
const COL_PLAYLIST_URI: u32 = 2;

#[derive(Msg)]
pub enum PlaylistsMsg {
    ShowTab,
    LoadPage(u32),
    NewPage(Page<SimplifiedPlaylist>),
    LoadThumb(String, gtk::TreeIter),
    NewThumb(gdk_pixbuf::Pixbuf, gtk::TreeIter),
    OpenChosenPlaylist,
    OpenPlaylist(Option<(String, String)>),
    GoToTrack(String),
}

pub struct PlaylistsModel {
    stream: EventStream<PlaylistsMsg>,
    spotify: Arc<SpotifyProxy>,
    store: gtk::ListStore,
    image_loader: ImageLoader,
}

#[widget]
impl Widget for PlaylistsTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> PlaylistsModel {
        let store = gtk::ListStore::new(&[
            gdk_pixbuf::Pixbuf::static_type(),
            String::static_type(),
            String::static_type(),
        ]);
        let stream = relm.stream().clone();
        PlaylistsModel {
            stream,
            spotify,
            store,
            image_loader: ImageLoader::new_with_resize(THUMB_SIZE),
        }
    }

    fn update(&mut self, event: PlaylistsMsg) {
        use PlaylistsMsg::*;
        match event {
            ShowTab => {
                self.model.store.clear();
                self.model.stream.emit(LoadPage(0))
            }
            LoadPage(offset) => {
                self.model.spotify.ask(
                    self.model.stream.clone(),
                    move |tx| SpotifyCmd::GetMyPlaylists {
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
                let playlists = page.items;
                for playlist in playlists {
                    let pos = store.insert_with_values(
                        None,
                        &[COL_PLAYLIST_NAME, COL_PLAYLIST_URI],
                        &[&playlist.name, &playlist.uri],
                    );

                    let image = crate::image_loader::find_best_thumb(&playlist.images, THUMB_SIZE);
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
                self.model
                    .store
                    .set_value(&pos, COL_PLAYLIST_THUMB, &thumb.to_value());
            }
            OpenChosenPlaylist => {
                let icon_view: &gtk::IconView = &self.playlists_view;
                let store: &gtk::ListStore = &self.model.store;
                self.model.stream.emit(OpenPlaylist(
                    icon_view
                        .get_selected_items()
                        .first()
                        .and_then(|path| store.get_iter(path))
                        .and_then(|iter| {
                            store
                                .get_value(&iter, COL_PLAYLIST_URI as i32)
                                .get::<String>()
                                .ok()
                                .flatten()
                                .zip(
                                    store
                                        .get_value(&iter, COL_PLAYLIST_NAME as i32)
                                        .get::<String>()
                                        .ok()
                                        .flatten(),
                                )
                        }),
                ));
            }
            OpenPlaylist(Some((uri, name))) => {
                self.playlist_view.emit(TrackListMsg::Reset(uri, true));

                let playlist_widget = self.playlist_view.widget();
                self.stack.set_child_title(playlist_widget, Some(&name));
                self.stack.set_visible_child(playlist_widget);
            }
            OpenPlaylist(None) => {}
            GoToTrack(uri) => {
                self.playlist_view.emit(TrackListMsg::GoToTrack(uri));
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
                        title: Some("Playlists"),
                    },

                    #[name="playlists_view"]
                    /*
                    gtk::TreeView {
                        model: Some(&self.model.store)),
                    }
                     */
                    gtk::IconView {
                        item_width: THUMB_SIZE,
                        pixbuf_column: COL_PLAYLIST_THUMB as i32,
                        text_column: COL_PLAYLIST_NAME as i32,
                        model: Some(&self.model.store),

                        item_activated(view, path) => PlaylistsMsg::OpenPlaylist(
                            view.get_model().and_then(|model| {
                                model.get_iter(path).and_then(|pos|
                                    model.get_value(&pos, COL_PLAYLIST_URI as i32).get::<String>().ok().flatten()
                                        .zip(model.get_value(&pos, COL_PLAYLIST_NAME as i32).get::<String>().ok().flatten()))
                            })),
                    }
                },
                #[name="playlist_view"]
                TrackList::<PlaylistTrack>(self.model.spotify.clone()),
            },
        }
    }

    fn init_view(&mut self) {
        self.breadcrumb.set_stack(Some(&self.stack));
    }
}
