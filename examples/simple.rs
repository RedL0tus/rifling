//! Simple example of Rifling

#[macro_use]
extern crate log;
extern crate hyper;
extern crate pretty_env_logger;
extern crate rifling;

use futures::Future;
use hyper::Server;

use rifling::{Constructor, Delivery, Hook};

use std::env;

fn main() {
    if let Err(_) = env::var("RIFLING_LOG") {
        env::set_var("RIFLING_LOG", "info")
    }
    info!("Bazinga!");
    pretty_env_logger::init_custom_env("RIFLING_LOG");
    let mut cons = Constructor::new();
    let hook = Hook::new("*", None, |_: &Delivery| info!("Bazinga!"));
    cons.register(hook);
    let addr = "0.0.0.0:4567".parse().unwrap();
    let server = Server::bind(&addr)
        .serve(cons)
        .map_err(|e| error!("Error: {:?}", e));
    info!("Service started");
    hyper::rt::run(server);
}
