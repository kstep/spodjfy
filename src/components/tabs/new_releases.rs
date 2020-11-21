use crate::components::lists::album::AlbumList;
use crate::components::lists::common::ContainerMsg;
use crate::components::lists::track::{TrackList, TrackMsg};
use crate::loaders::album::NewReleasesLoader;
use crate::loaders::track::AlbumLoader;
use crate::servers::spotify::SpotifyProxy;
use gtk::prelude::*;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use std::sync::Arc;

#[derive(Msg)]
pub enum NewReleasesMsg {
    ShowTab,
    OpenAlbum(String, String),
    GoToTrack(String),
}

pub struct NewReleasesModel {
    stream: EventStream<NewReleasesMsg>,
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for NewReleasesTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> NewReleasesModel {
        let stream = relm.stream().clone();
        NewReleasesModel { stream, spotify }
    }

    fn update(&mut self, event: NewReleasesMsg) {
        use NewReleasesMsg::*;
        match event {
            ShowTab => {
                self.albums_view.emit(ContainerMsg::Reset((), true));
            }
            OpenAlbum(uri, name) => {
                self.tracks_view.emit(ContainerMsg::Load(uri).into());

                let album_widget = self.tracks_view.widget();
                self.stack.set_child_title(album_widget, Some(&name));
                self.stack.set_visible_child(album_widget);
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

        let stream = self.model.stream.clone();
        self.albums_view.stream().observe(move |msg| {
            if let ContainerMsg::ActivateItem(uri, name) = msg {
                stream.emit(NewReleasesMsg::OpenAlbum(uri.clone(), name.clone()));
            }
        });
    }
}
