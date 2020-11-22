use crate::components::lists::TrackList;
use crate::loaders::RecommendLoader;
use crate::servers::spotify::SpotifyProxy;
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

#[derive(Default, Debug, Clone, Copy)]
struct SearchTerms(i16);

#[derive(Clone, Copy)]
struct SearchTermsIter(i16, i16);

impl Iterator for SearchTermsIter {
    type Item = SearchTerm;

    fn next(&mut self) -> Option<Self::Item> {
        while self.1 != 16384 {
            let item = self.0 & self.1;
            self.1 <<= 1;

            if item != 0 {
                return Some(unsafe { std::mem::transmute(self.1 >> 1) });
            }
        }
        None
    }
}
impl IntoIterator for SearchTerms {
    type Item = SearchTerm;
    type IntoIter = SearchTermsIter;

    fn into_iter(self) -> Self::IntoIter {
        SearchTermsIter(self.0, 1)
    }
}

impl SearchTerms {
    #[inline]
    fn add(&mut self, term: SearchTerm) {
        let mask = term as i16;
        self.0 |= mask;
    }
    #[inline]
    fn remove(&mut self, term: SearchTerm) {
        let mask = term as i16;
        self.0 &= !mask;
    }
    #[inline]
    fn update(&mut self, term: SearchTerm, is_set: bool) {
        let mask = term as i16;
        self.0 ^= (-(is_set as i16) ^ self.0) & mask;
    }
    #[inline]
    fn contains(&self, term: SearchTerm) -> bool {
        let mask = term as i16;
        self.0 & mask != 0
    }

    #[inline(always)]
    fn is_set(&self, term: u8) -> bool {
        let mask = 1i16 << term;
        self.0 & mask != 0
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(i16)]
pub enum SearchTerm {
    Tempo = 1,
    Duration = 2,
    Key = 4,
    Mode = 8,
    Instrumental = 16,
    Speech = 32,
    Acoustic = 64,
    Dance = 128,
    Energy = 256,
    Liveness = 512,
    Valence = 1024,
    Loudness = 2048,
    Popularity = 4096,
    TimeSign = 8192,
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
