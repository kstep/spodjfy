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

pub trait SpawnScope<T: 'static> {
    fn scope(&self) -> T;
}

impl<A: 'static, B: 'static, T: SpawnScope<A> + SpawnScope<B>> SpawnScope<(A, B)> for T {
    fn scope(&self) -> (A, B) {
        (
            <Self as SpawnScope<A>>::scope(self),
            <Self as SpawnScope<B>>::scope(self),
        )
    }
}

impl<A: 'static, B: 'static, C: 'static, T: SpawnScope<A> + SpawnScope<B> + SpawnScope<C>>
    SpawnScope<(A, B, C)> for T
{
    fn scope(&self) -> (A, B, C) {
        (
            <Self as SpawnScope<A>>::scope(self),
            <Self as SpawnScope<B>>::scope(self),
            <Self as SpawnScope<C>>::scope(self),
        )
    }
}

pub trait Spawn {
    fn spawn<S, F, R>(&self, body: F)
    where
        R: Future<Output = Result<(), SpawnError>> + 'static,
        F: FnOnce(Handle, S) -> R,
        Self: SpawnScope<S>,
        S: 'static,
    {
        self.gcontext()
            .spawn_local(body(self.pool(), self.scope()).map(|_| ()));
    }

    fn gcontext(&self) -> MainContext {
        MainContext::ref_thread_default()
    }
    fn pool(&self) -> Handle;
}
