use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::db;
use crate::middleware::{require_auth, require_role};
use crate::AppState;
use common::*;

#[derive(serde::Deserialize)]
pub struct StatsQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub cause: Option<String>,
    pub status: Option<String>,
}

pub async fn dashboard_stats(
    State(state): State<Arc<AppState>>,
    Query(q): Query<StatsQuery>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<DashboardStats>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    require_role(
        &state.db,
        &auth.user_id,
        &[Role::ProjectManager, Role::FinanceReviewer, Role::Administrator],
    )?;
    let stats = db::get_dashboard_stats(
        &state.db,
        q.from.as_deref(),
        q.to.as_deref(),
        q.cause.as_deref(),
        q.status.as_deref(),
    );
    Ok(Json(stats))
}

pub async fn ops_log(
    State(state): State<Arc<AppState>>,
    Query(q): Query<PaginationQuery>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<OpsLogEntry>>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    require_role(&state.db, &auth.user_id, &[Role::Administrator])?;
    let limit = q.per_page.unwrap_or(50);
    let offset = (q.page.unwrap_or(1) - 1) * limit;
    Ok(Json(db::get_ops_log(&state.db, limit, offset)))
}

#[derive(serde::Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

/// Sensitive action: unpublish a project (requires password re-entry).
pub async fn unpublish_project(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let user = require_role(&state.db, &auth.user_id, &[Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden: admin only".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let confirm: SensitiveActionConfirm = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    // Verify password
    let hash = db::get_user_password_hash(&state.db, &auth.user_id)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ApiError { error: "User not found".into() })))?;
    if !crate::auth::verify_password(&confirm.password, &hash) {
        return Err((StatusCode::UNAUTHORIZED, Json(ApiError { error: "Invalid password".into() })));
    }

    // Server-side two-step confirmation protocol
    match confirm.confirmation_token {
        None => {
            let token = db::create_confirmation_token(&state.db, &auth.user_id, "unpublish_project", &project_id)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;
            return Err((StatusCode::from_u16(428).unwrap(), Json(ApiError {
                error: format!("{{\"confirmation_token\":\"{}\",\"message\":\"Confirmation required. Resubmit with this token to execute.\"}}", token),
            })));
        }
        Some(ref token) => {
            if !db::consume_confirmation_token(&state.db, token, &auth.user_id, "unpublish_project", &project_id) {
                return Err((StatusCode::CONFLICT, Json(ApiError {
                    error: "Invalid, expired, or already-used confirmation token".into(),
                })));
            }
        }
    }

    let rows = db::update_project_status(&state.db, &project_id, "unpublished")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;
    if rows == 0 {
        return Err((StatusCode::NOT_FOUND, Json(ApiError {
            error: "Project not found".into(),
        })));
    }

    db::append_ops_log(&state.db, &auth.user_id, &user.display_name, "unpublish_project", &format!("Unpublished project {}", project_id));

    Ok(Json(ApiSuccess {
        message: "Project unpublished".into(),
    }))
}

/// Assign a role to a user (admin only, requires password confirmation).
pub async fn assign_role(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let admin = require_role(&state.db, &auth.user_id, &[Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden: admin only".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: AssignRoleRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    // Verify password (sensitive action)
    let hash = db::get_user_password_hash(&state.db, &auth.user_id)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ApiError { error: "User not found".into() })))?;
    if !crate::auth::verify_password(&req.password, &hash) {
        return Err((StatusCode::UNAUTHORIZED, Json(ApiError { error: "Invalid password".into() })));
    }

    // Validate the target role
    let role = Role::from_str(&req.role).ok_or_else(|| {
        (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid role".into() }))
    })?;

    // Verify target user exists
    let target_user = db::get_user_by_id(&state.db, &req.user_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiError { error: "Target user not found".into() })))?;

    let rows = db::update_user_role(&state.db, &req.user_id, role.as_str())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;
    if rows == 0 {
        return Err((StatusCode::NOT_FOUND, Json(ApiError { error: "User not found".into() })));
    }

    db::append_ops_log(
        &state.db,
        &auth.user_id,
        &admin.display_name,
        "assign_role",
        &format!("Changed role of user {} ({}) to {}", target_user.display_name, req.user_id, req.role),
    );

    Ok(Json(ApiSuccess {
        message: format!("Role updated to {}", req.role),
    }))
}

/// Bootstrap: promote the first registered user to admin if no admins exist.
/// Requires the user's own password for confirmation.
pub async fn bootstrap_admin(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;

    // Only allow if zero administrators exist
    if db::count_administrators(&state.db) > 0 {
        return Err((StatusCode::CONFLICT, Json(ApiError {
            error: "An administrator already exists. Use the role assignment endpoint instead.".into(),
        })));
    }

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let confirm: SensitiveActionConfirm = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    // Verify password
    let hash = db::get_user_password_hash(&state.db, &auth.user_id)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ApiError { error: "User not found".into() })))?;
    if !crate::auth::verify_password(&confirm.password, &hash) {
        return Err((StatusCode::UNAUTHORIZED, Json(ApiError { error: "Invalid password".into() })));
    }

    db::update_user_role(&state.db, &auth.user_id, Role::Administrator.as_str())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    let user = db::get_user_by_id(&state.db, &auth.user_id).unwrap();
    db::append_ops_log(
        &state.db,
        &auth.user_id,
        &user.display_name,
        "bootstrap_admin",
        "First administrator bootstrapped",
    );

    Ok(Json(ApiSuccess {
        message: "You are now an administrator".into(),
    }))
}

/// CSV export of donations with PII masking.
pub async fn export_csv(
    State(state): State<Arc<AppState>>,
    Query(q): Query<StatsQuery>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<(StatusCode, [(axum::http::header::HeaderName, &'static str); 2], String), StatusCode> {
    let auth = require_auth(request.extensions())?;
    require_role(
        &state.db,
        &auth.user_id,
        &[Role::FinanceReviewer, Role::Administrator],
    )?;
    let csv = db::export_donations_csv(&state.db, q.from.as_deref(), q.to.as_deref(), q.cause.as_deref(), q.status.as_deref());
    Ok((
        StatusCode::OK,
        [
            (axum::http::header::CONTENT_TYPE, "text/csv"),
            (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"donations.csv\""),
        ],
        csv,
    ))
}
