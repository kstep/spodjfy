use crate::config::Config;
use cairo::Format;
use gdk::prelude::*;
use gdk_pixbuf::{Colorspace, InterpType, Pixbuf, PixbufLoader, PixbufLoaderExt};
use rspotify::model::Image;
use std::f64::consts::PI;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::stream::StreamExt;
use tokio::sync::RwLock;
use ttl_cache::TtlCache;

#[derive(Clone, Debug)]
pub struct ImageData {
    data: Box<[u8]>,
    colorspace: Colorspace,
    has_alpha: bool,
    bits_per_sample: i32,
    width: i32,
    height: i32,
    row_stride: i32,
}

impl From<Pixbuf> for ImageData {
    fn from(pixbuf: Pixbuf) -> Self {
        Self {
            data: unsafe { Box::from(pixbuf.get_pixels() as &[u8]) },
            colorspace: pixbuf.get_colorspace(),
            has_alpha: pixbuf.get_has_alpha(),
            bits_per_sample: pixbuf.get_bits_per_sample(),
            width: pixbuf.get_width(),
            height: pixbuf.get_height(),
            row_stride: pixbuf.get_rowstride(),
        }
    }
}

impl From<ImageData> for Pixbuf {
    fn from(image: ImageData) -> Self {
        Pixbuf::from_mut_slice(
            image.data,
            image.colorspace,
            image.has_alpha,
            image.bits_per_sample,
            image.width,
            image.height,
            image.row_stride,
        )
    }
}

#[derive(Clone)]
pub struct ImageCache {
    cache: Arc<RwLock<TtlCache<String, ImageData>>>,
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

    pub fn convert(&self, image: ImageData) -> ImageData {
        let pixbuf: Pixbuf = image.into();
        let pixbuf = if self.resize > 0 {
            pixbuf.resize_cutup(self.resize).unwrap_or(pixbuf)
        } else {
            pixbuf
        };
        let pixbuf = if self.round {
            pixbuf
                .rounded()
                .ok()
                .and_then(|img| img.to_pixbuf())
                .unwrap_or(pixbuf)
        } else {
            pixbuf
        };
        pixbuf.into()
    }
}

impl ImageCache {
    pub fn with_converter(converter: ImageConverter) -> Self {
        ImageCache {
            cache: Arc::new(RwLock::new(TtlCache::new(4096))),
            converter,
        }
    }

    pub async fn put(&mut self, url: String, image: ImageData) -> ImageData {
        let image: ImageData = self.converter.convert(image.into()).into();
        self.cache
            .write()
            .await
            .insert(url, image.clone(), Duration::from_secs(600));
        image
    }

    pub async fn get(&self, url: &str) -> Option<ImageData> {
        debug!("trying to get {} from cache", url);
        self.cache.read().await.get(url).cloned()
    }

    pub async fn clear(&self) {
        self.cache.write().await.clear();
    }
}

#[derive(Clone)]
pub struct ImageLoader {
    cache_dir: PathBuf,
    cache: ImageCache,
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

    async fn load_from_file(&mut self, url: &str) -> Option<ImageData> {
        let mut cache_file = File::open(self.cache_file_path(url)?).await.ok()?;
        let mut buf = Vec::with_capacity(1024);
        let size = cache_file.read_to_end(&mut buf).await.ok()?;

        let loader = PixbufLoader::new();
        loader.write(&buf[..size]).ok()?;
        loader.close().ok()?;
        loader.get_pixbuf().map(Into::into)
    }

    async fn save_from_url(&mut self, url: &str) -> Option<ImageData> {
        let mut image_reply = reqwest::get(url).await.ok()?.bytes_stream();

        let file_name = self.cache_file_path(url)?;
        let mut cache_file = File::create(&file_name).await.ok()?;
        while let Some(chunk) = image_reply.next().await {
            let chunk = chunk.ok()?;
            cache_file.write(&chunk).await.ok()?;
        }
        drop(cache_file);

        Pixbuf::from_file(&file_name).ok().map(Into::into)
    }

    pub async fn load_from_url(&mut self, url: &str) -> Option<ImageData> {
        debug!("loading image from {}", url);
        if let Some(image) = self.cache.get(url).await {
            debug!("loaded from cache");
            return Some(image);
        } else if let Some(image) = self.load_from_file(url).await {
            debug!("loaded from file");
            return Some(self.cache.put(url.to_owned(), image).await);
        } else if let Some(image) = self.save_from_url(url).await {
            debug!("loaded from internet");
            Some(self.cache.put(url.to_owned(), image).await)
        } else {
            debug!("image not found");
            None
        }
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
