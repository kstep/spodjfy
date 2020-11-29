#![feature(fn_traits, unboxed_closures, bool_to_option)]

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
pub use servers::{LoginServer, Spotify, SpotifyCmd, SpotifyProxy, SpotifyServer};
