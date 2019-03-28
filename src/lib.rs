//! Rifling the GitHub webhook listener library

#[macro_use]
extern crate log;
extern crate hyper;

use futures::stream::Stream;
use futures::{future, Future};
use hyper::service::{NewService, Service};
use hyper::{Body, Error, Request, Response, Server, StatusCode};

use std::collections::HashMap;

type HookRegistry<F> = HashMap<String, Hook<F>>;

macro_rules! hooks_find_match {
    ($results:expr, $source:expr, $($pattern:expr), *) => {
        $(
            if let Some(hook) = $source.get($pattern) {
                $results.push(hook.clone())
            }
        )*
    }
}

#[derive(Default, Debug, Clone)]
pub struct Delivery<'a> {
    pub id: &'a str,
    pub event: &'a str,
    pub unparsed_payload: &'a str,
    pub signature: Option<&'a str>,
}

#[derive(Clone)]
pub struct Hook<F>
where
    F: Fn(&Delivery) + Clone + Send + 'static,
{
    event: &'static str,
    secret: Option<&'static str>,
    func: Box<F>,
}

#[derive(Clone)]
pub struct Constructor<F>
where
    F: Fn(&Delivery) + Clone + Send + 'static,
{
    hooks: HookRegistry<F>,
}

pub struct Handler<F>
where
    F: Fn(&Delivery) + Clone + Send + 'static,
{
    hooks: HookRegistry<F>,
}

impl<F> Constructor<F>
where
    F: Fn(&Delivery) + Clone + Send + 'static,
{
    pub fn new() -> Constructor<F> {
        Constructor {
            hooks: HashMap::new(),
        }
    }

    pub fn register(&mut self, hook: Hook<F>) {
        self.hooks.insert(hook.event.to_string(), hook.clone());
    }
}

impl<F> Hook<F>
where
    F: Fn(&Delivery) + Clone + Send + 'static,
{
    pub fn new(event: &'static str, secret: Option<&'static str>, func: F) -> Self {
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

    fn run(self, delivery: &Delivery) {
        let func = self.func;
        func(delivery);
    }

    fn handle_delivery(self, delivery: &Delivery) {
        if self.auth(delivery) {
            self.run(delivery)
        }
    }
}

impl<F> Handler<F>
where
    F: Fn(&Delivery) + Clone + Send + 'static,
{
    fn run_hooks(&self, delivery: &Delivery) {
        debug!("Handling '{}' event", delivery.event);
        let mut matched: Vec<Hook<F>> = Vec::new();
        hooks_find_match!(matched, self.hooks, delivery.event, "*");
        if matched.len() > 0 {
            for hook in matched {
                hook.handle_delivery(delivery);
            }
            info!("All matched hooks have been executed");
        } else {
            info!("No matched hook found, ignoring...");
        }
    }
}

impl<F> Service for Handler<F>
where
    F: Fn(&Delivery) + Clone + Send + 'static,
{
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Error;
    type Future = Box<Future<Item = Response<Body>, Error = Error> + Send>;

    fn call(&mut self, req: Request<Self::ReqBody>) -> Self::Future {
        info!("Executed!");
        let delivery = Delivery {
            id: "test ID",
            event: "push",
            unparsed_payload: "{event: \"push\"}",
            signature: None,
        };
        self.run_hooks(&delivery);
        Box::new(future::ok(
            Response::builder()
                .status(StatusCode::OK)
                .body("Bla!".into())
                .unwrap(),
        ))
    }
}

impl<F> From<&Constructor<F>> for Handler<F>
where
    F: Fn(&Delivery) + Clone + Send + 'static,
{
    fn from(constructor: &Constructor<F>) -> Self {
        Self {
            hooks: constructor.hooks.clone(),
        }
    }
}

impl<F> NewService for Constructor<F>
where
    F: Fn(&Delivery) + Clone + Send + 'static,
{
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Error;
    type Service = Handler<F>;
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
