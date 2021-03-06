use glib::Continue;
use gtk::{BoxExt, ImageExt, InfoBarExt, LabelExt, WidgetExt};
use relm::{EventStream, Relm, Update, Widget};
use relm_derive::Msg;

#[derive(Msg)]
pub enum NotifierMsg {
    Notify {
        message: String,
        timeout_ms: u32,
        kind: gtk::MessageType,
    },
    Hide,
}

pub struct Notifier {
    infobar: gtk::InfoBar,
    icon: gtk::Image,
    message: gtk::Label,

    stream: EventStream<NotifierMsg>,
    timer_id: Option<glib::SourceId>,
}

impl Notifier {
    fn stop_timer(&mut self) -> bool { self.timer_id.take().map(glib::source_remove).is_some() }

    fn start_timer(&mut self, timeout_ms: u32) {
        let stream = self.stream.clone();

        self.timer_id = Some(glib::timeout_add_local(timeout_ms, move || {
            stream.emit(NotifierMsg::Hide);

            Continue(false)
        }));
    }

    fn show(&self, message: &str, message_type: gtk::MessageType) {
        self.icon.set_from_icon_name(
            match message_type {
                gtk::MessageType::Warning => Some("dialog-warning"),
                gtk::MessageType::Info => Some("dialog-information"),
                gtk::MessageType::Question => Some("dialog-information"),
                gtk::MessageType::Error => Some("dialog-error"),
                _ => None,
            },
            gtk::IconSize::SmallToolbar,
        );

        self.message.set_text(message);

        self.infobar.set_message_type(message_type);

        self.infobar.set_revealed(true);
    }

    fn hide(&self) { self.infobar.set_revealed(false); }
}

impl Update for Notifier {
    type Model = ();
    type ModelParam = ();
    type Msg = NotifierMsg;

    fn model(_relm: &Relm<Self>, _param: Self::ModelParam) -> Self::Model {}

    fn update(&mut self, msg: NotifierMsg) {
        use NotifierMsg::*;

        match msg {
            Notify {
                message,
                timeout_ms,
                kind,
            } => {
                self.stop_timer();

                self.show(&message, kind);

                if timeout_ms > 0 {
                    self.start_timer(timeout_ms);
                }
            }
            Hide => {
                self.stop_timer();

                self.hide();
            }
        }
    }
}

impl Widget for Notifier {
    type Root = gtk::InfoBar;

    fn root(&self) -> Self::Root { self.infobar.clone() }

    fn view(relm: &Relm<Self>, _model: ()) -> Self {
        let stream = relm.stream().clone();

        let infobar = gtk::InfoBarBuilder::new()
            .valign(gtk::Align::Start)
            .halign(gtk::Align::Fill)
            .show_close_button(true)
            .revealed(false)
            .message_type(gtk::MessageType::Warning)
            .build();

        {
            let stream = stream.clone();

            infobar.connect_response(move |_, response| {
                if response == gtk::ResponseType::Close {
                    stream.emit(NotifierMsg::Hide);
                }
            });
        }

        let message = gtk::LabelBuilder::new()
            .wrap(true)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Fill)
            .xalign(0.0)
            .build();

        let icon = gtk::ImageBuilder::new().margin_start(5).build();
        let infobox = infobar.get_content_area();

        infobox.pack_start(&icon, false, false, 0);
        infobox.pack_start(&message, false, false, 0);
        infobar.show_all();

        Notifier {
            stream,
            icon,
            message,
            infobar,
            timer_id: None,
        }
    }
}
