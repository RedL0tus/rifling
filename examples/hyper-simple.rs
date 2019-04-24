//! Simple example of Rifling with Hyper

#[cfg(feature = "logging")]
#[macro_use]
extern crate log;
extern crate hyper;
extern crate pretty_env_logger;

#[macro_use]
extern crate rifling;

use hyper::rt::Future;
use hyper::Server;

use rifling::{Constructor, Delivery, DeliveryType, Hook};

use std::env;

fn main() {
    if let Err(_) = env::var("RIFLING_LOG") {
        env::set_var("RIFLING_LOG", "info")
    }
    pretty_env_logger::init_custom_env("RIFLING_LOG");
    let mut cons = Constructor::new();
    let hook = Hook::new("*", Some(String::from("secret")), |delivery: &Delivery| {
        #[cfg(feature = "parse")]
        {
            if let Some(payload) = &delivery.payload {
                info!(
                    "Bazinga! Received \"{}\" action!",
                    match delivery.delivery_type {
                        DeliveryType::GitHub => payload["action"].as_str().unwrap(),
                        DeliveryType::GitLab => payload["event_name"].as_str().unwrap(),
                    }
                );
            }
        }
        #[cfg(not(feature = "parse"))]
        {
            if let Some(event) = &delivery.event {
                info!("Received \"{}\" action!", event);
            }
        }
    });
    let another_hook = Hook::new("push", Some(String::from("secret")), |_: &Delivery| {
        info!("Pushed!");
    });
    let gitlab_push_hook = Hook::new("push_hook", Some(String::from("secret")), |_: &Delivery| {
        info!("GitLab pushed");
    });
    cons.register(hook);
    cons.register(another_hook);
    cons.register(gitlab_push_hook);
    let addr = "0.0.0.0:4567".parse().unwrap();
    let server = Server::bind(&addr)
        .serve(cons)
        .map_err(|e| println!("Error: {:?}", e));
    info!("Starting up...");
    hyper::rt::run(server);
}
