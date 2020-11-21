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
