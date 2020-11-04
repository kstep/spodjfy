use crate::components::spotify::{SpotifyCmd, SpotifyProxy};
use gdk_pixbuf::Pixbuf;
use glib::StaticType;
use gtk::prelude::*;
use gtk::{IconThemeExt, IconViewExt};
use relm::{Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::device::Device;
use rspotify::senum::DeviceType::*;
use std::sync::Arc;

#[derive(Msg)]
pub enum DevicesMsg {
    ShowTab,
    LoadList,
    NewList(Vec<Device>),
}

pub struct DevicesModel {
    relm: Relm<DevicesTab>,
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
        DevicesModel {
            relm: relm.clone(),
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
                self.model.relm.stream().emit(LoadList)
            }
            LoadList => {
                self.model.spotify.ask(
                    self.model.relm.stream().clone(),
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
            }
        }
    }
}
