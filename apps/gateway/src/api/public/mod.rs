//! `/public/*` — endpoints reachable without a session.
//!
//! Deliberately narrow: these handlers only read data the SpacetimeDB module
//! already replicates publicly (the `player` table), through the shared
//! [`crate::stdb::directory::PlayerDirectory`]. Nothing credential-adjacent —
//! the authoritative `account` table stays private to the module, and no
//! email or password material ever crosses this boundary.

pub mod accounts;

use axum::Router;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new().merge(accounts::router())
}
