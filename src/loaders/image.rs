use crate::config::Config;
use cairo::Format;
use gdk::prelude::*;
use gdk_pixbuf::{InterpType, Pixbuf, PixbufLoader, PixbufLoaderExt};
use gio::prelude::*;
use gio::NONE_CANCELLABLE;
use rspotify::model::Image;
use std::f64::consts::PI;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use ttl_cache::TtlCache;

#[derive(Clone)]
pub struct ImageCache {
    cache: Arc<RwLock<TtlCache<String, Pixbuf>>>,
    converter: ImageConverter,
}

unsafe impl Send for ImageCache {}

#[derive(Clone, Copy)]
pub struct ImageConverter {
    resize: i32,
    round: bool,
}

impl ImageConverter {
    pub fn new(resize: i32, round: bool) -> Self {
        Self { resize, round }
    }

    pub fn convert(&self, pixbuf: Pixbuf) -> Pixbuf {
        let pixbuf = if self.resize > 0 {
            pixbuf.resize_cutup(self.resize).unwrap_or(pixbuf)
        } else {
            pixbuf
        };
        if self.round {
            pixbuf
                .rounded()
                .ok()
                .and_then(|img| img.to_pixbuf())
                .unwrap_or(pixbuf)
        } else {
            pixbuf
        }
    }
}

impl ImageCache {
    pub fn with_converter(converter: ImageConverter) -> Self {
        ImageCache {
            cache: Arc::new(RwLock::new(TtlCache::new(4096))),
            converter,
        }
    }

    pub fn put(&mut self, url: String, pixbuf: Pixbuf) -> Pixbuf {
        let pixbuf = self.converter.convert(pixbuf);
        self.cache
            .write()
            .unwrap()
            .insert(url, pixbuf.clone(), Duration::from_secs(600));
        pixbuf
    }

    pub fn get(&self, url: &str) -> Option<Pixbuf> {
        self.cache.read().unwrap().get(url).cloned()
    }
}

pub struct ImageLoader {
    cache_dir: PathBuf,
    cache: ImageCache,
}

impl Default for ImageLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageLoader {
    pub fn new() -> Self {
        Self::with_resize(0, false)
    }

    pub fn with_resize(resize: i32, round: bool) -> Self {
        Self::with_converter(ImageConverter::new(resize, round))
    }

    pub fn with_converter(converter: ImageConverter) -> Self {
        Self {
            cache: ImageCache::with_converter(converter),
            cache_dir: Config::new().thumb_cache_dir(),
        }
    }

    pub fn set_converter(&mut self, converter: ImageConverter) {
        self.cache.converter = converter;
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
        if let Some(pixbuf) = self.cache.get(url) {
            return callback(Ok(Some(pixbuf)));
        }

        if let Some(pixbuf) = self.load_from_file(url) {
            let pixbuf = self.cache.put(url.to_owned(), pixbuf);
            return callback(Ok(Some(pixbuf)));
        }

        let cache_file = self.cache_file_path(url);
        let image_file = gio::File::new_for_uri(url);
        let key = url.to_owned();
        let mut cache = self.cache.clone();
        image_file.load_contents_async(NONE_CANCELLABLE, move |reply| match reply {
            Ok((data, _)) => match pixbuf_from_raw_bytes(&data) {
                Ok(Some(pixbuf)) => {
                    if let Some(Ok(mut cache_file)) = cache_file.map(std::fs::File::create) {
                        let _ = cache_file.write_all(&data);
                    }

                    let pixbuf = cache.put(key, pixbuf);
                    callback(Ok(Some(pixbuf)));
                }
                other => {
                    callback(other);
                }
            },
            Err(error) => {
                callback(Err(error));
            }
        });
    }

    pub fn size(&self) -> i32 {
        self.cache.converter.resize
    }

    pub fn find_best_thumb<
        'b,
        'a: 'b,
        I: IntoIterator<Item = &'a rspotify::model::image::Image>,
    >(
        &self,
        images: I,
    ) -> Option<&'b str> {
        let size = self.size();

        if size == 0 {
            return images.into_iter().next().map(|img| &*img.url);
        }

        let key = |img: &&Image| match img.height.unwrap_or(0).max(img.width.unwrap_or(0)) as i32 {
            0 => i32::MAX,
            dim if dim > size => dim / size,
            dim => size / dim + 1,
        };

        images.into_iter().min_by_key(key).map(|img| &*img.url)
    }
}

fn pixbuf_from_raw_bytes(data: &[u8]) -> Result<Option<Pixbuf>, glib::Error> {
    let loader = PixbufLoader::new();
    loader.write(data)?;
    loader.close()?;
    Ok(loader.get_pixbuf())
}

pub trait PixbufConvert {
    fn rounded(&self) -> Result<cairo::ImageSurface, cairo::Error>;
    fn resize(&self, size: i32) -> Option<Pixbuf>;
    fn resize_cutup(&self, size: i32) -> Option<Pixbuf>;
}

impl PixbufConvert for Pixbuf {
    fn rounded(&self) -> Result<cairo::ImageSurface, cairo::Error> {
        let width = self.get_width();
        let height = self.get_height();

        let size = width.max(height);
        let surface = cairo::ImageSurface::create(Format::ARgb32, size, size)?;
        let context = cairo::Context::new(&surface);
        let radius = (size >> 1) as f64;
        context.arc(radius, radius, radius, 0.0, 2.0 * PI);
        context.clip();

        context.set_source_pixbuf(
            self,
            radius - (width >> 1) as f64,
            radius - (height >> 1) as f64,
        );
        context.paint();

        Ok(surface)
    }

    fn resize(&self, size: i32) -> Option<Pixbuf> {
        let width = self.get_width();
        let height = self.get_height();
        let (new_width, new_height) = if width > height {
            (size, height * size / width)
        } else {
            (width * size / height, size)
        };
        self.scale_simple(new_width, new_height, InterpType::Nearest)
    }

    fn resize_cutup(&self, size: i32) -> Option<Pixbuf> {
        let width = self.get_width();
        let height = self.get_height();
        if width == height {
            return self.scale_simple(size, size, InterpType::Nearest);
        }

        let (new_width, new_height) = if width < height {
            (size, height * size / width)
        } else {
            (width * size / height, size)
        };

        let new_pixbuf = Pixbuf::new(
            self.get_colorspace(),
            self.get_has_alpha(),
            self.get_bits_per_sample(),
            size,
            size,
        )?;

        let mid_x = new_width / 2;
        let mid_y = new_height / 2;
        let half_size = size / 2;

        self.scale(
            &new_pixbuf,
            0,
            0,
            size,
            size,
            (mid_x - half_size) as f64,
            (mid_y - half_size) as f64,
            new_width as f64 / width as f64,
            new_height as f64 / height as f64,
            InterpType::Nearest,
        );

        Some(new_pixbuf)
    }
}

pub trait CairoSurfaceToPixbuf {
    fn to_pixbuf(&self) -> Option<Pixbuf>;
}

impl CairoSurfaceToPixbuf for cairo::ImageSurface {
    fn to_pixbuf(&self) -> Option<Pixbuf> {
        let mut data = Vec::new();
        self.write_to_png(&mut data).ok()?;
        let loader = PixbufLoader::with_type("png").ok()?;
        loader.write(&data).ok()?;
        loader.close().ok()?;
        loader.get_pixbuf()
    }
}
