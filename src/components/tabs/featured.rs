use crate::components::lists::{ContainerMsg, PlaylistList, TrackList, TrackMsg};
use crate::components::tabs::MusicTabMsg;
use crate::loaders::{FeaturedLoader, PlaylistLoader};
use crate::servers::spotify::SpotifyProxy;
use gtk::prelude::*;
use relm::{Relm, Widget};
use relm_derive::widget;
use std::sync::Arc;

pub struct FeaturedModel {
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for FeaturedTab {
    fn model(spotify: Arc<SpotifyProxy>) -> FeaturedModel {
        FeaturedModel { spotify }
    }

    fn update(&mut self, event: MusicTabMsg) {
        use MusicTabMsg::*;
        match event {
            ShowTab => {
                self.playlists_view.emit(ContainerMsg::Load(()));
            }
            OpenContainer(0, uri, name) => {
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

                #[name="playlists_view"]
                PlaylistList::<FeaturedLoader>(self.model.spotify.clone()) {
                    child: { title: Some("Featured") },
                },

                #[name="tracks_view"]
                TrackList::<PlaylistLoader>(self.model.spotify.clone()),
            },
        }
    }

    fn init_view(&mut self) {
        self.breadcrumb.set_stack(Some(&self.stack));
    }

    fn subscriptions(&mut self, relm: &Relm<Self>) {
        let stream = relm.stream().clone();
        self.playlists_view.stream().observe(move |msg| {
            if let ContainerMsg::ActivateItem(uri, name) = msg {
                stream.emit(MusicTabMsg::OpenContainer(0, uri.clone(), name.clone()));
            }
        });
    }
}
