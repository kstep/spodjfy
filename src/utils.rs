use crate::models::{COL_ITEM_NAME, COL_ITEM_URI};
use gtk::TreeModelExt;

pub fn humanize_time(time_ms: u32) -> String {
    let seconds = time_ms / 1000;
    let (minutes, seconds) = (seconds / 60, seconds % 60);
    let (hours, minutes) = (minutes / 60, minutes % 60);
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{}:{:02}", minutes, seconds)
    }
}

pub fn humanize_inexact_time(time_ms: u32) -> String {
    let seconds = time_ms / 1000;

    match seconds {
        0 => "less than a second".to_owned(),
        1 => "1 second".to_owned(),
        2..=59 => format!("{} seconds", seconds),
        60 => "1 minute".to_owned(),
        61..=3599 => format!("{} minutes", seconds / 60),
        3600 => "1 hour".to_owned(),
        _ => format!("{} hours", seconds / 3600),
    }
}

#[inline]
pub fn rate_to_stars(rate: u32) -> String {
    let stars = rate / 21 + 1;
    "\u{2B50}".repeat(stars as usize)
}

pub fn extract_uri_name(model: &gtk::TreeModel, path: &gtk::TreePath) -> Option<(String, String)> {
    model.get_iter(path).and_then(|pos| {
        model
            .get_value(&pos, COL_ITEM_URI as i32)
            .get::<String>()
            .ok()
            .flatten()
            .zip(
                model
                    .get_value(&pos, COL_ITEM_NAME as i32)
                    .get::<String>()
                    .ok()
                    .flatten(),
            )
    })
}

#[derive(Default, Debug, Clone, Copy)]
pub struct SearchTerms(i16);

#[derive(Clone, Copy)]
pub struct SearchTermsIter(i16, i16);

impl Iterator for SearchTermsIter {
    type Item = SearchTerm;

    fn next(&mut self) -> Option<Self::Item> {
        while self.1 != 16384 {
            let item = self.0 & self.1;
            self.1 <<= 1;

            if item != 0 {
                return Some(unsafe { std::mem::transmute(self.1 >> 1) });
            }
        }
        None
    }
}
impl IntoIterator for SearchTerms {
    type Item = SearchTerm;
    type IntoIter = SearchTermsIter;

    fn into_iter(self) -> Self::IntoIter {
        SearchTermsIter(self.0, 1)
    }
}

// TODO
#[allow(dead_code)]
impl SearchTerms {
    #[inline]
    pub fn add(&mut self, term: SearchTerm) {
        let mask = term as i16;
        self.0 |= mask;
    }
    #[inline]
    pub fn remove(&mut self, term: SearchTerm) {
        let mask = term as i16;
        self.0 &= !mask;
    }
    #[inline]
    pub fn update(&mut self, term: SearchTerm, is_set: bool) {
        let mask = term as i16;
        self.0 ^= (-(is_set as i16) ^ self.0) & mask;
    }
    #[inline]
    pub fn contains(&self, term: SearchTerm) -> bool {
        let mask = term as i16;
        self.0 & mask != 0
    }

    #[inline(always)]
    pub fn is_set(&self, term: u8) -> bool {
        let mask = 1i16 << term;
        self.0 & mask != 0
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(i16)]
pub enum SearchTerm {
    Tempo = 1,
    Duration = 2,
    Key = 4,
    Mode = 8,
    Instrumental = 16,
    Speech = 32,
    Acoustic = 64,
    Dance = 128,
    Energy = 256,
    Liveness = 512,
    Valence = 1024,
    Loudness = 2048,
    Popularity = 4096,
    TimeSign = 8192,
}
