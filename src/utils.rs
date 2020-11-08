use gdk_pixbuf::{Pixbuf, PixbufLoader, PixbufLoaderExt};
use gio::prelude::*;
use glib::translate::{from_glib_full, ToGlib, ToGlibPtr};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct ImageLoader {
    queue: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
    cache: HashMap<String, Pixbuf>,
    resize: i32,
}

impl Default for ImageLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageLoader {
    pub fn new() -> Self {
        Self::new_with_resize(0)
    }
    pub fn new_with_resize(resize: i32) -> Self {
        Self {
            cache: HashMap::new(),
            queue: Arc::new(Mutex::new(Vec::new())),
            resize,
        }
    }

    pub fn load_from_url<F>(&mut self, url: &str, callback: F)
    where
        F: FnOnce(Result<Option<Pixbuf>, glib::Error>) + Send + 'static,
    {
        self.process_queue();

        if self.cache.contains_key(url) {
            return callback(Ok(self.cache.get(url).cloned()));
        }

        let queue = self.queue.clone();
        let key = url.to_owned();
        pixbuf_from_url_async(url, self.resize, move |reply| {
            match reply {
                Ok((Some(pixbuf), data)) => {
                    //cache.lock().unwrap().insert(url, pixbuf.clone());
                    queue.lock().unwrap().push((key, data));
                    callback(Ok(Some(pixbuf)));
                }
                Ok((None, _)) => {
                    callback(Ok(None));
                }
                Err(error) => {
                    callback(Err(error));
                }
            }
        });
    }

    fn process_queue(&mut self) {
        let mut queue = self.queue.lock().unwrap();
        if !queue.is_empty() {
            for (url, data) in queue.drain(..) {
                if let Some(pb) = pixbuf_from_raw_bytes(&data, self.resize) {
                    self.cache.insert(url, pb);
                }
            }
        }
    }
}

fn pixbuf_from_raw_bytes(data: &[u8], resize: i32) -> Option<Pixbuf> {
    let loader = PixbufLoader::new();
    if resize > 0 {
        loader.set_size(resize, resize);
    }
    loader.write(data);
    loader.close();
    loader.get_pixbuf()
}

pub fn pixbuf_from_url_async<F>(url: &str, resize: i32, callback: F) -> gio::Cancellable
where
    F: FnOnce(Result<(Option<Pixbuf>, Vec<u8>), glib::Error>) + Send + 'static,
{
    let image_file = gio::File::new_for_uri(url);
    let cancel = gio::Cancellable::new();
    image_file.load_contents_async(Some(&cancel), move |reply| match reply {
        Ok((data, _)) => callback(Ok((pixbuf_from_raw_bytes(&data, resize), data))),
        Err(err) => {
            callback(Err(err));
        }
    });
    cancel
}

pub fn pixbuf_from_url(url: &str, resize: i32) -> Result<Pixbuf, glib::Error> {
    let image_file = gio::File::new_for_uri(url);
    let cancel = gio::Cancellable::new();
    image_file.read(Some(&cancel)).and_then(|stream| {
        if resize > 0 {
            Pixbuf::from_stream_at_scale(&stream, resize, resize, true, Some(&cancel))
        } else {
            Pixbuf::from_stream(&stream, Some(&cancel))
        }
    })
}

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

pub fn find_best_thumb<'b, 'a: 'b, I: IntoIterator<Item = &'a rspotify::model::image::Image>>(
    images: I,
    size: i32,
) -> Option<&'b str> {
    images
        .into_iter()
        .min_by_key(|img| (size - img.width.unwrap_or(0) as i32).abs())
        .map(|img| &*img.url)
}
