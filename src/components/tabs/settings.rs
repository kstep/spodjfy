use crate::components::win::Settings;
use crate::spotify::{SpotifyCmd, SpotifyProxy};
use gtk::{self, ButtonExt, EntryExt, FrameExt, GridExt, LabelExt, WidgetExt};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::io::Write;
use std::sync::Arc;

#[derive(Msg)]
pub enum SettingsMsg {
    ShowTab,
    Authorize,
    Save,
}

pub struct SettingsModel {
    settings: Settings,
    spotify: Arc<SpotifyProxy>,
}

#[widget]
impl Widget for SettingsTab {
    fn model((settings, spotify): (Settings, Arc<SpotifyProxy>)) -> SettingsModel {
        SettingsModel { settings, spotify }
    }

    fn update(&mut self, event: SettingsMsg) {
        use SettingsMsg::*;
        match event {
            ShowTab => {}
            Authorize => {
                let spotify: &SpotifyProxy = &self.model.spotify;
                spotify.tell(SpotifyCmd::OpenAuthorizeUrl);
                /*
                if let Some(code) = SpotifyProxy::get_code_url_from_user() {
                    spotify.tell(SpotifyCmd::AuthorizeUser { code });
                }
                 */
            }
            Save => {
                self.model.spotify.tell(SpotifyCmd::SetupClient {
                    id: self.model.settings.client_id.clone(),
                    secret: self.model.settings.client_secret.clone(),
                });

                directories::ProjectDirs::from("me", "kstep", "spodjfy")
                    .and_then(|dirs| {
                        std::fs::File::create(dirs.config_dir().join("settings.toml")).ok()
                    })
                    .and_then(|mut conf_file| {
                        toml::to_vec(&self.model.settings)
                            .ok()
                            .and_then(|data| conf_file.write_all(&data).ok())
                    })
                    .expect("Error saving settings");
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

                gtk::Box(gtk::Orientation::Horizontal, 5) {
                    halign: gtk::Align::End,
                    cell: {
                        left_attach: 1,
                        top_attach: 2,
                    },

                    gtk::Button {
                        hexpand: false,
                        label: "Save",

                        clicked(_) => SettingsMsg::Save,
                    },
                    gtk::Button {
                        hexpand: false,
                        label: "Authorize",

                        clicked(_) => SettingsMsg::Authorize,
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
