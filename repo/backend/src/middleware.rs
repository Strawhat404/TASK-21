use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::auth::validate_session_token;
use crate::db::DbPool;
use crate::AppState;

/// Per-session rate limit bucket.
#[derive(Clone)]
pub struct RateLimitState {
    pub buckets: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    pub max_requests: usize,
    pub window_secs: u64,
}

impl RateLimitState {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window_secs,
        }
    }

    pub fn check(&self, key: &str) -> bool {
        let mut buckets = self.buckets.lock();
        let now = Instant::now();
        let window = std::time::Duration::from_secs(self.window_secs);
        let entries = buckets.entry(key.to_string()).or_default();
        entries.retain(|t| now.duration_since(*t) < window);
        if entries.len() >= self.max_requests {
            false
        } else {
            entries.push(now);
            true
        }
    }
}

/// Rate-limiting middleware: 60 requests per minute per session token (authenticated)
/// or per client IP (anonymous).
pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let key = if let Some(auth_val) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        auth_val.to_string()
    } else {
        // For anonymous requests, key by client IP from X-Forwarded-For or
        // X-Real-IP header, falling back to a per-request "anonymous" prefix.
        let ip = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(|s| s.trim().to_string())
            .or_else(|| {
                headers
                    .get("x-real-ip")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());
        format!("anon:{}", ip)
    };

    if !state.rate_limiter.check(&key) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

/// Authentication extraction: reads Authorization header, validates token, injects user_id.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim_start_matches("Bearer ").to_string());

    if let Some(token) = auth_header {
        if let Some(user_id) = validate_session_token(&token, &state.hmac_secret) {
            request.extensions_mut().insert(AuthUser { user_id });
        }
    }

    Ok(next.run(request).await)
}

/// Nonce replay protection middleware.
pub async fn nonce_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Only enforce nonce on mutating requests
    let method = request.method().clone();
    if method == axum::http::Method::GET || method == axum::http::Method::OPTIONS {
        return Ok(next.run(request).await);
    }

    let nonce = headers.get("x-nonce").and_then(|v| v.to_str().ok());
    match nonce {
        Some(n) => {
            if !crate::db::consume_nonce(&state.db, n).unwrap_or(false) {
                return Err(StatusCode::CONFLICT); // Nonce already used or expired
            }
        }
        None => {
            // Nonce is mandatory for all mutating requests
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    Ok(next.run(request).await)
}

/// Authenticated user info injected into request extensions.
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: String,
}

/// Helper to extract AuthUser from request extensions.
pub fn require_auth(extensions: &axum::http::Extensions) -> Result<AuthUser, StatusCode> {
    extensions
        .get::<AuthUser>()
        .cloned()
        .ok_or(StatusCode::UNAUTHORIZED)
}

/// Helper to require a specific role.
pub fn require_role(db: &DbPool, user_id: &str, allowed: &[common::Role]) -> Result<common::UserProfile, StatusCode> {
    let user = crate::db::get_user_by_id(db, user_id).ok_or(StatusCode::UNAUTHORIZED)?;
    if allowed.contains(&user.role) {
        Ok(user)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

/// Verify the user owns the project (is its manager) or is an Administrator.
pub fn require_project_owner(db: &DbPool, user_id: &str, project_id: &str) -> Result<(), StatusCode> {
    let user = crate::db::get_user_by_id(db, user_id).ok_or(StatusCode::UNAUTHORIZED)?;
    if user.role == common::Role::Administrator {
        return Ok(());
    }
    let manager_id = crate::db::get_project_manager_id(db, project_id).ok_or(StatusCode::NOT_FOUND)?;
    if manager_id == user_id {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}
