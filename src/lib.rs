//! # Rifling
//!
//! Rifling is a framework to create Github Webhook listener, influenced by [afterparty](https://crates.io/crates/afterparty).
//!
//! Current version of rifling supports [hyper 0.12](https://crates.io/crates/hyper) only.
//!
//! It supports both `application/json` and `application/x-www-form-urlencoded` mode.
//!
//! Minimal Example:
//!
//! ```no_run
//! extern crate hyper;
//! extern crate rifling;
//!
//! use rifling::{Constructor, Delivery, Hook};
//! use hyper::{Server, Error};
//! use hyper::rt::{run, Future};
//!
//! fn main() {
//!     let mut cons = Constructor::new();
//!     let hook = Hook::new("*", Some(String::from("secret")), |delivery: &Delivery| println!("Received delivery: {:?}", delivery));
//!     cons.register(hook);
//!     let addr = "0.0.0.0:4567".parse().unwrap();
//!     let server = Server::bind(&addr).serve(cons).map_err(|e: Error| println!("Error: {:?}", e));
//!     run(server);
//! }
//! ```
//!
//! TODO in future versions:
//!  - Error handling.
//!  - Support other web frameworks (such as Tide).

extern crate hex;
#[macro_use]
extern crate log;
#[cfg(feature = "hyper-support")]
extern crate futures;
#[cfg(feature = "crypto-use-rustcrypto")]
extern crate hmac;
#[cfg(feature = "hyper-support")]
extern crate hyper;
#[cfg(feature = "crypto-use-ring")]
extern crate ring;
#[cfg(feature = "parse")]
extern crate serde_json;
#[cfg(feature = "crypto-use-rustcrypto")]
extern crate sha1;
extern crate url;

pub mod handler;
pub mod hook;

pub use handler::Constructor;
pub use handler::Delivery;
pub use handler::Handler;
pub use hook::Hook;
pub use hook::HookFunc;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
