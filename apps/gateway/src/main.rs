//! HTTP gateway for `apps/frontend` (Angular) and any non-Bevy client.
//!
//! It is a thin facade: the authoritative state lives in SpacetimeDB, and
//! the Bevy desktop client in `bins/game` talks to it directly over its own
//! protocol. This service exists for clients that prefer a plain HTTP surface
//! (login proxy, REST shims, webhooks, ...).
//!
//! Keep the gateway stateless and free of gameplay rules — anything that
//! runs authoritatively on the server belongs in `bevymmo_domain`.

use std::net::SocketAddr;

use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use serde::Serialize;
use tokio::signal;
use tracing::info;

use bevymmo_app_support::settings::Settings;

#[derive(Clone)]
struct AppState {
    /// SpacetimeDB module the gateway is configured to talk to. Surfaced on
    /// `/` so an operator can confirm wiring without grepping the config.
    spacetime_module: String,
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

    let state = AppState {
        spacetime_module: settings.spacetime_module,
    };
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .unwrap_or_else(|err| panic!("failed to bind gateway on {bind_addr}: {err}"));

    info!(%bind_addr, "BevyMMO gateway listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("gateway server crashed");
}

fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(welcome))
        .route("/health", get(health))
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
