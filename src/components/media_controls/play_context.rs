use itertools::Itertools;
use rspotify::model::{FullAlbum, FullArtist, FullPlaylist, FullShow, Image, Type};
use std::borrow::Cow;
use std::ops::Deref;

#[derive(Debug, Clone)]
pub enum PlayContext {
    Album(FullAlbum),
    Playlist(FullPlaylist),
    Artist(FullArtist),
    Show(FullShow),
}

impl PlayContext {
    pub fn uri(&self) -> &str {
        match self {
            PlayContext::Album(ctx) => &*ctx.uri,
            PlayContext::Artist(ctx) => &*ctx.uri,
            PlayContext::Playlist(ctx) => &*ctx.uri,
            PlayContext::Show(ctx) => &*ctx.uri,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            PlayContext::Album(ctx) => &*ctx.name,
            PlayContext::Artist(ctx) => &*ctx.name,
            PlayContext::Playlist(ctx) => &*ctx.name,
            PlayContext::Show(ctx) => &*ctx.name,
        }
    }

    pub fn artists(&self) -> Option<Cow<str>> {
        match self {
            PlayContext::Album(ctx) => Some(
                ctx.artists
                    .iter()
                    .map(|artist| &artist.name)
                    .join(", ")
                    .into(),
            ),
            PlayContext::Artist(_) => None,
            PlayContext::Playlist(ctx) => Some(
                ctx.owner
                    .display_name
                    .as_deref()
                    .unwrap_or(&ctx.owner.id)
                    .into(),
            ),
            PlayContext::Show(ctx) => Some(ctx.publisher.deref().into()),
        }
    }

    pub fn duration(&self) -> Option<u32> {
        match self {
            PlayContext::Album(ctx) => {
                if ctx.tracks.next.is_none() {
                    Some(ctx.tracks.items.iter().map(|track| track.duration_ms).sum())
                } else {
                    None
                }
            }
            PlayContext::Artist(_) => None,
            PlayContext::Playlist(ctx) => {
                if ctx.tracks.next.is_none() {
                    Some(
                        ctx.tracks
                            .items
                            .iter()
                            .filter_map(|track| track.track.as_ref())
                            .map(|track| track.duration_ms)
                            .sum(),
                    )
                } else {
                    None
                }
            }
            PlayContext::Show(ctx) => {
                if ctx.episodes.next.is_none() {
                    Some(
                        ctx.episodes
                            .items
                            .iter()
                            .map(|episode| episode.duration_ms)
                            .sum(),
                    )
                } else {
                    None
                }
            }
        }
    }

    pub fn genres(&self) -> Option<&Vec<String>> {
        match self {
            PlayContext::Album(ctx) => Some(&ctx.genres),
            PlayContext::Artist(ctx) => Some(&ctx.genres),
            PlayContext::Playlist(_) => None,
            PlayContext::Show(_) => None,
        }
    }

    pub fn images(&self) -> &Vec<Image> {
        match self {
            PlayContext::Album(ctx) => &ctx.images,
            PlayContext::Artist(ctx) => &ctx.images,
            PlayContext::Playlist(ctx) => &ctx.images,
            PlayContext::Show(ctx) => &ctx.images,
        }
    }

    pub fn tracks_number(&self) -> u32 {
        match self {
            PlayContext::Album(ctx) => ctx.tracks.total,
            PlayContext::Artist(_) => 0,
            PlayContext::Playlist(ctx) => ctx.tracks.total,
            PlayContext::Show(ctx) => ctx.episodes.total,
        }
    }

    pub fn kind(&self) -> Type {
        match self {
            PlayContext::Album(_) => Type::Album,
            PlayContext::Artist(_) => Type::Artist,
            PlayContext::Playlist(_) => Type::Playlist,
            PlayContext::Show(_) => Type::Show,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            PlayContext::Album(_) => "",
            PlayContext::Artist(_) => "",
            PlayContext::Playlist(ctx) => &ctx.description,
            PlayContext::Show(ctx) => &ctx.description,
        }
    }
}