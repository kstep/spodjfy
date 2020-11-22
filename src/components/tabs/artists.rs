use crate::components::lists::{AlbumList, ArtistList, ContainerMsg, TrackList, TrackMsg};
use crate::components::tabs::MusicTabMsg;
use crate::loaders::{
    AlbumLoader, ArtistLoader, ArtistTopTracksLoader, SavedArtistsLoader as SavedLoader,
};
use crate::servers::spotify::SpotifyProxy;
use gtk::prelude::*;
use relm::{Relm, Widget};
use relm_derive::widget;
use std::sync::Arc;

pub struct ArtistsModel {
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for ArtistsTab {
    fn model(spotify: Arc<SpotifyProxy>) -> ArtistsModel {
        ArtistsModel { spotify }
    }

    fn update(&mut self, event: MusicTabMsg) {
        use MusicTabMsg::*;
        match event {
            ShowTab => {
                self.artists_view.emit(ContainerMsg::Load(()));
            }
            OpenContainer(0, uri, name) => {
                self.albums_view.emit(ContainerMsg::Load(uri.clone()));
                self.top_tracks_view.emit(ContainerMsg::Load(uri).into());

                let artist_tab = &self.artist_view;
                self.stack.set_child_title(artist_tab, Some(&name));
                self.stack.set_visible_child(artist_tab);
            }
            OpenContainer(1, uri, name) => {
                self.tracks_view.emit(ContainerMsg::Load(uri).into());

                let tracks_tab = self.tracks_view.widget();
                self.stack.set_child_title(tracks_tab, Some(&name));
                self.stack.set_visible_child(tracks_tab);
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

                #[name="artists_view"]
                ArtistList::<SavedLoader>(self.model.spotify.clone()) {
                    child: {
                        title: Some("Artists"),
                    }
                },

                #[name="artist_view"]
                gtk::Paned(gtk::Orientation::Vertical) {
                    #[name="top_tracks_view"]
                    TrackList::<ArtistTopTracksLoader>(self.model.spotify.clone()),
                    #[name="albums_view"]
                    AlbumList::<ArtistLoader>(self.model.spotify.clone()),
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
        self.artists_view.stream().observe(move |msg| {
            if let ContainerMsg::ActivateItem(uri, name) = msg {
                stream.emit(MusicTabMsg::OpenContainer(0, uri.clone(), name.clone()));
            }
        });

        let stream = relm.stream().clone();
        self.albums_view.stream().observe(move |msg| {
            if let ContainerMsg::ActivateItem(uri, name) = msg {
                stream.emit(MusicTabMsg::OpenContainer(1, uri.clone(), name.clone()));
            }
        });

        let stream = relm.stream().clone();
        self.tracks_view.stream().observe(move |msg| {
            if let TrackMsg::PlayingNewTrack = msg {
                stream.emit(MusicTabMsg::PlaybackUpdate);
            }
        });

        let stream = relm.stream().clone();
        self.top_tracks_view.stream().observe(move |msg| {
            if let TrackMsg::PlayingNewTrack = msg {
                stream.emit(MusicTabMsg::PlaybackUpdate);
            }
        });
    }
}
