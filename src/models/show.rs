use crate::models::{common::*, PlaylistLike, COL_PLAYLIST_DURATION, COL_PLAYLIST_TOTAL_TRACKS};
use chrono::{DateTime, Utc};
use glib::{IsA, Type};
use rspotify::model::{FullShow, Image, Page, Show, SimplifiedShow, Type as ModelType};
use std::{collections::HashMap, time::SystemTime};

impl PlaylistLike for FullShow {
    fn id(&self) -> &str { &self.id }

    fn description(&self) -> &str { &self.description }

    fn publisher(&self) -> &str { &self.publisher }

    fn total_tracks(&self) -> u32 { self.episodes.total }
}

impl HasUri for FullShow {
    fn uri(&self) -> &str { &self.uri }
}

impl HasName for FullShow {
    fn name(&self) -> &str { &self.name }
}

impl ToSimple for FullShow {
    type Simple = SimplifiedShow;

    fn to_simple(&self) -> Self::Simple {
        SimplifiedShow {
            available_markets: self.available_markets.clone(),
            copyrights: self.copyrights.clone(),
            description: self.description.clone(),
            explicit: self.explicit,
            external_urls: self.external_urls.clone(),
            href: self.href.clone(),
            id: self.id.clone(),
            images: self.images.clone(),
            is_externally_hosted: self.is_externally_hosted,
            languages: self.languages.clone(),
            media_type: self.media_type.clone(),
            name: self.name.clone(),
            publisher: self.publisher.clone(),
            _type: ModelType::Show.to_string(),
            uri: self.uri.clone(),
        }
    }

    fn into_simple(self) -> Self::Simple {
        SimplifiedShow {
            available_markets: self.available_markets,
            copyrights: self.copyrights,
            description: self.description,
            explicit: self.explicit,
            external_urls: self.external_urls,
            href: self.href,
            id: self.id,
            images: self.images,
            is_externally_hosted: self.is_externally_hosted,
            languages: self.languages,
            media_type: self.media_type,
            name: self.name,
            publisher: self.publisher,
            _type: ModelType::Show.to_string(),
            uri: self.uri,
        }
    }
}

impl Merge for FullShow {
    fn merge(self, other: Self) -> Self {
        FullShow {
            available_markets: self.available_markets.merge(other.available_markets),
            copyrights: self.copyrights.merge(other.copyrights),
            description: self.description.merge(other.description),
            explicit: self.explicit || other.explicit,
            episodes: self.episodes.merge(other.episodes),
            external_urls: self.external_urls.merge(other.external_urls),
            href: self.href.merge(other.href),
            id: self.id.merge(other.id),
            images: self.images.merge(other.images),
            is_externally_hosted: self.is_externally_hosted.merge(other.is_externally_hosted),
            languages: self.languages.merge(other.languages),
            media_type: self.media_type.merge(other.media_type),
            name: self.name.merge(other.name),
            publisher: self.publisher.merge(other.publisher),
            _type: ModelType::Show.to_string(),
            uri: self.uri.merge(other.uri),
        }
    }
}

impl HasDuration for FullShow {
    fn duration(&self) -> u32 { self.episodes.items.iter().map(|episode| episode.duration_ms).sum() }

    fn duration_exact(&self) -> bool { self.episodes.items.len() == self.episodes.total as usize }
}

impl MissingColumns for FullShow {}

impl HasImages for FullShow {
    fn images(&self) -> &[Image] { &self.images }
}

impl RowLike for FullShow {
    fn content_types() -> Vec<Type> { Self::store_content_types() }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter { self.insert_into_store(store) }
}

impl PlaylistLike for SimplifiedShow {
    fn id(&self) -> &str { &self.id }

    fn description(&self) -> &str { &self.description }

    fn publisher(&self) -> &str { &self.publisher }
}

impl HasUri for SimplifiedShow {
    fn uri(&self) -> &str { &self.uri }
}

impl HasName for SimplifiedShow {
    fn name(&self) -> &str { &self.name }
}

impl ToFull for SimplifiedShow {
    type Full = FullShow;

    fn to_full(&self) -> Self::Full { unimplemented!() }

    fn into_full(self) -> Self::Full {
        FullShow {
            available_markets: self.available_markets,
            copyrights: self.copyrights,
            description: self.description,
            explicit: self.explicit,
            episodes: Page::empty(),
            external_urls: self.external_urls,
            href: self.href,
            id: self.id,
            images: self.images,
            is_externally_hosted: self.is_externally_hosted,
            languages: self.languages,
            media_type: self.media_type,
            name: self.name,
            publisher: self.publisher,
            _type: ModelType::Show.to_string(),
            uri: self.uri,
        }
    }
}

impl Empty for SimplifiedShow {
    fn empty() -> Self {
        SimplifiedShow {
            available_markets: Vec::new(),
            copyrights: Vec::new(),
            description: String::new(),
            explicit: false,
            external_urls: HashMap::new(),
            href: String::new(),
            id: String::new(),
            images: Vec::new(),
            is_externally_hosted: None,
            languages: Vec::new(),
            media_type: String::new(),
            name: String::new(),
            publisher: String::new(),
            _type: ModelType::Show.to_string(),
            uri: String::new(),
        }
    }

    fn is_empty(&self) -> bool { self.uri.is_empty() }
}

impl HasDuration for SimplifiedShow {
    fn duration_exact(&self) -> bool { false }
}

impl MissingColumns for SimplifiedShow {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_PLAYLIST_TOTAL_TRACKS, COL_PLAYLIST_DURATION]
    }
}

impl HasImages for SimplifiedShow {
    fn images(&self) -> &[Image] { &self.images }
}

impl RowLike for SimplifiedShow {
    fn content_types() -> Vec<Type> { Self::store_content_types() }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter { self.insert_into_store(store) }
}

impl PlaylistLike for Show {
    fn id(&self) -> &str { self.show.id() }

    fn description(&self) -> &str { self.show.description() }

    fn publisher(&self) -> &str { self.show.publisher() }
}

impl HasUri for Show {
    fn uri(&self) -> &str { self.show.uri() }
}

impl HasName for Show {
    fn name(&self) -> &str { self.show.name() }
}

impl HasDuration for Show {
    fn duration_exact(&self) -> bool { false }
}

impl MissingColumns for Show {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[COL_PLAYLIST_TOTAL_TRACKS, COL_PLAYLIST_DURATION]
    }
}

impl HasImages for Show {
    fn images(&self) -> &[Image] { &self.show.images }
}

impl RowLike for Show {
    fn content_types() -> Vec<Type> { Self::store_content_types() }

    fn append_to_store<S: IsA<gtk::ListStore>>(&self, store: &S) -> gtk::TreeIter { self.insert_into_store(store) }
}

impl Wrapper for Show {
    type For = SimplifiedShow;

    fn unwrap(self) -> Self::For { self.show }

    fn wrap(show: Self::For) -> Self {
        Show {
            added_at: DateTime::<Utc>::from(SystemTime::now()).date().format("%Y-%m-%d").to_string(),
            show,
        }
    }
}
