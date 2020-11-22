use crate::components::lists::{ContainerMsg, GetSelectedRows, ItemsListView, TrackMsg};
use crate::loaders::track::*;
use crate::loaders::{ContainerLoader, MissingColumns};
use glib::signal::Inhibit;
use glib::{Cast, IsA, ObjectExt};
use gtk::{
    CellLayoutExt, CellRendererExt, CellRendererPixbufExt, CellRendererTextExt, GtkMenuItemExt,
    MenuShellExt, TreeModelExt, TreeSelectionExt, TreeViewColumn, TreeViewExt,
    WidgetExt,
};
use relm::EventStream;
use std::ops::Deref;

const THUMB_SIZE: i32 = 32;

pub struct TrackView(gtk::TreeView);

impl Deref for TrackView {
    type Target = gtk::TreeView;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<gtk::TreeView> for TrackView {
    fn from(view: gtk::TreeView) -> Self {
        TrackView(view)
    }
}

impl AsRef<gtk::Widget> for TrackView {
    fn as_ref(&self) -> &gtk::Widget {
        self.0.upcast_ref()
    }
}

impl GetSelectedRows for TrackView {
    fn get_selected_rows(&self) -> (Vec<gtk::TreePath>, gtk::TreeModel) {
        self.0.get_selected_rows()
    }
}

impl<Loader> ItemsListView<Loader, TrackMsg<Loader>> for TrackView
where
    Loader: ContainerLoader + 'static,
    Loader::Item: MissingColumns,
{
    #[allow(clippy::redundant_clone)]
    fn create<S: IsA<gtk::TreeModel>>(stream: EventStream<TrackMsg<Loader>>, store: &S) -> Self {
        let items_view = gtk::TreeViewBuilder::new()
            .model(store)
            .expand(true)
            .reorderable(true)
            .has_tooltip(true)
            .build();

        items_view
            .get_selection()
            .set_mode(gtk::SelectionMode::Multiple);

        let base_column = gtk::TreeViewColumnBuilder::new()
            .resizable(true)
            .reorderable(true)
            .expand(true);

        let missing_columns = Loader::Item::missing_columns();

        if !missing_columns.contains(&COL_TRACK_NUMBER) {
            items_view.append_column(&{
                let text_cell = gtk::CellRendererText::new();
                text_cell.set_alignment(1.0, 0.5);

                let column = base_column
                    .clone()
                    .expand(false)
                    .title("#")
                    .sort_column_id(COL_TRACK_NUMBER as i32)
                    .alignment(1.0)
                    .build();
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_NUMBER as i32);
                column
            });
        }

        if !missing_columns.contains(&COL_TRACK_THUMB) {
            items_view.append_column(&{
                let icon_cell = gtk::CellRendererPixbuf::new();
                icon_cell.set_property_icon_name(Some("audio-x-generic-symbolic"));

                let column = TreeViewColumn::new();
                column.pack_start(&icon_cell, true);
                column.add_attribute(&icon_cell, "pixbuf", COL_TRACK_THUMB as i32);
                column
            });
        }

        if !missing_columns.contains(&COL_TRACK_NAME) {
            items_view.append_column(&{
                let text_cell = gtk::CellRendererText::new();
                let column = base_column
                    .clone()
                    .title("Title")
                    .sort_column_id(COL_TRACK_NAME as i32)
                    .build();
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_NAME as i32);
                column.add_attribute(&text_cell, "strikethrough", COL_TRACK_CANT_PLAY as i32);
                column
            });
        }

        if !missing_columns.contains(&COL_TRACK_DURATION) {
            items_view.append_column(&{
                let text_cell = gtk::CellRendererText::new();
                text_cell.set_alignment(1.0, 0.5);
                let column = base_column
                    .clone()
                    .expand(false)
                    .title("Duration")
                    .sort_column_id(COL_TRACK_DURATION_MS as i32)
                    .build();
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_DURATION as i32);
                column
            });
        }

        if !missing_columns.contains(&COL_TRACK_TIMELINE) {
            items_view.append_column(&{
                let text_cell = gtk::CellRendererText::new();
                text_cell.set_alignment(1.0, 0.5);
                let column = base_column
                    .clone()
                    .expand(false)
                    .title("Timeline")
                    .sort_column_id(COL_TRACK_NUMBER as i32)
                    .build();
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_TIMELINE as i32);
                column
            });
        }

        if !missing_columns.contains(&COL_TRACK_BPM) {
            items_view.append_column(&{
                let text_cell = gtk::CellRendererTextBuilder::new()
                    .xalign(1.0)
                    .editable(true)
                    .mode(gtk::CellRendererMode::Editable)
                    .build();

                {
                    let stream = stream.clone();
                    text_cell.connect_edited(move |_, path, new_text| {
                        if let Ok(bpm) = new_text.parse::<f32>() {
                            stream.emit(TrackMsg::NewBpm(path, bpm));
                        }
                    });
                }
                let column = base_column
                    .clone()
                    .expand(false)
                    .title("BPM")
                    .sort_column_id(COL_TRACK_BPM as i32)
                    .build();
                gtk::TreeViewColumnExt::set_cell_data_func(
                    &column,
                    &text_cell,
                    Some(Box::new(|_layout, cell, model, iter| {
                        let bpm: f32 = model
                            .get_value(iter, COL_TRACK_BPM as i32)
                            .get()
                            .ok()
                            .flatten()
                            .unwrap_or(0.0);
                        let _ = cell.set_property("text", &format!("{:.0}", bpm));
                    })),
                );
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_BPM as i32);
                column
            });
        }

        if !missing_columns.contains(&COL_TRACK_RATE) {
            let column_index = items_view.append_column(&{
                let text_cell = gtk::CellRendererText::new();
                let column = base_column
                    .clone()
                    .title("Rate")
                    .sort_column_id(COL_TRACK_RATE as i32)
                    .build();
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_RATE as i32);

                gtk::TreeViewColumnExt::set_cell_data_func(
                    &column,
                    &text_cell,
                    Some(Box::new(move |_layout, cell, model, pos| {
                        if let (Ok(Some(rate)), Some(cell)) = (
                            model.get_value(pos, COL_TRACK_RATE as i32).get::<u32>(),
                            cell.downcast_ref::<gtk::CellRendererText>(),
                        ) {
                            let stars = rate / 21 + 1;
                            cell.set_property_text(Some(&"\u{2B50}".repeat(stars as usize)));
                        }
                    })),
                );
                column
            }) - 1;

            items_view.connect_query_tooltip(move |tree, mut x, mut y, kbd, tooltip| {
                let column = match tree.get_column(column_index) {
                    Some(column) => column,
                    None => return false,
                };

                if let Some((Some(model), path, pos)) =
                    tree.get_tooltip_context(&mut x, &mut y, kbd)
                {
                    let (col_x0, col_x1) = {
                        let rect = tree.get_cell_area(Some(&path), Some(&column));
                        (rect.x, rect.x + rect.width)
                    };

                    if x <= col_x0 || col_x1 <= x {
                        return false;
                    }

                    if let Ok(Some(rate)) =
                        model.get_value(&pos, COL_TRACK_RATE as i32).get::<u32>()
                    {
                        tooltip.set_text(Some(&format!("Rating: {}", rate)));
                        tree.set_tooltip_cell(
                            &tooltip,
                            Some(&path),
                            Some(&column),
                            None::<&gtk::CellRendererText>,
                        );
                        return true;
                    }
                }

                false
            });
        }

        if !missing_columns.contains(&COL_TRACK_RELEASE_DATE) {
            items_view.append_column(&{
                let text_cell = gtk::CellRendererText::new();
                text_cell.set_alignment(1.0, 0.5);

                let column = base_column
                    .clone()
                    .expand(false)
                    .title("Released")
                    .sort_column_id(COL_TRACK_RELEASE_DATE as i32)
                    .build();
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_RELEASE_DATE as i32);
                column
            });
        }

        if !missing_columns.contains(&COL_TRACK_ARTISTS) {
            items_view.append_column(&{
                let text_cell = gtk::CellRendererText::new();
                let column = base_column
                    .clone()
                    .title("Artists")
                    .sort_column_id(COL_TRACK_ARTISTS as i32)
                    .build();
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_ARTISTS as i32);
                column
            });
        }

        if !missing_columns.contains(&COL_TRACK_ALBUM) {
            items_view.append_column(&{
                let text_cell = gtk::CellRendererText::new();
                let column = base_column
                    .clone()
                    .title("Album")
                    .sort_column_id(COL_TRACK_ALBUM as i32)
                    .build();
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_ALBUM as i32);
                column
            });
        }

        if !missing_columns.contains(&COL_TRACK_DESCRIPTION) {
            items_view.append_column(&{
                let text_cell = gtk::CellRendererText::new();
                let column = base_column
                    .clone()
                    .title("Description")
                    .sort_column_id(COL_TRACK_DESCRIPTION as i32)
                    .build();
                column.pack_start(&text_cell, true);
                column.add_attribute(&text_cell, "text", COL_TRACK_DESCRIPTION as i32);
                column
            });
        }

        {
            let stream = stream.clone();
            items_view.connect_button_press_event(move |_, event| {
                if event.get_button() == 3 {
                    stream.emit(TrackMsg::Parent(ContainerMsg::OpenContextMenu(
                        event.clone(),
                    )));
                    Inhibit(true)
                } else {
                    Inhibit(false)
                }
            });
        }

        items_view.set_search_column(COL_TRACK_NAME as i32);
        items_view.set_enable_search(true);
        items_view.set_search_equal_func(|model, col, needle, pos| {
            if let Ok(Some(haystack)) = model.get_value(pos, col).get::<&str>() {
                let haystack = haystack.to_ascii_lowercase();
                let needle = needle.to_ascii_lowercase();
                !haystack.contains(&needle)
            } else {
                true
            }
        });

        {
            let stream = stream.clone();
            items_view.connect_row_activated(move |tree, path, _col| {
                if let Some(track_uri) = tree.get_model().and_then(|store| {
                    store.get_iter(path).and_then(|pos| {
                        store
                            .get_value(&pos, COL_TRACK_URI as i32)
                            .get::<String>()
                            .ok()
                            .flatten()
                    })
                }) {
                    stream.emit(TrackMsg::PlayTracks(vec![track_uri]));
                }
            });
        }

        TrackView(items_view)
    }

    fn context_menu(&self, stream: EventStream<TrackMsg<Loader>>) -> gtk::Menu {
        let context_menu = gtk::Menu::new();

        macro_rules! menu {
            ($menu:ident, $stream:ident, $($item:tt),+) => {
                $($menu.append(&{
                    menu!(@ $stream, $item)
                });)+
            };
            (@ $stream:ident, ($title:literal => $msg:ident)) => {{
                let item = gtk::MenuItem::with_label($title);
                let stream = $stream.clone();
                item.connect_activate(move |_| stream.emit(TrackMsg::$msg));
                item
            }};
            (@ $stream:ident, (===)) => {
                gtk::SeparatorMenuItem::new()
            };
        }

        menu! {context_menu, stream,
            ("Play now" => PlayChosenTracks),
            ("Add to queue" => EnqueueChosenTracks),
            ("Add to library" => SaveChosenTracks),
            ("Add to playlist…" => AddChosenTracks),
            (===),
            ("Go to album" => GoToChosenTrackAlbum),
            ("Go to artist" => GoToChosenTrackArtist),
            ("Recommend similar" => RecommendTracks),
            (===),
            ("Remove from library" => UnsaveChosenTracks)
            //("Remove from playlist" => RemoveChosenTracks)
        };

        context_menu
    }

    fn thumb_size(&self) -> i32 {
        THUMB_SIZE
    }
}
