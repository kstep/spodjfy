use crate::config::{Config, Settings, SettingsRef};
use crate::servers::SpotifyRef;
use gtk::{
    self, BoxExt, ButtonExt, EntryExt, FrameExt, GridExt, LabelExt, LinkButtonExt, SwitchExt,
    WidgetExt,
};
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use tokio::runtime::Handle;

#[derive(Msg)]
pub enum SettingsMsg {
    ShowTab,
    GetAuthorizeUrl,
    SetAuthorizeUrl(String),
    Reset,
    Save,
}

pub struct SettingsModel {
    pool: Handle,
    stream: EventStream<SettingsMsg>,
    settings: SettingsRef,
    spotify: SpotifyRef,
    config: Config,
}

#[widget]
impl Widget for SettingsTab {
    fn model(
        relm: &Relm<Self>,
        (pool, spotify, settings): (Handle, SpotifyRef, SettingsRef),
    ) -> SettingsModel {
        let stream = relm.stream().clone();
        let config = Config::new();
        SettingsModel {
            pool,
            stream,
            settings,
            spotify,
            config,
        }
    }

    fn update(&mut self, event: SettingsMsg) {
        use SettingsMsg::*;
        match event {
            ShowTab => {
                // Hacky method to make code generator create set_text() method calls
                self.model.settings = self.model.settings.clone();
                self.model.stream.emit(GetAuthorizeUrl);
            }
            GetAuthorizeUrl => {
                let spotify = &self.model.spotify;
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
            Reset => {
                // Hacky method to make code generator create set_text() method calls
                let settings = self.model.settings.clone();
                *settings.write().unwrap() = self.model.config.load_settings();
                self.model.settings = settings;
            }
            Save => {
                self.save_settings();
            }
        }
    }

    fn save_settings(&mut self) {
        let settings = Settings {
            client_id: self.client_id_entry.get_text().into(),
            client_secret: self.client_secret_entry.get_text().into(),
            show_notifications: self.show_notifications_switch.get_active(),
        };

        self.model
            .config
            .save_settings(&settings)
            .expect("error saving settings");

        let id = settings.client_id.clone();
        let secret = settings.client_secret.clone();
        self.model
            .spotify
            .ask(
                self.model.stream.clone(),
                |tx| SpotifyCmd::SetupClient { tx, id, secret },
                SettingsMsg::SetAuthorizeUrl,
            )
            .unwrap();

        *self.model.settings.write().unwrap() = settings;
    }

    view! {
        gtk::Box(gtk::Orientation::Vertical, 0) {
            gtk::Frame {
                label: Some("Credentials"),
                margin_top: 10, margin_bottom: 10, margin_start: 10, margin_end: 10,

                gtk::Grid {
                    margin_top: 10, margin_bottom: 10, margin_start: 10, margin_end: 10,
                    row_spacing: 5,
                    column_spacing: 10,

                    #[name="client_id_label"]
                    gtk::Label {
                        cell: { left_attach: 0, top_attach: 0, },
                        halign: gtk::Align::Start,
                        text_with_mnemonic: "Client _ID",
                    },
                    #[name="client_id_entry"]
                    gtk::Entry {
                        cell: { left_attach: 1, top_attach: 0, },
                        text: &self.model.settings.read().unwrap().client_id,
                        hexpand: true,
                    },

                    #[name="client_secret_label"]
                    gtk::Label {
                        cell: { left_attach: 0, top_attach: 1, },
                        halign: gtk::Align::Start,
                        text_with_mnemonic: "Client _Secret",
                    },
                    #[name="client_secret_entry"]
                    gtk::Entry {
                        cell: { left_attach: 1, top_attach: 1, },
                        text: &self.model.settings.read().unwrap().client_secret,
                    },

                    #[name="client_auth_url_btn"]
                    gtk::LinkButton {
                        cell: { left_attach: 1, top_attach: 2, width: 2, },
                        label: "Open authorization URL",
                        halign: gtk::Align::Start,
                    },
                },
            },
            gtk::Frame {
                label: Some("Playback"),
                margin_top: 10, margin_bottom: 10, margin_start: 10, margin_end: 10,

                gtk::Grid {
                    column_homogeneous: true,
                    margin_top: 10, margin_bottom: 10, margin_start: 10, margin_end: 10,
                    row_spacing: 5,
                    column_spacing: 10,
                    hexpand: true,

                    gtk::Label {
                        halign: gtk::Align::Start,
                        cell: { left_attach: 0, top_attach: 0, },
                        text: "Show track notifications",
                    },
                    #[name="show_notifications_switch"]
                    gtk::Switch {
                        cell: { left_attach: 1, top_attach: 0, },
                        active: self.model.settings.read().unwrap().show_notifications,
                        halign: gtk::Align::End,
                    },
                },
            },

            gtk::ButtonBox {
                spacing: 10,
                margin_end: 10,
                halign: gtk::Align::End,
                homogeneous: true,

                gtk::Button {
                    label: "Reset",
                    clicked(_) => SettingsMsg::Reset,
                },
                gtk::Button {
                    label: "Save",
                    clicked(_) => SettingsMsg::Save,
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
