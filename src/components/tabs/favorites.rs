use crate::components::spotify::SpotifyCmd;
use crate::utils::ImageLoader;
use gdk_pixbuf::Pixbuf;
use glib::StaticType;
use gtk::prelude::*;
use gtk::{
    CellRendererPixbuf, GtkMenuExt, GtkMenuItemExt, Inhibit, TreeView, TreeViewColumn,
    TreeViewColumnBuilder,
};
use itertools::Itertools;
use relm::vendor::fragile::Fragile;
use relm::{Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::page::Page;
use rspotify::model::track::SavedTrack;
use std::sync::mpsc::Sender;
use std::sync::Arc;

#[derive(Msg)]
pub enum FavoritesMsg {
    ShowTab,
    LoadPage(u32),
    NewPage(Page<SavedTrack>),
    Click(gdk::EventButton),
    LoadThumb(String, gtk::TreeIter),
    NewThumb(gdk_pixbuf::Pixbuf, gtk::TreeIter),
}

const PAGE_LIMIT: u32 = 10;
const THUMB_SIZE: i32 = 32;

const COL_TRACK_ID: u32 = 0;
const COL_TRACK_THUMB: u32 = 1;
const COL_TRACK_NAME: u32 = 2;
const COL_TRACK_ARTISTS: u32 = 3;
const COL_TRACK_NUMBER: u32 = 4;
const COL_TRACK_ALBUM: u32 = 5;
const COL_TRACK_CAN_PLAY: u32 = 6;
const COL_TRACK_DURATION: u32 = 7;
const COL_TRACK_DURATION_MS: u32 = 8;

pub struct FavoritesModel {
    relm: Relm<FavoritesTab>,
    spotify_tx: Arc<Sender<SpotifyCmd>>,
    store: gtk::ListStore,
    image_loader: ImageLoader,
}

#[widget]
impl Widget for FavoritesTab {
    fn model(relm: &Relm<Self>, spotify_tx: Arc<Sender<SpotifyCmd>>) -> FavoritesModel {
        let store = gtk::ListStore::new(&[
            String::static_type(),
            Pixbuf::static_type(),
            String::static_type(),
            String::static_type(),
            u32::static_type(),
            String::static_type(),
            bool::static_type(),
            String::static_type(),
            u32::static_type(),
        ]);
        FavoritesModel {
            relm: relm.clone(),
            spotify_tx,
            store,
            image_loader: ImageLoader::new_with_resize(THUMB_SIZE),
        }
    }

    fn update(&mut self, event: FavoritesMsg) {
        use FavoritesMsg::*;
        match event {
            ShowTab => {
                self.model.store.clear();
                self.model.relm.stream().emit(LoadPage(0))
            }
            LoadPage(offset) => {
                let stream: relm::EventStream<_> = self.model.relm.stream().clone();
                let (_, tx) = relm::Channel::<Option<Page<SavedTrack>>>::new(move |reply| {
                    if let Some(page) = reply {
                        stream.emit(NewPage(page));
                    }
                });
                self.model
                    .spotify_tx
                    .send(SpotifyCmd::GetFavoriteTracks {
                        tx,
                        limit: PAGE_LIMIT,
                        offset,
                    })
                    .unwrap();
            }
            NewPage(page) => {
                let stream = self.model.relm.stream();
                let store = &self.model.store;
                let tracks = page.items;
                for track in tracks {
                    let track = track.track;
                    let pos = store.insert_with_values(
                        None,
                        &[
                            COL_TRACK_ID,
                            COL_TRACK_NAME,
                            COL_TRACK_ARTISTS,
                            COL_TRACK_NUMBER,
                            COL_TRACK_ALBUM,
                            COL_TRACK_CAN_PLAY,
                            COL_TRACK_DURATION,
                            COL_TRACK_DURATION_MS,
                        ],
                        &[
                            &track.id,
                            &track.name,
                            &track.artists.iter().map(|artist| &artist.name).join(", "),
                            &track.track_number,
                            &track.album.name,
                            &track.is_playable.unwrap_or(false),
                            &crate::utils::humanize_time(track.duration_ms),
                            &track.duration_ms,
                        ],
                    );

                    let image = crate::utils::find_best_thumb(track.album.images, THUMB_SIZE);

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
                self.model
                    .store
                    .set_value(&pos, COL_TRACK_THUMB, &thumb.to_value());
            }
            Click(event) if event.get_button() == 3 => {
                self.context_menu.popup_at_pointer(Some(&event));
            }
            Click(_) => (),
        }
    }

    view! {
        gtk::ScrolledWindow {
            #[name="tracks_view"]
            gtk::TreeView {
                model: Some(&__relm_model.store),

                button_press_event(_, event) => (FavoritesMsg::Click(event.clone()), Inhibit(false))
            },

            #[name="context_menu"]
            gtk::Menu {
                gtk::MenuItem { label: "Play now" },
                gtk::MenuItem { label: "Remove from library" },
            },
        }
    }

    fn init_view(&mut self) {
        let tree: &TreeView = &self.tracks_view;

        let text_cell = gtk::CellRendererText::new();
        let base_column = TreeViewColumnBuilder::new()
            .resizable(true)
            .reorderable(true)
            .expand(true);

        tree.append_column(&{
            let icon_cell = CellRendererPixbuf::new();
            //icon_cell.set_property_icon_name(Some("audio-x-generic-symbolic"));

            let column = TreeViewColumn::new();
            column.pack_start(&icon_cell, true);
            column.add_attribute(&icon_cell, "pixbuf", COL_TRACK_THUMB as i32);
            column
        });

        tree.append_column(&{
            let column = base_column
                .clone()
                .title("Title")
                .sort_column_id(COL_TRACK_NAME as i32)
                .build();
            column.pack_start(&text_cell, true);
            column.add_attribute(&text_cell, "text", COL_TRACK_NAME as i32);
            column
        });

        tree.append_column(&{
            let text_cell = gtk::CellRendererText::new();
            text_cell.set_alignment(1.0, 0.5);
            let column = base_column
                .clone()
                .title("Duration")
                .sort_column_id(COL_TRACK_DURATION_MS as i32)
                .build();
            column.pack_start(&text_cell, true);
            column.add_attribute(&text_cell, "text", COL_TRACK_DURATION as i32);
            column
        });

        tree.append_column(&{
            let column = base_column
                .clone()
                .title("Artists")
                .sort_column_id(COL_TRACK_ARTISTS as i32)
                .build();
            column.pack_start(&text_cell, true);
            column.add_attribute(&text_cell, "text", COL_TRACK_ARTISTS as i32);
            column
        });

        tree.append_column(&{
            let column = base_column
                .clone()
                .title("Album")
                .sort_column_id(COL_TRACK_ALBUM as i32)
                .build();
            column.pack_start(&text_cell, true);
            column.add_attribute(&text_cell, "text", COL_TRACK_ALBUM as i32);
            column
        });
    }
}
