//! The gateway's SpacetimeDB half: one connection per authenticated web
//! session, kept alive between HTTP requests so the account it authenticated
//! stays reachable — see [`session`] for why a stateless-per-request
//! connection would not work.

#[rustfmt::skip]
#[allow(clippy::all)]
pub mod module_bindings;

pub mod connection;
pub mod session;
