use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::db;
use crate::middleware::require_auth;
use crate::AppState;
use common::*;

pub async fn list_notifications(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<Notification>>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    Ok(Json(db::list_notifications(&state.db, &auth.user_id)))
}

pub async fn mark_read(
    State(state): State<Arc<AppState>>,
    Path(notif_id): Path<String>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    db::mark_notification_read(&state.db, &notif_id, &auth.user_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ApiSuccess {
        message: "Marked as read".into(),
    }))
}

pub async fn mark_all_read(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    db::mark_all_notifications_read(&state.db, &auth.user_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ApiSuccess {
        message: "All marked as read".into(),
    }))
}
