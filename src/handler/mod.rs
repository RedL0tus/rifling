//! Universal part between of different frameworks
//!
//! `Constructor` accepts settings from the user, and generates `Handler` from itself.
//!
//! The `Handler` struct should be created automatically by constructor, it is the actual handler of requests.

#[cfg(feature = "hyper-support")]
mod hyper;

#[cfg(feature = "parse")]
use serde_json::Value;
#[cfg(feature = "content-type-urlencoded")]
use url::form_urlencoded;

use std::collections::HashMap;

use super::hook::Hook;

/// Registry of hooks
pub type HookRegistry = HashMap<String, Hook>;

/// Find matched hooks from `HookRegistry`, accepting multiple keys.
#[macro_export]
macro_rules! hooks_find_match {
    ($source:expr, $($pattern:expr), *) => {{
        let mut result: Vec<Hook> = Vec::new();
        $(
            if let Some(hook) = $source.get($pattern) {
                result.push(hook.clone());
            }
        )*
        result
    }};
}

macro_rules! header_get_owned {
    ($headers:expr, $key:expr) => {
        if let Some(header_value) = $headers.get($key) {
            Some(header_value.to_owned())
        } else {
            None
        }
    };
}

/// Type of content
#[derive(Clone, Debug)]
pub enum ContentType {
    JSON,
    URLENCODED,
}

/// Source of the delivery
#[derive(Clone, Debug)]
pub enum DeliveryType {
    GitHub,
    GitLab,
    DockerHub,
}

#[cfg(not(feature = "parse"))]
#[doc(hidden)]
#[derive(Debug, Clone)]
pub enum Value {}

/// Constructor of the server
#[derive(Clone, Default)]
pub struct Constructor {
    pub hooks: HookRegistry,
}

/// Information gathered from the received request
/// Not sure what is included in the request, so all of the fields are wrapped in `Option<T>`
#[derive(Debug, Clone)]
pub struct Delivery {
    pub delivery_type: DeliveryType,
    pub content_type: ContentType,
    pub id: Option<String>,
    pub event: String,
    pub payload: Option<Value>,
    pub unparsed_payload: Option<String>,
    pub request_body: Option<String>, // for x-www-form-urlencoded authentication support
    pub signature: Option<String>,
}

/// Executor of the hooks, passed into futures.
pub struct Executor {
    matched_hooks: Vec<Hook>,
}

/// The main handler struct.
pub struct Handler {
    hooks: HookRegistry,
}

/// Main impl clause of the `Constructor`
impl Constructor {
    /// Create a new, empty `Constructor`
    pub fn new() -> Constructor {
        Constructor {
            ..Default::default()
        }
    }

    /// Register a hook to `Constructor`
    pub fn register(&mut self, hook: Hook) {
        self.hooks.insert(hook.event.to_string(), hook.clone());
    }
}

/// The main impl clause of `Delivery`
impl Delivery {
    /// Create a new Delivery
    pub fn new(
        headers: HashMap<String, String>,
        request_body: Option<String>,
    ) -> Result<Delivery, &'static str> {
        debug!("Received headers: {:#?}", &headers);
        // Identify delivery type
        let (mut event, delivery_type) = if let Some(event_string) = headers.get("x-github-event") {
            (event_string.to_owned(), DeliveryType::GitHub)
        } else if let Some(event_string) = headers.get("x-gitlab-event") {
            (event_string.to_owned(), DeliveryType::GitLab)
        } else if let Some(newrelic_id) = headers.get("x-newrelic-id") {
            // Determine source of delivery by NewRelic ID
            if newrelic_id == &"UQUFVFJUGwUJVlhaBgY=".to_string() {
                ("docker_push".to_string(), DeliveryType::DockerHub)
            } else {
                return Err("Could not determine delivery type");
            }
        } else {
            return Err("Could not determine delivery type");
        };
        event.make_ascii_lowercase();
        event = event.replace(" ", "_");
        // Get content type
        let content_type = if let Some(header_value) = headers.get("content-type") {
            match header_value.to_lowercase().as_str() {
                "application/json" => ContentType::JSON,
                "application/x-www-form-urlencoded" => ContentType::URLENCODED,
                _ => ContentType::JSON,
            }
        } else {
            ContentType::JSON
        };
        // Get delivery ID: only available in requests from GitHub
        let id = match delivery_type {
            DeliveryType::GitHub => header_get_owned!(&headers, "x-github-delivery"),
            _ => None,
        };
        let signature = match delivery_type {
            DeliveryType::GitHub => header_get_owned!(&headers, "x-hub-signature"),
            DeliveryType::GitLab => header_get_owned!(&headers, "x-gitlab-token"),
            _ => None,
        };
        let mut delivery = Self {
            delivery_type,
            content_type,
            id,
            event,
            payload: None,
            unparsed_payload: None,
            request_body: None,
            signature,
        };
        if request_body.is_some() {
            delivery.update_request_body(request_body);
        }
        Ok(delivery)
    }

    /// Update request body of the delivery
    pub fn update_request_body(&mut self, request_body: Option<String>) {
        let payload: Option<String> = match self.content_type {
            ContentType::JSON => request_body.clone(),
            #[cfg(feature = "content-type-urlencoded")]
            ContentType::URLENCODED => {
                if let Some(request_body_string) = request_body.clone() {
                    if let Some(payload_string) =
                        form_urlencoded::parse(request_body_string.as_bytes())
                            .into_owned()
                            .collect::<HashMap<String, String>>()
                            .get("payload")
                    {
                        Some(payload_string.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            #[cfg(not(feature = "content-type-urlencoded"))]
            _ => None,
        };
        debug!("Payload body set to: {:?}", &payload);
        #[cfg(feature = "parse")]
        let parsed_payload = if let Some(payload_string) = &payload {
            serde_json::from_str(payload_string.as_str()).ok()
        } else {
            None
        };
        #[cfg(not(feature = "parse"))]
        let parsed_payload = None;
        debug!("Parsed payload: {:#?}", &parsed_payload);
        // Update delivery
        self.request_body = request_body;
        self.unparsed_payload = payload;
        self.payload = parsed_payload;
    }
}

/// The main impl clause of `Executor`
impl Executor {
    /// Run the hooks
    pub fn run(self, delivery: Delivery) {
        for hook in self.matched_hooks {
            debug!("Running hook for '{}' event", &hook.event);
            hook.handle_delivery(&delivery);
        }
    }

    /// Test if there are no matched hook found
    pub fn is_empty(&self) -> bool {
        self.matched_hooks.len() == 0
    }
}

/// The main impl clause of Handler
impl Handler {
    fn get_hooks(&self, event: &str) -> Executor {
        debug!("Finding matched hooks for '{}' event", &event);
        let matched: Vec<Hook> = hooks_find_match!(self.hooks, event, "*");
        debug!("{} matched hook(s) found", matched.len());
        Executor {
            matched_hooks: matched,
        }
    }
}

/// Implement `From<&Constructor>` trait for `Handler`
/// As currently we don't have Generic Associate Types, I can only clone the registry.
impl From<&Constructor> for Handler {
    /// Create a handler object from constructor
    fn from(constructor: &Constructor) -> Self {
        debug!("Handler constructed");
        Self {
            hooks: constructor.hooks.clone(),
        }
    }
}
