//! Authenticated character-scoped reads. Cookie session, no new reducers.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::Json;
use axum::Router;
use serde::Serialize;
use uuid::Uuid;

use crate::api::auth::session_id_from_cookie;
use crate::api::error::AppError;
use crate::AppState;

#[derive(Serialize, utoipa::ToSchema)]
pub struct WalletResponse {
    pub gold: u64,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/v1/characters/:character_id/wallet", get(wallet))
}

/// Gold on one of the caller's characters. 0 if the character exists but
/// has no `character_wallet` row yet.
#[utoipa::path(
    get,
    tag = "auth",
    path = "/v1/characters/{character_id}/wallet",
    params(("character_id" = Uuid, Path, description = "Character UUID from /v1/profile")),
    responses(
        (status = 200, description = "Wallet gold for this character", body = WalletResponse),
        (status = 401, description = "No session, or the session expired", body = crate::api::error::ErrorResponse),
        (status = 403, description = "The character belongs to another account", body = crate::api::error::ErrorResponse),
        (status = 404, description = "No character with that id", body = crate::api::error::ErrorResponse),
    ),
)]
pub async fn wallet(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(character_id): Path<Uuid>,
) -> Result<Json<WalletResponse>, AppError> {
    let Some(id) = session_id_from_cookie(&headers) else {
        return Err(AppError::Unauthorized);
    };
    let Some(connection) = state.sessions.get(&id).await else {
        return Err(AppError::SessionExpired);
    };
    let Some(account_id) = connection.account_id() else {
        return Err(AppError::Unauthorized);
    };
    let Some(player) = connection.player(character_id) else {
        return Err(AppError::NotFound(format!(
            "no character with id {character_id}"
        )));
    };
    if player.account_id != account_id {
        return Err(AppError::Forbidden(
            "that character does not belong to this account".to_string(),
        ));
    }
    Ok(Json(WalletResponse {
        gold: connection.wallet_gold(character_id),
    }))
}
