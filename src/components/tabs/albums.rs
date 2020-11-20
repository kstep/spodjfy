use crate::components::lists::album::AlbumList;
use crate::components::lists::common::ContainerListMsg;
use crate::components::lists::track::{TrackList, TrackListMsg};
use crate::loaders::album::SavedLoader;
use crate::loaders::track::AlbumLoader;
use crate::servers::spotify::SpotifyProxy;
use gtk::prelude::*;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use std::sync::Arc;

#[derive(Msg)]
pub enum AlbumsMsg {
    ShowTab,
    OpenAlbum(String, String),
    GoToTrack(String),
}

pub struct AlbumsModel {
    stream: EventStream<AlbumsMsg>,
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for AlbumsTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> AlbumsModel {
        let stream = relm.stream().clone();
        AlbumsModel { stream, spotify }
    }

    fn update(&mut self, event: AlbumsMsg) {
        use AlbumsMsg::*;
        match event {
            ShowTab => {
                self.albums_view.emit(ContainerListMsg::Reset((), true));
            }
            OpenAlbum(uri, name) => {
                self.tracks_view.emit(TrackListMsg::Load(uri));

                let tracks_widget = self.tracks_view.widget();
                self.stack.set_child_title(tracks_widget, Some(&name));
                self.stack.set_visible_child(tracks_widget);
            }
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
                #[name="albums_view"]
                AlbumList<SavedLoader>(self.model.spotify.clone()) {
                    child: { title: Some("Albums") }
                },
                #[name="tracks_view"]
                TrackList::<AlbumLoader>(self.model.spotify.clone()),
            }
        }
    }

    fn init_view(&mut self) {
        self.breadcrumb.set_stack(Some(&self.stack));

        let stream = self.model.stream.clone();
        self.albums_view.stream().observe(move |msg| match msg {
            ContainerListMsg::ActivateItem(uri, name) => {
                stream.emit(AlbumsMsg::OpenAlbum(uri.clone(), name.clone()));
            }
            _ => {}
        });
    }
}
