use crate::loaders::{
    ContainerLoader, HasDuration, HasImages, ImageLoader, PageLike, RowLike, COL_ITEM_THUMB,
};
use crate::servers::spotify::SpotifyProxy;
use glib::{IsA, ToValue, Type};
use gtk::prelude::GtkListStoreExtManual;
use gtk::{
    BoxExt, ButtonExt, ContainerExt, GtkListStoreExt, GtkMenuExt, IconViewExt, ProgressBarExt,
    StatusbarExt, TreeSelectionExt, TreeViewExt, WidgetExt,
};
use relm::vendor::fragile::Fragile;
use relm::{EventStream, Relm, Update, Widget};
use relm_derive::Msg;
use std::convert::TryInto;
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Msg)]
pub enum ContainerMsg<Loader: ContainerLoader> {
    Clear,
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
}

pub trait ItemsListView<Loader, Message> {
    fn create<Store: IsA<gtk::TreeModel>>(stream: EventStream<Message>, store: &Store) -> Self;
    fn context_menu(&self, _stream: EventStream<Message>) -> gtk::Menu {
        gtk::Menu::new()
    }
    fn thumb_size(&self) -> i32;
}

#[doc(hidden)]
pub struct ContainerModel<Loader> {
    pub spotify: Arc<SpotifyProxy>,
    pub store: gtk::ListStore,
    pub items_loader: Option<Loader>,
    pub image_loader: ImageLoader,
    pub total_items: u32,
    pub total_duration: u32,
    pub total_duration_exact: bool,
    pub is_loading: bool,
}

impl<Loader> ContainerModel<Loader> {
    pub fn from_row<R: RowLike>(spotify: Arc<SpotifyProxy>) -> Self {
        Self::new(spotify, &R::content_types())
    }
    pub fn new(spotify: Arc<SpotifyProxy>, column_types: &[Type]) -> Self {
        let store = gtk::ListStore::new(column_types);
        let image_loader = ImageLoader::new();

        Self {
            store,
            spotify,
            image_loader,
            items_loader: None,
            total_items: 0,
            total_duration: 0,
            total_duration_exact: true,
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

pub struct ContainerList<Loader, ItemsView, Handler = (), Message = ContainerMsg<Loader>> {
    pub root: gtk::Box,
    pub items_view: ItemsView,
    pub status_bar: gtk::Statusbar,
    pub stream: EventStream<Message>,
    pub model: ContainerModel<Loader>,
    pub progress_bar: gtk::ProgressBar,
    pub refresh_btn: gtk::Button,
    pub context_menu: gtk::Menu,
    handler: PhantomData<Handler>,
}

impl<Loader, ItemsView, Handler, Message> ContainerList<Loader, ItemsView, Handler, Message> {
    pub fn clear_store(&mut self) {
        self.model.store.clear();
        self.model.total_items = 0;
        self.model.total_duration = 0;
        self.model.total_duration_exact = true;

        let status_ctx = self.status_bar.get_context_id("totals");
        self.status_bar.remove_all(status_ctx);
    }
}

impl<Loader, ItemsView, Handler, Message> ContainerList<Loader, ItemsView, Handler, Message>
where
    Loader: ContainerLoader,
{
    pub fn current_epoch(&self) -> usize {
        self.model
            .items_loader
            .as_ref()
            .map_or(0, |ldr| ldr.epoch())
    }
}

impl<Loader, ItemsView, Handler, Message> ContainerList<Loader, ItemsView, Handler, Message>
where
    Loader: ContainerLoader,
    Loader::ParentId: Clone,
    ContainerMsg<Loader>: Into<Message>,
{
    pub fn start_load(&mut self) {
        if let Some(ref mut loader) = self.model.items_loader {
            self.model.is_loading = true;
            *loader = Loader::new(loader.parent_id().clone());
            let epoch = loader.epoch();
            self.refresh_btn.set_visible(false);
            self.progress_bar.set_fraction(0.0);
            self.progress_bar.set_visible(true);
            self.progress_bar.pulse();
            self.stream
                .emit(ContainerMsg::LoadPage(Loader::Page::init_offset(), epoch).into());
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
                "Total {}: {}, total duration: {}{}",
                Loader::NAME,
                self.model.total_items,
                crate::utils::humanize_time(self.model.total_duration),
                if self.model.total_duration_exact {
                    ""
                } else {
                    "+"
                }
            )
        } else {
            format!("Total {}: {}", Loader::NAME, self.model.total_items)
        };

        self.status_bar.push(status_ctx, &totals);
    }
}

pub trait MessageHandler<Component, Message> {
    fn handle(component: &mut Component, message: Message) -> Option<Message>;
}
impl<Component, Message> MessageHandler<Component, Message> for () {
    fn handle(_component: &mut Component, message: Message) -> Option<Message> {
        Some(message)
    }
}

impl<Loader, ItemsView, Handler, Message> Update
    for ContainerList<Loader, ItemsView, Handler, Message>
where
    Loader: ContainerLoader + Clone + 'static,
    Loader::Item: RowLike + HasImages + HasDuration,
    Loader::Page: PageLike<Loader::Item>,
    Loader::ParentId: Clone + PartialEq,
    ItemsView: GetSelectedRows,
    Message: TryInto<ContainerMsg<Loader>> + relm::DisplayVariant + 'static,
    ContainerMsg<Loader>: Into<Message>,
    Handler: MessageHandler<Self, Message>,
{
    type Model = ContainerModel<Loader>;
    type ModelParam = Arc<SpotifyProxy>;
    type Msg = Message;

    fn model(_relm: &Relm<Self>, spotify: Self::ModelParam) -> Self::Model {
        ContainerModel::from_row::<Loader::Item>(spotify)
    }

    fn update(&mut self, event: Self::Msg) {
        use ContainerMsg::*;
        let event = match Handler::handle(self, event) {
            Some(ev) => ev,
            None => return,
        };

        if let Ok(msg) = event.try_into() {
            match msg {
                Clear => {
                    self.clear_store();
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
                    if let Some(ref mut loader) = self.model.items_loader {
                        *loader = Loader::new(loader.parent_id().clone());
                        self.clear_store();
                        self.start_load();
                    }
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
                                self.stream.clone(),
                                move |tx| loader.load_page(tx, offset),
                                move |page| NewPage(page, epoch).into(),
                            )
                            .unwrap();
                    }
                }
                NewPage(page, epoch) => {
                    if epoch != self.current_epoch() {
                        return;
                    }

                    let stream = &self.stream;
                    let store = &self.model.store;
                    let items = page.items();

                    self.progress_bar.set_fraction(
                        (page.num_offset() as f64 + items.len() as f64) / page.total() as f64,
                    );

                    let mut page_duration = 0;
                    let mut page_duration_exact = true;
                    for item in items {
                        let pos = item.append_to_store(store);

                        page_duration += item.duration();
                        page_duration_exact = page_duration_exact && item.duration_exact();

                        let image = self.model.image_loader.find_best_thumb(item.images());

                        if let Some(url) = image {
                            stream.emit(LoadThumb(url.to_owned(), pos).into());
                        }
                    }

                    self.model.total_duration += page_duration;
                    if !page_duration_exact {
                        self.model.total_duration_exact = false;
                    }

                    if let Some(next_offset) = page.next_offset() {
                        stream.emit(LoadPage(next_offset, epoch).into());
                    } else {
                        self.model.total_items = page.total();
                        self.finish_load();
                    }
                }
                LoadThumb(url, pos) => {
                    let stream = Fragile::new(self.stream.clone());
                    let pos = Fragile::new(pos);
                    self.model.image_loader.load_from_url(&url, move |loaded| {
                        if let Ok(Some(pb)) = loaded {
                            stream
                                .into_inner()
                                .emit(NewThumb(pb, pos.into_inner()).into());
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
                        .and_then(|path| crate::utils::extract_uri_name(&model, path))
                    {
                        self.stream.emit(ActivateItem(uri, name).into());
                    }
                }
                ActivateItem(_, _) => {}
                ActivateItems(_) => {}
                OpenContextMenu(event) => {
                    self.context_menu.popup_at_pointer(Some(&event));
                }
            }
        }
    }
}

impl<Loader, ItemsView, Handler, Message> Widget
    for ContainerList<Loader, ItemsView, Handler, Message>
where
    Loader: ContainerLoader + Clone + 'static,
    Loader::Item: RowLike + HasImages + HasDuration,
    Loader::Page: PageLike<Loader::Item>,
    Loader::ParentId: Clone + PartialEq,
    ItemsView: GetSelectedRows + AsRef<gtk::Widget> + ItemsListView<Loader, Message>,
    Message: TryInto<ContainerMsg<Loader>> + relm::DisplayVariant + 'static,
    ContainerMsg<Loader>: Into<Message>,
    Handler: MessageHandler<Self, Message>,
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
        refresh_btn.connect_clicked(move |_| stream.emit(ContainerMsg::Reload.into()));
        status_bar.pack_start(&refresh_btn, false, false, 0);

        root.add(&status_bar);

        let context_menu = items_view.context_menu(relm.stream().clone());
        context_menu.show_all();

        root.add(&context_menu);

        root.show_all();

        let stream = relm.stream().clone();

        ContainerList {
            root,
            stream,
            items_view,
            status_bar,
            progress_bar,
            refresh_btn,
            context_menu,
            model,
            handler: PhantomData,
        }
    }
}
