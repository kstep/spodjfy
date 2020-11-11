use crate::image_loader::ImageLoader;
use crate::spotify::{SpotifyCmd, SpotifyProxy};
use glib::StaticType;
use gtk::prelude::*;
use gtk::IconViewExt;
use relm::vendor::fragile::Fragile;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::artist::FullArtist;
use rspotify::model::page::CursorBasedPage;
use std::sync::Arc;

#[derive(Msg)]
pub enum ArtistsMsg {
    ShowTab,
    LoadPage(Option<String>),
    NewPage(CursorBasedPage<FullArtist>),
    LoadThumb(String, gtk::TreeIter),
    NewThumb(gdk_pixbuf::Pixbuf, gtk::TreeIter),
    OpenChosenArtist,
    Click(gdk::EventButton),
}

const THUMB_SIZE: i32 = 256;
const PAGE_LIMIT: u32 = 10;

const COL_ARTIST_THUMB: u32 = 0;
const COL_ARTIST_NAME: u32 = 1;
const COL_ARTIST_URI: u32 = 2;

pub struct ArtistsModel {
    stream: EventStream<ArtistsMsg>,
    spotify: Arc<SpotifyProxy>,
    store: gtk::ListStore,
    image_loader: ImageLoader,
}

#[widget]
impl Widget for ArtistsTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> ArtistsModel {
        let store = gtk::ListStore::new(&[
            gdk_pixbuf::Pixbuf::static_type(),
            String::static_type(),
            String::static_type(),
        ]);
        let stream = relm.stream().clone();
        ArtistsModel {
            stream,
            spotify,
            store,
            image_loader: ImageLoader::new_with_resize(THUMB_SIZE),
        }
    }

    fn update(&mut self, event: ArtistsMsg) {
        use ArtistsMsg::*;
        match event {
            ShowTab => {
                self.model.store.clear();
                self.model.stream.emit(LoadPage(None))
            }
            LoadPage(cursor) => {
                self.model.spotify.ask(
                    self.model.stream.clone(),
                    move |tx| SpotifyCmd::GetMyArtists {
                        tx,
                        limit: PAGE_LIMIT,
                        cursor,
                    },
                    NewPage,
                );
            }
            NewPage(page) => {
                let stream = &self.model.stream;
                let store = &self.model.store;
                let artists = page.items;
                for artist in artists {
                    let pos = store.insert_with_values(
                        None,
                        &[COL_ARTIST_NAME, COL_ARTIST_URI],
                        &[&artist.name, &artist.uri],
                    );

                    let image = crate::image_loader::find_best_thumb(&artist.images, THUMB_SIZE);
                    if let Some(url) = image {
                        stream.emit(LoadThumb(url.to_owned(), pos));
                    }
                }

                if page.next.is_some() {
                    stream.emit(LoadPage(page.next));
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
                    .set_value(&pos, COL_ARTIST_THUMB, &thumb.to_value());
            }
            OpenChosenArtist => {
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
                {
                    /*
                    self.artist_view.emit(TrackListMsg::Reset(uri));

                    let artist_widget = self.artist_view.widget();
                    self.stack.set_child_title(artist_widget, Some(&name));
                    self.stack.set_visible_child(artist_widget);
                    */
                }
            }
            Click(event) if event.get_event_type() == gdk::EventType::DoubleButtonPress => {
                self.model.stream.emit(OpenChosenArtist);
            }
            Click(_) => {}
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
                        title: Some("Artists"),
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
                        model: Some(&self.model.store),

                        button_press_event(_, event) => (ArtistsMsg::Click(event.clone()), Inhibit(false)),
                    }
                },
                //#[name="artist_view"]
                //TrackList::<FullTrack>(self.model.spotify.clone()),
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
