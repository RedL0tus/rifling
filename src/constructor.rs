//! Constructor of the service
//!
//! `Constructor` accepts settings from the user, and generates `Handler` from itself.
//!
//! In hyper, `Constructor` can be served using `Server::bind(&addr).serve(constructor)`.
//!
//! Example:
//!
//! ```
//! extern crate rifling;
//! extern crate hyper;
//!
//! use rifling::Constructor;
//!
//! let _ = hyper::Server::bind(&"0.0.0.0:4567".parse().unwrap()).serve(Constructor::new());
//! ```

use futures::{future, Future};
use hyper::service::NewService;
use hyper::{Body, Error};

use std::collections::HashMap;

use super::handler::Handler;
use super::hook::Hook;

/// Registry of hooks
pub type HookRegistry = HashMap<String, Hook>;

/// Constructor of the server
#[derive(Clone)]
pub struct Constructor {
    pub hooks: HookRegistry,
}

/// Main impl clause of the `Constructor`
impl Constructor {
    /// Create a new, empty `Constructor`
    pub fn new() -> Constructor {
        Constructor {
            hooks: HashMap::new(),
        }
    }

    /// Register a hook to `Constructor`
    pub fn register(&mut self, hook: Hook) {
        self.hooks.insert(hook.event.to_string(), hook.clone());
    }
}

/// Implement `NewService` trait to `Constructor`
impl NewService for Constructor {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Error;
    type Service = Handler;
    type Future = Box<Future<Item = Self::Service, Error = Self::InitError> + Send>;
    type InitError = Error;

    /// Create a new handler to handle the service
    fn new_service(&self) -> Self::Future {
        Box::new(future::ok(Handler::from(self)))
    }
}
