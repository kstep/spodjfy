use crate::components::lists::common::SetupViewSearch;
use crate::components::lists::{ContainerList, ContainerMsg, GetSelectedRows, ItemsListView};
use crate::loaders::{ContainerLoader, ImageConverter};
use crate::models::artist::*;
use glib::{Cast, IsA};
use gtk::{
    CellLayoutExt, CellRendererExt, CellRendererTextExt, IconViewExt, TreeModelExt, WidgetExt,
};
use relm::EventStream;
use std::io::Write;

pub type ArtistList<Loader> = ContainerList<Loader, ArtistView>;

const THUMB_SIZE: i32 = 128;
const ITEM_SIZE: i32 = (THUMB_SIZE as f32 * 2.25) as i32;

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
            .item_orientation(gtk::Orientation::Horizontal)
            .text_column(COL_ARTIST_NAME as i32)
            .pixbuf_column(COL_ARTIST_THUMB as i32)
            .has_tooltip(true)
            .item_padding(10)
            .item_width(ITEM_SIZE)
            .build();

        artist_view.connect_item_activated(move |view, path| {
            if let Some((uri, name)) = view
                .get_model()
                .and_then(|model| crate::utils::extract_uri_name(&model, path))
            {
                stream.emit(ContainerMsg::ActivateItem(uri, name).into());
            }
        });

        let cells = artist_view.get_cells();
        if let Some(cell) = cells.last() {
            cell.set_alignment(0.0, 0.0);
            cell.set_padding(10, 0);
            artist_view.set_cell_data_func(
                cell,
                Some(Box::new(move |_layout, cell, model, pos| {
                    if let (Ok(Some(name)), Ok(Some(genres)), Ok(Some(rate)), Some(cell)) = (
                        model.get_value(pos, COL_ARTIST_NAME as i32).get::<&str>(),
                        model.get_value(pos, COL_ARTIST_GENRES as i32).get::<&str>(),
                        model.get_value(pos, COL_ARTIST_RATE as i32).get::<u32>(),
                        cell.downcast_ref::<gtk::CellRendererText>(),
                    ) {
                        let stars = crate::utils::rate_to_stars(rate);
                        let info = if genres.is_empty() {
                            format!("<big>{}</big>\n{}", glib::markup_escape_text(name), stars)
                        } else {
                            let (genres, ellip) = if genres.len() < 35 {
                                (genres, "")
                            } else {
                                let mut cut = 35;
                                let bytes = genres.as_bytes();
                                let len = bytes.len();
                                while cut < len && bytes[cut] & 0b1100_0000 == 0b1000_0000 {
                                    cut += 1;
                                }
                                (
                                    match genres[..cut].rsplitn(2, ',').last() {
                                        Some(last) => last,
                                        None => &genres[..cut],
                                    },
                                    "…",
                                )
                            };
                            format!(
                                "<big>{}</big>\n<i>{}{}</i>\n{}",
                                glib::markup_escape_text(name),
                                glib::markup_escape_text(genres),
                                ellip,
                                stars
                            )
                        };

                        cell.set_property_markup(Some(&info));
                    }
                })),
            );
        }

        artist_view.connect_query_tooltip(|view, mut x, mut y, kbd, tooltip| {
            if let Some((model, path, pos)) = view.get_tooltip_context(&mut x, &mut y, kbd) {
                if let (Ok(Some(rate)), Ok(Some(genres)), Ok(Some(followers))) = (
                    model.get_value(&pos, COL_ARTIST_RATE as i32).get::<u32>(),
                    model
                        .get_value(&pos, COL_ARTIST_GENRES as i32)
                        .get::<&str>(),
                    model
                        .get_value(&pos, COL_ARTIST_FOLLOWERS as i32)
                        .get::<u64>(),
                ) {
                    let info = {
                        let mut info = Vec::with_capacity(100);
                        if !genres.is_empty() {
                            writeln!(&mut info, "Genres: {}", genres).unwrap();
                        }
                        if rate > 0 {
                            writeln!(&mut info, "Rating: {}", rate).unwrap();
                        }
                        if followers > 0 {
                            writeln!(&mut info, "Followers: {}", followers).unwrap();
                        }

                        info.shrink_to_fit();

                        unsafe { String::from_utf8_unchecked(info) }
                    };

                    tooltip.set_text(Some(info.trim_end()));
                    view.set_tooltip_cell(&tooltip, &path, view.get_cells().last());
                    return true;
                }
            }
            false
        });

        ArtistView(artist_view)
    }

    fn setup_search(&self, entry: &gtk::Entry) -> bool {
        self.0.setup_search(COL_ARTIST_NAME, Some(entry));
        true
    }

    fn thumb_converter(&self) -> ImageConverter {
        ImageConverter::new(THUMB_SIZE, true)
    }
}
