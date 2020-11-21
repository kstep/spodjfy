use crate::components::lists::{AlbumList, ContainerMsg, TrackList, TrackMsg};
use crate::components::tabs::MusicTabMsg;
use crate::loaders::{AlbumLoader, NewReleasesLoader};
use crate::servers::spotify::SpotifyProxy;
use gtk::prelude::*;
use relm::{Relm, Widget};
use relm_derive::widget;
use std::sync::Arc;

pub struct NewReleasesModel {
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for NewReleasesTab {
    fn model(spotify: Arc<SpotifyProxy>) -> NewReleasesModel {
        NewReleasesModel { spotify }
    }

    fn update(&mut self, event: MusicTabMsg) {
        use MusicTabMsg::*;
        match event {
            ShowTab => {
                self.albums_view.emit(ContainerMsg::Load(()));
            }
            OpenContainer(0, uri, name) => {
                self.tracks_view.emit(ContainerMsg::Load(uri).into());

                let album_widget = self.tracks_view.widget();
                self.stack.set_child_title(album_widget, Some(&name));
                self.stack.set_visible_child(album_widget);
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
                AlbumList::<NewReleasesLoader>(self.model.spotify.clone()) {
                    child: { title: Some("New releases") },
                },

                #[name="tracks_view"]
                TrackList::<AlbumLoader>(self.model.spotify.clone()),
            }
        }
    }

    fn init_view(&mut self) {
        self.breadcrumb.set_stack(Some(&self.stack));
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
}
