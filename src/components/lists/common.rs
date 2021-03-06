use crate::{
    broadcast,
    loaders::{ContainerLoader, ImageConverter, ImageLoader},
    models::{common::*, PageLike},
    services::SpotifyRef,
    utils::{Extract, RetryPolicy, Spawn, SpawnError},
    AppEvent,
};
use gdk_pixbuf::Pixbuf;
use glib::{bitflags::_core::time::Duration, Cast, IsA, MainContext, ToValue, Type};
use gtk::{
    prelude::GtkListStoreExtManual, BoxExt, ButtonExt, ContainerExt, EditableSignals, EntryExt, GtkListStoreExt, GtkMenuExt,
    IconViewExt, Inhibit, ProgressBarExt, StatusbarExt, TreeModelExt, TreeModelFilterExt, TreeSelectionExt, TreeViewExt,
    WidgetExt,
};
use relm::{EventStream, Relm, Update, Widget};
use relm_derive::Msg;
use rspotify::client::ClientError;
use std::{convert::TryInto, fmt::Debug, marker::PhantomData};
use tokio::runtime::Handle;

#[derive(Msg)]
pub enum ContainerMsg<Loader: ContainerLoader> {
    Clear,
    Load(Loader::ParentId),
    Reload,
    LoadPage(<Loader::Page as PageLike<Loader::Item>>::Offset, usize),
    NewPage(Loader::Page, usize),
    LoadThumb(String, gtk::TreeIter),
    ActivateChosenItems,
    ActivateItem(String, String),
    ActivateItems(Vec<String>),
    OpenContextMenu(gdk::EventButton),
    StartSearch,
    FinishSearch,
}

pub trait ItemsListView<Loader, Message> {
    fn create<Store: IsA<gtk::TreeModel>>(stream: EventStream<Message>, store: &Store) -> Self;
    fn context_menu(&self, _stream: EventStream<Message>) -> gtk::Menu { gtk::Menu::new() }
    fn setup_search(&self, _entry: &gtk::Entry) -> bool { false }
    fn thumb_converter(&self) -> ImageConverter;
}

#[doc(hidden)]

pub struct ContainerModel<Loader> {
    pub pool: Handle,
    pub spotify: SpotifyRef,
    pub context: MainContext,
    pub store: gtk::ListStore,
    pub items_loader: Option<Loader>,
    pub image_loader: ImageLoader,
    pub total_items: u32,
    pub total_duration: u32,
    pub total_duration_exact: bool,
    pub is_loading: bool,
}

impl<Loader> ContainerModel<Loader> {
    pub fn from_row<R: RowLike>(pool: Handle, spotify: SpotifyRef) -> Self { Self::new(pool, spotify, &R::content_types()) }

    pub fn new(pool: Handle, spotify: SpotifyRef, column_types: &[Type]) -> Self {
        let store = gtk::ListStore::new(column_types);
        let image_loader = ImageLoader::new();
        let context = MainContext::ref_thread_default();

        Self {
            pool,
            store,
            spotify,
            context,
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

pub struct ContainerList<Loader, ItemsView, Handler = NoopHandler, Message = ContainerMsg<Loader>> {
    pub root: gtk::Box,
    pub items_view: ItemsView,
    pub status_bar: gtk::Statusbar,
    pub stream: EventStream<Message>,
    pub model: ContainerModel<Loader>,
    pub progress_bar: gtk::ProgressBar,
    pub refresh_btn: gtk::Button,
    pub search_entry: gtk::Entry,
    pub search_btn: gtk::Button,
    pub context_menu: gtk::Menu,
    handler: PhantomData<Handler>,
}

impl<L, V, H, M: 'static> Extract<SpotifyRef> for ContainerList<L, V, H, M> {
    fn extract(&self) -> SpotifyRef { self.model.spotify.clone() }
}

impl<L, V, H, M: 'static> Extract<EventStream<M>> for ContainerList<L, V, H, M> {
    fn extract(&self) -> EventStream<M> { self.stream.clone() }
}

impl<L, V, H, M: 'static> Extract<gtk::ListStore> for ContainerList<L, V, H, M> {
    fn extract(&self) -> gtk::ListStore { self.model.store.clone() }
}

impl<Loader: Clone + 'static, V, H, M: 'static> Extract<Option<Loader>> for ContainerList<Loader, V, H, M> {
    fn extract(&self) -> Option<Loader> { self.model.items_loader.clone() }
}

impl<L, V, H, M: 'static> Extract<ImageLoader> for ContainerList<L, V, H, M> {
    fn extract(&self) -> ImageLoader { self.model.image_loader.clone() }
}

impl<L, V, H, M: 'static> Spawn for ContainerList<L, V, H, M> {
    fn pool(&self) -> Handle { self.model.pool.clone() }

    fn retry_policy(error: SpawnError, retry_count: usize) -> RetryPolicy<SpawnError> {
        match error {
            SpawnError::Spotify(ClientError::InvalidAuth(msg)) => {
                let _ = broadcast(AppEvent::SpotifyAuthError(msg));
                RetryPolicy::WaitRetry(Duration::from_secs(30))
            }
            SpawnError::Spotify(ClientError::RateLimited(timeout)) if retry_count < 10 => {
                RetryPolicy::WaitRetry(Duration::from_secs(timeout.unwrap_or(4) as u64 + 1))
            }
            SpawnError::Spotify(ClientError::RateLimited(_)) => {
                error!("aborting after {} retries...", retry_count);
                let _ = broadcast(AppEvent::SpotifyError(format!(
                    "Rate limit exceeded, requests stopped after {} retries",
                    retry_count
                )));
                RetryPolicy::ForwardError(error)
            }
            SpawnError::Spotify(error) => {
                error!("spotify error: {}", error);
                let _ = broadcast(AppEvent::SpotifyError(error.to_string()));
                RetryPolicy::ForwardError(SpawnError::Spotify(error))
            }
            error => RetryPolicy::ForwardError(error),
        }
    }
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
    pub fn current_epoch(&self) -> usize { self.model.items_loader.as_ref().map_or(0, |ldr| ldr.epoch()) }
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

            self.search_btn.set_visible(false);
            self.search_entry.set_text("");
            self.search_entry.set_visible(false);
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
        self.search_btn.set_visible(true);
        self.search_entry.set_visible(false);
        self.status_bar.remove_all(status_ctx);

        let totals = if self.model.total_duration > 0 {
            format!(
                "Total {}: {}, total duration: {}{}",
                Loader::NAME,
                self.model.total_items,
                crate::utils::humanize_time(self.model.total_duration),
                if self.model.total_duration_exact { "" } else { "+" }
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

pub struct NoopHandler;

impl<Component, Message> MessageHandler<Component, Message> for NoopHandler {
    fn handle(_component: &mut Component, message: Message) -> Option<Message> { Some(message) }
}

impl<Loader, ItemsView, Handler, Message> Update for ContainerList<Loader, ItemsView, Handler, Message>
where
    Loader: ContainerLoader + Clone + Send + 'static,
    Loader::Item: RowLike + HasImages + HasDuration,
    Loader::Page: PageLike<Loader::Item> + Send,
    <Loader::Page as PageLike<Loader::Item>>::Offset: Clone + Send,
    Loader::ParentId: Clone + PartialEq,
    ItemsView: GetSelectedRows,
    Message: TryInto<ContainerMsg<Loader>> + relm::DisplayVariant + 'static,
    <Message as TryInto<ContainerMsg<Loader>>>::Error: Debug,
    ContainerMsg<Loader>: Into<Message>,
    Handler: MessageHandler<Self, Message>,
{
    type Model = ContainerModel<Loader>;
    type ModelParam = (Handle, SpotifyRef);
    type Msg = Message;

    fn model(_relm: &Relm<Self>, (pool, spotify): Self::ModelParam) -> Self::Model {
        ContainerModel::from_row::<Loader::Item>(pool, spotify)
    }

    fn update(&mut self, event: Self::Msg) {
        use ContainerMsg::*;

        let event = match Handler::handle(self, event) {
            Some(ev) => ev,
            None => return,
        };

        match event.try_into() {
            Ok(msg) => match msg {
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
                    self.clear_store();
                    self.start_load();
                }
                LoadPage(offset, epoch) => {
                    if epoch != self.current_epoch() {
                        return;
                    }

                    type Inject<M, L> = (EventStream<M>, SpotifyRef, Option<L>);
                    self.spawn_args(
                        offset,
                        async move |pool, (stream, spotify, loader): Inject<_, Loader>, offset| {
                            if let Some(loader) = loader {
                                stream.emit(NewPage(pool.spawn(loader.load_page(spotify, offset.clone())).await??, epoch).into());
                            }

                            Ok(())
                        },
                    );
                }
                NewPage(page, epoch) => {
                    if epoch != self.current_epoch() {
                        return;
                    }

                    let stream = &self.stream;
                    let store = &self.model.store;
                    let items = page.items();

                    self.progress_bar
                        .set_fraction((page.num_offset() as f64 + items.len() as f64) / page.total() as f64);

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
                    self.spawn_args(
                        (url, pos),
                        async move |pool, (image_loader, store): (ImageLoader, gtk::ListStore), (url, pos)| {
                            if let Some(image) = pool.spawn(async move { image_loader.load_image(&url).await }).await? {
                                store.set_value(&pos, COL_ITEM_THUMB, &Pixbuf::from(image).to_value());
                            }
                            Ok(())
                        },
                    );
                }
                ActivateChosenItems => {
                    let (rows, model) = self.items_view.get_selected_rows();

                    if let Some((uri, name)) = rows.first().and_then(|path| crate::utils::extract_uri_name(&model, path)) {
                        self.stream.emit(ActivateItem(uri, name).into());
                    }
                }
                ActivateItem(..) => {}
                ActivateItems(_) => {}
                OpenContextMenu(event) => {
                    self.context_menu.popup_at_pointer(Some(&event));
                }
                StartSearch => {
                    self.search_btn.set_visible(false);
                    self.search_entry.set_text("");
                    self.search_entry.set_visible(true);
                    self.search_entry.grab_focus();
                }
                FinishSearch => {
                    self.search_entry.set_text("");
                    self.search_entry.set_visible(false);
                    self.search_btn.set_visible(true);
                }
            },
            Err(error) => {
                error!("unhandled container list event: {:?}", error);
            }
        }
    }
}

impl<Loader, ItemsView, Handler, Message> Widget for ContainerList<Loader, ItemsView, Handler, Message>
where
    Loader: ContainerLoader + Clone + Send + 'static,
    Loader::Item: RowLike + HasImages + HasDuration,
    Loader::Page: PageLike<Loader::Item> + Send,
    <Loader::Page as PageLike<Loader::Item>>::Offset: Clone + Send,
    Loader::ParentId: Clone + PartialEq,
    ItemsView: GetSelectedRows + AsRef<gtk::Widget> + ItemsListView<Loader, Message>,
    Message: TryInto<ContainerMsg<Loader>> + relm::DisplayVariant + 'static,
    <Message as TryInto<ContainerMsg<Loader>>>::Error: Debug,
    ContainerMsg<Loader>: Into<Message>,
    Handler: MessageHandler<Self, Message>,
{
    type Root = gtk::Box;

    fn root(&self) -> Self::Root { self.root.clone() }

    #[allow(non_upper_case_globals)]

    fn view(relm: &Relm<Self>, mut model: Self::Model) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let scroller = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        let items_view = ItemsView::create(relm.stream().clone(), &model.store);

        model.image_loader.set_converter(items_view.thumb_converter());

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

        let search_entry = gtk::Entry::new();
        let stream = relm.stream().clone();
        search_entry.connect_focus_out_event(move |_, _| {
            stream.emit(ContainerMsg::FinishSearch.into());
            Inhibit(false)
        });

        let stream = relm.stream().clone();
        search_entry.connect_key_press_event(move |_, event| {
            use gdk::{keys::constants::*, EventType::*};
            if let (KeyPress, Escape) = (event.get_event_type(), event.get_keyval()) {
                stream.emit(ContainerMsg::FinishSearch.into());
            }
            Inhibit(false)
        });

        let search_btn = gtk::Button::from_icon_name(Some("system-search"), gtk::IconSize::SmallToolbar);
        search_btn.set_tooltip_text(Some("Search list"));

        let stream = relm.stream().clone();
        search_btn.connect_clicked(move |_| {
            stream.emit(ContainerMsg::StartSearch.into());
        });

        if items_view.setup_search(&search_entry) {
            status_bar.pack_start(&search_entry, false, false, 0);
            status_bar.pack_start(&search_btn, false, false, 0);
            search_entry.hide();
        }

        let refresh_btn = gtk::Button::from_icon_name(Some("view-refresh"), gtk::IconSize::SmallToolbar);
        refresh_btn.set_tooltip_text(Some("Reload list"));

        let stream = relm.stream().clone();
        refresh_btn.connect_clicked(move |_| stream.emit(ContainerMsg::Reload.into()));
        status_bar.pack_start(&refresh_btn, false, false, 0);

        root.add(&status_bar);

        let context_menu = items_view.context_menu(relm.stream().clone());
        context_menu.show_all();

        //root.add(&context_menu);

        root.show_all();

        let stream = relm.stream().clone();

        ContainerList {
            root,
            stream,
            items_view,
            status_bar,
            progress_bar,
            search_btn,
            search_entry,
            refresh_btn,
            context_menu,
            model,
            handler: PhantomData,
        }
    }
}

pub trait SetupViewSearch {
    fn setup_search(&self, column: u32, entry: Option<&gtk::Entry>) -> Option<()>;

    fn wrap_filter<T: IsA<gtk::Entry> + IsA<gtk::Editable>>(model: &gtk::TreeModel, column: u32, entry: &T) -> gtk::TreeModel {
        let buffer = entry.get_buffer();
        let filter = gtk::TreeModelFilter::new(model, None);

        filter.set_visible_func(move |model, pos| {
            let needle = buffer.get_text();
            if needle.is_empty() {
                true
            } else {
                !Self::tree_view_search(model, column as i32, &needle, pos)
            }
        });

        {
            let filter = filter.clone();
            entry.connect_changed(move |_| filter.refilter());
        }

        filter.upcast()
    }

    fn tree_view_search(model: &gtk::TreeModel, column: i32, needle: &str, pos: &gtk::TreeIter) -> bool {
        if let Ok(Some(haystack)) = model.get_value(pos, column).get::<&str>() {
            let haystack = haystack.to_ascii_lowercase();
            let needle = needle.to_ascii_lowercase();
            !haystack.contains(&needle)
        } else {
            true
        }
    }
}

impl SetupViewSearch for gtk::TreeView {
    fn setup_search(&self, column: u32, entry: Option<&gtk::Entry>) -> Option<()> {
        self.set_search_column(column as i32);
        self.set_enable_search(true);
        self.set_search_entry(entry);
        self.set_search_equal_func(Self::tree_view_search);

        if let Some(entry) = entry {
            let view = self.clone();

            entry.connect_activate(move |_| {
                if let Some((model, pos)) = view.get_selection().get_selected() {
                    if let (Some(col), Some(path)) = (view.get_column(0), model.get_path(&pos)) {
                        view.emit_row_activated(&path, &col);
                    }
                }
            });
        }

        Some(())
    }
}

struct TreeModelIterator<'a> {
    model: &'a gtk::TreeModel,
    iter: Option<gtk::TreeIter>,
}

impl<'a> TreeModelIterator<'a> {
    fn new(model: &'a gtk::TreeModel, first: Option<gtk::TreeIter>) -> TreeModelIterator<'a> {
        Self {
            model,
            iter: first.or_else(|| model.get_iter_first()),
        }
    }
}

impl<'a> DoubleEndedIterator for TreeModelIterator<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.iter {
            Some(ref iter) => {
                let cur_iter = iter.clone();

                if !self.model.iter_previous(iter) {
                    self.iter = None;
                }

                Some(cur_iter)
            }
            None => None,
        }
    }
}

impl<'a> Iterator for TreeModelIterator<'a> {
    type Item = gtk::TreeIter;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter {
            Some(ref iter) => {
                let cur_iter = iter.clone();

                if !self.model.iter_next(iter) {
                    self.iter = None;
                }

                Some(cur_iter)
            }
            None => None,
        }
    }
}

impl SetupViewSearch for gtk::IconView {
    #[allow(non_upper_case_globals)]

    fn setup_search(&self, column: u32, entry: Option<&gtk::Entry>) -> Option<()> {
        let entry = entry?;
        let view = self.clone();

        entry.connect_key_press_event(move |entry, event| {
            use gdk::{keys::constants::*, EventType::*};

            #[allow(clippy::never_loop)]
            Inhibit(loop {
                if let Some(model) = view.get_model() {
                    let (iter, rev) = match (event.get_event_type(), event.get_keyval()) {
                        (KeyPress, key @ Up) | (KeyPress, key @ Down) => {
                            let cur_pos = view.get_selected_items().first().and_then(|path| model.get_iter(path));

                            (TreeModelIterator::new(&model, cur_pos), key == Up)
                        }
                        _ => break false,
                    };

                    let needle = entry.get_text();

                    let found = if rev {
                        iter.rev()
                            .skip(1)
                            .find(|pos| !Self::tree_view_search(&model, column as i32, &needle, &pos))
                            .and_then(|pos| model.get_path(&pos))
                    } else {
                        iter.skip(1)
                            .find(|pos| !Self::tree_view_search(&model, column as i32, &needle, &pos))
                            .and_then(|pos| model.get_path(&pos))
                    };

                    if let Some(path) = found {
                        view.unselect_all();
                        view.select_path(&path);
                        view.scroll_to_path(&path, false, 0.0, 0.0);
                    }

                    break true;
                } else {
                    break false;
                }
            })
        });

        let view = self.clone();

        entry.connect_changed(move |entry| {
            if let Some(model) = view.get_model() {
                let needle = entry.get_text();

                if let Some(path) = TreeModelIterator::new(&model, None)
                    .find(|pos| !Self::tree_view_search(&model, column as i32, &needle, &pos))
                    .and_then(|pos| model.get_path(&pos))
                {
                    view.unselect_all();
                    view.select_path(&path);
                    view.scroll_to_path(&path, false, 0.0, 0.0);
                }
            }
        });

        let view = self.clone();

        entry.connect_activate(move |_| {
            view.emit_activate_cursor_item();
        });

        Some(())
    }
}
