#![allow(incomplete_features)]
#![feature(
    fn_traits,
    unboxed_closures,
    never_type,
    associated_type_defaults,
    specialization
)]

#[macro_use]
extern crate log;

mod components;
mod config;
mod loaders;
mod models;
mod scopes;
mod servers;
mod utils;

pub use components::win::{Params, Win};
pub use config::Config;
pub use servers::{LoginServer, Spotify};
