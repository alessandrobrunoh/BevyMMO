//! `/public/accounts/*` — search the game's accounts by character name.
//!
//! An "account" as exposed here is the account/character pair the public
//! `player` table already models: the search returns each character's
//! display name with its stable `character_id`, and the detail endpoint
//! returns everything public about that character plus the `account_id` that
//! owns it. See `stdb::directory` for where the rows come from.

use axum::Json;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use serde::Deserialize;

use crate::AppState;
use crate::api::error::AppError;
use crate::stdb::directory::PlayerEntry;

/// Page size used when the caller does not send `limit`.
const DEFAULT_SEARCH_LIMIT: usize = 50;

/// Upper bound for `limit`. The scan is over an in-memory cache so this is
/// not about load — it is about keeping responses readable until real
/// pagination is needed.
const MAX_SEARCH_LIMIT: usize = 100;

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SearchQuery {
    /// Case-insensitive substring of the character name. Empty or omitted
    /// matches every account.
    pub q: Option<String>,
    /// Maximum number of results (1-100, default 50). Values outside the
    /// range are clamped, not rejected.
    pub limit: Option<usize>,
    /// Number of matching results to skip (default 0), for paging.
    pub offset: Option<usize>,
}

/// One search hit: the name plus the stable UUID to feed the detail endpoint.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct AccountSummary {
    pub character_id: uuid::Uuid,
    pub display_name: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/public/accounts/search", get(search))
        .route("/v1/public/accounts/:character_id", get(detail))
}

/// Search every account by character name. Results are ordered by display
/// name and paged with `limit`/`offset`.
#[utoipa::path(
    get,
    tag = "public",
    path = "/v1/public/accounts/search",
    params(SearchQuery),
    responses(
        (status = 200, description = "Matching accounts, ordered by name", body = Vec<AccountSummary>),
        (status = 503, description = "SpacetimeDB unreachable", body = crate::api::error::ErrorResponse),
    ),
)]
pub async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<AccountSummary>>, AppError> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_SEARCH_LIMIT)
        .clamp(1, MAX_SEARCH_LIMIT);
    let offset = query.offset.unwrap_or(0);
    let needle = query.q.as_deref().unwrap_or("");

    let entries = state
        .directory
        .search(needle, offset, limit)
        .await
        .map_err(unavailable)?;

    Ok(Json(
        entries
            .into_iter()
            .map(|entry| AccountSummary {
                character_id: entry.character_id,
                display_name: entry.display_name,
            })
            .collect(),
    ))
}

/// Everything public about one account's character: names, ids, online state.
#[utoipa::path(
    get,
    tag = "public",
    path = "/v1/public/accounts/{character_id}",
    params(("character_id" = uuid::Uuid, Path, description = "Stable character UUID returned by the search")),
    responses(
        (status = 200, description = "The account's public data", body = PlayerEntry),
        (status = 404, description = "No character with that id", body = crate::api::error::ErrorResponse),
        (status = 503, description = "SpacetimeDB unreachable", body = crate::api::error::ErrorResponse),
    ),
)]
pub async fn detail(
    State(state): State<AppState>,
    Path(character_id): Path<uuid::Uuid>,
) -> Result<Json<PlayerEntry>, AppError> {
    match state.directory.get(character_id).await {
        Ok(Some(entry)) => Ok(Json(entry)),
        Ok(None) => Err(AppError::NotFound(format!(
            "no account with character id {character_id}"
        ))),
        Err(reason) => Err(unavailable(reason)),
    }
}

/// The directory's only failure mode: SpacetimeDB could not be reached (and
/// reconnecting just failed). The reason stays in the server log — the client
/// gets a retryable 503, not upstream internals.
fn unavailable(reason: String) -> AppError {
    tracing::error!("account directory unavailable: {reason}");
    AppError::ServiceUnavailable
}
