use std::any::Any;
use crate::{
    components::{
        lists::{AlbumList, ContainerMsg, TrackList, TrackMsg},
        tabs::{MusicTabModel, MusicTabMsg, MusicTabParams, TracksObserver},
    },
    loaders::{AlbumLoader, NewReleasesLoader},
};
use gtk::prelude::*;
use relm::{Relm, Widget};
use relm_derive::widget;
use rspotify::model::{AlbumId, TrackId};

#[widget]
impl Widget for NewReleasesTab {
    view! {
        gtk::Box(gtk::Orientation::Vertical, 1) {
            #[name="breadcrumb"]
            gtk::StackSwitcher {},

            #[name="stack"]
            gtk::Stack {
                vexpand: true,

                #[name="albums_view"]
                AlbumList::<NewReleasesLoader>((self.model.pool.clone(), self.model.spotify.clone())) {
                    child: { title: Some("New releases") },
                },

                #[name="tracks_view"]
                TrackList::<AlbumLoader>((self.model.pool.clone(), self.model.spotify.clone())),
            }
        }
    }

    fn model(params: MusicTabParams) -> MusicTabModel { MusicTabModel::from_params(params) }

    fn update(&mut self, event: MusicTabMsg) {
        use MusicTabMsg::*;

        match event {
            ShowTab => {
                self.albums_view.emit(ContainerMsg::Load(()));
            }
            OpenContainer(0, id, name) => {
                if let Ok(id) = (id as Box<dyn Any>).downcast::<AlbumId>() {
                    self.tracks_view.emit(ContainerMsg::Load(*id).into());

                    let album_widget = self.tracks_view.widget();

                    self.stack.set_child_title(album_widget, Some(&name));

                    self.stack.set_visible_child(album_widget);
                }
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

        self.albums_view.stream().observe(move |msg| {
            if let ContainerMsg::ActivateItem(uri, name) = msg {
                stream.emit(MusicTabMsg::OpenContainer(0, uri.clone(), name.clone()));
            }
        });

        self.tracks_view.stream().observe(TracksObserver::new(relm.stream()));
    }
}
