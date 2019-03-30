//! Rifling the GitHub webhook listener library

extern crate hex;
#[macro_use]
extern crate log;
extern crate hyper;
extern crate ring;
extern crate url;

use futures::stream::Stream;
use futures::{future, Future};
use hex::FromHex;
use hyper::service::{NewService, Service};
use hyper::{Body, Error, Request, Response, StatusCode};
use ring::digest;
use ring::hmac;
use url::form_urlencoded;

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
    pub unparsed_payload: Option<String>,
    pub request_body: Option<String>, // for x-www-form-urlencoded authentication support
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
    fn new(
        id: Option<String>,
        event: Option<String>,
        signature: Option<String>,
        payload: Option<String>,
        payload_body: Option<String>,
    ) -> Delivery {
        // TODO: Add functionality to parse the payload
        Self {
            id,
            event,
            unparsed_payload: payload,
            request_body: payload_body,
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
                if let Some(request) = &delivery.request_body {
                    let signature_hex = signature[5..signature.len()].as_bytes();
                    if let Ok(signature_bytes) = Vec::from_hex(signature_hex) {
                        let secret_bytes = secret.as_bytes();
                        let request_bytes = request.as_bytes();
                        let key = hmac::SigningKey::new(&digest::SHA1, &secret_bytes);
                        return hmac::verify_with_own_key(&key, &request_bytes, &signature_bytes)
                            .is_ok();
                    }
                }
            }
            return false;
        }
        return true;
    }

    fn handle_delivery(self, delivery: &Delivery) {
        if self.auth(delivery) {
            debug!("Valid payload found");
            self.func.run(delivery)
        }
        debug!("Invalid payload");
    }
}

impl Executor {
    fn run(self, delivery: Delivery) {
        for hook in self.matched_hooks {
            debug!("Running hook for '{}' event", &hook.event);
            hook.handle_delivery(&delivery);
        }
    }

    fn is_empty(&self) -> bool {
        self.matched_hooks.len() <= 0
    }
}

impl Handler {
    fn get_hooks(&self, event: &str) -> Executor {
        debug!("Finding macthed hooks for '{}' event", event);
        let mut matched: Vec<Hook> = Vec::new();
        hooks_find_match!(matched, self.hooks, event, "*");
        debug!("{} matched hook(s) found", matched.len());
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
        let event = get_header_value!(headers, "X-Github-Event");
        if let Some(event_string) = event {
            let executor = self.get_hooks(event_string.as_str());
            if executor.is_empty() {
                return Box::new(future::ok(
                    Response::builder()
                        .status(StatusCode::ACCEPTED)
                        .body("No matched hook found".into())
                        .unwrap(),
                ));
            }
            let id = get_header_value!(&headers, "X-Github-Delivery");
            let signature = get_header_value!(&headers, "X-Hub-Signature");
            if let Some(content_type) = get_header_value!(&headers, "content-type") {
                Box::new(
                    req.into_body()
                        .concat2()
                        .map(move |chunk| match content_type.as_str() {
                            "application/json" => {
                                if let Ok(payload) = String::from_utf8(chunk.to_vec()) {
                                    let request_body = payload.clone();
                                    (Some(payload), Some(request_body))
                                } else {
                                    (None, None)
                                }
                            }
                            "application/x-www-form-urlencoded" => {
                                let request_body = if let Ok(payload_body) =
                                    String::from_utf8(chunk.to_vec().clone())
                                {
                                    Some(payload_body)
                                } else {
                                    None
                                };
                                let params = form_urlencoded::parse(chunk.as_ref())
                                    .into_owned()
                                    .collect::<HashMap<String, String>>();
                                if let Some(payload) = params.get("payload") {
                                    (Some(payload.clone()), request_body)
                                } else {
                                    (None, request_body)
                                }
                            }
                            _ => (None, None),
                        })
                        .and_then(move |(payload, request_body)| {
                            if payload.is_some() {
                                let delivery = Delivery::new(
                                    id,
                                    Some(event_string),
                                    signature,
                                    payload,
                                    request_body,
                                );
                                debug!("Received delivery: {:?}", &delivery);
                                executor.run(delivery);
                                future::ok(
                                    Response::builder()
                                        .status(StatusCode::OK)
                                        .body("OK".into())
                                        .unwrap(),
                                )
                            } else {
                                future::ok(
                                    Response::builder()
                                        .status(StatusCode::ACCEPTED)
                                        .body("Invalid payload".into())
                                        .unwrap(),
                                )
                            }
                        }),
                )
            } else {
                Box::new(future::ok(
                    Response::builder()
                        .status(StatusCode::ACCEPTED)
                        .body("Invalid payload".into())
                        .unwrap(),
                ))
            }
        } else {
            Box::new(future::ok(
                Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .body("What are you doing here?".into())
                    .unwrap(),
            ))
        }
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
    use ring::digest;
    use ring::hmac;

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn payload_authentication() {
        let secret = "secret";
        let hook = Hook::new("*", Some(secret.clone()), |_: &Delivery| {});
        let payload = String::from(r#"{"zen": "Bazinga!"}"#);
        let request_body = payload.clone();
        let secret_bytes = secret.as_bytes();
        let request_bytes = request_body.as_bytes();
        let key = hmac::SigningKey::new(&digest::SHA1, &secret_bytes);
        let mut signature = String::new();
        hmac::sign(&key, &request_bytes)
            .as_ref()
            .write_hex(&mut signature)
            .unwrap();
        let signature_field = String::from(format!("sha1={}", signature));
        let delivery = Delivery {
            id: None,
            event: Some(String::from("push")),
            unparsed_payload: Some(payload),
            request_body: Some(request_body),
            signature: Some(signature_field),
        };
        assert!(hook.auth(&delivery));
    }
}
