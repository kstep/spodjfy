use crate::models::{common::*, track::constants::*, TrackLike};
use rspotify::model::{DatePrecision, FullEpisode, Image, SimplifiedEpisode, SimplifiedShow, Type as ModelType};

impl TrackLike for FullEpisode {
    fn description(&self) -> Option<&str> { Some(&self.description) }

    fn is_playable(&self) -> bool { self.is_playable }

    fn rate(&self) -> u32 { 0 }

    fn release_date(&self) -> Option<&str> { Some(&self.release_date) }
}

impl HasId for FullEpisode {
    fn id(&self) -> &str { self.id.as_ref() }
}

impl HasName for FullEpisode {
    fn name(&self) -> &str { &self.name }
}

impl HasDuration for FullEpisode {
    fn duration(&self) -> u32 { self.duration_ms }
}

impl HasImages for FullEpisode {
    fn images(&self) -> &[Image] { &self.images }
}

impl MissingColumns for FullEpisode {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[
            COL_TRACK_ARTISTS,
            COL_TRACK_ALBUM,
            COL_TRACK_BPM,
            COL_TRACK_RATE,
            COL_TRACK_SAVED,
        ]
    }
}

impl TrackLike for SimplifiedEpisode {
    fn description(&self) -> Option<&str> { Some(&self.description) }

    fn is_playable(&self) -> bool { self.is_playable }

    fn rate(&self) -> u32 { 0 }

    fn release_date(&self) -> Option<&str> { Some(&self.release_date) }
}

impl HasId for SimplifiedEpisode {
    fn id(&self) -> &str { self.id.as_ref() }
}

impl HasName for SimplifiedEpisode {
    fn name(&self) -> &str { &self.name }
}

impl HasDuration for SimplifiedEpisode {
    fn duration(&self) -> u32 { self.duration.as_millis() as _ }
}

impl HasImages for SimplifiedEpisode {
    fn images(&self) -> &[Image] { &self.images }
}

impl MissingColumns for SimplifiedEpisode {
    fn missing_columns() -> &'static [u32]
    where
        Self: Sized,
    {
        &[
            COL_TRACK_ARTISTS,
            COL_TRACK_ALBUM,
            COL_TRACK_BPM,
            COL_TRACK_RATE,
            COL_TRACK_SAVED,
        ]
    }
}

#[allow(deprecated)]

impl ToFull for SimplifiedEpisode {
    type Full = FullEpisode;

    fn to_full(&self) -> Self::Full {
        FullEpisode {
            audio_preview_url: self.audio_preview_url.clone(),
            description: self.description.clone(),
            duration_ms: self.duration_ms,
            explicit: self.explicit,
            external_urls: self.external_urls.clone(),
            href: self.href.clone(),
            id: self.id.clone(),
            images: self.images.clone(),
            is_externally_hosted: self.is_externally_hosted,
            is_playable: self.is_playable,
            language: self.language.clone(),
            languages: self.languages.clone(),
            name: self.name.clone(),
            release_date: self.release_date.clone(),
            release_date_precision: self.release_date_precision,
            resume_point: self.resume_point.clone(),
            show: SimplifiedShow::empty(),
            _type: ModelType::Episode.to_string(),
            uri: "".to_string(),
        }
    }

    fn into_full(self) -> Self::Full {
        FullEpisode {
            audio_preview_url: self.audio_preview_url,
            description: self.description,
            duration_ms: self.duration_ms,
            explicit: self.explicit,
            external_urls: self.external_urls,
            href: self.href,
            id: self.id,
            images: self.images,
            is_externally_hosted: self.is_externally_hosted,
            is_playable: self.is_playable,
            language: self.language,
            languages: self.languages,
            name: self.name,
            release_date: self.release_date,
            release_date_precision: self.release_date_precision,
            resume_point: self.resume_point,
            show: SimplifiedShow::empty(),
            _type: ModelType::Episode.to_string(),
            uri: "".to_string(),
        }
    }
}

#[allow(deprecated)]

impl ToSimple for FullEpisode {
    type Simple = SimplifiedEpisode;

    fn to_simple(&self) -> Self::Simple {
        SimplifiedEpisode {
            audio_preview_url: self.audio_preview_url.clone(),
            description: self.description.clone(),
            duration_ms: self.duration_ms,
            explicit: self.explicit,
            external_urls: self.external_urls.clone(),
            href: self.href.clone(),
            id: self.id.clone(),
            images: self.images.clone(),
            is_externally_hosted: self.is_externally_hosted,
            is_playable: self.is_playable,
            language: self.language.clone(),
            languages: self.languages.clone(),
            name: self.name.clone(),
            release_date: self.release_date.clone(),
            release_date_precision: self.release_date_precision,
            resume_point: self.resume_point.clone(),
            _type: ModelType::Episode.to_string(),
            uri: self.uri.clone(),
        }
    }

    fn into_simple(self) -> Self::Simple {
        SimplifiedEpisode {
            audio_preview_url: self.audio_preview_url,
            description: self.description,
            duration_ms: self.duration_ms,
            explicit: self.explicit,
            external_urls: self.external_urls,
            href: self.href,
            id: self.id,
            images: self.images,
            is_externally_hosted: self.is_externally_hosted,
            is_playable: self.is_playable,
            language: self.language,
            languages: self.languages,
            name: self.name,
            release_date: self.release_date,
            release_date_precision: self.release_date_precision,
            resume_point: self.resume_point,
            _type: ModelType::Episode.to_string(),
            uri: self.uri,
        }
    }
}

impl Merge for FullEpisode {
    fn merge(self, other: Self) -> Self {
        FullEpisode {
            audio_preview_url: self.audio_preview_url.merge(other.audio_preview_url),
            description: self.description.merge(other.description),
            duration_ms: self.duration_ms.merge(other.duration_ms),
            explicit: self.explicit,
            external_urls: self.external_urls.merge(other.external_urls),
            href: self.href.merge(other.href),
            id: self.id.merge(other.id),
            images: self.images.merge(other.images),
            is_externally_hosted: self.is_externally_hosted || other.is_externally_hosted,
            is_playable: self.is_playable || other.is_playable,
            language: self.language.merge(other.language),
            languages: self.languages.merge(other.languages),
            name: self.name.merge(other.name),
            release_date: self.release_date.merge(other.release_date),
            release_date_precision: if self.release_date_precision == DatePrecision::Year {
                other.release_date_precision
            } else {
                self.release_date_precision
            },
            resume_point: self.resume_point.merge(other.resume_point),
            show: self.show.merge(other.show),
            _type: ModelType::Episode.to_string(),
            uri: self.uri.merge(other.uri),
        }
    }
}
