#![allow(clippy::redundant_field_names)]

use futures::FutureExt;
use glib::MainContext;
use rspotify::client::ClientError;
use std::future::Future;
use thiserror::Error;
use tokio::runtime::Handle;
use tokio::task::JoinError;

mod lists;
mod media_controls;
mod notifier;
mod tabs;
pub mod win;

#[derive(Error, Debug)]
pub enum SpawnError {
    #[error("join error: {0}")]
    Join(#[from] JoinError),
    #[error(transparent)]
    Spotify(#[from] ClientError),
}

pub trait Spawn {
    type Scope: 'static;
    fn spawn<F, R>(&self, body: F)
    where
        R: Future<Output = Result<(), SpawnError>> + 'static,
        F: FnOnce(Handle, Self::Scope) -> R,
    {
        self.gcontext()
            .spawn_local(body(self.pool(), self.scope()).map(|_| ()));
    }

    fn scope(&self) -> Self::Scope;

    fn gcontext(&self) -> MainContext {
        MainContext::ref_thread_default()
    }
    fn pool(&self) -> Handle;
}
