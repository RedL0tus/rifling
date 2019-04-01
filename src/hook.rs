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

use super::handler::Delivery;

/// The part of the hook that will be executed after validating the payload
pub trait HookFunc: HookFuncClone + Sync + Send {
    fn run(&self, delivery: &Delivery);
}

/// To let `Clone` trait work for trait object, an extra trait like this is necessary.
/// Inspired by https://stackoverflow.com/a/30353928
pub trait HookFuncClone {
    fn clone_box(&self) -> Box<HookFunc>;
}

/// The actual hook, contains the event it's going to listen, the secret to authenticate the payload, and the function to execute.
#[derive(Clone)]
pub struct Hook {
    pub event: &'static str,
    pub secret: Option<String>,
    pub func: Box<HookFunc>, // To allow the registration of multiple hooks, it has to be a trait object.
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

/// To make `HookFunc` trait object cloneable
impl<T> HookFuncClone for T
where
    T: HookFunc + Clone + 'static,
{
    /// Create a cloned boxed `HookFunc` object.
    fn clone_box(&self) -> Box<HookFunc> {
        Box::new(self.clone())
    }
}

/// To make `HookFunc` trait object cloneable
impl Clone for Box<HookFunc> {
    /// Use `clone_box()` to clone it self.
    fn clone(&self) -> Box<HookFunc> {
        self.clone_box()
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
            func: Box::new(func),
        }
    }

    /// Authenticate the payload
    pub fn auth(&self, delivery: &Delivery) -> bool {
        if let Some(secret) = &self.secret {
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

    /// Handle the request
    pub fn handle_delivery(self, delivery: &Delivery) {
        if self.auth(delivery) {
            debug!("Valid payload found");
            self.func.run(delivery)
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
            unparsed_payload: Some(payload),
            request_body: Some(request_body),
            signature: Some(signature_field),
        };
        assert!(hook.auth(&delivery));
    }
}
