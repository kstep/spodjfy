use crate::components::track_list::{TrackList, TrackListMsg};
use crate::image_loader::ImageLoader;
use crate::servers::spotify::{SpotifyCmd, SpotifyProxy};
use glib::StaticType;
use gtk::prelude::*;
use gtk::IconViewExt;
use relm::vendor::fragile::Fragile;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::page::Page;
use rspotify::model::show::{Show, SimplifiedEpisode};
use std::sync::Arc;

#[derive(Msg)]
pub enum ShowsMsg {
    ShowTab,
    LoadPage(u32),
    NewPage(Page<Show>),
    LoadThumb(String, gtk::TreeIter),
    NewThumb(gdk_pixbuf::Pixbuf, gtk::TreeIter),
    OpenChosenShow,
    OpenShow(Option<(String, String)>),
    GoToTrack(String),
}

const THUMB_SIZE: i32 = 256;
const PAGE_LIMIT: u32 = 10;

const COL_SHOW_THUMB: u32 = 0;
const COL_SHOW_NAME: u32 = 1;
const COL_SHOW_URI: u32 = 3;

pub struct ShowsModel {
    stream: EventStream<ShowsMsg>,
    spotify: Arc<SpotifyProxy>,
    store: gtk::ListStore,
    image_loader: ImageLoader,
}

#[widget]
impl Widget for ShowsTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> ShowsModel {
        let store = gtk::ListStore::new(&[
            gdk_pixbuf::Pixbuf::static_type(),
            String::static_type(),
            String::static_type(),
            String::static_type(),
        ]);
        let stream = relm.stream().clone();
        ShowsModel {
            stream,
            spotify,
            store,
            image_loader: ImageLoader::new_with_resize(THUMB_SIZE),
        }
    }

    fn update(&mut self, event: ShowsMsg) {
        use ShowsMsg::*;
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
                        move |tx| SpotifyCmd::GetMyShows {
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
                let shows = page.items;
                for show in shows {
                    let pos = store.insert_with_values(
                        None,
                        &[COL_SHOW_NAME, COL_SHOW_URI],
                        &[&show.show.name, &show.show.uri],
                    );

                    let image = crate::image_loader::find_best_thumb(&show.show.images, THUMB_SIZE);
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
            OpenChosenShow => {
                let icon_view: &gtk::IconView = &self.shows_view;
                let store: &gtk::ListStore = &self.model.store;
                self.model.stream.emit(OpenShow(
                    icon_view
                        .get_selected_items()
                        .first()
                        .and_then(|path| store.get_iter(path))
                        .and_then(|iter| {
                            store
                                .get_value(&iter, COL_SHOW_URI as i32)
                                .get::<String>()
                                .ok()
                                .flatten()
                                .zip(
                                    store
                                        .get_value(&iter, COL_SHOW_NAME as i32)
                                        .get::<String>()
                                        .ok()
                                        .flatten(),
                                )
                        }),
                ));
            }
            OpenShow(Some((uri, name))) => {
                self.show_view.emit(TrackListMsg::Reset(uri, true));

                let show_widget = self.show_view.widget();
                self.stack.set_child_title(show_widget, Some(&name));
                self.stack.set_visible_child(show_widget);
            }
            OpenShow(None) => {}
            GoToTrack(uri) => {
                self.show_view.emit(TrackListMsg::GoToTrack(uri));
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
                        title: Some("Shows"),
                    },

                    #[name="shows_view"]
                    /*
                    gtk::TreeView {
                        model: Some(&self.model.store)),
                    }
                     */
                    gtk::IconView {
                        item_width: THUMB_SIZE,
                        pixbuf_column: COL_SHOW_THUMB as i32,
                        text_column: COL_SHOW_NAME as i32,
                        model: Some(&self.model.store),

                        item_activated(view, path) => ShowsMsg::OpenShow(
                            view.get_model().and_then(|model| {
                                model.get_iter(path).and_then(|pos|
                                    model.get_value(&pos, COL_SHOW_URI as i32).get::<String>().ok().flatten()
                                        .zip(model.get_value(&pos, COL_SHOW_NAME as i32).get::<String>().ok().flatten()))
                            })),
                    }
                },
                #[name="show_view"]
                TrackList::<SimplifiedEpisode>(self.model.spotify.clone()),
            }
        }
    }

    fn init_view(&mut self) {
        self.breadcrumb.set_stack(Some(&self.stack));
        /*
        let tree: &TreeView = &self.shows_view;

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
