//! Universal part between of different frameworks
//!
//! `Constructor` accepts settings from the user, and generates `Handler` from itself.
//!
//! The `Handler` struct should be created automatically by constructor, it is the actual handler of requests.

mod hyper;

use serde_json::Value;
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

pub enum ContentType {
    JSON,
    URLENCODED,
}

/// Constructor of the server
#[derive(Clone, Default)]
pub struct Constructor {
    pub hooks: HookRegistry,
}

/// Information gathered from the received request
/// Not sure what is included in the request, so all of the fields are wrapped in `Option<T>`
#[derive(Default, Debug, Clone)]
pub struct Delivery {
    pub id: Option<String>,
    pub event: Option<String>,
    pub payload: Option<Value>,
    pub unparsed_payload: Option<String>,
    pub request_body: Option<String>, // for x-www-form-urlencoded authentication support
    pub signature: Option<String>,
}

/// (Private) Executor of the hooks, passed into futures.
/// It should not be used outside of the crate.
struct Executor {
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
        id: Option<String>,
        event: Option<String>,
        signature: Option<String>,
        content_type: ContentType,
        payload_body: Option<String>,
    ) -> Delivery {
        let payload: Option<String> = match content_type {
            ContentType::JSON => payload_body.clone(),
            ContentType::URLENCODED => {
                if let Some(request_body_string) = &payload_body {
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
        };
        let parsed_payload = if let Some(payload_string) = &payload {
            serde_json::from_str(payload_string.as_str()).ok()
        } else {
            None
        };
        Self {
            id,
            event,
            payload: parsed_payload,
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
        self.matched_hooks.len() == 0
    }
}

/// The main impl clause of Handler
impl Handler {
    fn get_hooks(&self, event: &str) -> Executor {
        debug!("Finding matched hooks for '{}' event", event);
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
        Self {
            hooks: constructor.hooks.clone(),
        }
    }
}
