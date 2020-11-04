use crate::components::spotify::{SpotifyCmd, SpotifyProxy};
use glib::StaticType;
use gtk::prelude::*;
use gtk::{IconThemeExt, IconView, IconViewExt, TreeModelExt};
use relm::{Relm, Widget, EventStream};
use relm_derive::{widget, Msg};
use rspotify::model::device::Device;
use rspotify::senum::DeviceType::*;
use std::sync::Arc;

#[derive(Msg)]
pub enum DevicesMsg {
    ShowTab,
    LoadList,
    NewList(Vec<Device>),
    UseChosenDevice,
    Click(gdk::EventButton),
}

pub struct DevicesModel {
    stream: EventStream<DevicesMsg>,
    spotify: Arc<SpotifyProxy>,
    store: gtk::ListStore,
    icon_theme: gtk::IconTheme,
}

const COL_DEVICE_THUMB: u32 = 0;
const COL_DEVICE_ID: u32 = 1;
const COL_DEVICE_NAME: u32 = 2;
const COL_DEVICE_ACTIVE: u32 = 3;

#[widget]
impl Widget for DevicesTab {
    fn model(relm: &Relm<Self>, spotify: Arc<SpotifyProxy>) -> DevicesModel {
        let store = gtk::ListStore::new(&[
            gdk_pixbuf::Pixbuf::static_type(), // icon
            String::static_type(),             // id
            String::static_type(),             // name
            bool::static_type(),               // active
        ]);
        let icon_theme = gtk::IconTheme::get_default().unwrap_or_else(gtk::IconTheme::new);
        let stream = relm.stream().clone();
        DevicesModel {
            stream,
            spotify,
            store,
            icon_theme,
        }
    }

    fn update(&mut self, event: DevicesMsg) {
        use DevicesMsg::*;
        match event {
            ShowTab => {
                self.model.store.clear();
                self.model.stream.emit(LoadList);
            }
            LoadList => {
                self.model.spotify.ask(
                    self.model.stream.clone(),
                    move |tx| SpotifyCmd::GetDevices { tx },
                    NewList,
                );
            }
            NewList(devices) => {
                let store = &self.model.store;
                for device in devices {
                    let thumb_name = match device._type {
                        Tablet => "computer-apple-ipad-symbolic",
                        Smartphone => "phone-apple-iphone-symbolic",
                        Speaker => "audio-speakers-symbolic",
                        TV => "tv-symbolic",
                        AudioDongle => "bluetooth-symbolic",
                        GameConsole => "application-games-symbolic",
                        //CastVideo => "",
                        //CastAudio => "",
                        //Automobile => "",
                        Computer => "computer-symbolic",
                        _ => "audio-card-symbolic",
                    };
                    let icon_theme: &gtk::IconTheme = &self.model.icon_theme;
                    let thumb = icon_theme
                        .load_icon(thumb_name, 256, gtk::IconLookupFlags::NO_SVG)
                        .unwrap()
                        .unwrap();

                    store.insert_with_values(
                        None,
                        &[
                            COL_DEVICE_THUMB,
                            COL_DEVICE_ID,
                            COL_DEVICE_NAME,
                            COL_DEVICE_ACTIVE,
                        ],
                        &[&thumb, &device.id, &device.name, &device.is_active],
                    );
                }
            }
            UseChosenDevice => {
                let devices_view: &IconView = &self.devices_view;
                let selected = devices_view.get_selected_items();
                let store: &gtk::ListStore = &self.model.store;

                if let Some(id) = selected
                    .first()
                    .and_then(|path| store.get_iter(path))
                    .and_then(|pos| {
                        store
                            .get_value(&pos, COL_DEVICE_ID as i32)
                            .get::<String>()
                            .ok()
                            .flatten()
                    })
                {
                    self.model.spotify.tell(SpotifyCmd::UseDevice { id });
                }
            }
            Click(event) if event.get_button() == 3 => {
                self.context_menu.popup_at_pointer(Some(&event));
            }
            Click(event) if event.get_event_type() == gdk::EventType::DoubleButtonPress => {
                self.model.stream.emit(UseChosenDevice);
            }
            Click(_) => {}
        }
    }

    view! {
        gtk::ScrolledWindow {
            #[name="devices_view"]
            /*
            gtk::TreeView {
                model: Some(&__relm_model.store)),
            }
             */
            gtk::IconView {
                pixbuf_column: COL_DEVICE_THUMB as i32,
                text_column: COL_DEVICE_NAME as i32,
                model: Some(&__relm_model.store),
                selection_mode: gtk::SelectionMode::Single,

                button_press_event(_, event) => (DevicesMsg::Click(event.clone()), Inhibit(false))
            },

            #[name="context_menu"]
            gtk::Menu {
                gtk::MenuItem {
                    label: "Play on this device",
                    activate(_) => DevicesMsg::UseChosenDevice,
                },
            }
        }
    }
}
