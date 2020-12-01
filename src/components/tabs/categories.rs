use crate::components::lists::{CategoryList, ContainerMsg, PlaylistList, TrackList, TrackMsg};
use crate::components::tabs::{MusicTabModel, MusicTabMsg, MusicTabParams, TracksObserver};
use crate::loaders::{CategoriesLoader, CategoryLoader, PlaylistLoader};
use gtk::prelude::*;
use relm::{Relm, Widget};
use relm_derive::widget;

#[widget]
impl Widget for CategoriesTab {
    fn model(params: MusicTabParams) -> MusicTabModel {
        MusicTabModel::from_params(params)
    }

    fn update(&mut self, event: MusicTabMsg) {
        use MusicTabMsg::*;
        match event {
            ShowTab => {
                self.categories_view.emit(ContainerMsg::Load(()));
            }
            OpenContainer(0, id, name) => {
                self.playlists_view.emit(ContainerMsg::Load(id));

                let playlists_tab = self.playlists_view.widget();
                self.stack.set_child_title(playlists_tab, Some(&name));
                self.stack.set_visible_child(playlists_tab);
            }
            OpenContainer(1, uri, name) => {
                self.tracks_view.emit(ContainerMsg::Load(uri).into());

                let tracks_tab = self.tracks_view.widget();
                self.stack.set_child_title(tracks_tab, Some(&name));
                self.stack.set_visible_child(tracks_tab);
            }
            GoToTrack(uri) => {
                self.tracks_view.emit(TrackMsg::GoToTrack(uri));
            }
            _ => {}
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
                CategoryList<CategoriesLoader>((self.model.pool.clone(), self.model.spotify.clone())) {
                    child: {
                        title: Some("Categories"),
                    },
                },

                #[name="playlists_view"]
                PlaylistList::<CategoryLoader>((self.model.pool.clone(), self.model.spotify.clone())),

                #[name="tracks_view"]
                TrackList::<PlaylistLoader>((self.model.pool.clone(), self.model.spotify.clone())),
            }
        }
    }

    fn init_view(&mut self) {
        self.breadcrumb.set_stack(Some(&self.stack));
    }

    fn subscriptions(&mut self, relm: &Relm<Self>) {
        let stream = relm.stream().clone();
        self.categories_view.stream().observe(move |msg| {
            if let ContainerMsg::ActivateItem(uri, name) = msg {
                stream.emit(MusicTabMsg::OpenContainer(0, uri.clone(), name.clone()));
            }
        });

        let stream = relm.stream().clone();
        self.playlists_view.stream().observe(move |msg| {
            if let ContainerMsg::ActivateItem(uri, name) = msg {
                stream.emit(MusicTabMsg::OpenContainer(1, uri.clone(), name.clone()));
            }
        });

        self.tracks_view
            .stream()
            .observe(TracksObserver::new(relm.stream()));
    }
}
