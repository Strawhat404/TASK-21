use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use qrcode::QrCode;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::db;
use crate::middleware::{require_auth, require_role, require_project_owner};
use crate::AppState;
use common::*;

pub async fn create_fulfillment(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<FulfillmentRecord>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let _user = require_role(&state.db, &auth.user_id, &[Role::ProjectManager, Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;

    #[derive(serde::Deserialize)]
    struct Req { project_id: String }
    let req: Req = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    require_project_owner(&state.db, &auth.user_id, &req.project_id)
        .map_err(|s| (s, Json(ApiError { error: "You do not manage this project".into() })))?;

    let id = uuid::Uuid::new_v4().to_string();
    db::create_fulfillment(&state.db, &id, &req.project_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    let record = db::get_fulfillment(&state.db, &id).unwrap();
    Ok(Json(record))
}

/// List fulfillments — scoped to project manager/admin of that project.
pub async fn list_fulfillments(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<FulfillmentRecord>>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    require_project_owner(&state.db, &auth.user_id, &project_id)
        .map_err(|s| (s, Json(ApiError { error: "Not authorized for this project".into() })))?;
    Ok(Json(db::list_fulfillments(&state.db, &project_id)))
}

/// Get single fulfillment — scoped to its project owner.
pub async fn get_fulfillment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<FulfillmentRecord>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let f = db::get_fulfillment(&state.db, &id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiError { error: "Not found".into() })))?;
    require_project_owner(&state.db, &auth.user_id, &f.project_id)
        .map_err(|s| (s, Json(ApiError { error: "Not authorized".into() })))?;
    Ok(Json(f))
}

/// Generate an OTP/QR code for a checkpoint — ownership checked.
pub async fn generate_code(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<CheckpointCodeResponse>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let _user = require_role(&state.db, &auth.user_id, &[Role::ProjectManager, Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: GenerateCheckpointCodeRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    // Ownership check via fulfillment's project
    let project_id = db::get_fulfillment_project_id(&state.db, &req.fulfillment_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiError { error: "Fulfillment not found".into() })))?;
    require_project_owner(&state.db, &auth.user_id, &project_id)
        .map_err(|s| (s, Json(ApiError { error: "You do not manage this fulfillment's project".into() })))?;

    use rand::Rng;
    let code: String = format!("{:06}", rand::thread_rng().gen_range(0..1_000_000));

    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::minutes(10))
        .unwrap()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    let code_id = uuid::Uuid::new_v4().to_string();
    db::store_checkpoint_code(&state.db, &code_id, &req.fulfillment_id, req.checkpoint.as_str(), &code, &expires_at)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    // Generate QR code SVG encoding the OTP code
    let qr_svg = QrCode::new(&code)
        .ok()
        .map(|qr| {
            let svg = qr
                .render::<qrcode::render::svg::Color>()
                .min_dimensions(200, 200)
                .build();
            svg
        });

    Ok(Json(CheckpointCodeResponse {
        code,
        expires_at,
        checkpoint: req.checkpoint.as_str().to_string(),
        qr_code_svg: qr_svg,
    }))
}

/// Record a checkpoint — ownership checked via fulfillment's project.
pub async fn record_checkpoint(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: RecordCheckpointRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    // Ownership check
    let project_id = db::get_fulfillment_project_id(&state.db, &req.fulfillment_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiError { error: "Fulfillment not found".into() })))?;
    require_project_owner(&state.db, &auth.user_id, &project_id)
        .map_err(|s| (s, Json(ApiError { error: "You do not manage this fulfillment's project".into() })))?;

    // Validate code
    let valid = db::consume_checkpoint_code(&state.db, &req.fulfillment_id, req.checkpoint.as_str(), &req.code)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    if !valid {
        return Err((StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid or expired code".into() })));
    }

    let fulfillment = db::get_fulfillment(&state.db, &req.fulfillment_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiError { error: "Fulfillment not found".into() })))?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Time-consistency rules
    match req.checkpoint {
        CheckpointKind::Arrival => {}
        CheckpointKind::Start => {
            if let Some(ref arrival) = fulfillment.arrival_at {
                if let Ok(arrival_time) = chrono::NaiveDateTime::parse_from_str(arrival, "%Y-%m-%d %H:%M:%S") {
                    if let Ok(now_time) = chrono::NaiveDateTime::parse_from_str(&now, "%Y-%m-%d %H:%M:%S") {
                        let diff = now_time.signed_duration_since(arrival_time);
                        if diff.num_hours() > 2 {
                            return Err((StatusCode::BAD_REQUEST, Json(ApiError {
                                error: "Start must be within 2 hours of arrival".into()
                            })));
                        }
                    }
                }
            } else {
                return Err((StatusCode::BAD_REQUEST, Json(ApiError {
                    error: "Arrival must be recorded before start".into()
                })));
            }
        }
        CheckpointKind::End => {
            if let Some(ref start) = fulfillment.start_at {
                if let Ok(start_time) = chrono::NaiveDateTime::parse_from_str(start, "%Y-%m-%d %H:%M:%S") {
                    if let Ok(now_time) = chrono::NaiveDateTime::parse_from_str(&now, "%Y-%m-%d %H:%M:%S") {
                        if now_time <= start_time {
                            return Err((StatusCode::BAD_REQUEST, Json(ApiError {
                                error: "End timestamp must be strictly after start timestamp".into()
                            })));
                        }
                    }
                }
            } else {
                return Err((StatusCode::BAD_REQUEST, Json(ApiError {
                    error: "Start must be recorded before end".into()
                })));
            }
        }
    }

    db::record_checkpoint(&state.db, &req.fulfillment_id, req.checkpoint.as_str(), &now)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    if req.checkpoint == CheckpointKind::End {
        let updated = db::get_fulfillment(&state.db, &req.fulfillment_id).unwrap();
        if updated.arrival_at.is_some() && updated.start_at.is_some() && updated.end_at.is_some() {
            let record_data = format!(
                "{}|{}|{}|{}|{}",
                updated.id, updated.project_id,
                updated.arrival_at.as_deref().unwrap_or(""),
                updated.start_at.as_deref().unwrap_or(""),
                updated.end_at.as_deref().unwrap_or(""),
            );
            let mut hasher = Sha256::new();
            hasher.update(record_data.as_bytes());
            let hash = hex::encode(hasher.finalize());
            let _ = db::complete_fulfillment(&state.db, &req.fulfillment_id, &hash);
        }
    }

    db::append_ops_log(&state.db, &auth.user_id, "user", "record_checkpoint",
        &format!("Recorded {} for fulfillment {}", req.checkpoint.as_str(), req.fulfillment_id));

    Ok(Json(ApiSuccess { message: format!("Checkpoint {} recorded", req.checkpoint.as_str()) }))
}

/// Service proof — scoped to project owner.
pub async fn service_proof(
    State(state): State<Arc<AppState>>,
    Path(fulfillment_id): Path<String>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ServiceProof>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let f = db::get_fulfillment(&state.db, &fulfillment_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiError { error: "Fulfillment not found".into() })))?;

    require_project_owner(&state.db, &auth.user_id, &f.project_id)
        .map_err(|s| (s, Json(ApiError { error: "Not authorized".into() })))?;

    if !f.is_complete {
        return Err((StatusCode::BAD_REQUEST, Json(ApiError { error: "Fulfillment not yet complete".into() })));
    }

    let project = db::get_project_detail(&state.db, &f.project_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiError { error: "Project not found".into() })))?;

    Ok(Json(ServiceProof {
        fulfillment_id: f.id,
        project_id: f.project_id,
        project_title: project.title,
        arrival_at: f.arrival_at.unwrap_or_default(),
        start_at: f.start_at.unwrap_or_default(),
        end_at: f.end_at.unwrap_or_default(),
        service_record_hash: f.service_record_hash.unwrap_or_default(),
        generated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    }))
}
