use crate::components::lists::{ContainerMsg, TrackList, TrackMsg};
use crate::components::tabs::{MusicTabModel, MusicTabMsg, MusicTabParams, TracksObserver};
use crate::loaders::QueueLoader;
use relm::{Relm, Widget};
use relm_derive::widget;

#[widget]
impl Widget for QueueTab {
    fn model(params: MusicTabParams) -> MusicTabModel {
        MusicTabModel::from_params(params)
    }

    fn update(&mut self, event: MusicTabMsg) {
        use MusicTabMsg::*;
        match event {
            ShowTab => {
                self.tracks_view.emit(ContainerMsg::Load(()).into());
            }
            GoToTrack(uri) => {
                self.tracks_view.emit(TrackMsg::GoToTrack(uri));
            }
            _ => {}
        }
    }

    view! {
        #[name="tracks_view"]
        TrackList::<QueueLoader>((self.model.pool.clone(), self.model.spotify.clone()))
    }

    fn subscriptions(&mut self, relm: &Relm<Self>) {
        self.tracks_view
            .stream()
            .observe(TracksObserver::new(relm.stream()));
    }
}
