//! The handler of requests
//!
//! The `Handler` struct should be created automatically by constructor.
//!
//! Currently handler only supports `Hyper`, it is possible to make it support other frameworks.
//!
//! This part of the library shouldn't be used in most cases.

use futures::stream::Stream;
use futures::{future, Future};
use hyper::service::Service;
use hyper::{Body, Error, Request, Response, StatusCode};
use url::form_urlencoded;

use std::collections::HashMap;

use super::constructor::{Constructor, HookRegistry};
use super::hook::Hook;

/// Find matched hooks from `HookRegistry`, accepting multiple keys.
#[macro_export]
macro_rules! hooks_find_match {
    ($results:expr, $source:expr, $($pattern:expr), *) => {
        $(
            if let Some(hook) = $source.get($pattern) {
                $results.push(hook.clone())
            }
        )*
    }
}

/// Get Option<String> typed header value from HeaderMap<HeaderValue> of hyper.
macro_rules! hyper_get_header_value {
    ($headers:expr, $key:expr) => {
        if let Some(value) = $headers.get($key) {
            if let Ok(inner) = value.to_str() {
                Some(String::from(inner.clone()))
            } else {
                None
            }
        } else {
            None
        }
    };
}

/// Information gathered from the received request
/// Not sure what is included in the request, so all of the fields are wrapped in `Option<T>`
#[derive(Default, Debug, Clone)]
pub struct Delivery {
    pub id: Option<String>,
    pub event: Option<String>,
    pub unparsed_payload: Option<String>,
    pub request_body: Option<String>, // for x-www-form-urlencoded authentication support
    pub signature: Option<String>,
}

/// (Private) Executor of the hooks, passed into futures.
struct Executor {
    matched_hooks: Vec<Hook>,
}

/// The main handler struct.
pub struct Handler {
    hooks: HookRegistry,
}

/// The main impl clause of `Delivery`
impl Delivery {
    /// Create a new Delivery
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

/// The main impl clause of `Executor`
impl Executor {
    /// Run the hooks
    fn run(self, delivery: Delivery) {
        for hook in self.matched_hooks {
            debug!("Running hook for '{}' event", &hook.event);
            hook.handle_delivery(&delivery);
        }
    }

    /// Test if there are no matched hook found
    fn is_empty(&self) -> bool {
        self.matched_hooks.len() <= 0
    }
}

/// The main impl clause of Handler
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

/// Implement `From<&Constructor>` trait for `Handler`
impl From<&Constructor> for Handler {
    /// Create a handler object from constructor
    fn from(constructor: &Constructor) -> Self {
        Self {
            hooks: constructor.hooks.clone(),
        }
    }
}

/// Implement `Service` struct from `Hyper` to `Handler`
impl Service for Handler {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Error;
    type Future = Box<Future<Item = Response<Body>, Error = Error> + Send + 'static>;

    /// Handle the request
    ///
    /// It can definitely be simplified, and it's ugly, but it can work.
    fn call(&mut self, req: Request<Self::ReqBody>) -> Self::Future {
        fn response(status_code: StatusCode, body: &'static str) -> Response<Body> {
            Response::builder()
                .status(status_code)
                .body(body.into())
                .unwrap()
        }
        let headers = req.headers().clone();
        let event = if let Some(event_str) = hyper_get_header_value!(&headers, "X-Github-Event") {
            event_str.clone()
        } else {
            // Invalid payload without a event header
            return Box::new(future::ok(response(
                StatusCode::ACCEPTED,
                "Invalid payload",
            )));
        };
        let executor = self.get_hooks(&event);
        if executor.is_empty() {
            // No matched hook found
            return Box::new(future::ok(response(
                StatusCode::ACCEPTED,
                "No matched hook configured",
            )));
        }
        let id = hyper_get_header_value!(&headers, "X-Github-Delivery");
        let signature = hyper_get_header_value!(&headers, "X-Hub-Signature");
        let content_type = hyper_get_header_value!(&headers, "content-type");
        if let None = content_type.clone() {
            // No valid content-type header found
            return Box::new(future::ok(response(
                StatusCode::ACCEPTED,
                "Invalid payload",
            )));
        }
        Box::new(
            req.into_body()
                .concat2()
                .map(move |chunk| {
                    let request_body = String::from_utf8(chunk.to_vec()).ok();
                    match content_type.unwrap().as_str() {
                        "application/json" => (request_body.clone(), request_body),
                        "application/x-www-form-urlencoded" => {
                            let params = form_urlencoded::parse(chunk.as_ref())
                                .into_owned()
                                .collect::<HashMap<String, String>>();
                            let payload = if let Some(payload_string) = params.get("payload") {
                                Some(payload_string.clone())
                            } else {
                                None
                            };
                            (payload, request_body)
                        }
                        _ => (None, None),
                    }
                })
                .and_then(move |(payload, request_body)| {
                    if payload.is_some() {
                        let delivery =
                            Delivery::new(id, Some(event), signature, payload, request_body);
                        debug!("Received delivery: {:?}", &delivery);
                        executor.run(delivery);
                        future::ok(response(StatusCode::OK, "OK"))
                    } else {
                        future::ok(response(StatusCode::ACCEPTED, "Invalid payload"))
                    }
                }),
        )
    }
}
