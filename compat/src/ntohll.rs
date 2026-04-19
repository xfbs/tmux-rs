//! 64-bit network-to-host byte order conversion. On little-endian hosts
//! `ntohll` is a byte-reverse; on big-endian hosts it's the identity.
//! Replaces a C2Rust port of OpenBSD's `ntohll.c` with the idiomatic
//! `u64::from_be`.

/// Convert a 64-bit unsigned integer from network byte order to host byte order.
pub fn ntohll(v: u64) -> u64 {
    u64::from_be(v)
}
