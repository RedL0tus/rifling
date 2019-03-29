//! Rifling the GitHub webhook listener library

#[macro_use]
extern crate log;
extern crate hyper;

use futures::stream::Stream;
use futures::{future, Future};
use hyper::header::{HeaderMap, HeaderValue};
use hyper::service::{NewService, Service};
use hyper::{Body, Error, Request, Response, Server, StatusCode};

use std::collections::HashMap;

type HookRegistry = HashMap<String, Hook>;

macro_rules! hooks_find_match {
    ($results:expr, $source:expr, $($pattern:expr), *) => {
        $(
            if let Some(hook) = $source.get($pattern) {
                $results.push(hook.clone())
            }
        )*
    }
}

pub trait HookFunc: HookFuncClone + Sync + Send {
    fn run(&self, delivery: &Delivery);
}

// Inspired by https://stackoverflow.com/a/30353928
pub trait HookFuncClone {
    fn clone_box(&self) -> Box<HookFunc>;
}

#[derive(Default, Debug, Clone)]
pub struct Delivery<'a> {
    pub id: &'a str,
    pub event: &'a str,
    pub unparsed_payload: String,
    pub signature: Option<&'a str>,
}

#[derive(Clone)]
pub struct Hook {
    event: &'static str,
    secret: Option<&'static str>,
    func: Box<HookFunc>,
}

#[derive(Clone)]
pub struct Constructor {
    hooks: HookRegistry,
}

pub struct Handler {
    hooks: HookRegistry,
}

pub struct Executor {
    matched_hooks: Vec<Hook>,
}

impl<F> HookFunc for F
where
    F: Fn(&Delivery) + Clone + Sync + Send + 'static,
{
    fn run(&self, delivery: &Delivery) {
        self(delivery)
    }
}

impl<T> HookFuncClone for T
where
    T: HookFunc + Clone + 'static,
{
    fn clone_box(&self) -> Box<HookFunc> {
        Box::new(self.clone())
    }
}

impl Clone for Box<HookFunc> {
    fn clone(&self) -> Box<HookFunc> {
        self.clone_box()
    }
}

impl<'a> From<(HeaderMap<HeaderValue>, String)> for Delivery<'a> {
    fn from((headers, body): (HeaderMap<HeaderValue>, String)) -> Self {
        Self {
            id: "Unimplemented",
            event: "Unimplemented",
            unparsed_payload: body.clone(),
            signature: None,
        }
    }
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

impl Hook {
    pub fn new(
        event: &'static str,
        secret: Option<&'static str>,
        func: impl HookFunc + 'static,
    ) -> Self {
        Self {
            event,
            secret,
            func: Box::new(func),
        }
    }

    fn auth(&self, delivery: &Delivery) -> bool {
        if let Some(secret) = self.secret {
            true // Unimplemented
        } else {
            true
        }
    }

    fn handle_delivery(self, delivery: &Delivery) {
        if self.auth(delivery) {
            self.func.run(delivery)
        }
    }
}

impl Executor {
    fn run(self, delivery: Delivery) {
        for hook in self.matched_hooks {
            hook.handle_delivery(&delivery);
        }
    }
}

impl Handler {
    fn get_hooks(&self, event: &str) -> Executor {
        debug!("Handling '{}' event", event);
        let mut matched: Vec<Hook> = Vec::new();
        hooks_find_match!(matched, self.hooks, event, "*");
        Executor {
            matched_hooks: matched,
        }
    }
}

impl Service for Handler {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Error;
    type Future = Box<Future<Item = Response<Body>, Error = Error> + Send + 'static>;

    fn call(&mut self, req: Request<Self::ReqBody>) -> Self::Future {
        let headers = req.headers().clone();
        let event = "push"; // Unimplemented, it can be acquired from the headers
        let executor = self.get_hooks(event);
        Box::new(
            req.into_body()
                .concat2()
                .map(|chunk| String::from_utf8(chunk.to_vec()).unwrap())
                .and_then(move |body| {
                    executor.run(Delivery::from((headers, body)));
                    future::ok(
                        Response::builder()
                            .status(StatusCode::OK)
                            .body("Async bla!".into())
                            .unwrap(),
                    )
                }),
        )
    }
}

impl From<&Constructor> for Handler {
    fn from(constructor: &Constructor) -> Self {
        Self {
            hooks: constructor.hooks.clone(),
        }
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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
