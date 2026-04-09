use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::db;
use crate::middleware::{require_auth, require_role, require_project_owner};
use crate::AppState;
use common::*;

pub async fn list_comments(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
) -> Json<Vec<Comment>> {
    Json(db::list_comments(&state.db, &project_id))
}

pub async fn create_comment(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: CreateCommentRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    // Check moderation config
    let mod_config = db::get_moderation_config(&state.db);
    if !mod_config.comments_enabled {
        return Err((StatusCode::FORBIDDEN, Json(ApiError { error: "Comments are currently disabled".into() })));
    }

    // Check for sensitive words
    let matched = crate::routes::moderation::check_sensitive_words(&req.body, &mod_config.sensitive_words);
    if !matched.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError { error: format!("Comment contains disallowed words: {}", matched.join(", ")) }),
        ));
    }

    let moderation_status = if mod_config.require_pre_moderation {
        "pending_review"
    } else {
        "approved"
    };

    let comment_id = uuid::Uuid::new_v4().to_string();
    db::create_comment(&state.db, &comment_id, &req.project_id, &auth.user_id, &req.body, moderation_status)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    let msg = if moderation_status == "pending_review" {
        "Comment submitted for moderation review"
    } else {
        "Comment posted"
    };
    Ok(Json(ApiSuccess { message: msg.into() }))
}

/// Delete a comment (admin-only, requires password re-entry).
pub async fn delete_comment(
    State(state): State<Arc<AppState>>,
    Path(comment_id): Path<String>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let user = require_role(&state.db, &auth.user_id, &[Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden: admin only".into() })))?;

    // Password re-entry required for comment removal
    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let confirm: SensitiveActionConfirm = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    let hash = db::get_user_password_hash(&state.db, &auth.user_id)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ApiError { error: "User not found".into() })))?;
    if !crate::auth::verify_password(&confirm.password, &hash) {
        return Err((StatusCode::UNAUTHORIZED, Json(ApiError { error: "Invalid password".into() })));
    }

    // Server-side two-step confirmation protocol
    match confirm.confirmation_token {
        None => {
            let token = db::create_confirmation_token(&state.db, &auth.user_id, "delete_comment", &comment_id)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;
            return Err((StatusCode::from_u16(428).unwrap(), Json(ApiError {
                error: format!("{{\"confirmation_token\":\"{}\",\"message\":\"Confirmation required. Resubmit with this token to execute.\"}}", token),
            })));
        }
        Some(ref token) => {
            if !db::consume_confirmation_token(&state.db, token, &auth.user_id, "delete_comment", &comment_id) {
                return Err((StatusCode::CONFLICT, Json(ApiError {
                    error: "Invalid, expired, or already-used confirmation token".into(),
                })));
            }
        }
    }

    db::delete_comment(&state.db, &comment_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    db::append_ops_log(&state.db, &auth.user_id, &user.display_name, "delete_comment", &format!("Deleted comment {}", comment_id));

    Ok(Json(ApiSuccess {
        message: "Comment deleted".into(),
    }))
}

// ── Tickets ──

pub async fn submit_ticket(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: SubmitTicketRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    let ticket_id = uuid::Uuid::new_v4().to_string();
    db::create_ticket(&state.db, &ticket_id, &req.project_id, &auth.user_id, &req.subject, &req.body)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    Ok(Json(ApiSuccess {
        message: format!("Ticket {} created", ticket_id),
    }))
}

/// List tickets — scoped to project manager/admin of that project, or any finance reviewer.
pub async fn list_tickets(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<Ticket>>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    // Must be project owner, admin, or finance reviewer
    let user = db::get_user_by_id(&state.db, &auth.user_id)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ApiError { error: "User not found".into() })))?;
    if user.role != Role::Administrator && user.role != Role::FinanceReviewer {
        require_project_owner(&state.db, &auth.user_id, &project_id)
            .map_err(|s| (s, Json(ApiError { error: "Not authorized to view tickets for this project".into() })))?;
    }
    Ok(Json(db::list_tickets(&state.db, &project_id)))
}

/// Respond to ticket — must be manager of that ticket's project, or admin.
pub async fn respond_ticket(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let _user = require_role(&state.db, &auth.user_id, &[Role::ProjectManager, Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: RespondTicketRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    // IDOR: verify caller owns the ticket's project
    let project_id = db::get_ticket_project_id(&state.db, &req.ticket_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiError { error: "Ticket not found".into() })))?;
    require_project_owner(&state.db, &auth.user_id, &project_id)
        .map_err(|s| (s, Json(ApiError { error: "You do not manage this ticket's project".into() })))?;

    db::respond_ticket(&state.db, &req.ticket_id, &req.response)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    Ok(Json(ApiSuccess {
        message: "Ticket responded".into(),
    }))
}
