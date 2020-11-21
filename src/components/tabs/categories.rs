use crate::components::lists::{CategoryList, ContainerMsg, PlaylistList, TrackList, TrackMsg};
use crate::loaders::{CategoriesLoader, CategoryLoader, PlaylistLoader};
use crate::servers::spotify::SpotifyProxy;
use gtk::prelude::*;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use std::sync::Arc;

#[derive(Msg)]
pub enum CategoriesMsg {
    ShowTab,
    OpenCategory(String, String),
    OpenPlaylist(String, String),
    GoToTrack(String),
}

pub struct CategoriesModel {
    stream: EventStream<CategoriesMsg>,
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for CategoriesTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> CategoriesModel {
        let stream = relm.stream().clone();
        CategoriesModel { stream, spotify }
    }

    fn update(&mut self, event: CategoriesMsg) {
        use CategoriesMsg::*;
        match event {
            ShowTab => {
                self.categories_view.emit(ContainerMsg::Load(()));
            }
            OpenCategory(id, name) => {
                self.playlists_view.emit(ContainerMsg::Load(id));

                let playlists_tab = self.playlists_view.widget();
                self.stack.set_child_title(playlists_tab, Some(&name));
                self.stack.set_visible_child(playlists_tab);
            }
            OpenPlaylist(uri, name) => {
                self.tracks_view.emit(ContainerMsg::Load(uri).into());

                let tracks_tab = self.tracks_view.widget();
                self.stack.set_child_title(tracks_tab, Some(&name));
                self.stack.set_visible_child(tracks_tab);
            }
            GoToTrack(uri) => {
                self.tracks_view.emit(TrackMsg::GoToTrack(uri));
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

                #[name="categories_view"]
                CategoryList<CategoriesLoader>(self.model.spotify.clone()) {
                    child: {
                        title: Some("Categories"),
                    },
                },

                #[name="playlists_view"]
                PlaylistList::<CategoryLoader>(self.model.spotify.clone()),

                #[name="tracks_view"]
                TrackList::<PlaylistLoader>(self.model.spotify.clone()),
            }
        }
    }

    fn init_view(&mut self) {
        self.breadcrumb.set_stack(Some(&self.stack));

        let stream = self.model.stream.clone();
        self.categories_view.stream().observe(move |msg| {
            if let ContainerMsg::ActivateItem(uri, name) = msg {
                stream.emit(CategoriesMsg::OpenCategory(uri.clone(), name.clone()));
            }
        });

        let stream = self.model.stream.clone();
        self.playlists_view.stream().observe(move |msg| {
            if let ContainerMsg::ActivateItem(uri, name) = msg {
                stream.emit(CategoriesMsg::OpenPlaylist(uri.clone(), name.clone()));
            }
        });
    }
}
