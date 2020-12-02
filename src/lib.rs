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

pub static mut DEFAULT_TOKIO_RUNTIME: Option<tokio::runtime::Runtime> = None;

pub use components::win::{Params, Win};
pub use config::Config;
pub use services::{LoginService, RefreshTokenService, Spotify};
