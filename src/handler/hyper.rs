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

use std::collections::HashMap;

use super::Constructor;
use super::Delivery;
use super::Handler;

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
        let headers = req
            .headers()
            .clone()
            .into_iter()
            .map(|(name, content)| {
                let key = if let Some(header_name) = name {
                    header_name.as_str().to_string()
                } else {
                    "unknown".to_string().to_lowercase()
                };
                let value = if let Ok(header_value) = content.to_str() {
                    header_value.to_string()
                } else {
                    "unknown".to_string()
                };
                (key, value)
            })
            .collect::<HashMap<String, String>>();
        let mut delivery = match Delivery::new(headers, None) {
            Ok(delivery_inner) => delivery_inner,
            Err(err_msg) => return Box::new(future::ok(response(StatusCode::ACCEPTED, err_msg))),
        };
        let executor = self.get_hooks(delivery.event.as_str());
        if executor.is_empty() {
            // No matched hook found
            return Box::new(future::ok(response(
                StatusCode::ACCEPTED,
                "No matched hook configured",
            )));
        }
        Box::new(
            req.into_body()
                .concat2()
                .map(move |chunk| String::from_utf8(chunk.to_vec()).ok())
                .and_then(move |request_body| {
                    if request_body.is_some() {
                        delivery.update_request_body(request_body);
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
