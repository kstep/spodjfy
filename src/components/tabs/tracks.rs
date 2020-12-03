use crate::{
    components::{
        lists::{ContainerMsg, TrackList, TrackMsg},
        tabs::{MusicTabModel, MusicTabMsg, MusicTabParams, TracksObserver},
    },
    loaders::{MyTopTracksLoader, SavedTracksLoader as SavedLoader},
};
use gtk::prelude::*;
use relm::{Relm, Widget};
use relm_derive::widget;

#[widget]
impl Widget for TracksTab {
    view! {
        gtk::Box(gtk::Orientation::Vertical, 1) {
            #[name="breadcrumb"]
            gtk::StackSwitcher {},

            #[name="stack"]
            gtk::Stack {
                vexpand: true,

                #[name="saved_tracks_view"]
                TrackList::<SavedLoader>((self.model.pool.clone(), self.model.spotify.clone())) {
                    child: {
                        title: Some("Saved Tracks"),
                    }
                },
                #[name="top_tracks_view"]
                TrackList::<MyTopTracksLoader>((self.model.pool.clone(), self.model.spotify.clone())) {
                    child: {
                        title: Some("Top Tracks"),
                    }
                },
            },
        },
    }

    fn model(params: MusicTabParams) -> MusicTabModel { MusicTabModel::from_params(params) }

    fn update(&mut self, event: MusicTabMsg) {
        use MusicTabMsg::*;

        match event {
            ShowTab => {
                self.saved_tracks_view.emit(ContainerMsg::Load(()).into());

                self.top_tracks_view.emit(ContainerMsg::Load(()).into());
            }
            GoToTrack(uri) => {
                self.saved_tracks_view.emit(ContainerMsg::Load(()).into());

                self.saved_tracks_view.emit(TrackMsg::GoToTrack(uri));
            }
            _ => {}
        }
    }

    fn init_view(&mut self) { self.breadcrumb.set_stack(Some(&self.stack)); }

    fn subscriptions(&mut self, relm: &Relm<Self>) {
        self.saved_tracks_view.stream().observe(TracksObserver::new(relm.stream()));

        self.top_tracks_view.stream().observe(TracksObserver::new(relm.stream()));
    }
}
