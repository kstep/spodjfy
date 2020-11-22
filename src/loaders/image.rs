use crate::config::Config;
use cairo::{Format, ImageSurface};
use gdk::prelude::*;
use gdk_pixbuf::{InterpType, Pixbuf, PixbufLoader, PixbufLoaderExt};
use gio::prelude::*;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::io::{ErrorKind, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

type JobQueue = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

pub struct ImageLoader {
    cache_dir: PathBuf,
    queue: JobQueue,
    cache: HashMap<String, Pixbuf>,
    pub resize: i32,
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
            cache_dir: Config::new().thumb_cache_dir(),
        }
    }

    fn cache_file_path(&self, url: &str) -> Option<PathBuf> {
        let uuid = url.split('/').last()?;
        if uuid.contains(|ch| !matches!(ch, '0'..='9' | 'a'..='f')) {
            return None;
        }
        let dir_name = self.cache_dir.join(&uuid[0..1]).join(&uuid[1..3]);
        if !dir_name.exists() {
            std::fs::create_dir_all(&dir_name).ok()?;
        }

        Some(dir_name.join(&uuid))
    }

    fn save_to_file(&self, url: &str, data: &[u8]) -> Result<(), std::io::Error> {
        let mut cache_file = std::fs::File::create(
            self.cache_file_path(url)
                .ok_or_else(|| std::io::Error::from(ErrorKind::NotFound))?,
        )?;
        cache_file.write_all(data)?;
        cache_file.flush()?;
        Ok(())
    }

    fn load_from_file(&mut self, url: &str) -> Option<Pixbuf> {
        let mut cache_file = std::fs::File::open(self.cache_file_path(url)?).ok()?;
        let mut buf = Vec::with_capacity(1024);
        cache_file.read_to_end(&mut buf).ok()?;
        pixbuf_from_raw_bytes(&buf).ok()?
    }

    pub fn load_from_url<F>(&mut self, url: &str, callback: F)
    where
        F: FnOnce(Result<Option<Pixbuf>, glib::Error>) + Send + 'static,
    {
        self.process_queue();

        if let Some(pixbuf) = self.cache.get(url) {
            let pixbuf = if self.resize > 0 {
                resize_image(pixbuf, self.resize)
            } else {
                Some(pixbuf.clone())
            };
            return callback(Ok(pixbuf));
        }

        if let Some(pixbuf) = self.load_from_file(url) {
            self.cache.insert(url.to_owned(), pixbuf.clone());
            let pixbuf = if self.resize > 0 {
                resize_image(&pixbuf, self.resize)
            } else {
                Some(pixbuf)
            };
            return callback(Ok(pixbuf));
        }

        let queue = self.queue.clone();
        let key = url.to_owned();
        let resize = self.resize;
        pixbuf_from_url_async(url, move |reply| match reply {
            Ok((Some(pixbuf), data)) => {
                queue.lock().unwrap().push((key, data));
                let pixbuf = if resize > 0 {
                    resize_image(&pixbuf, resize)
                } else {
                    Some(pixbuf)
                };
                callback(Ok(pixbuf));
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
                let _ = self.save_to_file(&url, &data);
                if let Ok(Some(pb)) = pixbuf_from_raw_bytes(&data) {
                    self.cache.insert(url, pb);
                }
            }
        }
    }

    pub fn size(&self) -> i32 {
        self.resize
    }

    pub fn find_best_thumb<
        'b,
        'a: 'b,
        I: IntoIterator<Item = &'a rspotify::model::image::Image>,
    >(
        &self,
        images: I,
    ) -> Option<&'b str> {
        find_best_thumb(images, self.resize)
    }
}

fn pixbuf_from_raw_bytes(data: &[u8]) -> Result<Option<Pixbuf>, glib::Error> {
    let loader = PixbufLoader::new();
    loader.write(data)?;
    loader.close()?;
    Ok(loader.get_pixbuf())
}

pub fn pixbuf_from_url_async<F>(url: &str, callback: F) -> gio::Cancellable
where
    F: FnOnce(Result<(Option<Pixbuf>, Vec<u8>), glib::Error>) + Send + 'static,
{
    let image_file = gio::File::new_for_uri(url);
    let cancel = gio::Cancellable::new();
    image_file.load_contents_async(Some(&cancel), move |reply| match reply {
        Ok((data, _)) => {
            let result = pixbuf_from_raw_bytes(&data);
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

pub fn round_corners(pixbuf: &Pixbuf) -> Result<cairo::ImageSurface, cairo::Error> {
    let width = pixbuf.get_width();
    let height = pixbuf.get_height();

    let size = width.max(height);
    let surface = cairo::ImageSurface::create(Format::ARgb32, size, size)?;
    let context = cairo::Context::new(&surface);
    let radius = (size >> 1) as f64;
    context.arc(radius, radius, radius, 0.0, 2.0 * PI);
    context.clip();

    context.set_source_pixbuf(
        &pixbuf,
        radius - (width >> 1) as f64,
        radius - (height >> 1) as f64,
    );
    context.paint();

    Ok(surface)
}

fn resize_image(pixbuf: &Pixbuf, size: i32) -> Option<Pixbuf> {
    let width = pixbuf.get_width();
    let height = pixbuf.get_height();
    let (new_width, new_height) = if width > height {
        (size, height * size / width)
    } else {
        (width * size / height, size)
    };
    pixbuf.scale_simple(new_width, new_height, InterpType::Nearest)
}
