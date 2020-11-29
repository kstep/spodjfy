use crate::models::common::*;
use crate::models::{
    TrackLike, COL_TRACK_ALBUM, COL_TRACK_ARTISTS, COL_TRACK_BPM, COL_TRACK_RATE, COL_TRACK_SAVED,
};
use rspotify::model::{FullEpisode, Image, SimplifiedEpisode};

impl TrackLike for FullEpisode {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some(&self.description)
    }

    fn is_playable(&self) -> bool {
        self.is_playable
    }

    fn rate(&self) -> u32 {
        0
    }

    fn release_date(&self) -> Option<&str> {
        Some(&self.release_date)
    }
}

impl HasUri for FullEpisode {
    fn uri(&self) -> &str {
        &self.uri
    }
}

impl HasName for FullEpisode {
    fn name(&self) -> &str {
        &self.name
    }
}

impl HasDuration for FullEpisode {
    fn duration(&self) -> u32 {
        self.duration_ms
    }
}

impl HasImages for FullEpisode {
    fn images(&self) -> &[Image] {
        &self.images
    }
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
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> Option<&str> {
        Some(&self.description)
    }

    fn is_playable(&self) -> bool {
        self.is_playable
    }

    fn rate(&self) -> u32 {
        0
    }

    fn release_date(&self) -> Option<&str> {
        Some(&self.release_date)
    }
}

impl HasUri for SimplifiedEpisode {
    fn uri(&self) -> &str {
        &self.uri
    }
}

impl HasName for SimplifiedEpisode {
    fn name(&self) -> &str {
        &self.name
    }
}

impl HasDuration for SimplifiedEpisode {
    fn duration(&self) -> u32 {
        self.duration_ms
    }
}

impl HasImages for SimplifiedEpisode {
    fn images(&self) -> &[Image] {
        &self.images
    }
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
