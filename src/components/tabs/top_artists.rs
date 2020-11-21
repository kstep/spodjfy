use crate::components::lists::{AlbumList, ContainerMsg, TrackList};
use crate::loaders::{AlbumLoader, ArtistLoader, ImageLoader};
use crate::servers::spotify::{SpotifyCmd, SpotifyProxy};
use glib::StaticType;
use gtk::prelude::*;
use gtk::IconViewExt;
use relm::vendor::fragile::Fragile;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::artist::FullArtist;
use rspotify::model::page::Page;
use std::sync::Arc;

#[derive(Msg)]
pub enum TopArtistsMsg {
    ShowTab,
    LoadPage(u32),
    NewPage(Page<FullArtist>),
    LoadThumb(String, gtk::TreeIter),
    NewThumb(gdk_pixbuf::Pixbuf, gtk::TreeIter),
    OpenArtist(Option<(String, String)>),
    OpenAlbum(String, String),
}

const THUMB_SIZE: i32 = 256;
const PAGE_LIMIT: u32 = 10;

const COL_ARTIST_THUMB: u32 = 0;
const COL_ARTIST_NAME: u32 = 1;
const COL_ARTIST_URI: u32 = 2;

pub struct TopArtistsModel {
    stream: EventStream<TopArtistsMsg>,
    spotify: Arc<SpotifyProxy>,
    store: gtk::ListStore,
    image_loader: ImageLoader,
}

#[widget]
impl Widget for TopArtistsTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> TopArtistsModel {
        let store = gtk::ListStore::new(&[
            gdk_pixbuf::Pixbuf::static_type(),
            String::static_type(),
            String::static_type(),
        ]);
        let stream = relm.stream().clone();
        TopArtistsModel {
            stream,
            spotify,
            store,
            image_loader: ImageLoader::new_with_resize(THUMB_SIZE),
        }
    }

    fn update(&mut self, event: TopArtistsMsg) {
        use TopArtistsMsg::*;
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
                        move |tx| SpotifyCmd::GetMyTopArtists {
                            tx,
                            offset,
                            limit: PAGE_LIMIT,
                        },
                        NewPage,
                    )
                    .unwrap();
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

                    let image = self.model.image_loader.find_best_thumb(&artist.images);
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
                    .set_value(&pos, COL_ARTIST_THUMB, &thumb.to_value());
            }
            OpenArtist(Some((uri, name))) => {
                self.albums_view.emit(ContainerMsg::Reset(uri, true));

                let albums_tab = self.albums_view.widget();
                self.stack.set_child_title(albums_tab, Some(&name));
                self.stack.set_visible_child(albums_tab);
            }
            OpenArtist(None) => {}
            OpenAlbum(uri, name) => {
                self.tracks_view.emit(ContainerMsg::Load(uri).into());

                let tracks_tab = self.tracks_view.widget();
                self.stack.set_child_title(tracks_tab, Some(&name));
                self.stack.set_visible_child(tracks_tab);
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

                #[name="artists_tab"]
                gtk::ScrolledWindow {
                    child: {
                        title: Some("Top artists"),
                    },

                    #[name="artists_view"]
                    gtk::IconView {
                        item_width: THUMB_SIZE,
                        pixbuf_column: COL_ARTIST_THUMB as i32,
                        text_column: COL_ARTIST_NAME as i32,
                        model: Some(&self.model.store),

                        item_activated(view, path) => TopArtistsMsg::OpenArtist(
                            view.get_model().and_then(|model| {
                                model.get_iter(path).and_then(|pos|
                                    model.get_value(&pos, COL_ARTIST_URI as i32).get::<String>().ok().flatten()
                                        .zip(model.get_value(&pos, COL_ARTIST_NAME as i32).get::<String>().ok().flatten()))
                            })),
                    }
                },

                #[name="albums_view"]
                AlbumList::<ArtistLoader>(self.model.spotify.clone()),

                #[name="tracks_view"]
                TrackList::<AlbumLoader>(self.model.spotify.clone()),
            }
        }
    }

    fn init_view(&mut self) {
        self.breadcrumb.set_stack(Some(&self.stack));

        let stream = self.model.stream.clone();
        self.albums_view.stream().observe(move |msg| {
            if let ContainerMsg::ActivateItem(uri, name) = msg {
                stream.emit(TopArtistsMsg::OpenAlbum(uri.clone(), name.clone()));
            }
        });
    }
}
