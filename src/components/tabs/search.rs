// TODO
#![allow(unused_imports, dead_code)]
use crate::components::lists::TrackList;
use crate::loaders::RecommendLoader;
use crate::servers::spotify::SpotifyProxy;
use crate::utils::{SearchTerm, SearchTerms};
use gtk::{
    BoxExt, ButtonExt, CheckMenuItemExt, ContainerExt, EntryExt, FlowBoxChildExt, FlowBoxExt,
    GtkMenuExt, GtkMenuItemExt, LabelExt, MenuBarExt, MenuButtonExt, OrientableExt, WidgetExt,
};
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Msg, Copy, Clone)]
pub enum SearchMsg {
    ShowTab,
    AddSearchTerm(SearchTerm, bool),
}

// TODO
pub struct SearchModel {
    spotify: Arc<SpotifyProxy>,
    search_terms: Rc<RefCell<SearchTerms>>,
    _stream: EventStream<SearchMsg>,
}

#[widget]
impl Widget for SearchTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> SearchModel {
        let _stream = relm.stream().clone();
        SearchModel {
            spotify,
            _stream,
            search_terms: Rc::new(RefCell::new(SearchTerms::default())),
        }
    }

    fn update(&mut self, msg: SearchMsg) {
        use SearchMsg::*;
        match msg {
            ShowTab => {}
            AddSearchTerm(term, add) => {
                self.model.search_terms.borrow_mut().update(term, add);
                self.search_terms_box.invalidate_filter();
            }
        }
    }

    view! {
        gtk::Box(gtk::Orientation::Vertical, 10) {

            #[name="search_terms_menu"]
            gtk::Menu {
                gtk::CheckMenuItem {
                    label: "BPM",
                    toggled(btn) => SearchMsg::AddSearchTerm(SearchTerm::Tempo, btn.get_active()),
                },
                gtk::CheckMenuItem {
                    label: "Duration",
                    toggled(btn) => SearchMsg::AddSearchTerm(SearchTerm::Duration, btn.get_active()),
                },
                gtk::CheckMenuItem {
                    label: "Key",
                    toggled(btn) => SearchMsg::AddSearchTerm(SearchTerm::Key, btn.get_active()),
                },
                gtk::CheckMenuItem {
                    label: "Mode",
                    toggled(btn) => SearchMsg::AddSearchTerm(SearchTerm::Mode, btn.get_active()),
                },
                gtk::CheckMenuItem {
                    label: "Instrumental",
                    toggled(btn) => SearchMsg::AddSearchTerm(SearchTerm::Instrumental, btn.get_active()),
                },
                gtk::CheckMenuItem {
                    label: "Speech",
                    toggled(btn) => SearchMsg::AddSearchTerm(SearchTerm::Speech, btn.get_active()),
                },
                gtk::CheckMenuItem {
                    label: "Acoustic",
                    toggled(btn) => SearchMsg::AddSearchTerm(SearchTerm::Acoustic, btn.get_active()),
                },
                gtk::CheckMenuItem {
                    label: "Dance",
                    toggled(btn) => SearchMsg::AddSearchTerm(SearchTerm::Dance, btn.get_active()),
                },
                gtk::CheckMenuItem {
                    label: "Energy",
                    toggled(btn) => SearchMsg::AddSearchTerm(SearchTerm::Energy, btn.get_active()),
                },
                gtk::CheckMenuItem {
                    label: "Liveness",
                    toggled(btn) => SearchMsg::AddSearchTerm(SearchTerm::Liveness, btn.get_active()),
                },
                gtk::CheckMenuItem {
                    label: "Valence",
                    toggled(btn) => SearchMsg::AddSearchTerm(SearchTerm::Valence, btn.get_active()),
                },
                gtk::CheckMenuItem {
                    label: "Loudness",
                    toggled(btn) => SearchMsg::AddSearchTerm(SearchTerm::Loudness, btn.get_active()),
                },
                gtk::CheckMenuItem {
                    label: "Popularity",
                    toggled(btn) => SearchMsg::AddSearchTerm(SearchTerm::Popularity, btn.get_active()),
                },
                gtk::CheckMenuItem {
                    label: "Time signature",
                    toggled(btn) => SearchMsg::AddSearchTerm(SearchTerm::TimeSign, btn.get_active()),
                },
            },

            gtk::ButtonBox {
                homogeneous: true,
                gtk::Button {
                    image: Some(&gtk::Image::from_icon_name(Some("system-search"), gtk::IconSize::Button)),
                    label: "Search",
                    always_show_image: true,
                },
                gtk::Button {
                    image: Some(&gtk::Image::from_icon_name(Some("process-stop"), gtk::IconSize::Button)),
                    label: "Reset",
                    always_show_image: true,
                },
                gtk::MenuButton {
                    image: Some(&gtk::Image::from_icon_name(Some("list-add"), gtk::IconSize::Button)),
                    label: "Add term",
                    always_show_image: true,
                    popup: Some(&search_terms_menu),
                },
            },

            #[name="search_terms_box"]
            gtk::FlowBox {
                //homogeneous: true,

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

                #[name="tempo_term_box"]
                gtk::Fixed {
                    property_width_request: 200,
                    gtk::Box(gtk::Orientation::Horizontal, 10) {
                        homogeneous: true,
                        gtk::Label {
                            halign: gtk::Align::End,
                            text: "BPM"
                        },
                        gtk::SpinButton {

                        },
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                    },
                },

                #[name="duration_term_box"]
                gtk::Fixed {
                    gtk::Box(gtk::Orientation::Horizontal, 10) {
                        property_width_request: 200,
                        homogeneous: true,
                        gtk::Label {
                            halign: gtk::Align::End,
                            text: "Duration"
                        },

                        gtk::Entry {
                        },
                        gtk::Entry {
                        },
                        gtk::Entry {
                        },
                    },
                },

                #[name="key_term_box"]
                gtk::Fixed {
                    property_width_request: 200,
                    gtk::Box(gtk::Orientation::Horizontal, 10) {
                        homogeneous: true,
                        gtk::Label {
                            halign: gtk::Align::End,
                            text: "Key"
                        },
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                    },
                },

                #[name="mode_term_box"]
                gtk::Fixed {
                    property_width_request: 200,
                    gtk::Box(gtk::Orientation::Horizontal, 10) {
                        property_width_request: 200,
                        homogeneous: true,
                        gtk::Label {
                            halign: gtk::Align::End,
                            text: "Mode"
                        },
                        gtk::CheckButton {
                           label: "Minor"
                        },
                        gtk::CheckButton {
                           label: "Major"
                        },
                    },
                },

                #[name="instrument_term_box"]
                gtk::Fixed {
                    property_width_request: 200,
                    gtk::Box(gtk::Orientation::Horizontal, 10) {
                        homogeneous: true,
                        gtk::Label {
                            halign: gtk::Align::End,
                            text: "Instrumental"
                        },
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                    },
                },

                #[name="speech_term_box"]
                gtk::Fixed {
                    property_width_request: 200,
                    gtk::Box(gtk::Orientation::Horizontal, 10) {
                        homogeneous: true,
                        gtk::Label {
                            halign: gtk::Align::End,
                            text: "Speech"
                        },
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                    },
                },

                #[name="acoustic_term_box"]
                gtk::Fixed {
                    property_width_request: 200,
                    gtk::Box(gtk::Orientation::Horizontal, 10) {
                        homogeneous: true,
                        gtk::Label {
                            halign: gtk::Align::End,
                            text: "Acoustic"
                        },
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                    },
                },

                #[name="dance_term_box"]
                gtk::Fixed {
                    property_width_request: 200,
                    gtk::Box(gtk::Orientation::Horizontal, 10) {
                        homogeneous: true,
                        gtk::Label {
                            halign: gtk::Align::End,
                            text: "Dance"
                        },
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                    },
                },

                #[name="energy_term_box"]
                gtk::Fixed {
                    property_width_request: 200,
                    gtk::Box(gtk::Orientation::Horizontal, 10) {
                        homogeneous: true,
                        gtk::Label {
                            halign: gtk::Align::End,
                            text: "Energy"
                        },
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                    },
                },

                #[name="liveness_term_box"]
                gtk::Fixed {
                    property_width_request: 200,
                    gtk::Box(gtk::Orientation::Horizontal, 10) {
                        homogeneous: true,
                        gtk::Label {
                            halign: gtk::Align::End,
                            text: "Liveness"
                        },
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                    },
                },

                #[name="valence_term_box"]
                gtk::Fixed {
                    property_width_request: 200,
                    gtk::Box(gtk::Orientation::Horizontal, 10) {
                        homogeneous: true,
                        gtk::Label {
                            halign: gtk::Align::End,
                            text: "Valence"
                        },
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                    },
                },

                #[name="loudness_term_box"]
                gtk::Fixed {
                    property_width_request: 200,
                    gtk::Box(gtk::Orientation::Horizontal, 10) {
                        homogeneous: true,
                        gtk::Label {
                            halign: gtk::Align::End,
                            text: "Loudness"
                        },
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                    },
                },

                #[name="popularity_term_box"]
                gtk::Fixed {
                    property_width_request: 200,
                    gtk::Box(gtk::Orientation::Horizontal, 10) {
                        homogeneous: true,
                        gtk::Label {
                            halign: gtk::Align::End,
                            text: "Popularity"
                        },
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                    },
                },

                #[name="time_sign_term_box"]
                gtk::Fixed {
                    property_width_request: 200,
                    gtk::Box(gtk::Orientation::Horizontal, 10) {
                        homogeneous: true,
                        gtk::Label {
                            halign: gtk::Align::End,
                            text: "Time sig."
                        },
                        gtk::SpinButton {

                        },
                        gtk::SpinButton {},
                        gtk::SpinButton {},
                    },
                },

            },
            //TrackList::<RecommendLoader>(self.model.spotify.clone())
        }
    }

    fn init_view(&mut self) {
        let search_terms = self.model.search_terms.clone();
        self.search_terms_box
            .set_filter_func(Some(Box::new(move |term_box| {
                let index = term_box.get_index();
                search_terms.borrow().is_set(index as u8)
            })));
    }
}
