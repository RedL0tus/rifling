//! Implementation of Hyper's service-related traits for constructor and handler
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

use futures::stream::Stream;
use futures::{future, Future};
use hyper::service::{NewService, Service};
use hyper::{Body, Error, Request, Response, StatusCode};

use super::Constructor;
use super::ContentType;
use super::Delivery;
use super::DeliveryType;
use super::Handler;

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
        debug!("Creating new service");
        Box::new(future::ok(Handler::from(self)))
    }
}

/// Implement `Service` struct from `Hyper` to `Handler`
impl Service for Handler {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Error;
    type Future = Box<Future<Item = Response<Body>, Error = Error> + Send + 'static>;

    /// Handle the request
    fn call(&mut self, req: Request<Self::ReqBody>) -> Self::Future {
        fn response(status_code: StatusCode, body: &'static str) -> Response<Body> {
            Response::builder()
                .status(status_code)
                .body(body.into())
                .unwrap()
        }
        let headers = req.headers();
        let (mut event, delivery_type) =
            if let Some(event_string) = hyper_get_header_value!(&headers, "X-Github-Event") {
                (event_string.clone(), DeliveryType::GitHub)
            } else if let Some(event_string) = hyper_get_header_value!(&headers, "X-Gitlab-Event") {
                (event_string.clone(), DeliveryType::GitLab)
            } else {
                // Invalid payload without a event header
                return Box::new(future::ok(response(
                    StatusCode::ACCEPTED,
                    "Invalid payload",
                )));
            };
        event.make_ascii_lowercase();
        event = event.replace(" ", "_");
        let executor = self.get_hooks(event.as_str());
        if executor.is_empty() {
            // No matched hook found
            return Box::new(future::ok(response(
                StatusCode::ACCEPTED,
                "No matched hook configured",
            )));
        }
        let id = hyper_get_header_value!(&headers, "X-Github-Delivery");
        let signature = match delivery_type {
            DeliveryType::GitHub => hyper_get_header_value!(&headers, "X-Hub-Signature"),
            DeliveryType::GitLab => hyper_get_header_value!(&headers, "X-Gitlab-Token"),
        };
        let content_type = hyper_get_header_value!(&headers, "content-type");
        if content_type.is_none() {
            // No valid content-type header found
            return Box::new(future::ok(response(
                StatusCode::ACCEPTED,
                "Invalid payload",
            )));
        }
        Box::new(
            req.into_body()
                .concat2()
                .map(move |chunk| String::from_utf8(chunk.to_vec()).ok())
                .and_then(move |request_body| {
                    let content_type: ContentType = match content_type.unwrap().as_str() {
                        "application/x-www-form-urlencoded" => ContentType::URLENCODED,
                        _ => ContentType::JSON, // Default
                    };
                    if request_body.is_some() {
                        let delivery = Delivery::new(
                            delivery_type,
                            id,
                            Some(event),
                            signature,
                            content_type,
                            request_body,
                        );
                        debug!("Received delivery: {:#?}", &delivery);
                        executor.run(delivery);
                        future::ok(response(StatusCode::OK, "OK"))
                    } else {
                        future::ok(response(StatusCode::ACCEPTED, "Invalid payload"))
                    }
                }),
        )
    }
}
