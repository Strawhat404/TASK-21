use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;

use crate::db;
use crate::middleware::{require_auth, require_role};
use crate::AppState;
use common::*;

/// Track an analytics event with dedup and burst detection.
pub async fn track_event(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let user_id = require_auth(request.extensions())
        .ok()
        .map(|a| a.user_id);

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: TrackEventRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    let event_kind_str = req.event_kind.as_str();

    // Check for duplicate within 3 seconds
    let is_dup = db::is_duplicate_event(&state.db, event_kind_str, &req.target_id, &req.session_id);

    // Check for suspicious burst
    let is_suspicious = db::is_suspicious_burst(&state.db, &req.session_id);

    let event_id = uuid::Uuid::new_v4().to_string();
    db::insert_event(
        &state.db,
        &event_id,
        event_kind_str,
        &req.target_type,
        &req.target_id,
        &req.session_id,
        user_id.as_deref(),
        req.dwell_ms,
        is_dup,
        is_suspicious,
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    // If suspicious, flag for admin review (append to ops log)
    if is_suspicious {
        db::append_ops_log(
            &state.db,
            user_id.as_deref().unwrap_or("anonymous"),
            "system",
            "suspicious_burst",
            &format!("Suspicious event burst from session {}", req.session_id),
        );
    }

    // Fire webhooks for the event
    crate::routes::webhooks::fire_webhooks(
        &state,
        &format!("event.{}", event_kind_str),
        &serde_json::json!({
            "event_id": event_id,
            "event_kind": event_kind_str,
            "target_type": req.target_type,
            "target_id": req.target_id,
            "session_id": req.session_id,
        }),
    )
    .await;

    Ok(Json(ApiSuccess {
        message: if is_dup { "Event recorded (duplicate)".into() } else { "Event recorded".into() },
    }))
}

/// Get data quality metrics for the KPI console.
pub async fn data_quality(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<DataQualityMetrics>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    require_role(&state.db, &auth.user_id, &[Role::ProjectManager, Role::FinanceReviewer, Role::Administrator])?;
    Ok(Json(db::get_data_quality_metrics(&state.db)))
}

/// List suspicious events for admin review.
pub async fn suspicious_events(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<AnalyticsEvent>>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    require_role(&state.db, &auth.user_id, &[Role::Administrator])?;
    Ok(Json(db::list_suspicious_events(&state.db, 100)))
}
