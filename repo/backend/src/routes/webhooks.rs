use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::Arc;

use crate::db;
use crate::middleware::{require_auth, require_role};
use crate::AppState;
use common::*;

type HmacSha256 = Hmac<Sha256>;

pub async fn create_webhook(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<WebhookConfig>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let user = require_role(&state.db, &auth.user_id, &[Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden: admin only".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: CreateWebhookRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    // Validate URL is local/on-prem (private network)
    if !is_local_url(&req.url) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError { error: "Webhooks are only supported for on-prem devices on the local network".into() }),
        ));
    }

    let webhook_id = uuid::Uuid::new_v4().to_string();
    // Generate signing secret
    use rand::Rng;
    let secret_bytes: [u8; 32] = rand::thread_rng().gen();
    let secret = hex::encode(secret_bytes);
    let event_types_json = serde_json::to_string(&req.event_types).unwrap_or_else(|_| "[]".to_string());

    db::create_webhook(&state.db, &webhook_id, &req.name, &req.url, &secret, &event_types_json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    db::append_ops_log(&state.db, &auth.user_id, &user.display_name, "create_webhook",
        &format!("Created webhook '{}' -> {}", req.name, req.url));

    let hooks = db::list_webhooks(&state.db);
    let hook = hooks.into_iter().find(|h| h.id == webhook_id).unwrap();
    Ok(Json(hook))
}

pub async fn list_webhooks(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<WebhookConfig>>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    require_role(&state.db, &auth.user_id, &[Role::Administrator])?;
    Ok(Json(db::list_webhooks(&state.db)))
}

pub async fn delete_webhook(
    State(state): State<Arc<AppState>>,
    Path(webhook_id): Path<String>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let user = require_role(&state.db, &auth.user_id, &[Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden".into() })))?;

    db::delete_webhook(&state.db, &webhook_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    db::append_ops_log(&state.db, &auth.user_id, &user.display_name, "delete_webhook",
        &format!("Deleted webhook {}", webhook_id));

    Ok(Json(ApiSuccess { message: "Webhook deleted".into() }))
}

pub async fn webhook_deliveries(
    State(state): State<Arc<AppState>>,
    Path(webhook_id): Path<String>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<WebhookDeliveryLog>>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    require_role(&state.db, &auth.user_id, &[Role::Administrator])?;
    Ok(Json(db::list_webhook_deliveries(&state.db, &webhook_id, 50)))
}

/// Fire webhooks for a given event type. Retries up to 3 times with exponential backoff.
/// Signs payload with HMAC-SHA256 using the webhook's secret.
pub async fn fire_webhooks(state: &AppState, event_type: &str, payload: &serde_json::Value) {
    let hooks = db::get_webhooks_for_event(&state.db, event_type);
    let payload_str = serde_json::to_string(payload).unwrap_or_default();
    let summary = if payload_str.len() > 200 {
        format!("{}...", &payload_str[..200])
    } else {
        payload_str.clone()
    };

    for hook in hooks {
        let db = state.db.clone();
        let url = hook.url.clone();
        let secret = hook.secret.clone();
        let hook_id = hook.id.clone();
        let evt = event_type.to_string();
        let summary = summary.clone();
        let payload_str = payload_str.clone();

        tokio::spawn(async move {
            let max_attempts = 3;
            for attempt in 1..=max_attempts {
                let result = deliver_webhook(&url, &secret, &evt, &payload_str).await;
                let log_id = uuid::Uuid::new_v4().to_string();

                match result {
                    Ok(status) => {
                        let success = status >= 200 && status < 300;
                        let _ = db::log_webhook_delivery(
                            &db, &log_id, &hook_id, &evt, &summary, attempt,
                            Some(status as i32), success, None,
                        );
                        if success {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = db::log_webhook_delivery(
                            &db, &log_id, &hook_id, &evt, &summary, attempt,
                            None, false, Some(&e),
                        );
                    }
                }

                if attempt < max_attempts {
                    // Exponential backoff: 1s, 2s, 4s
                    let delay = std::time::Duration::from_secs(1 << (attempt - 1));
                    tokio::time::sleep(delay).await;
                }
            }
        });
    }
}

/// Deliver a single webhook call with HMAC signature.
async fn deliver_webhook(url: &str, secret: &str, event_type: &str, payload: &str) -> Result<u16, String> {
    // Compute HMAC signature
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| e.to_string())?;
    mac.update(payload.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("X-Webhook-Event", event_type)
        .header("X-Webhook-Signature", &signature)
        .body(payload.to_string())
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(resp.status().as_u16())
}

/// Check if a URL targets a local/private network by parsing the host component.
fn is_local_url(url: &str) -> bool {
    // Parse as a URL so we examine only the host, not the path/query
    let parsed = match url::Url::parse(url) {
        Ok(u) => u,
        Err(_) => return false, // Reject unparseable URLs
    };

    // Only allow http (not https to an external CA, not other schemes)
    if parsed.scheme() != "http" {
        return false;
    }

    let host_str = match parsed.host_str() {
        Some(h) => h.to_lowercase(),
        None => return false,
    };

    // Check well-known local hostnames
    if host_str == "localhost" || host_str.ends_with(".local") || host_str == "[::1]" {
        return true;
    }

    // Try to parse as IP address and check against private ranges
    if let Ok(ip) = host_str.parse::<std::net::Ipv4Addr>() {
        return ip.is_loopback()      // 127.0.0.0/8
            || ip.is_private()        // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
            || ip.is_link_local();    // 169.254.0.0/16
    }
    if let Ok(ip) = host_str.trim_matches(|c| c == '[' || c == ']').parse::<std::net::Ipv6Addr>() {
        return ip.is_loopback();      // ::1
    }

    false
}
