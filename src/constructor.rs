use futures::{future, Future};
use hyper::service::NewService;
use hyper::{Body, Error};

use std::collections::HashMap;

use super::handler::Handler;
use super::hook::Hook;

pub type HookRegistry = HashMap<String, Hook>;

#[derive(Clone)]
pub struct Constructor {
    pub hooks: HookRegistry,
}

impl Constructor {
    pub fn new() -> Constructor {
        Constructor {
            hooks: HashMap::new(),
        }
    }

    pub fn register(&mut self, hook: Hook) {
        self.hooks.insert(hook.event.to_string(), hook.clone());
    }
}

impl NewService for Constructor {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Error;
    type Service = Handler;
    type Future = Box<Future<Item = Self::Service, Error = Self::InitError> + Send>;
    type InitError = Error;

    fn new_service(&self) -> Self::Future {
        Box::new(future::ok(Handler::from(self)))
    }
}
