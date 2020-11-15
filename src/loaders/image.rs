use gdk_pixbuf::{Pixbuf, PixbufLoader, PixbufLoaderExt};
use gio::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type JobQueue = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

pub struct ImageLoader {
    queue: JobQueue,
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

        if let Some(pixbuf) = self.cache.get(url) {
            return callback(Ok(Some(pixbuf.clone())));
        }

        let queue = self.queue.clone();
        let key = url.to_owned();
        pixbuf_from_url_async(url, self.resize, move |reply| match reply {
            Ok((Some(pixbuf), data)) => {
                queue.lock().unwrap().push((key, data));
                callback(Ok(Some(pixbuf)));
            }
            Ok((None, _)) => {
                callback(Ok(None));
            }
            Err(error) => {
                callback(Err(error));
            }
        });
    }

    fn process_queue(&mut self) {
        let mut queue = self.queue.lock().unwrap();
        if !queue.is_empty() {
            for (url, data) in queue.drain(..) {
                if let Ok(Some(pb)) = pixbuf_from_raw_bytes(&data, self.resize) {
                    self.cache.insert(url, pb);
                }
            }
        }
    }

    pub fn size(&self) -> i32 {
        self.resize
    }
}

fn pixbuf_from_raw_bytes(data: &[u8], resize: i32) -> Result<Option<Pixbuf>, glib::Error> {
    let loader = PixbufLoader::new();
    if resize > 0 {
        loader.set_size(resize, resize);
    }
    loader.write(data)?;
    loader.close()?;
    Ok(loader.get_pixbuf())
}

pub fn pixbuf_from_url_async<F>(url: &str, resize: i32, callback: F) -> gio::Cancellable
where
    F: FnOnce(Result<(Option<Pixbuf>, Vec<u8>), glib::Error>) + Send + 'static,
{
    let image_file = gio::File::new_for_uri(url);
    let cancel = gio::Cancellable::new();
    image_file.load_contents_async(Some(&cancel), move |reply| match reply {
        Ok((data, _)) => {
            let result = pixbuf_from_raw_bytes(&data, resize);
            callback(result.map(|pixbuf| (pixbuf, data)));
        }
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

pub fn find_best_thumb<'b, 'a: 'b, I: IntoIterator<Item = &'a rspotify::model::image::Image>>(
    images: I,
    size: i32,
) -> Option<&'b str> {
    images
        .into_iter()
        .min_by_key(|img| (size - img.width.unwrap_or(0) as i32).abs())
        .map(|img| &*img.url)
}
