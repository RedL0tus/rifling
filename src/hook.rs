use hex::FromHex;
use ring::digest;
use ring::hmac;

use super::handler::Delivery;

pub trait HookFunc: HookFuncClone + Sync + Send {
    fn run(&self, delivery: &Delivery);
}

// Inspired by https://stackoverflow.com/a/30353928
pub trait HookFuncClone {
    fn clone_box(&self) -> Box<HookFunc>;
}

#[derive(Clone)]
pub struct Hook {
    pub event: &'static str,
    secret: Option<&'static str>,
    func: Box<HookFunc>,
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
