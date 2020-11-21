use crate::loaders::{COL_ITEM_NAME, COL_ITEM_URI};
use gtk::TreeModelExt;

pub fn humanize_time(time_ms: u32) -> String {
    let seconds = time_ms / 1000;
    let (minutes, seconds) = (seconds / 60, seconds % 60);
    let (hours, minutes) = (minutes / 60, minutes % 60);
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{}:{:02}", minutes, seconds)
    }
}

pub fn humanize_inexact_time(time_ms: u32) -> String {
    let seconds = time_ms / 1000;

    match seconds {
        0 => format!("less than a second"),
        1 => "1 second".to_owned(),
        2..=59 => format!("{} seconds", seconds),
        60 => "1 minute".to_owned(),
        61..=3599 => format!("{} minutes", seconds / 60),
        3600 => "1 hour".to_owned(),
        _ => format!("{} hours", seconds / 3600),
    }
}

pub fn extract_uri_name(model: &gtk::TreeModel, path: &gtk::TreePath) -> Option<(String, String)> {
    model.get_iter(path).and_then(|pos| {
        model
            .get_value(&pos, COL_ITEM_URI as i32)
            .get::<String>()
            .ok()
            .flatten()
            .zip(
                model
                    .get_value(&pos, COL_ITEM_NAME as i32)
                    .get::<String>()
                    .ok()
                    .flatten(),
            )
    })
}
