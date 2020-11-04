use crate::components::win::Settings;
use gtk::{self, ButtonExt, EntryExt, FrameExt, GridExt, LabelExt, WidgetExt};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::io::Write;
use std::sync::{Arc, RwLock};

#[derive(Msg)]
pub enum SettingsMsg {
    ShowTab,
    GetCode,
    Save,
}

#[widget]
impl Widget for SettingsTab {
    fn model(settings: Arc<RwLock<Settings>>) -> Arc<RwLock<Settings>> {
        settings
    }

    fn update(&mut self, event: SettingsMsg) {
        use SettingsMsg::*;
        match event {
            ShowTab => {
                let settings = self.model.read().unwrap();
                self.client_id_entry.set_text(&*settings.client_id);
                self.client_secret_entry.set_text(&*settings.client_secret);
            }
            GetCode => {}
            Save => {
                {
                    let mut settings = self.model.write().unwrap();
                    settings.client_id = self.client_id_entry.get_text().into();
                    settings.client_secret = self.client_secret_entry.get_text().into();
                }

                directories::ProjectDirs::from("me", "kstep", "spodjfy")
                    .and_then(|dirs| {
                        std::fs::File::create(dirs.config_dir().join("settings.toml")).ok()
                    })
                    .and_then(|mut conf_file| {
                        toml::to_vec(&*self.model.read().unwrap())
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
                    text: &*__relm_model.read().unwrap().client_id,
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
                    text: &*__relm_model.read().unwrap().client_secret,
                    cell: {
                        left_attach: 1,
                        top_attach: 1,
                    }
                },

                gtk::Button {
                    hexpand: false,
                    halign: gtk::Align::End,
                    label: "Save",
                    cell: {
                        left_attach: 1,
                        top_attach: 2,
                    },

                    clicked(_) => SettingsMsg::Save,
                }
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
