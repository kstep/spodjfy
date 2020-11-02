use gio::FileExt;

pub fn pixbuf_from_url(url: &str) -> Result<gdk_pixbuf::Pixbuf, glib::Error> {
    let image_file = gio::File::new_for_uri(url);
    image_file
        .read(None::<&gio::Cancellable>)
        .and_then(|stream| gdk_pixbuf::Pixbuf::from_stream(&stream, None::<&gio::Cancellable>))
}
