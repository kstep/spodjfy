use crate::components::lists::{ContainerMsg, PlaylistList, TrackList, TrackMsg};
use crate::loaders::{ShowLoader, ShowsLoader};
use crate::servers::spotify::SpotifyProxy;
use gtk::prelude::*;
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use std::sync::Arc;

#[derive(Msg)]
pub enum ShowsMsg {
    ShowTab,
    OpenShow(String, String),
    GoToTrack(String),
}

pub struct ShowsModel {
    stream: EventStream<ShowsMsg>,
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for ShowsTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> ShowsModel {
        let stream = relm.stream().clone();
        ShowsModel { stream, spotify }
    }

    fn update(&mut self, event: ShowsMsg) {
        use ShowsMsg::*;
        match event {
            ShowTab => {
                self.shows_view.emit(ContainerMsg::Reset((), true));
            }
            OpenShow(uri, name) => {
                self.tracks_view.emit(ContainerMsg::Load(uri).into());

                let show_widget = self.tracks_view.widget();
                self.stack.set_child_title(show_widget, Some(&name));
                self.stack.set_visible_child(show_widget);
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

                #[name="shows_view"]
                PlaylistList::<ShowsLoader>(self.model.spotify.clone()) {
                    child: { title: Some("Shows") },
                },

                #[name="tracks_view"]
                TrackList::<ShowLoader>(self.model.spotify.clone()),
            }
        }
    }

    fn init_view(&mut self) {
        self.breadcrumb.set_stack(Some(&self.stack));

        let stream = self.model.stream.clone();
        self.shows_view.stream().observe(move |msg| {
            if let ContainerMsg::ActivateItem(uri, name) = msg {
                stream.emit(ShowsMsg::OpenShow(uri.clone(), name.clone()));
            }
        });
    }
}
