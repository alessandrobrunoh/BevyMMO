//! `/public/*` — endpoints reachable without a session.
//!
//! Two kinds of public data:
//! - Live rows the SpacetimeDB module already replicates (`player`, `market`,
//!   order books) via [`crate::stdb::directory::PlayerDirectory`].
//! - The compiled game catalog (`/public/catalog/*`), built from
//!   `bevymmo_content` at process start — no module connection required.
//!
//! Nothing credential-adjacent: the authoritative `account` table stays
//! private to the module, and no email or password material ever crosses
//! this boundary.

pub mod accounts;
pub mod catalog;
pub mod markets;

use axum::Router;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(accounts::router())
        .merge(catalog::router())
        .merge(markets::router())
}
