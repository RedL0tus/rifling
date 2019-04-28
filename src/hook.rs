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

#[cfg(any(feature = "crypto-use-rustcrypto", feature = "crypto-use-ring"))]
use hex::FromHex;
#[cfg(feature = "crypto-use-rustcrypto")]
use hmac::{Hmac, Mac};
#[cfg(feature = "crypto-use-ring")]
use ring::digest;
#[cfg(feature = "crypto-use-ring")]
use ring::hmac;
#[cfg(feature = "crypto-use-rustcrypto")]
use sha1::Sha1;

use std::sync::Arc;

use super::handler::Delivery;
use super::handler::DeliveryType;

#[cfg(feature = "crypto-use-rustcrypto")]
type HmacSha1 = Hmac<Sha1>;

/// Unwrap `Option<T>` or return false
#[macro_export]
macro_rules! unwrap_or_false {
    ($e:expr) => {
        match $e {
            Some(content) => content,
            _ => return false,
        }
    };
}

/// The part of the hook that will be executed after validating the payload
/// You can implement this trait to your own struct
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

    #[cfg(feature = "crypto-use-ring")]
    /// Authenticate the payload from GitHub using `ring`
    pub fn auth_github(&self, delivery: &Delivery) -> bool {
        let secret = unwrap_or_false!(&self.secret);
        let signature = unwrap_or_false!(&delivery.signature);
        debug!("Received signature: {}", signature);
        let request_body = unwrap_or_false!(&delivery.request_body);
        debug!("Request body: {}", &request_body);
        let signature_hex = signature[5..signature.len()].as_bytes();
        if let Ok(signature_bytes) = Vec::from_hex(signature_hex) {
            let secret_bytes = secret.as_bytes();
            let request_body_bytes = request_body.as_bytes();
            let key = hmac::SigningKey::new(&digest::SHA1, &secret_bytes);
            debug!("Validating payload with given secret");
            return hmac::verify_with_own_key(&key, &request_body_bytes, &signature_bytes).is_ok();
        }
        debug!("Invalid signature");
        return false;
    }

    #[cfg(feature = "crypto-use-rustcrypto")]
    /// Authenticate the payload from GitHub using crates provided by RustCrypto team
    pub fn auth_github(&self, delivery: &Delivery) -> bool {
        let secret = unwrap_or_false!(&self.secret);
        let signature = unwrap_or_false!(&delivery.signature);
        debug!("Received signature: {}", &signature);
        let request_body = unwrap_or_false!(&delivery.request_body);
        debug!("Request body: {}", &request_body);
        let signature_hex = signature[5..signature.len()].as_bytes();
        if let Ok(signature_bytes) = Vec::from_hex(signature_hex) {
            let secret_bytes = secret.as_bytes();
            let request_body_bytes = request_body.as_bytes();
            let mut mac = unwrap_or_false!(HmacSha1::new_varkey(secret_bytes).ok());
            mac.input(request_body_bytes);
            debug!("Validating payload with given secret");
            return mac.verify(&signature_bytes).is_ok();
        }
        debug!("Invalid signature");
        return false;
    }

    #[cfg(all(
        not(feature = "crypto-use-rustcrypto"),
        not(feature = "crypto-use-ring")
    ))]
    /// With no cryptography library enabled, we are unable to authenticate payload.
    fn auth_github(&self, _delivery: &Delivery) -> bool {
        warn!(
            "Unable to authenticate GitHub payload due to lack of cryptography support, passing..."
        );
        true
    }

    /// Authenticate payload from GitLab, it does not require any cryptography algorithm
    fn auth_gitlab(&self, delivery: &Delivery) -> bool {
        let secret = unwrap_or_false!(&self.secret);
        let signature = unwrap_or_false!(&delivery.signature);
        debug!("Received token: {}", &signature);
        if signature == secret {
            true
        } else {
            debug!("Invalid token");
            false
        }
    }

    /// Authenticate payload
    pub fn auth(&self, delivery: &Delivery) -> bool {
        if self.secret.is_some() {
            match delivery.delivery_type {
                DeliveryType::GitHub => self.auth_github(delivery),
                DeliveryType::GitLab => self.auth_gitlab(delivery),
            }
        } else {
            debug!("No secret given, passing...");
            true
        }
    }

    /// Handle the request
    pub fn handle_delivery(self, delivery: &Delivery) {
        if self.auth(delivery) {
            debug!("Valid payload found");
            self.func.run(delivery);
            return;
        }
        debug!("Invalid payload");
    }
}

#[cfg(any(feature = "crypto-use-rustcrypto", feature = "crypto-use-ring"))]
#[cfg(test)]
mod tests {
    #[cfg(feature = "crypto-use-rustcrypto")]
    use super::HmacSha1;
    use super::*;
    use hex::ToHex;
    #[cfg(feature = "crypto-use-ring")]
    use ring::digest;
    #[cfg(feature = "crypto-use-ring")]
    use ring::hmac;
    use std::collections::HashMap;

    /// Test GitHub payload authentication with `ring`: Valid signature
    #[cfg(feature = "crypto-use-ring")]
    #[test]
    fn payload_authentication_github_ring() {
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
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("X-Github-Event".to_string(), "push".to_string());
        headers.insert("X-Hub-Signature".to_string(), signature_field);
        let delivery = Delivery::new(headers, Some(request_body));
        assert!(hook.auth(&delivery.unwrap()));
    }

    /// Test GitHub payload authentication with crates from RustCrypto team: Valid signature
    #[cfg(feature = "crypto-use-rustcrypto")]
    #[test]
    fn payload_authentication_github_rustcrypto() {
        let secret = String::from("secret");
        let hook = Hook::new("*", Some(secret.clone()), |_: &Delivery| {});
        let payload = String::from(r#"{"zen": "Bazinga!"}"#);
        let request_body = payload.clone();
        let secret_bytes = secret.as_bytes();
        let request_bytes = request_body.as_bytes();
        let mut mac = HmacSha1::new_varkey(&secret_bytes).expect("Invalid key");
        mac.input(&request_bytes);
        let mut signature = String::new();
        mac.result()
            .code()
            .as_ref()
            .write_hex(&mut signature)
            .expect("Invalid signature");
        let signature_field = String::from(format!("sha1={}", signature));
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("X-Github-Event".to_string(), "push".to_string());
        headers.insert("X-Hub-Signature".to_string(), signature_filed);
        let delivery = Delivery::new(headers, Some(request_body));
        assert!(hook.auth(&delivery.unwrap()));
        //assert!(true);
    }

    /// Test GitHub payload authentication: Invalid signature
    #[test]
    fn payload_authentication_github_fail() {
        let secret = String::from("secret");
        let hook = Hook::new("*", Some(secret.clone()), |_: &Delivery| {});
        let payload = String::from(r#"{"zen": "Another test!"}"#);
        let request_body = payload.clone();
        let signature_field = String::from("sha1=ec760ee6d10bf638089f078b5a0c23f6575821e7");
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("X-Github-Event".to_string(), "push".to_string());
        headers.insert("X-Hub-Signature".to_string(), signature_field);
        let delivery = Delivery::new(headers, Some(request_body));
        assert_eq!(hook.auth(&delivery.unwrap()), false);
    }
}

#[cfg(test)]
mod tests_gitlab {
    use super::*;
    use std::collections::HashMap;

    /// Test GitLab payload authentication: Valid token
    #[test]
    fn payload_authentication_gitlab() {
        let secret = String::from("secret");
        let hook = Hook::new("*", Some(secret), |_: &Delivery| {});
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("X-Gitlab-Event".to_string(), "push".to_string());
        headers.insert("X-Gitlab-Token".to_string(), "secret".to_string());
        let delivery = Delivery::new(headers, None);
        assert!(hook.auth(&delivery.unwrap()));
    }

    /// Test GitLab payload authentication: Invalid token
    #[test]
    fn payload_authentication_gitlab_fail() {
        let secret = String::from("secret");
        let hook = Hook::new("*", Some(String::from("AnotherSecret")), |_: &Delivery| {});
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("X-Gitlab-Event".to_string(), "push".to_string());
        headers.insert("X-Gitlab-Token".to_string(), secret);
        let delivery = Delivery::new(headers, None);
        assert_eq!(hook.auth(&delivery.unwrap()), false);
    }
}
