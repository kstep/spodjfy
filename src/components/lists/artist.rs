use crate::components::lists::{ContainerList, ContainerMsg, GetSelectedRows, ItemsListView};
use crate::loaders::artist::*;
use crate::loaders::ContainerLoader;
use glib::{Cast, IsA};
use gtk::IconViewExt;
use relm::EventStream;

pub type ArtistList<Loader> = ContainerList<Loader, ArtistView>;

const THUMB_SIZE: i32 = 192;

pub struct ArtistView(gtk::IconView);

impl From<gtk::IconView> for ArtistView {
    fn from(view: gtk::IconView) -> Self {
        ArtistView(view)
    }
}

impl AsRef<gtk::Widget> for ArtistView {
    fn as_ref(&self) -> &gtk::Widget {
        self.0.upcast_ref()
    }
}

impl GetSelectedRows for ArtistView {
    fn get_selected_rows(&self) -> (Vec<gtk::TreePath>, gtk::TreeModel) {
        self.0.get_selected_rows()
    }
}

impl<Loader, Message> ItemsListView<Loader, Message> for ArtistView
where
    Loader: ContainerLoader,
    Message: 'static,
    ContainerMsg<Loader>: Into<Message>,
{
    #[allow(clippy::redundant_clone)]
    fn create<S: IsA<gtk::TreeModel>>(stream: EventStream<Message>, store: &S) -> Self {
        let artist_view = gtk::IconViewBuilder::new()
            .model(store)
            .expand(true)
            .reorderable(true)
            .text_column(COL_ARTIST_NAME as i32)
            .pixbuf_column(COL_ARTIST_THUMB as i32)
            .item_padding(10)
            .item_width(THUMB_SIZE)
            .build();

        artist_view.connect_item_activated(move |view, path| {
            if let Some((uri, name)) = view
                .get_model()
                .and_then(|model| crate::utils::extract_uri_name(&model, path))
            {
                stream.emit(ContainerMsg::ActivateItem(uri, name).into());
            }
        });

        ArtistView(artist_view)
    }

    fn thumb_size(&self) -> i32 {
        THUMB_SIZE
    }
}
