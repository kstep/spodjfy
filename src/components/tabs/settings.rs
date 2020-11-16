use crate::config::{Config, Settings};
use crate::servers::spotify::{SpotifyCmd, SpotifyProxy};
use gtk::{
    self, BoxExt, ButtonExt, EntryExt, FrameExt, GridExt, LabelExt, LinkButtonExt, WidgetExt,
};
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use std::sync::Arc;

#[derive(Msg)]
pub enum SettingsMsg {
    ShowTab,
    GetAuthorizeUrl,
    SetAuthorizeUrl(String),
    Save,
}

pub struct SettingsModel {
    stream: EventStream<SettingsMsg>,
    settings: Settings,
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for SettingsTab {
    fn model(
        relm: &Relm<Self>,
        (settings, spotify): (Settings, Arc<SpotifyProxy>),
    ) -> SettingsModel {
        let stream = relm.stream().clone();
        SettingsModel {
            stream,
            settings,
            spotify,
        }
    }

    fn update(&mut self, event: SettingsMsg) {
        use SettingsMsg::*;
        match event {
            ShowTab => {
                self.model.stream.emit(GetAuthorizeUrl);
            }
            GetAuthorizeUrl => {
                let spotify: &SpotifyProxy = &self.model.spotify;
                spotify
                    .ask(
                        self.model.stream.clone(),
                        |tx| SpotifyCmd::GetAuthorizeUrl { tx },
                        SetAuthorizeUrl,
                    )
                    .unwrap();
            }
            SetAuthorizeUrl(url) => {
                self.client_auth_url_btn.set_uri(&url);
                self.client_auth_url_btn.set_visible(true);
            }
            Save => {
                let id = self.model.settings.client_id.clone();
                let secret = self.model.settings.client_secret.clone();
                self.model
                    .spotify
                    .ask(
                        self.model.stream.clone(),
                        |tx| SpotifyCmd::SetupClient { tx, id, secret },
                        SetAuthorizeUrl,
                    )
                    .unwrap();

                Config::new()
                    .save_settings(&self.model.settings)
                    .expect("error saving settings");
            }
        }
    }

    view! {
        gtk::Frame {
            label: Some("Credentials"),
            gtk::Grid {
                column_homogeneous: true,
                margin_top: 50,
                margin_bottom: 50,
                margin_start: 50,
                margin_end: 50,
                row_spacing: 5,
                column_spacing: 5,

                #[name="client_id_label"]
                gtk::Label {
                    text_with_mnemonic: "Client _ID",
                    halign: gtk::Align::End,
                    cell: {
                        left_attach: 0,
                        top_attach: 0,
                    }
                },
                #[name="client_id_entry"]
                gtk::Entry {
                    text: &self.model.settings.client_id,
                    cell: {
                        left_attach: 1,
                        top_attach: 0,
                    }
                },

                #[name="client_secret_label"]
                gtk::Label {
                    text_with_mnemonic: "Client _Secret",
                    halign: gtk::Align::End,
                    cell: {
                        left_attach: 0,
                        top_attach: 1,
                    }
                },
                #[name="client_secret_entry"]
                gtk::Entry {
                    text: &self.model.settings.client_secret,
                    cell: {
                        left_attach: 1,
                        top_attach: 1,
                    }
                },


                gtk::ButtonBox(gtk::Orientation::Horizontal) {
                    spacing: 5,
                    halign: gtk::Align::End,
                    cell: {
                        left_attach: 1,
                        top_attach: 3,
                    },

                    #[name="client_auth_url_btn"]
                    gtk::LinkButton {
                        visible: false,
                        label: "Open authorization URL",
                        halign: gtk::Align::Start,
                    },

                    gtk::Button {
                        label: "Save",
                        clicked(_) => SettingsMsg::Save,
                    },
                },
            },
        }
    }

    fn init_view(&mut self) {
        self.client_id_label
            .set_mnemonic_widget(Some(&self.client_id_entry));
        self.client_secret_label
            .set_mnemonic_widget(Some(&self.client_secret_entry));
    }
}
