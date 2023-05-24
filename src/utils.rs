use crate::models::{COL_ITEM_NAME, COL_ITEM_ID};
use glib::{
    bitflags::_core::{future::Future, time::Duration},
    MainContext,
};
use gtk::TreeModelExt;
use std::sync::Arc;
use rspotify::ClientError;
use thiserror::Error;
use tokio::{runtime::Handle, sync::RwLock, task::JoinError};

pub type AsyncCell<T> = Arc<RwLock<T>>;

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
            .get_value(&pos, COL_ITEM_ID as i32)
            .get::<String>()
            .ok()
            .flatten()
            .zip(model.get_value(&pos, COL_ITEM_NAME as i32).get::<String>().ok().flatten())
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
    type IntoIter = SearchTermsIter;
    type Item = SearchTerm;

    fn into_iter(self) -> Self::IntoIter { SearchTermsIter(self.0, 1) }
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

#[derive(Error, Debug)]
pub enum SpawnError {
    #[error("join error: {0}")]
    Join(#[from] JoinError),
    #[error(transparent)]
    Spotify(#[from] ClientError),
}

pub trait Extract<T: 'static> {
    fn extract(&self) -> T;
}

impl<A: 'static, B: 'static, T: Extract<A> + Extract<B>> Extract<(A, B)> for T {
    fn extract(&self) -> (A, B) { (<Self as Extract<A>>::extract(self), <Self as Extract<B>>::extract(self)) }
}

impl<A: 'static, B: 'static, C: 'static, T: Extract<A> + Extract<B> + Extract<C>> Extract<(A, B, C)> for T {
    fn extract(&self) -> (A, B, C) {
        (
            <Self as Extract<A>>::extract(self),
            <Self as Extract<B>>::extract(self),
            <Self as Extract<C>>::extract(self),
        )
    }
}

#[derive(Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub enum RetryPolicy<E> {
    Repeat,
    WaitRetry(Duration),
    ForwardError(E),
}

pub trait Spawn {
    fn spawn<S, F, R>(&self, mut body: F)
    where
        R: Future<Output = Result<(), SpawnError>> + 'static,
        F: FnMut(Handle, S) -> R + 'static,
        Self: Extract<S>,
        S: Clone + 'static,
    {
        self.spawn_args((), move |pool, scope, _| body(pool, scope));
    }

    fn spawn_args<S, A, F, R>(&self, args: A, mut body: F)
    where
        A: Clone + 'static,
        R: Future<Output = Result<(), SpawnError>> + 'static,
        F: FnMut(Handle, S, A) -> R + 'static,
        Self: Extract<S>,
        S: Clone + 'static,
    {
        let pool = self.pool();
        let scope = self.extract();

        self.gcontext().spawn_local(async move {
            let mut retry_count = 0;

            loop {
                match body(pool.clone(), scope.clone(), args.clone()).await {
                    Ok(_) => break,
                    Err(error) => {
                        error!("spawn error: {}", error);

                        match Self::retry_policy(error, retry_count) {
                            RetryPolicy::ForwardError(_) => {
                                break;
                            }
                            RetryPolicy::Repeat => {
                                info!("repeating...");
                            }
                            RetryPolicy::WaitRetry(timeout) => {
                                info!("retry after {:.2} secs...", timeout.as_secs_f32());

                                glib::timeout_future(timeout.as_millis() as u32).await;
                            }
                        }
                    }
                }

                retry_count += 1;
            }
        });
    }

    fn gcontext(&self) -> MainContext { MainContext::ref_thread_default() }

    fn pool(&self) -> Handle;
    fn retry_policy(error: SpawnError, _retry_count: usize) -> RetryPolicy<SpawnError> { RetryPolicy::ForwardError(error) }
}
