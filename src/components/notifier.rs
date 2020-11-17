use glib::Continue;
use gtk::{ContainerExt, InfoBarExt, LabelExt, WidgetExt};
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
    message: gtk::Label,

    stream: EventStream<NotifierMsg>,
    timer_id: Option<glib::SourceId>,
}

impl Notifier {
    fn stop_timer(&mut self) -> bool {
        self.timer_id.take().map(glib::source_remove).is_some()
    }

    fn start_timer(&mut self, timeout_ms: u32) {
        let stream = self.stream.clone();
        self.timer_id = Some(glib::timeout_add_local(timeout_ms, move || {
            stream.emit(NotifierMsg::Hide);
            Continue(false)
        }));
    }

    fn show(&self, message: &str, message_type: gtk::MessageType) {
        self.message.set_text(message);
        self.infobar.set_message_type(message_type);
        self.infobar.set_revealed(true);
    }

    fn hide(&self) {
        self.infobar.set_revealed(false);
    }
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

    fn root(&self) -> Self::Root {
        self.infobar.clone()
    }

    fn view(relm: &Relm<Self>, _model: ()) -> Self {
        let stream = relm.stream().clone();

        let infobar = gtk::InfoBarBuilder::new()
            .valign(gtk::Align::Start)
            .halign(gtk::Align::Fill)
            .spacing(10)
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

        let message = gtk::Label::new(None);
        infobar.get_content_area().add(&message);

        infobar.show_all();

        Notifier {
            stream,
            message,
            infobar,
            timer_id: None,
        }
    }
}
