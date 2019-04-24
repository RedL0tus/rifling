Rifling
=======

[![license](https://img.shields.io/github/license/RedL0tus/rifling.svg)](LICENSE) [![crates.io](http://meritbadge.herokuapp.com/rifling)](https://crates.io/crates/rifling) [![docs.rs](https://docs.rs/rifling/badge.svg)](https://docs.rs/rifling/)

Rifling is a library to create Github Webhook listener, influenced by [afterparty](https://crates.io/crates/afterparty).

If you want a commandline tool rather than a library, please consult [trigger](https://github.com/RedL0tus/trigger).

Features
--------

 - Supports both `application/json` mode and `application/x-www-form-urlencoded` mode.
 - (Potentially) support for different web frameworks.
 - Optional payload parsing support. Using `serde_json`'s untyped parsing functionality.
 - Optional payload authentication support with `ring` or libraries from RustCrypto team.
 - Optional logging.

Optional features
-----------------

 - Web frameworks:
   - `hyper-support` (default): Support of hyper. Example: [hyper-simple.rs](examples/hyper-simple.rs)
 - Payload authentication (does not affect usage):
   - `crypto-use-ring` (default): Use [`ring`](https://crates.io/crates/ring) as cryptography library. This MAY be faster but has some C code.
   - `crypto-use-rustcrypto`: Use libraries from RustCrypto team ([`hmac`](https://crates.io/crates/hmac) and [`sha-1`](https://crates.io/crates/sha-1)). These libraries are pure Rust implementations of these algorithms, which can be linked with `musl`.
 - Payload parsing:
   - `parse` (default): Parse the payload. Parsed payload will be present in `Delivery.payload` as `Option<Value>`.
 - Logging:
   - `logging` (default): Use the official [`log`](https://crates.io/crates/log) crate to log.
   - `logging-print`: Use `println` macro to print log. Will be ignored when `logging` is enabled.

License
-------

MIT License. See [LICENSE](LICENSE).