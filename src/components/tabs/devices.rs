use crate::components::tabs::MusicTabParams;
use crate::servers::SpotifyRef;
use gdk_pixbuf::{InterpType, Pixbuf};
use glib::StaticType;
use gtk::prelude::*;
use gtk::{IconThemeExt, IconView, IconViewExt, TreeModelExt};
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};
use rspotify::model::{Device, DeviceType};
use tokio::runtime::Handle;

#[derive(Msg)]
pub enum DevicesMsg {
    ShowTab,
    LoadList,
    NewList(Vec<Device>),
    UseChosenDevice,
    UseDevice(Option<String>),
}

pub struct DevicesModel {
    pool: Handle,
    stream: EventStream<DevicesMsg>,
    spotify: SpotifyRef,
    store: gtk::ListStore,
}

const ICON_SIZE: i32 = 64;
const MINOR_ICON_SIZE: i32 = 16;

const COL_DEVICE_ICON: u32 = 0;
const COL_DEVICE_ID: u32 = 1;
const COL_DEVICE_NAME: u32 = 2;
const COL_DEVICE_ACTIVE: u32 = 3;
const COL_DEVICE_TYPE: u32 = 4;

#[widget]
impl Widget for DevicesTab {
    fn model(relm: &Relm<Self>, (pool, spotify): MusicTabParams) -> DevicesModel {
        let store = gtk::ListStore::new(&[
            gdk_pixbuf::Pixbuf::static_type(), // icon
            String::static_type(),             // id
            String::static_type(),             // name
            bool::static_type(),               // active
            u8::static_type(),                 // type
        ]);
        let stream = relm.stream().clone();
        DevicesModel {
            pool,
            stream,
            spotify,
            store,
        }
    }

    fn icon_theme(&self) -> gtk::IconTheme {
        gtk::IconTheme::new()
    }

    fn update(&mut self, event: DevicesMsg) {
        use DevicesMsg::*;
        match event {
            ShowTab => {
                self.model.store.clear();
                self.model.stream.emit(LoadList);
            }
            LoadList => {
                self.model
                    .spotify
                    .ask(
                        self.model.stream.clone(),
                        move |tx| SpotifyCmd::GetMyDevices { tx },
                        NewList,
                    )
                    .unwrap();
            }
            NewList(devices) => {
                let store = &self.model.store;
                let icon_theme = self.icon_theme();
                for device in devices {
                    let type_num = device._type.clone() as u8;
                    let icon = self.get_device_icon(&icon_theme, device._type, device.is_active);

                    store.insert_with_values(
                        None,
                        &[
                            COL_DEVICE_ICON,
                            COL_DEVICE_ID,
                            COL_DEVICE_NAME,
                            COL_DEVICE_ACTIVE,
                            COL_DEVICE_TYPE,
                        ],
                        &[
                            &icon,
                            &device.id,
                            &device.name,
                            &device.is_active,
                            &type_num,
                        ],
                    );
                }
            }
            UseChosenDevice => {
                let devices_view: &IconView = &self.devices_view;
                let selected = devices_view.get_selected_items();
                let store: &gtk::ListStore = &self.model.store;

                self.model.stream.emit(UseDevice(
                    selected
                        .first()
                        .and_then(|path| store.get_iter(path))
                        .and_then(|pos| {
                            store
                                .get_value(&pos, COL_DEVICE_ID as i32)
                                .get::<String>()
                                .ok()
                                .flatten()
                        }),
                ));
            }
            UseDevice(device_id) => {
                if let Some(id) = device_id {
                    let store: &gtk::ListStore = &self.model.store;
                    let icon_theme = self.icon_theme();

                    store.foreach(|_model, _path, pos| {
                        let device_type = Self::device_type_from_num(
                            store
                                .get_value(pos, COL_DEVICE_TYPE as i32)
                                .get::<u8>()
                                .unwrap()
                                .unwrap(),
                        );
                        let is_active = store
                            .get_value(pos, COL_DEVICE_ID as i32)
                            .get::<&str>()
                            .unwrap()
                            .unwrap()
                            == id;
                        let icon = self.get_device_icon(&icon_theme, device_type, is_active);
                        store.set_value(pos, COL_DEVICE_ICON, &icon.to_value());
                        false
                    });

                    self.model
                        .spotify
                        .tell(SpotifyCmd::UseDevice { id, play: false })
                        .unwrap();
                }
            }
        }
    }

    fn device_type_from_num(num: u8) -> DeviceType {
        match num {
            0 => DeviceType::Computer,
            1 => DeviceType::Tablet,
            2 => DeviceType::Smartphone,
            3 => DeviceType::Speaker,
            4 => DeviceType::TV,
            5 => DeviceType::AVR,
            6 => DeviceType::STB,
            7 => DeviceType::AudioDongle,
            8 => DeviceType::GameConsole,
            9 => DeviceType::CastVideo,
            10 => DeviceType::CastAudio,
            11 => DeviceType::Automobile,
            _ => DeviceType::Unknown,
        }
    }

    fn get_device_icon(
        &self,
        icon_theme: &gtk::IconTheme,
        tpe: DeviceType,
        is_active: bool,
    ) -> Pixbuf {
        use DeviceType::*;
        let icon_name = match tpe {
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

        let icon = icon_theme
            .load_icon(icon_name, ICON_SIZE, gtk::IconLookupFlags::GENERIC_FALLBACK)
            .unwrap()
            .unwrap();

        let checkmark = icon_theme
            .load_icon(
                if is_active {
                    "checkbox-checked-symbolic"
                } else {
                    "checkbox-symbolic"
                },
                MINOR_ICON_SIZE,
                gtk::IconLookupFlags::GENERIC_FALLBACK,
            )
            .unwrap()
            .unwrap();
        checkmark.composite(
            &icon,
            0,
            0,
            32,
            32,
            0.0,
            0.0,
            1.0,
            1.0,
            InterpType::Nearest,
            255,
        );
        icon
    }

    view! {
        gtk::ScrolledWindow {
            #[name="devices_view"]
            gtk::IconView {
                item_width: ICON_SIZE,
                pixbuf_column: COL_DEVICE_ICON as i32,
                text_column: COL_DEVICE_NAME as i32,
                model: Some(&self.model.store),
                selection_mode: gtk::SelectionMode::Single,

                item_activated(view, path) => DevicesMsg::UseDevice(
                    view.get_model().and_then(|model| {
                        model.get_iter(path).and_then(|pos| model.get_value(&pos, COL_DEVICE_ID as i32).get::<String>().ok().flatten())
                    })),
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
