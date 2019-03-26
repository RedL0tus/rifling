//! Simple example of Rifling

#[macro_use]
extern crate log;
extern crate rifling;
extern crate actix_web;
extern crate pretty_env_logger;

use rifling::Constructor;
use actix_web::{server, App};
use actix_web_async_await::compat;

use std::env;

fn main() {
    if let Err(_) = env::var("RIFLING_LOG") {
        env::set_var("RIFLING_LOG", "info")
    }
    info!("Bazinga!");
    pretty_env_logger::init_custom_env("RIFLING_LOG");
    let serv = server::new(
        || Constructor::new())
    ).bind("0.0.0.0:4567").unwrap().run();
}