#![allow(clippy::redundant_field_names)]

use glib::MainContext;
use rspotify::client::ClientError;
use std::future::Future;
use std::time::Duration;
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

#[derive(Debug, Eq, PartialEq)]
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
        Self: SpawnScope<S>,
        S: Clone + 'static,
    {
        self.spawn_args((), move |pool, scope, _| body(pool, scope));
    }

    fn spawn_args<S, A, F, R>(&self, args: A, mut body: F)
    where
        A: Clone + 'static,
        R: Future<Output = Result<(), SpawnError>> + 'static,
        F: FnMut(Handle, S, A) -> R + 'static,
        Self: SpawnScope<S>,
        S: Clone + 'static,
    {
        let pool = self.pool();
        let scope = self.scope();
        self.gcontext().spawn_local(async move {
            loop {
                match body(pool.clone(), scope.clone(), args.clone()).await {
                    Ok(_) => break (),
                    Err(error) => match Self::retry_policy(error) {
                        RetryPolicy::ForwardError(error) => {
                            error!("spawn error: {}", error);
                            break ();
                        }
                        RetryPolicy::Repeat => {}
                        RetryPolicy::WaitRetry(timeout) => {
                            glib::timeout_future(timeout.as_millis() as u32).await;
                        }
                    },
                }
            }
        });
    }

    fn gcontext(&self) -> MainContext {
        MainContext::ref_thread_default()
    }
    fn pool(&self) -> Handle;

    fn retry_policy(error: SpawnError) -> RetryPolicy<SpawnError> {
        RetryPolicy::ForwardError(error)
    }
}
