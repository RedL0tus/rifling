//! Rifling the GitHub webhook listener library

#[macro_use]
extern crate log;
extern crate actix_web;

use actix_web::{http, server, App, Request, HttpResponse, Responder, Result};
use actix_web::server::{IntoHttpHandler, HttpHandler, HttpHandlerTask};

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
    where F: Fn(&Delivery) + Clone + 'static
{
    event: &'static str,
    secret: Option<&'static str>,
    func: Box<F>,
}

#[derive(Clone)]
pub struct Constructor<F>
    where F: Fn(&Delivery) + Clone + 'static
{
    hooks: HookRegistry<F>
}

pub struct Handler<F>
    where F: Fn(&Delivery) + Clone + 'static
{
    hooks: HookRegistry<F>
}

impl<F> Constructor<F>
    where F: Fn(&Delivery) + Clone + 'static
{
    pub fn new() -> Constructor<F> {
        Constructor{
            hooks: HashMap::new()
        }
    }
}

impl<F> Hook<F>
    where F: Fn(&Delivery) + Clone + 'static
{
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
    where F: Fn(&Delivery) + Clone + 'static
{
    fn run_hooks (&self, delivery: &Delivery) {
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

    fn from(constructor: Constructor<F>) -> Self {
        Self {
            hooks: constructor.hooks
        }
    }
}

impl<F> IntoHttpHandler for Constructor<F>
    where F: Fn(&Delivery) + Clone + 'static
{
    type Handler = Handler<F>;

    fn into_handler(self) -> Self::Handler {
        Handler::from(self)
    }
}

impl<F> HttpHandler for Handler<F>
    where F: Fn(&Delivery) + Clone + 'static
{
    type Task = Box<HttpHandlerTask>;

    fn handle(&self, req: Request) -> Result<Self::Task, Request> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
