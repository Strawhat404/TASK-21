use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;

use crate::auth::{create_session_token, generate_nonce, hash_password, verify_password};
use crate::db;
use crate::middleware::{require_auth, AuthUser};
use crate::AppState;
use common::*;

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ApiError>)> {
    if req.email.is_empty() || req.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: "Email required and password must be at least 8 characters".into(),
            }),
        ));
    }

    if db::get_user_by_email(&state.db, &req.email).is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiError {
                error: "Email already registered".into(),
            }),
        ));
    }

    let password_hash = hash_password(&req.password).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError { error: e }),
        )
    })?;

    // Allow self-registration as Supporter, ProjectManager, or FinanceReviewer.
    // Administrator role requires admin assignment post-registration.
    let assigned_role = match req.role {
        Role::Administrator => Role::Supporter,
        other => other,
    };

    let user_id = uuid::Uuid::new_v4().to_string();
    db::create_user(
        &state.db,
        &user_id,
        &req.email,
        &req.display_name,
        &password_hash,
        assigned_role.as_str(),
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    let token = create_session_token(&user_id, &state.hmac_secret, 86400);
    let user = db::get_user_by_id(&state.db, &user_id).unwrap();

    db::append_ops_log(&state.db, &user_id, &req.display_name, "register", &format!("New user registered: {}", mask_email(&req.email)));

    Ok(Json(AuthResponse { token, user }))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ApiError>)> {
    let (user_id, _email, display_name, password_hash, _role, _dnd_s, _dnd_e) =
        db::get_user_by_email(&state.db, &req.email).ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ApiError {
                    error: "Invalid credentials".into(),
                }),
            )
        })?;

    if !verify_password(&req.password, &password_hash) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Invalid credentials".into(),
            }),
        ));
    }

    let token = create_session_token(&user_id, &state.hmac_secret, 86400);
    let user = db::get_user_by_id(&state.db, &user_id).unwrap();

    db::append_ops_log(&state.db, &user_id, &display_name, "login", "User logged in");

    Ok(Json(AuthResponse { token, user }))
}

pub async fn me(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<UserProfile>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    let user = db::get_user_by_id(&state.db, &auth.user_id).ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(user))
}

pub async fn get_nonce(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let nonce = generate_nonce();
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::minutes(5))
        .unwrap()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let _ = db::store_nonce(&state.db, &nonce, &expires_at);
    Json(serde_json::json!({ "nonce": nonce }))
}

pub async fn update_dnd(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let req: DndSettings = serde_json::from_slice(&body)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let tz = if req.timezone.is_empty() { None } else { Some(req.timezone.as_str()) };
    db::update_dnd(&state.db, &auth.user_id, &req.dnd_start, &req.dnd_end, tz)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ApiSuccess {
        message: "DND settings updated".into(),
    }))
}

/// Mask an email for logging: "john.doe@example.com" → "j***@example.com"
fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((local, domain)) => {
            let first = local.chars().next().unwrap_or('*');
            format!("{}***@{}", first, domain)
        }
        None => "***".to_string(),
    }
}
