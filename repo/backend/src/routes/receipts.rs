use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::db;
use crate::middleware::{require_auth, require_role};
use crate::AppState;
use common::*;

const MAX_FILE_SIZE: i64 = 10 * 1024 * 1024; // 10 MB
const ALLOWED_TYPES: &[&str] = &["application/pdf", "image/jpeg", "image/png"];

pub async fn upload_receipt(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ReceiptRecord>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let _user = require_role(&state.db, &auth.user_id, &[Role::ProjectManager, Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 16_777_216)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: UploadReceiptRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    // Validate file type
    if !ALLOWED_TYPES.contains(&req.file_type.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError { error: format!("Invalid file type '{}'. Allowed: PDF, JPG, PNG", req.file_type) }),
        ));
    }

    // Validate file size
    if req.file_size > MAX_FILE_SIZE {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError { error: format!("File too large ({} bytes). Max 10 MB", req.file_size) }),
        ));
    }

    // Decode base64 file data
    use base64::{engine::general_purpose::STANDARD, Engine};
    let file_data = STANDARD.decode(&req.file_data_base64).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid base64 file data".into() }))
    })?;

    // Validate actual size matches declared size
    if file_data.len() as i64 != req.file_size {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError { error: "Declared file_size does not match actual data length".into() }),
        ));
    }

    // SHA-256 fingerprint
    let mut hasher = Sha256::new();
    hasher.update(&file_data);
    let fingerprint = hex::encode(hasher.finalize());

    // Check duplicate
    if db::receipt_fingerprint_exists(&state.db, &fingerprint) {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiError { error: "Duplicate receipt: a file with identical content already exists".into() }),
        ));
    }

    // Verify expense exists and caller owns the project (or is admin)
    let project_id = db::get_expense_project_id(&state.db, &req.expense_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiError { error: "Expense not found".into() })))?;
    crate::middleware::require_project_owner(&state.db, &auth.user_id, &project_id)
        .map_err(|s| (s, Json(ApiError { error: "You do not own the project for this expense".into() })))?;

    // Encrypt file data at rest using AES-256-GCM
    let encrypted_data = crate::crypto::encrypt_bytes(&file_data, &state.encryption_key)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: format!("Encryption failed: {}", e) })))?;

    let receipt_id = uuid::Uuid::new_v4().to_string();
    db::create_receipt(
        &state.db,
        &receipt_id,
        &req.expense_id,
        &req.file_name,
        &req.file_type,
        req.file_size,
        &encrypted_data,
        &fingerprint,
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    db::append_ops_log(&state.db, &auth.user_id, "staff", "upload_receipt", &format!("Uploaded receipt {} for expense {}", receipt_id, req.expense_id));

    let receipts = db::list_receipts_for_expense(&state.db, &req.expense_id);
    let receipt = receipts.into_iter().find(|r| r.id == receipt_id).unwrap();
    Ok(Json(receipt))
}

pub async fn review_receipt(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let user = require_role(&state.db, &auth.user_id, &[Role::FinanceReviewer, Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: ReviewReceiptRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    // Mandatory rejection reason
    if !req.verified && req.rejection_reason.as_ref().map_or(true, |r| r.trim().is_empty()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError { error: "Rejection reason is required when rejecting a receipt".into() }),
        ));
    }

    db::review_receipt(
        &state.db,
        &req.receipt_id,
        req.verified,
        &auth.user_id,
        req.rejection_reason.as_deref(),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    let action = if req.verified { "verified" } else { "rejected" };
    db::append_ops_log(&state.db, &auth.user_id, &user.display_name, "review_receipt", &format!("Receipt {} {}", req.receipt_id, action));

    Ok(Json(ApiSuccess { message: format!("Receipt {}", action) }))
}

/// List receipts for an expense — project manager/admin/finance only.
pub async fn list_receipts(
    State(state): State<Arc<AppState>>,
    Path(expense_id): Path<String>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<ReceiptRecord>>, (StatusCode, Json<ApiError>)> {
    let auth = crate::middleware::require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let user = db::get_user_by_id(&state.db, &auth.user_id)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ApiError { error: "User not found".into() })))?;
    if user.role != Role::Administrator && user.role != Role::FinanceReviewer {
        // Look up project from expense → budget_line → project
        let project_id = db::get_expense_project_id(&state.db, &expense_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiError { error: "Expense not found".into() })))?;
        crate::middleware::require_project_owner(&state.db, &auth.user_id, &project_id)
            .map_err(|s| (s, Json(ApiError { error: "Not authorized".into() })))?;
    }
    Ok(Json(db::list_receipts_for_expense(&state.db, &expense_id)))
}

pub async fn pending_receipts(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<ReceiptRecord>>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    require_role(&state.db, &auth.user_id, &[Role::FinanceReviewer, Role::Administrator])?;
    Ok(Json(db::list_uploaded_receipts(&state.db)))
}
