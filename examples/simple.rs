//! Simple example of Rifling

#[macro_use]
extern crate log;
extern crate rifling;
extern crate hyper;
extern crate pretty_env_logger;

use hyper::Server;
use futures::Future;

use rifling::{Constructor, Hook, Delivery};

use std::env;

fn main() {
    if let Err(_) = env::var("RIFLING_LOG") {
        env::set_var("RIFLING_LOG", "info")
    }
    info!("Bazinga!");
    pretty_env_logger::init_custom_env("RIFLING_LOG");
    let mut cons = Constructor::new();
    //let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let hook = Hook::new("*", None, |_: &Delivery| {info!("Bazinga!")});
    cons.register(hook);
    let addr = "0.0.0.0:4567".parse().unwrap();
    let server = Server::bind(&addr).serve(cons).map_err(|e| error!("Error: {:?}", e));
    info!("Service started");
    hyper::rt::run(server);
}