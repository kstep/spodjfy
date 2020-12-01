use crate::components::lists::{ContainerMsg, PlaylistList, TrackList, TrackMsg};
use crate::components::tabs::{MusicTabModel, MusicTabMsg, MusicTabParams, TracksObserver};
use crate::loaders::{PlaylistLoader, SavedPlaylistsLoader as SavedLoader};
use gtk::prelude::*;
use relm::{Relm, Widget};
use relm_derive::widget;

#[widget]
impl Widget for PlaylistsTab {
    fn model(params: MusicTabParams) -> MusicTabModel {
        MusicTabModel::from_params(params)
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
                PlaylistList::<SavedLoader>((self.model.pool.clone(), self.model.spotify.clone())) {
                    child: { title: Some("Playlists") },
                },

                #[name="tracks_view"]
                TrackList::<PlaylistLoader>((self.model.pool.clone(), self.model.spotify.clone())),
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

        self.tracks_view
            .stream()
            .observe(TracksObserver::new(relm.stream()));
    }
}
