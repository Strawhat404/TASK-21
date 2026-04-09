use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;

use crate::db;
use crate::middleware::{require_auth, require_role};
use crate::AppState;
use common::*;

/// Check text against the sensitive word list. Returns list of matched words.
pub fn check_sensitive_words(text: &str, words: &[String]) -> Vec<String> {
    let lower = text.to_lowercase();
    words
        .iter()
        .filter(|w| !w.is_empty() && lower.contains(&w.to_lowercase()))
        .cloned()
        .collect()
}

pub async fn get_config(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ModerationConfig>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    require_role(&state.db, &auth.user_id, &[Role::Administrator])?;
    Ok(Json(db::get_moderation_config(&state.db)))
}

pub async fn update_config(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let user = require_role(&state.db, &auth.user_id, &[Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden: admin only".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let config: ModerationConfig = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    db::update_moderation_config(&state.db, &config)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    db::append_ops_log(&state.db, &auth.user_id, &user.display_name, "update_moderation_config",
        &format!("comments_enabled={}, pre_mod={}, words={}", config.comments_enabled, config.require_pre_moderation, config.sensitive_words.len()));

    Ok(Json(ApiSuccess { message: "Moderation config updated".into() }))
}

pub async fn pending_comments(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<Comment>>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let user = require_role(&state.db, &auth.user_id, &[Role::ProjectManager, Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden".into() })))?;

    let all_pending = db::list_pending_comments(&state.db);

    // Admins see all; PMs only see comments on projects they manage
    if user.role == Role::Administrator {
        Ok(Json(all_pending))
    } else {
        let filtered: Vec<Comment> = all_pending
            .into_iter()
            .filter(|c| {
                crate::middleware::require_project_owner(&state.db, &auth.user_id, &c.project_id).is_ok()
            })
            .collect();
        Ok(Json(filtered))
    }
}

pub async fn moderate_comment(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let user = require_role(&state.db, &auth.user_id, &[Role::ProjectManager, Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: ModerateCommentRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    // Project-scope check: PMs can only moderate comments on their own projects
    if user.role != Role::Administrator {
        let comment_project_id = db::get_comment_project_id(&state.db, &req.comment_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiError { error: "Comment not found".into() })))?;
        crate::middleware::require_project_owner(&state.db, &auth.user_id, &comment_project_id)
            .map_err(|s| (s, Json(ApiError { error: "You can only moderate comments on your own projects".into() })))?;
    }

    db::moderate_comment(&state.db, &req.comment_id, req.approved)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    let action = if req.approved { "approved" } else { "rejected" };
    db::append_ops_log(&state.db, &auth.user_id, &user.display_name, "moderate_comment", &format!("Comment {} {}", req.comment_id, action));

    Ok(Json(ApiSuccess { message: format!("Comment {}", action) }))
}
