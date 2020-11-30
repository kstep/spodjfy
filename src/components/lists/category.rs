use crate::components::lists::{ContainerList, ContainerMsg, GetSelectedRows, ItemsListView};
use crate::loaders::{ContainerLoader, ImageConverter};
use crate::models::category::*;
use glib::{Cast, IsA};
use gtk::IconViewExt;
use relm::EventStream;

pub type CategoryList<Loader> = ContainerList<Loader, CategoryView>;

const THUMB_SIZE: i32 = 192;

pub struct CategoryView(gtk::IconView);

impl From<gtk::IconView> for CategoryView {
    fn from(view: gtk::IconView) -> Self {
        CategoryView(view)
    }
}

impl AsRef<gtk::Widget> for CategoryView {
    fn as_ref(&self) -> &gtk::Widget {
        self.0.upcast_ref()
    }
}

impl GetSelectedRows for CategoryView {
    fn get_selected_rows(&self) -> (Vec<gtk::TreePath>, gtk::TreeModel) {
        self.0.get_selected_rows()
    }
}

impl<Loader, Message> ItemsListView<Loader, Message> for CategoryView
where
    Loader: ContainerLoader,
    Message: 'static,
    ContainerMsg<Loader>: Into<Message>,
{
    fn create<S: IsA<gtk::TreeModel>>(stream: EventStream<Message>, store: &S) -> Self {
        let categories_view = gtk::IconViewBuilder::new()
            .model(store)
            .expand(true)
            .reorderable(true)
            .text_column(COL_CATEGORY_NAME as i32)
            .pixbuf_column(COL_CATEGORY_ICON as i32)
            .item_padding(10)
            .item_width(THUMB_SIZE)
            .build();

        categories_view.connect_item_activated(move |view, path| {
            if let Some((uri, name)) = view
                .get_model()
                .and_then(|model| crate::utils::extract_uri_name(&model, path))
            {
                stream.emit(ContainerMsg::ActivateItem(uri, name).into());
            }
        });

        CategoryView(categories_view)
    }

    fn thumb_converter(&self) -> ImageConverter {
        ImageConverter::new(THUMB_SIZE, true)
    }
}
