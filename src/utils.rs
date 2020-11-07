use gdk_pixbuf::Pixbuf;
use gio::FileExt;
use glib::translate::{from_glib_full, ToGlib, ToGlibPtr};
use std::collections::hash_map::Entry;
use std::collections::HashMap;

pub struct ImageLoader {
    cache: HashMap<String, Pixbuf>,
    resize: i32,
}

impl ImageLoader {
    pub fn new() -> Self {
        Self::new_with_resize(0)
    }
    pub fn new_with_resize(resize: i32) -> Self {
        Self {
            cache: HashMap::new(),
            resize,
        }
    }

    pub fn load_from_url<F>(&mut self, url: String, callback: F)
    where
        F: FnOnce(Result<Pixbuf, glib::Error>) + Send + 'static,
    {
        match self.cache.entry(url) {
            Entry::Occupied(entry) => callback(Ok(entry.get().clone())),
            Entry::Vacant(entry) => {
                match pixbuf_from_url(&*entry.key(), self.resize) {
                    Ok(image) => {
                        entry.insert(image.clone());
                        callback(Ok(image));
                    }
                    err @ Err(_) => callback(err),
                }
                //pixbuf_from_url_async(&*entry.key(), resize, callback);
            }
        }
    }
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

pub fn pixbuf_from_url_async<F>(url: &str, resize: i32, callback: F) -> gio::Cancellable
where
    F: FnOnce(Result<Pixbuf, glib::Error>) + Send + 'static,
{
    let image_file = gio::File::new_for_uri(url);
    let cancel = gio::Cancellable::new();

    debug!("loading url {} with file {:?}", url, image_file);
    match image_file.read(Some(&cancel)) {
        Err(err) => callback(Err(err)),
        Ok(stream) => {
            pixbuf_from_stream_async(stream, resize, &cancel, callback);
        }
    }

    cancel
}

fn pixbuf_from_stream_async<S, F>(stream: S, resize: i32, cancel: &gio::Cancellable, callback: F)
where
    F: FnOnce(Result<Pixbuf, glib::Error>) + Send + 'static,
    S: glib::IsA<gio::InputStream>,
{
    unsafe extern "C" fn connect_async_trampoline<
        R: FnOnce(Result<Pixbuf, glib::Error>) + Send + 'static,
    >(
        _source: *mut gobject_sys::GObject,
        reply: *mut gio_sys::GAsyncResult,
        user_data: glib_sys::gpointer,
    ) {
        debug!("loaded pixbuf: {:?}", reply);
        let mut error = std::ptr::null_mut();
        let pixbuf = gdk_pixbuf_sys::gdk_pixbuf_new_from_stream_finish(reply, &mut error);
        let result: Result<Pixbuf, glib::Error> = if error.is_null() {
            Ok(from_glib_full(pixbuf))
        } else {
            Err(from_glib_full(error))
        };
        debug!("result: {:?}", result);
        let callback: Box<R> = Box::from_raw(user_data as *mut _);
        callback(result);
    }

    debug!("ready to call async pixbuf load for stream {:?}", stream);
    let user_data: Box<F> = Box::new(callback);
    let callback = connect_async_trampoline::<F>;

    unsafe {
        if resize > 0 {
            debug!("calling gdk_pixbuf_new_from_stream_at_scale_async(...)");
            gdk_pixbuf_sys::gdk_pixbuf_new_from_stream_at_scale_async(
                stream.as_ref().to_glib_none().0,
                resize,
                resize,
                true.to_glib(),
                cancel.to_glib_none().0,
                Some(callback),
                Box::into_raw(user_data) as *mut _,
            )
        } else {
            debug!("calling gdk_pixbuf_new_from_stream_async(...)");
            gdk_pixbuf_sys::gdk_pixbuf_new_from_stream_async(
                stream.as_ref().to_glib_none().0,
                cancel.to_glib_none().0,
                Some(callback),
                Box::into_raw(user_data) as *mut _,
            )
        }
    }
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
