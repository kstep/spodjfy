use crate::{
    components::{
        lists::{ContainerMsg, PlaylistList, TrackList, TrackMsg},
        tabs::{MusicTabModel, MusicTabMsg, MusicTabParams, TracksObserver},
    },
    loaders::{ShowLoader, ShowsLoader},
};
use gtk::prelude::*;
use relm::{Relm, Widget};
use relm_derive::widget;

#[widget]
impl Widget for ShowsTab {
    view! {
        gtk::Box(gtk::Orientation::Vertical, 1) {
            #[name="breadcrumb"]
            gtk::StackSwitcher {},

            #[name="stack"]
            gtk::Stack {
                vexpand: true,

                #[name="shows_view"]
                PlaylistList::<ShowsLoader>((self.model.pool.clone(), self.model.spotify.clone())) {
                    child: { title: Some("Shows") },
                },

                #[name="tracks_view"]
                TrackList::<ShowLoader>((self.model.pool.clone(), self.model.spotify.clone())),
            }
        }
    }

    fn model(params: MusicTabParams) -> MusicTabModel { MusicTabModel::from_params(params) }

    fn update(&mut self, event: MusicTabMsg) {
        use MusicTabMsg::*;

        match event {
            ShowTab => {
                self.shows_view.emit(ContainerMsg::Load(()));
            }
            OpenContainer(0, uri, name) => {
                self.tracks_view.emit(ContainerMsg::Load(uri).into());

                let show_widget = self.tracks_view.widget();

                self.stack.set_child_title(show_widget, Some(&name));

                self.stack.set_visible_child(show_widget);
            }
            GoToTrack(uri) => {
                self.tracks_view.emit(TrackMsg::GoToTrack(uri));
            }
            _ => {}
        }
    }

    fn init_view(&mut self) { self.breadcrumb.set_stack(Some(&self.stack)); }

    fn subscriptions(&mut self, relm: &Relm<Self>) {
        let stream = relm.stream().clone();

        self.shows_view.stream().observe(move |msg| {
            if let ContainerMsg::ActivateItem(uri, name) = msg {
                stream.emit(MusicTabMsg::OpenContainer(0, uri.clone(), name.clone()));
            }
        });

        self.tracks_view.stream().observe(TracksObserver::new(relm.stream()));
    }
}
