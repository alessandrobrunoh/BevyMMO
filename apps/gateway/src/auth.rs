//! `/auth/*` and `/profile` — the HTTP surface `apps/frontend` uses instead
//! of talking to SpacetimeDB directly.
//!
//! Every handler here is a thin translation: validate the shape of the
//! request, drive a [`GatewayConnection`], translate its result into an HTTP
//! status and a small JSON body. The actual rules (email format, password
//! policy, uniqueness, ownership) live in the SpacetimeDB module's
//! `reducers::account` — this layer never re-implements them, only reports
//! what the module decided.

use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::stdb::connection::{CharacterSummary, GatewayConnection};

const SESSION_COOKIE_NAME: &str = "bevymmo_session";

/// How long the cookie itself persists in the browser. Deliberately longer
/// than [`crate::stdb::session::SessionStore`]'s server-side idle timeout
/// (30 minutes): the two are independent expirations by design. A cookie
/// surviving a week is a normal "stay signed in"; the connection behind it
/// still gets reaped after half an hour of inactivity, at which point
/// `/profile` reports `401` and the frontend sends the user back to
/// `/login` — an explicit re-authentication, not a silent extension.
const SESSION_COOKIE_MAX_AGE_SECS: i64 = 7 * 24 * 60 * 60;

#[derive(Deserialize)]
pub struct AuthRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
struct ProfileResponse {
    account_id: u64,
    characters: Vec<CharacterSummary>,
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<AuthRequest>,
) -> Response {
    let connection = match open_connection(&state).await {
        Ok(connection) => connection,
        Err(response) => return response,
    };
    if let Err(reason) = connection.register(body.email, body.password).await {
        connection.disconnect();
        return error_response(StatusCode::BAD_REQUEST, reason);
    }
    authenticated_response(&state, connection).await
}

pub async fn login(State(state): State<AppState>, Json(body): Json<AuthRequest>) -> Response {
    let connection = match open_connection(&state).await {
        Ok(connection) => connection,
        Err(response) => return response,
    };
    if let Err(reason) = connection.login(body.email, body.password).await {
        connection.disconnect();
        return error_response(StatusCode::BAD_REQUEST, reason);
    }
    authenticated_response(&state, connection).await
}

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

pub async fn profile(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(id) = session_id_from_cookie(&headers) else {
        return error_response(StatusCode::UNAUTHORIZED, "not authenticated".to_string());
    };
    let Some(connection) = state.sessions.get(&id).await else {
        // Cookie is still present client-side, but the server-side
        // connection behind it was reaped for inactivity (or never existed,
        // e.g. a stale cookie from a restarted gateway).
        return error_response(StatusCode::UNAUTHORIZED, "session expired".to_string());
    };
    let Some(account_id) = connection.account_id() else {
        return error_response(StatusCode::UNAUTHORIZED, "not authenticated".to_string());
    };
    let characters = connection.characters().unwrap_or_default();
    Json(ProfileResponse {
        account_id,
        characters,
    })
    .into_response()
}

/// Opens a fresh SpacetimeDB connection for a `register`/`login` attempt.
/// Not yet stored in the session store or cookied — that only happens once
/// the reducer call on it actually succeeds, so a failed attempt never hands
/// the browser a session id for a connection nobody can use.
async fn open_connection(state: &AppState) -> Result<GatewayConnection, Response> {
    GatewayConnection::connect(&state.spacetime_uri, &state.spacetime_module)
        .await
        .map_err(|err| {
            tracing::error!("gateway could not reach SpacetimeDB: {err}");
            error_response(
                StatusCode::BAD_GATEWAY,
                "could not reach the game server".to_string(),
            )
        })
}

/// Common tail of `register`/`login`: the reducer call already succeeded on
/// `connection`, so mint a session id for it, cookie the response, and
/// return the same profile shape `/profile` would.
async fn authenticated_response(state: &AppState, connection: GatewayConnection) -> Response {
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
        None => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "authenticated but no session was recorded".to_string(),
        ),
    };
    response
        .headers_mut()
        .insert(header::SET_COOKIE, session_cookie(&id, state.cookie_secure));
    response
}

fn error_response(status: StatusCode, message: String) -> Response {
    (status, Json(ErrorResponse { error: message })).into_response()
}

fn session_id_from_cookie(headers: &HeaderMap) -> Option<String> {
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
