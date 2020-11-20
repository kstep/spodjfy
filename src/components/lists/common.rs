use crate::loaders::common::{ContainerLoader, HasImages, MissingColumns, COL_ITEM_THUMB};
use crate::loaders::image::ImageLoader;
use crate::loaders::paged::{PageLike, RowLike};
use crate::servers::spotify::SpotifyProxy;
use glib::{IsA, ToValue, Type};
use gtk::prelude::GtkListStoreExtManual;
use gtk::{
    BoxExt, ButtonExt, ContainerExt, GtkListStoreExt, GtkMenuExt, IconViewExt, ProgressBarExt,
    StatusbarExt, TreeSelectionExt, TreeViewExt, WidgetExt,
};
use relm::vendor::fragile::Fragile;
use relm::{DisplayVariant, EventStream, IntoOption, Relm, Update, Widget};
use std::sync::Arc;

#[derive(Clone)]
pub enum ContainerListMsg<Loader: ContainerLoader, Other = ()> {
    Clear,
    Reset(Loader::ParentId, bool),
    Load(Loader::ParentId),
    Reload,
    LoadPage(<Loader::Page as PageLike<Loader::Item>>::Offset, usize),
    NewPage(Loader::Page, usize),
    LoadThumb(String, gtk::TreeIter),
    NewThumb(gdk_pixbuf::Pixbuf, gtk::TreeIter),
    ActivateChosenItems,
    ActivateItem(String, String),
    ActivateItems(Vec<String>),
    OpenContextMenu(gdk::EventButton),
    Custom(Other),
}

impl<Loader: ContainerLoader, Other> IntoOption<Self> for ContainerListMsg<Loader, Other> {
    fn into_option(self) -> Option<Self> {
        Some(self)
    }
}

impl<Loader: ContainerLoader, Other> DisplayVariant for ContainerListMsg<Loader, Other> {
    fn display_variant(&self) -> &'static str {
        use ContainerListMsg::*;
        match self {
            Clear => "Clear",
            Reset(_, _) => "Reset",
            Load(_) => "Load",
            Reload => "Reload",
            LoadPage(_, _) => "LoadPage",
            NewPage(_, _) => "NewPage",
            LoadThumb(_, _) => "LoadThumb",
            NewThumb(_, _) => "NewThumb",
            ActivateChosenItems => "ActivateChosenItems",
            ActivateItem(_, _) => "ActivateItem",
            ActivateItems(_) => "ActivateItems",
            OpenContextMenu(_) => "OpenContextMenu",
            Custom(_) => "Custom",
        }
    }
}

pub trait ItemsListView<Loader: ContainerLoader> {
    type CustomMsg: 'static;

    fn create<S: IsA<gtk::TreeModel>>(
        stream: EventStream<ContainerListMsg<Loader, Self::CustomMsg>>,
        store: &S,
    ) -> Self;
    fn context_menu(
        &self,
        _stream: EventStream<ContainerListMsg<Loader, Self::CustomMsg>>,
    ) -> gtk::Menu {
        gtk::Menu::new()
    }
    fn thumb_size(&self) -> i32;
}

#[doc(hidden)]
pub struct ContainerListModel<Loader: ContainerLoader, Message> {
    pub stream: EventStream<Message>,
    pub spotify: Arc<SpotifyProxy>,
    pub store: gtk::ListStore,
    pub items_loader: Option<Loader>,
    pub image_loader: ImageLoader,
    pub total_items: u32,
    pub total_duration: u32, // TODO
    pub is_loading: bool,
}

impl<Loader: ContainerLoader, Message> ContainerListModel<Loader, Message> {
    pub fn from_row<R: RowLike>(
        stream: EventStream<Message>,
        spotify: Arc<SpotifyProxy>,
    ) -> Self {
        Self::new(stream, spotify, &R::content_types())
    }
    pub fn new(
        stream: EventStream<Message>,
        spotify: Arc<SpotifyProxy>,
        column_types: &[Type],
    ) -> Self {
        let store = gtk::ListStore::new(column_types);
        let image_loader = ImageLoader::new();

        Self {
            stream,
            store,
            spotify,
            image_loader,
            items_loader: None,
            total_items: 0,
            total_duration: 0,
            is_loading: false,
        }
    }
}

pub trait GetSelectedRows {
    fn get_selected_rows(&self) -> (Vec<gtk::TreePath>, gtk::TreeModel);
}

impl GetSelectedRows for gtk::TreeView {
    fn get_selected_rows(&self) -> (Vec<gtk::TreePath>, gtk::TreeModel) {
        let select = self.get_selection();
        select.get_selected_rows()
    }
}

impl GetSelectedRows for gtk::IconView {
    fn get_selected_rows(&self) -> (Vec<gtk::TreePath>, gtk::TreeModel) {
        let items = self.get_selected_items();
        (items, self.get_model().unwrap())
    }
}

pub struct ContainerList<Loader: ContainerLoader, ItemsView: ItemsListView<Loader>> {
    pub root: gtk::Box,
    pub items_view: ItemsView,
    pub status_bar: gtk::Statusbar,
    pub model: ContainerListModel<Loader, ContainerListMsg<Loader, ItemsView::CustomMsg>>,
    pub progress_bar: gtk::ProgressBar,
    pub refresh_btn: gtk::Button,
    pub context_menu: gtk::Menu,
}

impl<Loader, ItemsView> ContainerList<Loader, ItemsView>
where
    Loader: ContainerLoader,
    Loader::Item: RowLike,
    ItemsView: ItemsListView<Loader>,
{
    pub fn clear_store(&mut self) {
        self.model.store.clear();
        self.model.total_items = 0;

        let status_ctx = self.status_bar.get_context_id("totals");
        self.status_bar.remove_all(status_ctx);
    }

    pub fn start_load(&mut self) {
        if let Some(ref mut loader) = self.model.items_loader {
            self.model.is_loading = true;
            *loader = Loader::new(loader.parent_id().clone());
            let epoch = loader.uuid();
            self.refresh_btn.set_visible(false);
            self.progress_bar.set_fraction(0.0);
            self.progress_bar.set_visible(true);
            self.progress_bar.pulse();
            self.model.stream.emit(ContainerListMsg::LoadPage(
                Loader::Page::init_offset(),
                epoch,
            ));
        }
    }

    pub fn finish_load(&mut self) {
        let status_ctx = self.status_bar.get_context_id("totals");
        self.model.is_loading = false;
        self.progress_bar.set_visible(false);
        self.refresh_btn.set_visible(true);
        self.status_bar.remove_all(status_ctx);

        let totals = if self.model.total_duration > 0 {
            format!(
                "Total items: {}, total duration: {}",
                self.model.total_items,
                crate::utils::humanize_time(self.model.total_duration)
            )
        } else {
            format!("Total items: {}", self.model.total_items)
        };

        self.status_bar.push(status_ctx, &totals);
    }

    pub fn current_epoch(&self) -> usize {
        self.model.items_loader.as_ref().map_or(0, |ldr| ldr.uuid())
    }
}

impl<Loader, ItemsView> Update for ContainerList<Loader, ItemsView>
where
    Loader: ContainerLoader,
    Loader::Item: RowLike + HasImages,
    Loader::ParentId: PartialEq,
    ItemsView: ItemsListView<Loader> + GetSelectedRows + AsRef<gtk::Widget>,
{
    type Model = ContainerListModel<Loader, Self::Msg>;
    type ModelParam = Arc<SpotifyProxy>;
    type Msg = ContainerListMsg<Loader, ItemsView::CustomMsg>;

    fn model(relm: &Relm<Self>, spotify: Self::ModelParam) -> Self::Model {
        ContainerListModel::from_row::<Loader::Item>(relm.stream().clone(), spotify)
    }

    fn update(&mut self, event: Self::Msg) {
        use crate::utils;
        use ContainerListMsg::*;
        match event {
            Clear => {
                self.clear_store();
            }
            Reset(artist_id, reload) => {
                self.model.items_loader = Some(Loader::new(artist_id));
                self.clear_store();
                if reload {
                    self.start_load();
                }
            }
            Load(parent_id) => {
                if self
                    .model
                    .items_loader
                    .as_ref()
                    .filter(|loader| loader.parent_id() == &parent_id)
                    .is_none()
                {
                    self.model.items_loader = Some(Loader::new(parent_id));
                    self.clear_store();
                    self.start_load()
                }
            }
            Reload => {
                self.clear_store();
                self.start_load();
            }
            LoadPage(offset, epoch) => {
                if epoch != self.current_epoch() {
                    return;
                }

                if let Some(ref loader) = self.model.items_loader {
                    let loader = loader.clone();
                    self.model
                        .spotify
                        .ask(
                            self.model.stream.clone(),
                            move |tx| loader.load_page(tx, offset),
                            move |page| NewPage(page, epoch),
                        )
                        .unwrap();
                }
            }
            NewPage(page, epoch) => {
                if epoch != self.current_epoch() {
                    return;
                }

                let stream = &self.model.stream;
                let store = &self.model.store;
                let items = page.items();

                self.progress_bar.set_fraction(
                    (page.num_offset() as f64 + items.len() as f64) / page.total() as f64,
                );

                for item in items {
                    let pos = item.append_to_store(store);

                    let image = self.model.image_loader.find_best_thumb(item.images());

                    if let Some(url) = image {
                        stream.emit(LoadThumb(url.to_owned(), pos));
                    }
                }

                if let Some(next_offset) = page.next_offset() {
                    stream.emit(LoadPage(next_offset, epoch));
                } else {
                    self.model.total_items = page.total();
                    self.finish_load();
                }
            }
            LoadThumb(url, pos) => {
                let stream = Fragile::new(self.model.stream.clone());
                let pos = Fragile::new(pos);
                self.model.image_loader.load_from_url(&url, move |loaded| {
                    if let Ok(Some(pb)) = loaded {
                        stream.into_inner().emit(NewThumb(pb, pos.into_inner()));
                    }
                });
            }
            NewThumb(thumb, pos) => {
                self.model
                    .store
                    .set_value(&pos, COL_ITEM_THUMB, &thumb.to_value());
            }
            ActivateChosenItems => {
                let (rows, model) = self.items_view.get_selected_rows();

                if let Some((uri, name)) = rows
                    .first()
                    .and_then(|path| utils::extract_uri_name(&model, path))
                {
                    self.model.stream.emit(ActivateItem(uri, name));
                }
            }
            ActivateItem(_, _) => {}
            ActivateItems(_) => {}
            OpenContextMenu(event) => {
                self.context_menu.popup_at_pointer(Some(&event));
            }
            Custom(_) => {}
        }
    }
}

impl<Loader, ItemsView> Widget for ContainerList<Loader, ItemsView>
where
    Loader: ContainerLoader,
    Loader::Item: RowLike + HasImages + MissingColumns,
    Loader::ParentId: PartialEq,
    ItemsView: AsRef<gtk::Widget> + ItemsListView<Loader> + GetSelectedRows,
{
    type Root = gtk::Box;

    fn root(&self) -> Self::Root {
        self.root.clone()
    }

    fn view(relm: &Relm<Self>, mut model: Self::Model) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let scroller = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);

        let items_view = ItemsView::create(relm.stream().clone(), &model.store);
        model.image_loader.resize = items_view.thumb_size();

        scroller.add(items_view.as_ref());

        root.add(&scroller);

        let status_bar = gtk::Statusbar::new();

        let progress_bar = gtk::ProgressBarBuilder::new()
            .valign(gtk::Align::Center)
            .width_request(200)
            .visible(false)
            .show_text(true)
            .build();
        status_bar.pack_end(&progress_bar, false, true, 0);

        let refresh_btn =
            gtk::Button::from_icon_name(Some("view-refresh"), gtk::IconSize::SmallToolbar);
        let stream = relm.stream().clone();
        refresh_btn.connect_clicked(move |_| stream.emit(ContainerListMsg::Reload));
        status_bar.pack_start(&refresh_btn, false, false, 0);

        root.add(&status_bar);

        let context_menu = items_view.context_menu(relm.stream().clone());
        context_menu.show_all();

        root.add(&context_menu);

        root.show_all();

        ContainerList {
            root,
            items_view,
            status_bar,
            progress_bar,
            refresh_btn,
            context_menu,
            model,
        }
    }
}
