use crate::components::lists::{AlbumList, ContainerMsg, TrackList, TrackMsg};
use crate::components::tabs::{MusicTabModel, MusicTabMsg, MusicTabParams};
use crate::loaders::{AlbumLoader, SavedAlbumsLoader as SavedLoader};
use gtk::prelude::*;
use relm::{Relm, Widget};
use relm_derive::widget;

#[widget]
impl Widget for AlbumsTab {
    fn model(params: MusicTabParams) -> MusicTabModel {
        MusicTabModel::from_params(params)
    }

    fn update(&mut self, event: MusicTabMsg) {
        use MusicTabMsg::*;
        match event {
            ShowTab => {
                self.albums_view.emit(ContainerMsg::Load(()));
            }
            OpenContainer(0, uri, name) => {
                self.tracks_view.emit(ContainerMsg::Load(uri).into());

                let tracks_widget = self.tracks_view.widget();
                self.stack.set_child_title(tracks_widget, Some(&name));
                self.stack.set_visible_child(tracks_widget);
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
                #[name="albums_view"]
                AlbumList<SavedLoader>((self.model.pool.clone(), self.model.spotify.clone())) {
                    child: { title: Some("Albums") }
                },
                #[name="tracks_view"]
                TrackList::<AlbumLoader>((self.model.pool.clone(), self.model.spotify.clone())),
            }
        }
    }

    fn subscriptions(&mut self, relm: &Relm<Self>) {
        let stream = relm.stream().clone();
        self.albums_view.stream().observe(move |msg| {
            if let ContainerMsg::ActivateItem(uri, name) = msg {
                stream.emit(MusicTabMsg::OpenContainer(0, uri.clone(), name.clone()));
            }
        });

        let stream = relm.stream().clone();
        self.tracks_view.stream().observe(move |msg| {
            if let TrackMsg::PlayingNewTrack = msg {
                stream.emit(MusicTabMsg::PlaybackUpdate);
            }
        });
    }

    fn init_view(&mut self) {
        self.breadcrumb.set_stack(Some(&self.stack));
    }
}
