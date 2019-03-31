//! Simple example of Rifling with Hyper

#[macro_use]
extern crate log;
extern crate hyper;
extern crate pretty_env_logger;

extern crate rifling;

use hyper::Server;
use hyper::rt::Future;

use rifling::{Constructor, Delivery, Hook};

use std::env;

fn main() {
    if let Err(_) = env::var("RIFLING_LOG") {
        env::set_var("RIFLING_LOG", "info")
    }
    pretty_env_logger::init_custom_env("RIFLING_LOG");
    let mut cons = Constructor::new();
    let hook = Hook::new("*", Some("secret"), |_: &Delivery| info!("Bazinga!"));
    let another_hook = Hook::new("push", Some("secret"), |_: &Delivery| info!("Pushed!"));
    cons.register(hook);
    cons.register(another_hook);
    let addr = "0.0.0.0:4567".parse().unwrap();
    let server = Server::bind(&addr)
        .serve(cons)
        .map_err(|e| error!("Error: {:?}", e));
    info!("Starting up...");
    hyper::rt::run(server);
}
