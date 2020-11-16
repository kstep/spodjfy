use crate::components::lists::track::{TrackList, TrackListMsg};
use crate::loaders::image::ImageLoader;
use crate::loaders::track::PlaylistLoader;
use crate::servers::spotify::{SpotifyCmd, SpotifyProxy};
use glib::StaticType;
use gtk::prelude::*;
use gtk::IconViewExt;
use relm::vendor::fragile::Fragile;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::category::Category;
use rspotify::model::page::Page;
use std::sync::Arc;

#[derive(Msg)]
pub enum CategoriesMsg {
    ShowTab,
    LoadPage(u32),
    NewPage(Page<Category>),
    LoadThumb(String, gtk::TreeIter),
    NewThumb(gdk_pixbuf::Pixbuf, gtk::TreeIter),
    OpenChosenCategory,
    OpenCategory(Option<(String, String)>),
    GoToTrack(String),
}

const ICON_SIZE: i32 = 256;
const PAGE_LIMIT: u32 = 10;

const COL_CATEGORY_ICON: u32 = 0;
const COL_CATEGORY_NAME: u32 = 1;
const COL_CATEGORY_ID: u32 = 3;

pub struct CategoriesModel {
    stream: EventStream<CategoriesMsg>,
    spotify: Arc<SpotifyProxy>,
    store: gtk::ListStore,
    image_loader: ImageLoader,
}

#[widget]
impl Widget for CategoriesTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> CategoriesModel {
        let store = gtk::ListStore::new(&[
            gdk_pixbuf::Pixbuf::static_type(),
            String::static_type(),
            String::static_type(),
            String::static_type(),
        ]);
        let stream = relm.stream().clone();
        CategoriesModel {
            stream,
            spotify,
            store,
            image_loader: ImageLoader::new_with_resize(ICON_SIZE),
        }
    }

    fn update(&mut self, event: CategoriesMsg) {
        use CategoriesMsg::*;
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
                        move |tx| SpotifyCmd::GetCategories {
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
                let categories = page.items;
                for category in categories {
                    let pos = store.insert_with_values(
                        None,
                        &[COL_CATEGORY_NAME, COL_CATEGORY_ID],
                        &[&category.name, &category.id],
                    );

                    let image = crate::loaders::image::find_best_thumb(&category.icons, ICON_SIZE);
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
            OpenChosenCategory => {
                /*
                let icon_view: &gtk::IconView = &self.categories_view;
                let store: &gtk::ListStore = &self.model.store;
                self.model.stream.emit(OpenCategory(
                    icon_view
                        .get_selected_items()
                        .first()
                        .and_then(|path| store.get_iter(path))
                        .and_then(|iter| {
                            store
                                .get_value(&iter, COL_CATEGORY_URI as i32)
                                .get::<String>()
                                .ok()
                                .flatten()
                                .zip(
                                    store
                                        .get_value(&iter, COL_CATEGORY_NAME as i32)
                                        .get::<String>()
                                        .ok()
                                        .flatten(),
                                )
                        }),
                ));
                 */
            }
            OpenCategory(Some((id, name))) => {
                /*
                self.tracks_view.emit(TrackListMsg::Reset(uri, true));

                let category_widget = self.tracks_view.widget();
                self.stack.set_child_title(category_widget, Some(&name));
                self.stack.set_visible_child(category_widget);
                 */
            }
            OpenCategory(None) => {}
            GoToTrack(uri) => {
                self.tracks_view.emit(TrackListMsg::GoToTrack(uri));
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
                            title: Some("Categories"),
                        },

                        #[name="categories_view"]
                        /*
                        gtk::TreeView {
                            model: Some(&self.model.store)),
                        }
                         */
                        gtk::IconView {
                            item_width: ICON_SIZE,
                            pixbuf_column: COL_CATEGORY_ICON as i32,
                            text_column: COL_CATEGORY_NAME as i32,
                            model: Some(&self.model.store),

    /*
                            item_activated(view, path) => CategoriesMsg::OpenCategory(
                                view.get_model().and_then(|model| {
                                    model.get_iter(path).and_then(|pos|
                                        model.get_value(&pos, COL_CATEGORY_URI as i32).get::<String>().ok().flatten()
                                            .zip(model.get_value(&pos, COL_CATEGORY_NAME as i32).get::<String>().ok().flatten()))
                                })),
     */
                        }
                    },
                    #[name="tracks_view"]
                    TrackList::<PlaylistLoader>(self.model.spotify.clone()),
                }
            }
        }

    fn init_view(&mut self) {
        self.breadcrumb.set_stack(Some(&self.stack));
        /*
        let tree: &TreeView = &self.categories_view;

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