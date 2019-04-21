//! Hook
//!
//! Hook is a struct that contains the information needed for validating the payload and the work after that.
//!
//! ## Example
//!
//! To create a Hook object, use the new method:
//!
//! ```
//! extern crate rifling;
//!
//! use rifling::{Hook, Delivery};
//!
//! // Create the hook
//! let hook = Hook::new("push", None, |_: &Delivery| println!("Pushed!"));
//! ```
//!
//! The last parameter is a trait object of the trait `HookFunc`, it's currently implemented to `Fn(&Delivery)`.
//! `Delivery` contains the information of the request received.
//!
//! To use the hook, you need to register it to the `Constructor`.

use hex::FromHex;
use ring::digest;
use ring::hmac;

use std::sync::Arc;

use super::handler::Delivery;

macro_rules! unwrap_or_false {
    ($e:expr) => {
        match $e {
            Some(content) => content,
            _ => return false,
        }
    };
}

/// The part of the hook that will be executed after validating the payload
pub trait HookFunc: Sync + Send {
    fn run(&self, delivery: &Delivery);
}

/// The actual hook, contains the event it's going to listen, the secret to authenticate the payload, and the function to execute.
#[derive(Clone)]
pub struct Hook {
    pub event: &'static str,
    pub secret: Option<String>,
    pub func: Arc<HookFunc>, // To allow the registration of multiple hooks, it has to be a trait object.
}

/// Implement `HookFunc` to `Fn(&Delivery)`.
impl<F> HookFunc for F
where
    F: Fn(&Delivery) + Clone + Sync + Send + 'static,
{
    /// Run the function
    fn run(&self, delivery: &Delivery) {
        self(delivery)
    }
}

/// Main impl clause of `Hook`()
impl Hook {
    /// Create a new hook
    ///
    /// Example:
    ///
    /// ```
    /// extern crate rifling;
    ///
    /// use rifling::{Hook, Delivery};
    ///
    /// let hook = Hook::new("push", None, |_: &Delivery| println!("Pushed!"));
    /// ```
    pub fn new(event: &'static str, secret: Option<String>, func: impl HookFunc + 'static) -> Self {
        Self {
            event,
            secret,
            func: Arc::new(func),
        }
    }

    /// Authenticate the payload
    pub fn auth(&self, delivery: &Delivery) -> bool {
        if let Some(secret) = &self.secret {
            let signature = unwrap_or_false!(&delivery.signature);
            debug!("Signature: {}", signature);
            let request_body = unwrap_or_false!(&delivery.request_body);
            debug!("Request body: {}", &request_body);
            let signature_hex = signature[5..signature.len()].as_bytes();
            if let Ok(signature_bytes) = Vec::from_hex(signature_hex) {
                let secret_bytes = secret.as_bytes();
                let request_body_bytes = request_body.as_bytes();
                let key = hmac::SigningKey::new(&digest::SHA1, &secret_bytes);
                debug!("Validating payload with secret");
                return hmac::verify_with_own_key(&key, &request_body_bytes, &signature_bytes)
                    .is_ok();
            }
            debug!("Invalid signature");
            return false;
        } else {
            debug!("No secret given, passing...");
            return true;
        }
    }

    /// Handle the request
    pub fn handle_delivery(self, delivery: &Delivery) {
        if self.auth(delivery) {
            debug!("Valid payload found");
            self.func.run(delivery);
        }
        debug!("Invalid payload");
    }
}

#[cfg(test)]
mod tests {
    use super::super::handler::Delivery;
    use super::*;
    use hex::ToHex;
    use ring::digest;
    use ring::hmac;

    /// Test payload authentication: Valid signature
    #[test]
    fn payload_authentication() {
        let secret = String::from("secret");
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
            payload: None,
            unparsed_payload: Some(payload),
            request_body: Some(request_body),
            signature: Some(signature_field),
        };
        assert!(hook.auth(&delivery));
    }

    /// Test payload authentication: Invalid signature
    #[test]
    fn payload_authentication_fail() {
        let secret = String::from("secret");
        let hook = Hook::new("*", Some(secret.clone()), |_: &Delivery| {});
        let payload = String::from(r#"{"zen": "Another test!"}"#);
        let signature = String::from("sha1=ec760ee6d10bf638089f078b5a0c23f6575821e7");
        let delivery = Delivery {
            id: None,
            event: Some(String::from("push")),
            payload: None,
            unparsed_payload: Some(payload.clone()),
            request_body: Some(payload),
            signature: Some(signature),
        };
        assert_eq!(hook.auth(&delivery), false);
    }
}
