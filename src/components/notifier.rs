use glib::Continue;
use gtk::{Inhibit, LabelExt, RevealerExt, StyleContextExt, WidgetExt};
use relm::{EventStream, Relm, Widget};
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum NotifierMsg {
    Notify {
        title: String,
        body: String,
        timeout_ms: u32,
    },
    Hide,
}

pub struct NotifierModel {
    stream: EventStream<NotifierMsg>,
    timer_id: Option<glib::SourceId>,
}

#[widget]
impl Widget for Notifier {
    fn model(relm: &Relm<Self>, _params: ()) -> NotifierModel {
        NotifierModel {
            stream: relm.stream().clone(),
            timer_id: None,
        }
    }

    fn update(&mut self, msg: NotifierMsg) {
        use NotifierMsg::*;
        match msg {
            Notify {
                title,
                body,
                timeout_ms,
            } => {
                self.stop_timer();
                self.title.set_text(&title);
                self.body.set_text(&body);
                self.revealer.set_reveal_child(true);

                if timeout_ms > 0 {
                    let stream = self.model.stream.clone();
                    self.model.timer_id = Some(glib::timeout_add_local(timeout_ms, move || {
                        stream.emit(NotifierMsg::Hide);
                        Continue(false)
                    }));
                }
            }
            Hide => {
                self.revealer.set_reveal_child(false);
                self.stop_timer();
            }
        }
    }

    fn stop_timer(&mut self) -> bool {
        if let Some(timer_id) = self.model.timer_id.take() {
            glib::source_remove(timer_id);
            true
        } else {
            false
        }
    }

    view! {
        #[name="revealer"]
        gtk::Revealer {
            halign: gtk::Align::Center,
            valign: gtk::Align::Start,
            can_focus: false,

            gtk::EventBox {
                #[name="container"]
                gtk::Box(gtk::Orientation::Vertical, 2) {
                    #[name="title"]
                    gtk::Label {},
                    #[name="body"]
                    gtk::Label {},
                },
                button_press_event(_, _) => (NotifierMsg::Hide, Inhibit(true)),
            }
        }
    }

    fn init_view(&mut self) {
        let style = self.container.get_style_context();
        style.add_class("app-notification");
    }
}
