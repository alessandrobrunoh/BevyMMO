//! HTTP gateway for `apps/frontend` (Angular) and any non-Bevy client.
//!
//! It is a thin facade: the authoritative state lives in SpacetimeDB, and
//! the Bevy desktop client in `bins/game` talks to it directly over its own
//! protocol. This service exists for clients that prefer a plain HTTP surface
//! (login proxy, REST shims, webhooks, ...).
//!
//! Free of gameplay rules — anything that runs authoritatively on the server
//! belongs in `bevymmo_domain` — but *not* stateless: `/auth/*` holds one
//! live SpacetimeDB connection per logged-in browser (see [`stdb::session`]
//! for why a one-shot HTTP call to SpacetimeDB cannot substitute for one).

mod auth;
mod stdb;

use std::net::SocketAddr;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderValue, Method, StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use serde::Serialize;
use tokio::signal;
use tower_http::cors::CorsLayer;
use tracing::info;

use bevymmo_app_support::settings::Settings;
use stdb::session::SessionStore;

#[derive(Clone)]
struct AppState {
    /// SpacetimeDB module the gateway is configured to talk to. Surfaced on
    /// `/` so an operator can confirm wiring without grepping the config.
    spacetime_module: String,
    /// WebSocket URL of the SpacetimeDB instance — see `stdb::connection`
    /// for why the gateway needs a real connection, not a one-shot HTTP call.
    spacetime_uri: String,
    /// Live SpacetimeDB connections, one per authenticated web session.
    sessions: SessionStore,
    /// Whether the session cookie is marked `Secure`. See
    /// `GatewaySettings::cookie_secure`'s doc comment.
    cookie_secure: bool,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Serialize)]
struct WelcomeResponse {
    message: &'static str,
    service: &'static str,
    /// The SpacetimeDB module name the gateway is wired to. Useful to confirm
    /// the gateway and the desktop client are pointing at the same world.
    spacetime_module: String,
}

#[tokio::main]
async fn main() {
    init_tracing();

    let settings = Settings::load();
    let bind_addr: SocketAddr = settings
        .gateway
        .bind_addr
        .parse()
        .expect("gateway.bind_addr is not a valid host:port");

    let sessions = SessionStore::new();
    sessions.spawn_reaper();

    let cors_origin: HeaderValue = settings
        .gateway
        .cors_origin
        .parse()
        .expect("gateway.cors_origin is not a valid header value");

    let state = AppState {
        spacetime_module: settings.spacetime_module,
        spacetime_uri: settings.spacetime_uri,
        sessions,
        cookie_secure: settings.gateway.cookie_secure,
    };
    let app = build_router(state, cors_origin);

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .unwrap_or_else(|err| panic!("failed to bind gateway on {bind_addr}: {err}"));

    info!(%bind_addr, "BevyMMO gateway listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("gateway server crashed");
}

fn build_router(state: AppState, cors_origin: HeaderValue) -> Router {
    // `Any` origin is not an option: the session cookie needs
    // `Access-Control-Allow-Credentials`, which browsers refuse to honor
    // together with a wildcard `Access-Control-Allow-Origin`. See
    // `GatewaySettings::cors_origin`'s doc comment.
    let cors = CorsLayer::new()
        .allow_origin(cors_origin)
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    Router::new()
        .route("/", get(welcome))
        .route("/health", get(health))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/profile", get(auth::profile))
        .layer(cors)
        .with_state(state)
}

async fn welcome(State(state): State<AppState>) -> Json<WelcomeResponse> {
    Json(WelcomeResponse {
        message: "Welcome to the BevyMMO gateway",
        service: "bevymmo_gateway",
        spacetime_module: state.spacetime_module,
    })
}

async fn health() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok",
            service: "bevymmo_gateway",
        }),
    )
}

/// Resolves on the first Ctrl+C / SIGTERM. `axum::serve` then drains in-flight
/// requests before returning, so a deploy does not sever a request mid-flight.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Ctrl+C received, shutting down"),
        _ = terminate => info!("SIGTERM received, shutting down"),
    }
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info,bevymmo_gateway=debug"))
        .expect("failed to build log filter");

    fmt().with_env_filter(filter).with_target(false).init();
}
