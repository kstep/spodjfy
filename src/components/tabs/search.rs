use crate::components::lists::track::{TrackList, TrackListMsg};
use crate::loaders::track::RecommendLoader;
use crate::servers::spotify::SpotifyProxy;
use gtk::{ButtonExt, LabelExt};
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use std::sync::Arc;

#[derive(Msg)]
pub enum SearchMsg {
    ShowTab,
}

pub struct SearchModel {
    spotify: Arc<SpotifyProxy>,
    stream: EventStream<SearchMsg>,
}

#[widget]
impl Widget for SearchTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> SearchModel {
        let stream = relm.stream().clone();
        SearchModel { spotify, stream }
    }

    fn update(&mut self, msg: SearchMsg) {
        use SearchMsg::*;
        match msg {
            ShowTab => {}
        }
    }

    view! {
        gtk::Box(gtk::Orientation::Vertical, 0) {
            gtk::ScrolledWindow {
                gtk::Grid {
                /*
                    gtk::Label {
                        text: "Genres"
                    },
                    gtk::Label {
                        text: "Artists"
                    },
                    gtk::Label {
                        text: "Tracks"
                    },
                 */

                    gtk::Label {
                       text: "BPM"
                    },
                    gtk::SpinButton {},
                    gtk::SpinButton {},
                    gtk::SpinButton {},

                    gtk::Label {
                       text: "Duration"
                    },
                    gtk::SpinButton {},
                    gtk::SpinButton {},
                    gtk::SpinButton {},

                    gtk::Label {
                       text: "Key"
                    },
                    gtk::SpinButton {},
                    gtk::SpinButton {},
                    gtk::SpinButton {},

                    gtk::Label {
                       text: "Mode"
                    },
                    gtk::CheckButton {
                       label: "Minor"
                    },
                    gtk::CheckButton {
                       label: "Major"
                    },

                    gtk::Label {
                       text: "Instrumental"
                    },
                    gtk::SpinButton {},
                    gtk::SpinButton {},
                    gtk::SpinButton {},

                    gtk::Label {
                       text: "Speech"
                    },
                    gtk::SpinButton {},
                    gtk::SpinButton {},
                    gtk::SpinButton {},

                    gtk::Label {
                       text: "Acoustic"
                    },
                    gtk::SpinButton {},
                    gtk::SpinButton {},
                    gtk::SpinButton {},

                    gtk::Label {
                       text: "Dance"
                    },
                    gtk::SpinButton {},
                    gtk::SpinButton {},
                    gtk::SpinButton {},

                    gtk::Label {
                       text: "Energy"
                    },
                    gtk::SpinButton {},
                    gtk::SpinButton {},
                    gtk::SpinButton {},

                    gtk::Label {
                       text: "Liveness"
                    },
                    gtk::SpinButton {},
                    gtk::SpinButton {},
                    gtk::SpinButton {},

                    gtk::Label {
                       text: "Valence"
                    },
                    gtk::SpinButton {},
                    gtk::SpinButton {},
                    gtk::SpinButton {},

                    gtk::Label {
                       text: "Loudness"
                    },
                    gtk::SpinButton {},
                    gtk::SpinButton {},
                    gtk::SpinButton {},

                    gtk::Label {
                       text: "Popularity"
                    },
                    gtk::SpinButton {},
                    gtk::SpinButton {},
                    gtk::SpinButton {},

                    gtk::Label {
                       text: "Time sig."
                    },
                    gtk::SpinButton {},
                    gtk::SpinButton {},
                    gtk::SpinButton {},
                },
            },
            TrackList::<RecommendLoader>(self.model.spotify.clone())
        }
    }
}
