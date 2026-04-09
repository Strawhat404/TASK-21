use server::{auth, crypto, db, middleware, routes, AppState};

use axum::{
    middleware as axum_mw,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let db_path = std::env::var("DB_PATH")
        .unwrap_or_else(|_| "data/fund_transparency.db".to_string());
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let db = db::init_db(&db_path);

    // Host-managed key paths — outside repo tree by default
    let hmac_path = std::env::var("HMAC_KEY_PATH")
        .unwrap_or_else(|_| "/var/lib/fund_transparency/hmac.key".to_string());
    let hmac_secret = load_or_create_secret(&hmac_path);
    let encryption_key = crypto::load_or_create_key().unwrap_or_else(|e| {
        eprintln!("FATAL: {}", e);
        std::process::exit(1);
    });
    let rate_limiter = middleware::RateLimitState::new(60, 60); // 60 requests per 60 seconds

    let state = Arc::new(AppState {
        db,
        hmac_secret,
        encryption_key,
        rate_limiter,
    });

    // CORS: restrict to configured origins; fall back to same-origin only in production.
    let allowed_origins = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:8080,http://127.0.0.1:8080".to_string());
    let origins: Vec<axum::http::HeaderValue> = allowed_origins
        .split(',')
        .filter_map(|o| o.trim().parse().ok())
        .collect();
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderName::from_static("x-nonce"),
        ]);

    // Public routes (no auth required) – minimal surface
    let public_routes = Router::new()
        .route("/api/auth/register", post(routes::auth_routes::register))
        .route("/api/auth/login", post(routes::auth_routes::login))
        .route("/api/auth/nonce", get(routes::auth_routes::get_nonce))
        .route("/api/projects", get(routes::projects::list_projects))
        .route("/api/projects/{id}", get(routes::projects::get_project))
        .route("/api/projects/{id}/comments", get(routes::comments::list_comments))
        // Event tracking (allows anonymous)
        .route("/api/events/track", post(routes::events::track_event));

    // Authenticated routes
    let auth_routes = Router::new()
        .route("/api/auth/me", get(routes::auth_routes::me))
        .route("/api/auth/dnd", put(routes::auth_routes::update_dnd))
        // Projects
        .route("/api/projects", post(routes::projects::create_project))
        .route("/api/projects/updates", post(routes::projects::post_update))
        .route("/api/projects/expenses", post(routes::projects::record_expense))
        .route("/api/projects/{id}/favorite", post(routes::projects::toggle_favorite))
        .route("/api/projects/{id}/subscribe", post(routes::projects::subscribe))
        .route("/api/projects/{id}/unsubscribe", post(routes::projects::unsubscribe))
        .route("/api/updates/{id}/like", post(routes::projects::toggle_like))
        .route("/api/favorites", get(routes::projects::list_favorites))
        .route("/api/favorites/projects", get(routes::projects::list_favorite_projects))
        // Gated data endpoints (moved from public)
        .route("/api/projects/{id}/tickets", get(routes::comments::list_tickets))
        .route("/api/projects/{id}/expenses", get(routes::projects::get_expenses))
        .route("/api/projects/{id}/fulfillments", get(routes::fulfillment::list_fulfillments))
        .route("/api/fulfillments/{id}", get(routes::fulfillment::get_fulfillment))
        .route("/api/fulfillments/{id}/proof", get(routes::fulfillment::service_proof))
        .route("/api/expenses/{id}/receipts", get(routes::receipts::list_receipts))
        // Donations & Refunds
        .route("/api/donations", post(routes::donations::donate))
        .route("/api/donations/mine", get(routes::donations::my_donations))
        .route("/api/donations/refund", post(routes::donations::request_refund))
        .route("/api/donations/refund/approve", post(routes::donations::approve_refund))
        .route("/api/donations/refund/pending", get(routes::donations::pending_refunds))
        // Comments & Tickets
        .route("/api/comments", post(routes::comments::create_comment))
        .route("/api/comments/{id}/delete", post(routes::comments::delete_comment))
        .route("/api/tickets", post(routes::comments::submit_ticket))
        .route("/api/tickets/respond", post(routes::comments::respond_ticket))
        // Notifications
        .route("/api/notifications", get(routes::notifications::list_notifications))
        .route("/api/notifications/{id}/read", post(routes::notifications::mark_read))
        .route("/api/notifications/read-all", post(routes::notifications::mark_all_read))
        // Receipts
        .route("/api/receipts/upload", post(routes::receipts::upload_receipt))
        .route("/api/receipts/review", post(routes::receipts::review_receipt))
        .route("/api/receipts/pending", get(routes::receipts::pending_receipts))
        // Content Moderation
        .route("/api/moderation/config", get(routes::moderation::get_config))
        .route("/api/moderation/config", put(routes::moderation::update_config))
        .route("/api/moderation/comments/pending", get(routes::moderation::pending_comments))
        .route("/api/moderation/comments/review", post(routes::moderation::moderate_comment))
        // Fulfillment Verification
        .route("/api/fulfillments", post(routes::fulfillment::create_fulfillment))
        .route("/api/fulfillments/code", post(routes::fulfillment::generate_code))
        .route("/api/fulfillments/checkpoint", post(routes::fulfillment::record_checkpoint))
        // Event Analytics
        .route("/api/events/quality", get(routes::events::data_quality))
        .route("/api/events/suspicious", get(routes::events::suspicious_events))
        // Webhooks
        .route("/api/webhooks", post(routes::webhooks::create_webhook))
        .route("/api/webhooks", get(routes::webhooks::list_webhooks))
        .route("/api/webhooks/{id}", delete(routes::webhooks::delete_webhook))
        .route("/api/webhooks/{id}/deliveries", get(routes::webhooks::webhook_deliveries))
        // Finance
        .route("/api/finance/pending", get(routes::finance::pending_expenses))
        .route("/api/finance/review", post(routes::finance::review_expense))
        // Admin
        .route("/api/admin/stats", get(routes::admin::dashboard_stats))
        .route("/api/admin/ops-log", get(routes::admin::ops_log))
        .route("/api/admin/projects/{id}/unpublish", post(routes::admin::unpublish_project))
        .route("/api/admin/export/csv", get(routes::admin::export_csv))
        .route("/api/admin/assign-role", post(routes::admin::assign_role))
        .route("/api/admin/bootstrap", post(routes::admin::bootstrap_admin))
        .layer(axum_mw::from_fn_with_state(state.clone(), middleware::auth_middleware));

    let app = Router::new()
        .merge(public_routes)
        .merge(auth_routes)
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            middleware::nonce_middleware,
        ))
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            middleware::rate_limit_middleware,
        ))
        .layer(cors)
        .fallback_service(ServeDir::new("static"))
        .with_state(state);

    let addr = "0.0.0.0:3000";
    println!("Fund Transparency server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn load_or_create_secret(path: &str) -> Vec<u8> {
    use std::path::Path;
    let p = Path::new(path);
    if p.exists() {
        std::fs::read(p).expect("Failed to read HMAC key")
    } else {
        use rand::Rng;
        let secret: Vec<u8> = (0..64).map(|_| rand::thread_rng().gen()).collect();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(p, &secret).expect("Failed to write HMAC key");
        secret
    }
}
