//! Rifling the GitHub webhook listener library

extern crate hex;
#[macro_use]
extern crate log;
extern crate hyper;
extern crate ring;
extern crate url;

pub mod constructor;
pub mod handler;
pub mod hook;

pub use constructor::Constructor;
pub use handler::Delivery;
pub use hook::Hook;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
