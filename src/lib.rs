#![allow(incomplete_features)]
#![feature(
    fn_traits,
    unboxed_closures,
    never_type,
    associated_type_defaults,
    specialization,
    async_closure
)]

#[macro_use]
extern crate log;

mod components;
mod config;
mod loaders;
mod models;
mod scopes;
mod services;
mod utils;

pub use components::win::{Params, Win};
pub use config::Config;
pub use services::{LoginService, RefreshTokenService, Spotify};

use lazy_static::lazy_static;
use tokio::{
    runtime::Handle,
    sync::broadcast::{channel, Receiver, RecvError, SendError, Sender},
    task::JoinHandle,
};

#[derive(Clone)]
pub enum AppEvent {
    SpotifyAuthError(String),
    SpotifyError(String),
}

const EVENT_BUS_SIZE: usize = 1024;

lazy_static! {
    pub static ref EVENT_BUS: Sender<AppEvent> = channel::<AppEvent>(EVENT_BUS_SIZE).0;
}

pub fn subscribe() -> Receiver<AppEvent> { EVENT_BUS.subscribe() }

pub fn observe<F: FnMut(AppEvent) + Send + 'static>(pool: &Handle, mut callback: F) -> JoinHandle<Result<!, RecvError>> {
    let mut rx = subscribe();

    pool.spawn(async move {
        loop {
            match rx.recv().await {
                Err(closed @ RecvError::Closed) => {
                    break Err(closed);
                }
                Err(RecvError::Lagged(count)) => {
                    warn!("skipped {} messages", count);
                }
                Ok(event) => {
                    callback(event);
                }
            }
        }
    })
}

pub fn broadcast<Event: Into<AppEvent>>(event: Event) -> Result<usize, SendError<AppEvent>> { EVENT_BUS.send(event.into()) }
