//! Rifling the GitHub webhook listener library

extern crate hex;
#[macro_use]
extern crate log;
extern crate ring;
extern crate hyper;

use hex::FromHex;
use ring::hmac;
use ring::digest;
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

macro_rules! get_header_value {
    ($headers:expr, $key:expr) => {
        if let Some(value) = $headers.get($key) {
            if let Ok(str) = value.to_str() {
                Some(String::from(str.clone()))
            } else {
                None
            }
        } else {
            None
        }
    };
}

pub trait HookFunc: HookFuncClone + Sync + Send {
    fn run(&self, delivery: &Delivery);
}

// Inspired by https://stackoverflow.com/a/30353928
pub trait HookFuncClone {
    fn clone_box(&self) -> Box<HookFunc>;
}

#[derive(Default, Debug, Clone)]
pub struct Delivery {
    pub id: Option<String>,
    pub event: Option<String>,
    pub unparsed_payload: String,
    pub signature: Option<String>,
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

impl Delivery {
    fn generate(headers: HeaderMap<HeaderValue>, body: String) -> Delivery {
        let id = get_header_value!(&headers, "X-Github-Delivery");
        let event = get_header_value!(&headers, "X-Github-Event");
        let signature = get_header_value!(&headers, "X-Hub-Signature");
        // TODO: Add functionality to parse the payload
        Self {
            id,
            event,
            unparsed_payload: body.clone(),
            signature,
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

    pub fn auth(&self, delivery: &Delivery) -> bool {
        if let Some(secret) = self.secret {
            if let Some(signature) = &delivery.signature {
                let prefix = signature[5..signature.len()].as_bytes();
                if let Ok(sigbytes) =  Vec::from_hex(prefix) {
                    let sbytes = secret.as_bytes();
                    let pbytes = delivery.unparsed_payload.as_bytes();
                    let key = hmac::SigningKey::new(&digest::SHA1, &sbytes);
                    return hmac::verify_with_own_key(&key, &pbytes, &sigbytes).is_ok();
                }
            }
        }
        false
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
                    executor.run(Delivery::generate(headers, body));
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
    use super::*;
    use hex::ToHex;
    use ring::hmac;
    use ring::digest;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn payload_authentication() {
        let secret = "secret";
        let hook = Hook::new("*", Some(secret.clone()), |_: &Delivery| {});
        let payload = r#"{"zen": "Bazinga!"}"#;
        let sbytes = secret.as_bytes();
        let pbytes = payload.as_bytes();
        let key = hmac::SigningKey::new(&digest::SHA1, &sbytes);
        let mut signature = String::new();
        hmac::sign(&key, &pbytes).as_ref().write_hex(&mut signature).unwrap();
        let signature_field = String::from(format!("sha1={}", signature));
        let delivery = Delivery {
            id: None,
            event: Some(String::from("push")),
            unparsed_payload: String::from(payload),
            signature: Some(signature_field)
        };
        assert!(hook.auth(&delivery));
    }
}
