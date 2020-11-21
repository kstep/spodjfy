use crate::components::lists::{AlbumList, ArtistList, ContainerMsg, TrackList};
use crate::loaders::{AlbumLoader, ArtistLoader, SavedArtistsLoader as SavedLoader};
use crate::servers::spotify::SpotifyProxy;
use gtk::prelude::*;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use std::sync::Arc;

#[derive(Msg)]
pub enum ArtistsMsg {
    ShowTab,
    OpenArtist(String, String),
    OpenAlbum(String, String),
}

pub struct ArtistsModel {
    stream: EventStream<ArtistsMsg>,
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for ArtistsTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> ArtistsModel {
        let stream = relm.stream().clone();
        ArtistsModel { stream, spotify }
    }

    fn update(&mut self, event: ArtistsMsg) {
        use ArtistsMsg::*;
        match event {
            ShowTab => {
                self.artists_view.emit(ContainerMsg::Reset((), true));
            }
            OpenArtist(uri, name) => {
                self.albums_view.emit(ContainerMsg::Reset(uri, true));

                let albums_tab = self.albums_view.widget();
                self.stack.set_child_title(albums_tab, Some(&name));
                self.stack.set_visible_child(albums_tab);
            }
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

                #[name="artists_view"]
                ArtistList::<SavedLoader>(self.model.spotify.clone()) {
                    child: {
                        title: Some("Artists"),
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
        self.artists_view.stream().observe(move |msg| {
            if let ContainerMsg::ActivateItem(uri, name) = msg {
                stream.emit(ArtistsMsg::OpenArtist(uri.clone(), name.clone()));
            }
        });

        let stream = self.model.stream.clone();
        self.albums_view.stream().observe(move |msg| {
            if let ContainerMsg::ActivateItem(uri, name) = msg {
                stream.emit(ArtistsMsg::OpenAlbum(uri.clone(), name.clone()));
            }
        });
    }
}
