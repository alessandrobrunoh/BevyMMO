//! `/auth/*` and `/profile` — the HTTP surface `apps/frontend` uses instead
//! of talking to SpacetimeDB directly.
//!
//! Every handler here is a thin translation: validate the shape of the
//! request, drive a [`GatewayConnection`], translate its result into an
//! [`AppError`] or a small JSON body. The actual rules (email format, password
//! policy, uniqueness, ownership) live in the SpacetimeDB module's
//! `reducers::account` — this layer never re-implements them, only reports
//! what the module decided.

use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};

use crate::api::error::AppError;
use crate::stdb::connection::{CharacterSummary, GatewayConnection};
use crate::AppState;

const SESSION_COOKIE_NAME: &str = "bevymmo_session";

/// How long the cookie itself persists in the browser. Deliberately longer
/// than [`crate::stdb::session::SessionStore`]'s server-side idle timeout
/// (30 minutes): the two are independent expirations by design. A cookie
/// surviving a week is a normal "stay signed in"; the connection behind it
/// still gets reaped after half an hour of inactivity, at which point
/// `/profile` reports `401` and the frontend sends the user back to
/// `/login` — an explicit re-authentication, not a silent extension.
const SESSION_COOKIE_MAX_AGE_SECS: i64 = 7 * 24 * 60 * 60;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AuthRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ProfileResponse {
    account_id: u64,
    characters: Vec<CharacterSummary>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/auth/register", post(register))
        .route("/v1/auth/login", post(login))
        .route("/v1/auth/logout", post(logout))
        .route("/v1/profile", get(profile))
}

/// Creates a new account and authenticates the caller as it. Sets the session
/// cookie on success.
#[utoipa::path(
    post,
    tag = "auth",
    path = "/v1/auth/register",
    request_body = AuthRequest,
    responses(
        (status = 200, description = "Account created and logged in; session cookie set", body = ProfileResponse),
        (status = 400, description = "Rejected by the module (email taken, bad format, weak password)", body = crate::api::error::ErrorResponse),
        (status = 502, description = "SpacetimeDB unreachable", body = crate::api::error::ErrorResponse),
    ),
)]
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<AuthRequest>,
) -> Result<Response, AppError> {
    let connection = open_connection(&state).await?;
    if let Err(reason) = connection.register(body.email, body.password).await {
        connection.disconnect();
        return Err(AppError::BadRequest(reason));
    }
    authenticated_response(&state, connection).await
}

/// Authenticates the caller as an existing account. Sets the session cookie
/// on success.
#[utoipa::path(
    post,
    tag = "auth",
    path = "/v1/auth/login",
    request_body = AuthRequest,
    responses(
        (status = 200, description = "Logged in; session cookie set", body = ProfileResponse),
        (status = 400, description = "Invalid email or password", body = crate::api::error::ErrorResponse),
        (status = 502, description = "SpacetimeDB unreachable", body = crate::api::error::ErrorResponse),
    ),
)]
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<AuthRequest>,
) -> Result<Response, AppError> {
    let connection = open_connection(&state).await?;
    if let Err(reason) = connection.login(body.email, body.password).await {
        connection.disconnect();
        return Err(AppError::BadRequest(reason));
    }
    authenticated_response(&state, connection).await
}

/// Ends the session behind the caller's cookie and clears the cookie.
#[utoipa::path(
    post,
    tag = "auth",
    path = "/v1/auth/logout",
    responses((status = 204, description = "Session ended; cookie cleared")),
)]
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(id) = session_id_from_cookie(&headers) {
        state.sessions.end(&id).await;
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, clear_cookie(state.cookie_secure));
    response
}

/// The caller's own account and character roster, from their session cookie.
#[utoipa::path(
    get,
    tag = "auth",
    path = "/v1/profile",
    responses(
        (status = 200, description = "The authenticated account", body = ProfileResponse),
        (status = 401, description = "No session, or the session expired", body = crate::api::error::ErrorResponse),
    ),
)]
pub async fn profile(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ProfileResponse>, AppError> {
    let Some(id) = session_id_from_cookie(&headers) else {
        return Err(AppError::Unauthorized);
    };
    let Some(connection) = state.sessions.get(&id).await else {
        // Cookie is still present client-side, but the server-side
        // connection behind it was reaped for inactivity (or never existed,
        // e.g. a stale cookie from a restarted gateway).
        return Err(AppError::SessionExpired);
    };
    let Some(account_id) = connection.account_id() else {
        return Err(AppError::Unauthorized);
    };
    Ok(Json(ProfileResponse {
        account_id,
        characters: connection.characters().unwrap_or_default(),
    }))
}

/// Opens a fresh SpacetimeDB connection for a `register`/`login` attempt.
/// Not yet stored in the session store or cookied — that only happens once
/// the reducer call on it actually succeeds, so a failed attempt never hands
/// the browser a session id for a connection nobody can use.
async fn open_connection(state: &AppState) -> Result<GatewayConnection, AppError> {
    GatewayConnection::connect(&state.spacetime_uri, &state.spacetime_module)
        .await
        .map_err(|err| {
            tracing::error!("gateway could not reach SpacetimeDB: {err}");
            AppError::BadGateway
        })
}

/// Common tail of `register`/`login`: the reducer call already succeeded on
/// `connection`, so mint a session id for it, cookie the response, and
/// return the same profile shape `/profile` would.
async fn authenticated_response(
    state: &AppState,
    connection: GatewayConnection,
) -> Result<Response, AppError> {
    let account_id = connection.account_id();
    let characters = connection.characters().unwrap_or_default();
    let id = state.sessions.create(connection).await;

    let mut response = match account_id {
        Some(account_id) => Json(ProfileResponse {
            account_id,
            characters,
        })
        .into_response(),
        // Should not happen — the reducer call just returned `Ok`, which
        // only happens after `bind_session` writes the `Session` row this
        // reads back — but fail closed with a real status rather than
        // panicking on the `unwrap` that would otherwise be tempting here.
        None => {
            return Err(AppError::Internal(
                "authenticated but no session was recorded".to_string(),
            ))
        }
    };
    response
        .headers_mut()
        .insert(header::SET_COOKIE, session_cookie(&id, state.cookie_secure));
    Ok(response)
}

/// The session id on the request cookie, if any. Shared with the
/// authenticated character routes that reuse the same cookie.
pub(crate) fn session_id_from_cookie(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';').find_map(|pair| {
        let (name, value) = pair.trim().split_once('=')?;
        (name == SESSION_COOKIE_NAME).then(|| value.to_string())
    })
}

fn session_cookie(id: &str, secure: bool) -> HeaderValue {
    let secure_attr = if secure { "; Secure" } else { "" };
    let value = format!(
        "{SESSION_COOKIE_NAME}={id}; Path=/; HttpOnly; SameSite=Lax; Max-Age={SESSION_COOKIE_MAX_AGE_SECS}{secure_attr}"
    );
    HeaderValue::from_str(&value).expect("cookie header value is always valid ASCII")
}

fn clear_cookie(secure: bool) -> HeaderValue {
    let secure_attr = if secure { "; Secure" } else { "" };
    let value =
        format!("{SESSION_COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{secure_attr}");
    HeaderValue::from_str(&value).expect("cookie header value is always valid ASCII")
}
