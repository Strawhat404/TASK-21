//! Extended API integration tests.
//!
//! Complements `integration.rs` with broader route coverage: comments,
//! tickets, notifications, favorites, subscriptions, likes, DND, moderation
//! config, dashboard stats, ops log, refunds, events, receipts edge cases,
//! fulfillment time-consistency, and webhook URL validation edge cases.
//!
//! Style matches `integration.rs` (axum `.oneshot()` via tower::util::ServiceExt).

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware as axum_mw,
    routing::{delete, get, post, put},
    Router,
};
use server::{auth, db, middleware, routes, AppState};
use std::sync::Arc;
use tower::util::ServiceExt;

fn test_db() -> db::DbPool {
    db::init_db(":memory:")
}

fn test_state() -> Arc<AppState> {
    let pool = test_db();
    let key: [u8; 32] = [0xAB; 32];
    Arc::new(AppState {
        db: pool,
        hmac_secret: b"extended-tests-hmac-secret-key-ok".to_vec(),
        encryption_key: key,
        rate_limiter: middleware::RateLimitState::new(10_000, 60),
    })
}

/// Build a router with every route the extended tests touch.
fn build_app(state: Arc<AppState>) -> Router {
    // Public
    let public = Router::new()
        .route("/api/auth/register", post(routes::auth_routes::register))
        .route("/api/auth/login", post(routes::auth_routes::login))
        .route("/api/auth/nonce", get(routes::auth_routes::get_nonce))
        .route("/api/projects", get(routes::projects::list_projects))
        .route("/api/projects/:id", get(routes::projects::get_project))
        .route(
            "/api/projects/:id/comments",
            get(routes::comments::list_comments),
        )
        .route("/api/events/track", post(routes::events::track_event));

    // Authenticated
    let authed = Router::new()
        .route("/api/auth/me", get(routes::auth_routes::me))
        .route("/api/auth/dnd", put(routes::auth_routes::update_dnd))
        // Projects
        .route("/api/projects", post(routes::projects::create_project))
        .route("/api/projects/updates", post(routes::projects::post_update))
        .route(
            "/api/projects/expenses",
            post(routes::projects::record_expense),
        )
        .route(
            "/api/projects/:id/favorite",
            post(routes::projects::toggle_favorite),
        )
        .route(
            "/api/projects/:id/subscribe",
            post(routes::projects::subscribe),
        )
        .route(
            "/api/projects/:id/unsubscribe",
            post(routes::projects::unsubscribe),
        )
        .route("/api/updates/:id/like", post(routes::projects::toggle_like))
        .route("/api/favorites", get(routes::projects::list_favorites))
        .route(
            "/api/favorites/projects",
            get(routes::projects::list_favorite_projects),
        )
        .route(
            "/api/projects/:id/expenses",
            get(routes::projects::get_expenses),
        )
        .route(
            "/api/projects/:id/tickets",
            get(routes::comments::list_tickets),
        )
        .route(
            "/api/projects/:id/fulfillments",
            get(routes::fulfillment::list_fulfillments),
        )
        .route(
            "/api/fulfillments/:id",
            get(routes::fulfillment::get_fulfillment),
        )
        .route(
            "/api/fulfillments/:id/proof",
            get(routes::fulfillment::service_proof),
        )
        .route(
            "/api/expenses/:id/receipts",
            get(routes::receipts::list_receipts),
        )
        // Donations
        .route("/api/donations", post(routes::donations::donate))
        .route("/api/donations/mine", get(routes::donations::my_donations))
        .route(
            "/api/donations/refund",
            post(routes::donations::request_refund),
        )
        .route(
            "/api/donations/refund/approve",
            post(routes::donations::approve_refund),
        )
        .route(
            "/api/donations/refund/pending",
            get(routes::donations::pending_refunds),
        )
        // Comments / tickets
        .route("/api/comments", post(routes::comments::create_comment))
        .route(
            "/api/comments/:id/delete",
            post(routes::comments::delete_comment),
        )
        .route("/api/tickets", post(routes::comments::submit_ticket))
        .route(
            "/api/tickets/respond",
            post(routes::comments::respond_ticket),
        )
        // Notifications
        .route(
            "/api/notifications",
            get(routes::notifications::list_notifications),
        )
        .route(
            "/api/notifications/:id/read",
            post(routes::notifications::mark_read),
        )
        .route(
            "/api/notifications/read-all",
            post(routes::notifications::mark_all_read),
        )
        // Receipts
        .route("/api/receipts/upload", post(routes::receipts::upload_receipt))
        .route("/api/receipts/review", post(routes::receipts::review_receipt))
        .route(
            "/api/receipts/pending",
            get(routes::receipts::pending_receipts),
        )
        // Moderation
        .route(
            "/api/moderation/config",
            get(routes::moderation::get_config).put(routes::moderation::update_config),
        )
        .route(
            "/api/moderation/comments/pending",
            get(routes::moderation::pending_comments),
        )
        .route(
            "/api/moderation/comments/review",
            post(routes::moderation::moderate_comment),
        )
        // Fulfillment
        .route(
            "/api/fulfillments",
            post(routes::fulfillment::create_fulfillment),
        )
        .route(
            "/api/fulfillments/code",
            post(routes::fulfillment::generate_code),
        )
        .route(
            "/api/fulfillments/checkpoint",
            post(routes::fulfillment::record_checkpoint),
        )
        // Events analytics
        .route("/api/events/quality", get(routes::events::data_quality))
        .route(
            "/api/events/suspicious",
            get(routes::events::suspicious_events),
        )
        // Admin
        .route("/api/admin/stats", get(routes::admin::dashboard_stats))
        .route("/api/admin/ops-log", get(routes::admin::ops_log))
        .route(
            "/api/admin/projects/:id/unpublish",
            post(routes::admin::unpublish_project),
        )
        .route("/api/admin/export/csv", get(routes::admin::export_csv))
        .route("/api/admin/assign-role", post(routes::admin::assign_role))
        .route("/api/admin/bootstrap", post(routes::admin::bootstrap_admin))
        // Finance
        .route("/api/finance/pending", get(routes::finance::pending_expenses))
        .route("/api/finance/review", post(routes::finance::review_expense))
        // Webhooks
        .route(
            "/api/webhooks",
            post(routes::webhooks::create_webhook).get(routes::webhooks::list_webhooks),
        )
        .route("/api/webhooks/:id", delete(routes::webhooks::delete_webhook))
        .route(
            "/api/webhooks/:id/deliveries",
            get(routes::webhooks::webhook_deliveries),
        )
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ));

    Router::new()
        .merge(public)
        .merge(authed)
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            middleware::nonce_middleware,
        ))
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            middleware::rate_limit_middleware,
        ))
        .with_state(state)
}

// ─── Shared helpers ───────────────────────────────────────────────────

async fn get_nonce(app: &Router) -> String {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/nonce")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(resp.into_body(), 1_048_576).await.unwrap();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    val["nonce"].as_str().unwrap().to_string()
}

/// Build an authenticated POST/PUT/DELETE request with the usual headers.
async fn send_post(
    app: &Router,
    uri: &str,
    token: Option<&str>,
    body: &serde_json::Value,
) -> axum::response::Response {
    let nonce = get_nonce(app).await;
    let mut req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("Content-Type", "application/json")
        .header("X-Nonce", &nonce);
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }
    app.clone()
        .oneshot(req.body(Body::from(serde_json::to_vec(body).unwrap())).unwrap())
        .await
        .unwrap()
}

async fn send_get(app: &Router, uri: &str, token: Option<&str>) -> axum::response::Response {
    let mut req = Request::builder().uri(uri);
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }
    app.clone().oneshot(req.body(Body::empty()).unwrap()).await.unwrap()
}

async fn send_put(
    app: &Router,
    uri: &str,
    token: Option<&str>,
    body: &serde_json::Value,
) -> axum::response::Response {
    let nonce = get_nonce(app).await;
    let mut req = Request::builder()
        .method("PUT")
        .uri(uri)
        .header("Content-Type", "application/json")
        .header("X-Nonce", &nonce);
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }
    app.clone()
        .oneshot(req.body(Body::from(serde_json::to_vec(body).unwrap())).unwrap())
        .await
        .unwrap()
}

fn make_user(db: &db::DbPool, id: &str, email: &str, role: &str, password: &str) {
    let hash = auth::hash_password(password).unwrap();
    db::create_user(db, id, email, id, &hash, role).unwrap();
}

async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1_048_576).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// ══════════════════════════════════════════════════════════════════════
// Project detail & listing
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn route_get_project_by_id_returns_detail() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    let bl = vec![("bl1".to_string(), "Pipes".to_string(), 40_000i64)];
    db::create_project(
        &state.db, "p1", "Test", "Desc", "health", "11111", 100_000, "mgr1", &bl,
    )
    .unwrap();
    let app = build_app(state);

    let resp = send_get(&app, "/api/projects/p1", None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    assert_eq!(val["id"], "p1");
    assert_eq!(val["title"], "Test");
    assert_eq!(val["status"], "active");
    assert_eq!(val["goal_cents"], 100_000);
    assert_eq!(val["budget_lines"].as_array().unwrap().len(), 1);
    assert_eq!(val["budget_lines"][0]["name"], "Pipes");
}

#[tokio::test]
async fn route_get_project_by_id_returns_404_when_missing() {
    let state = test_state();
    let app = build_app(state);
    let resp = send_get(&app, "/api/projects/does-not-exist", None).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn route_list_projects_filters_by_cause() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    db::create_project(
        &state.db, "px", "Health Thing", "D", "health", "10001", 5000, "mgr1", &[],
    )
    .unwrap();
    db::create_project(
        &state.db, "py", "Edu Thing", "D", "education", "10002", 5000, "mgr1", &[],
    )
    .unwrap();
    let app = build_app(state);

    let resp = send_get(&app, "/api/projects?cause=health", None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    let items = val["items"].as_array().unwrap();
    for it in items {
        assert_eq!(it["cause"], "health", "filter should only return health projects");
    }
    // Our health project "Health Thing" should be there; "Edu Thing" should not.
    let titles: Vec<&str> = items.iter().filter_map(|i| i["title"].as_str()).collect();
    assert!(titles.contains(&"Health Thing"));
    assert!(!titles.contains(&"Edu Thing"));
}

#[tokio::test]
async fn route_list_projects_filters_by_zip_code() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    db::create_project(
        &state.db, "pa", "Zip A", "D", "health", "99999", 5000, "mgr1", &[],
    )
    .unwrap();
    db::create_project(
        &state.db, "pb", "Zip B", "D", "health", "88888", 5000, "mgr1", &[],
    )
    .unwrap();
    let app = build_app(state);

    let resp = send_get(&app, "/api/projects?zip_code=99999", None).await;
    let val = body_json(resp).await;
    for it in val["items"].as_array().unwrap() {
        assert_eq!(it["zip_code"], "99999");
    }
}

#[tokio::test]
async fn route_list_projects_pagination_respects_per_page_cap() {
    let state = test_state();
    let app = build_app(state);
    // per_page > 100 should be clamped to 100; we check the response is still structured.
    let resp = send_get(&app, "/api/projects?per_page=5000", None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    assert!(val["per_page"].as_i64().unwrap() <= 100);
}

// ══════════════════════════════════════════════════════════════════════
// Comments / Tickets
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn route_list_comments_for_project_returns_empty_when_none() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    let app = build_app(state);

    let resp = send_get(&app, "/api/projects/p1/comments", None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    assert_eq!(val.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn route_post_comment_happy_path_appears_in_listing() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "sup1", "s@t.com", "supporter", "pw12345678");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    let token = auth::create_session_token("sup1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({ "project_id": "p1", "body": "Nice project" });
    let resp = send_post(&app, "/api/comments", Some(&token), &body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let list = send_get(&app, "/api/projects/p1/comments", None).await;
    let val = body_json(list).await;
    let arr = val.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["body"], "Nice project");
}

#[tokio::test]
async fn route_post_comment_rejected_with_sensitive_word() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "sup1", "s@t.com", "supporter", "pw12345678");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();

    // Turn on sensitive-word filter
    db::update_moderation_config(
        &state.db,
        &common::ModerationConfig {
            comments_enabled: true,
            require_pre_moderation: false,
            sensitive_words: vec!["forbidden".into()],
        },
    )
    .unwrap();
    let token = auth::create_session_token("sup1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({ "project_id": "p1", "body": "This has forbidden content" });
    let resp = send_post(&app, "/api/comments", Some(&token), &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_post_comment_rejected_when_disabled() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "sup1", "s@t.com", "supporter", "pw12345678");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    db::update_moderation_config(
        &state.db,
        &common::ModerationConfig {
            comments_enabled: false,
            require_pre_moderation: false,
            sensitive_words: vec![],
        },
    )
    .unwrap();

    let token = auth::create_session_token("sup1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({ "project_id": "p1", "body": "Should fail" });
    let resp = send_post(&app, "/api/comments", Some(&token), &body).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn route_post_comment_lands_in_pending_with_pre_moderation() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "sup1", "s@t.com", "supporter", "pw12345678");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    db::update_moderation_config(
        &state.db,
        &common::ModerationConfig {
            comments_enabled: true,
            require_pre_moderation: true,
            sensitive_words: vec![],
        },
    )
    .unwrap();
    let token = auth::create_session_token("sup1", &state.hmac_secret, 3600);
    let app = build_app(state.clone());

    let body = serde_json::json!({ "project_id": "p1", "body": "Please review" });
    let resp = send_post(&app, "/api/comments", Some(&token), &body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Pre-moderation: comment is stored but not public
    let pending = db::list_pending_comments(&state.db);
    assert_eq!(pending.len(), 1, "New comment should be pending review");
    assert_eq!(pending[0].body, "Please review");

    let public = send_get(&app, "/api/projects/p1/comments", None).await;
    let val = body_json(public).await;
    assert_eq!(
        val.as_array().unwrap().len(),
        0,
        "Pending comments must not appear on the public list"
    );
}

#[tokio::test]
async fn route_submit_ticket_happy_path() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "sup1", "s@t.com", "supporter", "pw12345678");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    let token = auth::create_session_token("sup1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "project_id": "p1",
        "subject": "Question",
        "body": "When will the project start?"
    });
    let resp = send_post(&app, "/api/tickets", Some(&token), &body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    assert!(val["message"].as_str().unwrap().starts_with("Ticket "));
}

#[tokio::test]
async fn route_respond_ticket_owner_succeeds_non_owner_rejected() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m1@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "mgr2", "m2@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "sup1", "s@t.com", "supporter", "pw12345678");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    db::create_ticket(&state.db, "t1", "p1", "sup1", "Q", "body").unwrap();

    let mgr1_tok = auth::create_session_token("mgr1", &state.hmac_secret, 3600);
    let mgr2_tok = auth::create_session_token("mgr2", &state.hmac_secret, 3600);
    let app = build_app(state);

    // Owner manager succeeds
    let resp_ok = send_post(
        &app,
        "/api/tickets/respond",
        Some(&mgr1_tok),
        &serde_json::json!({ "ticket_id": "t1", "response": "We start in 2 weeks." }),
    )
    .await;
    assert_eq!(resp_ok.status(), StatusCode::OK);

    // Non-owner manager is rejected
    let resp_forbid = send_post(
        &app,
        "/api/tickets/respond",
        Some(&mgr2_tok),
        &serde_json::json!({ "ticket_id": "t1", "response": "Hi" }),
    )
    .await;
    assert_eq!(resp_forbid.status(), StatusCode::FORBIDDEN);
}

// ══════════════════════════════════════════════════════════════════════
// Notifications
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn route_list_notifications_empty_for_new_user() {
    let state = test_state();
    make_user(&state.db, "u1", "u@t.com", "supporter", "pw12345678");
    let token = auth::create_session_token("u1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_get(&app, "/api/notifications", Some(&token)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    assert_eq!(val.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn route_list_notifications_scoped_to_user() {
    let state = test_state();
    make_user(&state.db, "u1", "u1@t.com", "supporter", "pw12345678");
    make_user(&state.db, "u2", "u2@t.com", "supporter", "pw12345678");
    db::create_notification(&state.db, "n1", "u1", "Hello u1", "Body").unwrap();
    db::create_notification(&state.db, "n2", "u2", "Hello u2", "Body").unwrap();

    let u1_tok = auth::create_session_token("u1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_get(&app, "/api/notifications", Some(&u1_tok)).await;
    let val = body_json(resp).await;
    let arr = val.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["user_id"], "u1");
}

#[tokio::test]
async fn route_mark_notification_read_flips_flag() {
    let state = test_state();
    make_user(&state.db, "u1", "u@t.com", "supporter", "pw12345678");
    db::create_notification(&state.db, "n1", "u1", "T", "B").unwrap();

    let token = auth::create_session_token("u1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_post(
        &app,
        "/api/notifications/n1/read",
        Some(&token),
        &serde_json::json!({}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let list = send_get(&app, "/api/notifications", Some(&token)).await;
    let val = body_json(list).await;
    assert_eq!(val[0]["is_read"], true);
}

#[tokio::test]
async fn route_mark_all_read() {
    let state = test_state();
    make_user(&state.db, "u1", "u@t.com", "supporter", "pw12345678");
    db::create_notification(&state.db, "n1", "u1", "T1", "B").unwrap();
    db::create_notification(&state.db, "n2", "u1", "T2", "B").unwrap();

    let token = auth::create_session_token("u1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_post(
        &app,
        "/api/notifications/read-all",
        Some(&token),
        &serde_json::json!({}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let list = send_get(&app, "/api/notifications", Some(&token)).await;
    let val = body_json(list).await;
    for n in val.as_array().unwrap() {
        assert_eq!(n["is_read"], true);
    }
}

// ══════════════════════════════════════════════════════════════════════
// Favorites / Subscriptions / Likes
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn route_favorite_toggles_on_and_off() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "u1", "u@t.com", "supporter", "pw12345678");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();

    let token = auth::create_session_token("u1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let r1 = send_post(
        &app,
        "/api/projects/p1/favorite",
        Some(&token),
        &serde_json::json!({}),
    )
    .await;
    assert_eq!(r1.status(), StatusCode::OK);
    let v1 = body_json(r1).await;
    assert_eq!(v1["favorited"], true);

    let r2 = send_post(
        &app,
        "/api/projects/p1/favorite",
        Some(&token),
        &serde_json::json!({}),
    )
    .await;
    let v2 = body_json(r2).await;
    assert_eq!(v2["favorited"], false);
}

#[tokio::test]
async fn route_list_favorites_after_toggle() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "u1", "u@t.com", "supporter", "pw12345678");
    db::create_project(&state.db, "p1", "P1", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    db::create_project(&state.db, "p2", "P2", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();

    let token = auth::create_session_token("u1", &state.hmac_secret, 3600);
    let app = build_app(state);

    send_post(
        &app,
        "/api/projects/p1/favorite",
        Some(&token),
        &serde_json::json!({}),
    )
    .await;
    let resp = send_get(&app, "/api/favorites", Some(&token)).await;
    let val = body_json(resp).await;
    let ids: Vec<&str> = val.as_array().unwrap().iter().filter_map(|v| v.as_str()).collect();
    assert!(ids.contains(&"p1"));
    assert!(!ids.contains(&"p2"));
}

#[tokio::test]
async fn route_subscribe_and_unsubscribe() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "u1", "u@t.com", "supporter", "pw12345678");
    db::create_project(&state.db, "p1", "P1", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();

    let token = auth::create_session_token("u1", &state.hmac_secret, 3600);
    let app = build_app(state.clone());

    let sub = send_post(
        &app,
        "/api/projects/p1/subscribe",
        Some(&token),
        &serde_json::json!({}),
    )
    .await;
    assert_eq!(sub.status(), StatusCode::OK);
    assert!(db::get_project_subscribers(&state.db, "p1").contains(&"u1".to_string()));

    let unsub = send_post(
        &app,
        "/api/projects/p1/unsubscribe",
        Some(&token),
        &serde_json::json!({}),
    )
    .await;
    assert_eq!(unsub.status(), StatusCode::OK);
    assert!(!db::get_project_subscribers(&state.db, "p1").contains(&"u1".to_string()));
}

#[tokio::test]
async fn route_toggle_like_on_and_off() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "u1", "u@t.com", "supporter", "pw12345678");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    db::create_spending_update(&state.db, "up1", "p1", "mgr1", "Title", "Body").unwrap();

    let token = auth::create_session_token("u1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let r1 = send_post(
        &app,
        "/api/updates/up1/like",
        Some(&token),
        &serde_json::json!({}),
    )
    .await;
    let v1 = body_json(r1).await;
    assert_eq!(v1["liked"], true);

    let r2 = send_post(
        &app,
        "/api/updates/up1/like",
        Some(&token),
        &serde_json::json!({}),
    )
    .await;
    let v2 = body_json(r2).await;
    assert_eq!(v2["liked"], false);
}

// ══════════════════════════════════════════════════════════════════════
// DND settings
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn route_update_dnd_persists() {
    let state = test_state();
    make_user(&state.db, "u1", "u@t.com", "supporter", "pw12345678");
    let token = auth::create_session_token("u1", &state.hmac_secret, 3600);
    let app = build_app(state.clone());

    let body = serde_json::json!({
        "dnd_start": "22:00",
        "dnd_end": "06:30",
        "timezone": "+05:30"
    });
    let resp = send_put(&app, "/api/auth/dnd", Some(&token), &body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let user = db::get_user_by_id(&state.db, "u1").unwrap();
    assert_eq!(user.dnd_start, "22:00");
    assert_eq!(user.dnd_end, "06:30");
    assert_eq!(user.timezone, "+05:30");
}

#[tokio::test]
async fn route_update_dnd_unauthenticated_rejected() {
    let state = test_state();
    let app = build_app(state);
    let body = serde_json::json!({ "dnd_start": "22:00", "dnd_end": "06:00", "timezone": "UTC" });
    let resp = send_put(&app, "/api/auth/dnd", None, &body).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ══════════════════════════════════════════════════════════════════════
// Moderation config
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn route_moderation_config_admin_only() {
    let state = test_state();
    make_user(&state.db, "u1", "u@t.com", "supporter", "pw12345678");
    let token = auth::create_session_token("u1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_get(&app, "/api/moderation/config", Some(&token)).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn route_moderation_config_update_persists() {
    let state = test_state();
    make_user(&state.db, "admin1", "a@t.com", "administrator", "adminpass");
    let token = auth::create_session_token("admin1", &state.hmac_secret, 3600);
    let app = build_app(state.clone());

    // Default config shows comments_enabled = true
    let get_resp = send_get(&app, "/api/moderation/config", Some(&token)).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
    let val = body_json(get_resp).await;
    assert_eq!(val["comments_enabled"], true);

    // Update: disable comments
    let body = serde_json::json!({
        "comments_enabled": false,
        "require_pre_moderation": true,
        "sensitive_words": ["badword"]
    });
    let upd = send_put(&app, "/api/moderation/config", Some(&token), &body).await;
    assert_eq!(upd.status(), StatusCode::OK);

    // Verify via another read
    let get2 = send_get(&app, "/api/moderation/config", Some(&token)).await;
    let v2 = body_json(get2).await;
    assert_eq!(v2["comments_enabled"], false);
    assert_eq!(v2["require_pre_moderation"], true);
    assert_eq!(v2["sensitive_words"].as_array().unwrap()[0], "badword");
}

// ══════════════════════════════════════════════════════════════════════
// Admin stats & ops log
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn route_admin_stats_denies_supporter() {
    let state = test_state();
    make_user(&state.db, "u1", "u@t.com", "supporter", "pw12345678");
    let token = auth::create_session_token("u1", &state.hmac_secret, 3600);
    let app = build_app(state);
    let resp = send_get(&app, "/api/admin/stats", Some(&token)).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn route_admin_stats_allows_pm_finance_admin() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    make_user(&state.db, "fin", "f@t.com", "finance_reviewer", "p1234567");
    make_user(&state.db, "adm", "a@t.com", "administrator", "p1234567");
    let app = build_app(state.clone());

    for uid in ["mgr", "fin", "adm"] {
        let tok = auth::create_session_token(uid, &state.hmac_secret, 3600);
        let resp = send_get(&app, "/api/admin/stats", Some(&tok)).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "{} should have access to /admin/stats",
            uid
        );
    }
}

#[tokio::test]
async fn route_admin_ops_log_admin_only() {
    let state = test_state();
    make_user(&state.db, "fin", "f@t.com", "finance_reviewer", "p1234567");
    make_user(&state.db, "adm", "a@t.com", "administrator", "p1234567");
    let app = build_app(state.clone());

    let fin_tok = auth::create_session_token("fin", &state.hmac_secret, 3600);
    let resp = send_get(&app, "/api/admin/ops-log", Some(&fin_tok)).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let adm_tok = auth::create_session_token("adm", &state.hmac_secret, 3600);
    let resp2 = send_get(&app, "/api/admin/ops-log", Some(&adm_tok)).await;
    assert_eq!(resp2.status(), StatusCode::OK);
    let val = body_json(resp2).await;
    // Seed data produces at least one ops_log entry
    assert!(val.as_array().unwrap().len() >= 1);
}

// ══════════════════════════════════════════════════════════════════════
// Donations + refunds
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn route_my_donations_scoped_to_caller() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    make_user(&state.db, "d1", "d1@t.com", "supporter", "p1234567");
    make_user(&state.db, "d2", "d2@t.com", "supporter", "p1234567");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &[]).unwrap();
    db::create_donation(&state.db, "dd1", "PLG-A", "p1", "d1", 1000, "cash", false, None, None).unwrap();
    db::create_donation(&state.db, "dd2", "PLG-B", "p1", "d2", 2000, "cash", false, None, None).unwrap();

    let d1_tok = auth::create_session_token("d1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_get(&app, "/api/donations/mine", Some(&d1_tok)).await;
    let val = body_json(resp).await;
    let arr = val.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["donor_id"], "d1");
}

#[tokio::test]
async fn route_request_refund_by_non_donor_rejected() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    make_user(&state.db, "d1", "d1@t.com", "supporter", "p1234567");
    make_user(&state.db, "d2", "d2@t.com", "supporter", "p1234567");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &[]).unwrap();
    db::create_donation(&state.db, "dd1", "PLG-A", "p1", "d1", 1000, "cash", false, None, None).unwrap();

    let d2_tok = auth::create_session_token("d2", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({ "donation_id": "dd1", "reason": "changed mind" });
    let resp = send_post(&app, "/api/donations/refund", Some(&d2_tok), &body).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn route_request_refund_by_donor_creates_pending_reversal() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    make_user(&state.db, "d1", "d1@t.com", "supporter", "p1234567");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &[]).unwrap();
    db::create_donation(&state.db, "dd1", "PLG-A", "p1", "d1", 1000, "cash", false, None, None).unwrap();

    let d1_tok = auth::create_session_token("d1", &state.hmac_secret, 3600);
    let app = build_app(state.clone());

    let body = serde_json::json!({ "donation_id": "dd1", "reason": "duplicate" });
    let resp = send_post(&app, "/api/donations/refund", Some(&d1_tok), &body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Pending reversals list includes it (finance/admin only)
    let pending = db::list_pending_reversals(&state.db);
    assert_eq!(pending.len(), 1);
    assert!(pending[0].is_reversal);
    assert_eq!(pending[0].amount_cents, -1000);
    assert!(pending[0].reversal_approved.is_none());
}

#[tokio::test]
async fn route_pending_refunds_finance_admin_only() {
    let state = test_state();
    make_user(&state.db, "sup", "s@t.com", "supporter", "p1234567");
    make_user(&state.db, "fin", "f@t.com", "finance_reviewer", "p1234567");
    let app = build_app(state.clone());

    let sup_tok = auth::create_session_token("sup", &state.hmac_secret, 3600);
    let resp = send_get(&app, "/api/donations/refund/pending", Some(&sup_tok)).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let fin_tok = auth::create_session_token("fin", &state.hmac_secret, 3600);
    let resp2 = send_get(&app, "/api/donations/refund/pending", Some(&fin_tok)).await;
    assert_eq!(resp2.status(), StatusCode::OK);
}

#[tokio::test]
async fn route_donation_to_inactive_project_rejected() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    make_user(&state.db, "d", "d@t.com", "supporter", "p1234567");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &[]).unwrap();
    db::update_project_status(&state.db, "p1", "closed").unwrap();

    let tok = auth::create_session_token("d", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({ "project_id": "p1", "amount_cents": 100 });
    let resp = send_post(&app, "/api/donations", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_donation_bad_payment_method_rejected() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    make_user(&state.db, "d", "d@t.com", "supporter", "p1234567");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &[]).unwrap();
    let tok = auth::create_session_token("d", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "project_id": "p1",
        "amount_cents": 100,
        "payment_method": "crypto"
    });
    let resp = send_post(&app, "/api/donations", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_donation_to_missing_project_returns_404() {
    let state = test_state();
    make_user(&state.db, "d", "d@t.com", "supporter", "p1234567");
    let tok = auth::create_session_token("d", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({ "project_id": "nope", "amount_cents": 100 });
    let resp = send_post(&app, "/api/donations", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ══════════════════════════════════════════════════════════════════════
// Events analytics
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn route_events_quality_accepts_pm_fin_admin() {
    let state = test_state();
    make_user(&state.db, "adm", "a@t.com", "administrator", "p1234567");
    let app = build_app(state.clone());
    let tok = auth::create_session_token("adm", &state.hmac_secret, 3600);
    let resp = send_get(&app, "/api/events/quality", Some(&tok)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    assert!(val["total_events"].is_i64());
    assert!(val["events_by_kind"].is_array());
}

#[tokio::test]
async fn route_events_quality_denies_supporter() {
    let state = test_state();
    make_user(&state.db, "sup", "s@t.com", "supporter", "p1234567");
    let tok = auth::create_session_token("sup", &state.hmac_secret, 3600);
    let app = build_app(state);
    let resp = send_get(&app, "/api/events/quality", Some(&tok)).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn route_events_suspicious_admin_only() {
    let state = test_state();
    make_user(&state.db, "fin", "f@t.com", "finance_reviewer", "p1234567");
    make_user(&state.db, "adm", "a@t.com", "administrator", "p1234567");
    let app = build_app(state.clone());

    let fin_tok = auth::create_session_token("fin", &state.hmac_secret, 3600);
    let r1 = send_get(&app, "/api/events/suspicious", Some(&fin_tok)).await;
    assert_eq!(r1.status(), StatusCode::FORBIDDEN);

    let adm_tok = auth::create_session_token("adm", &state.hmac_secret, 3600);
    let r2 = send_get(&app, "/api/events/suspicious", Some(&adm_tok)).await;
    assert_eq!(r2.status(), StatusCode::OK);
}

#[tokio::test]
async fn route_track_event_flags_duplicate_message() {
    let state = test_state();
    let app = build_app(state);

    let body = serde_json::json!({
        "event_kind": "click",
        "target_type": "button",
        "target_id": "dup-btn",
        "session_id": "sess-dup"
    });
    let r1 = send_post(&app, "/api/events/track", None, &body).await;
    assert_eq!(r1.status(), StatusCode::OK);
    let v1 = body_json(r1).await;
    assert_eq!(v1["message"], "Event recorded");

    let r2 = send_post(&app, "/api/events/track", None, &body).await;
    assert_eq!(r2.status(), StatusCode::OK);
    let v2 = body_json(r2).await;
    assert_eq!(v2["message"], "Event recorded (duplicate)");
}

// ══════════════════════════════════════════════════════════════════════
// Project update / expense
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn route_post_project_update_non_owner_rejected() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m1@t.com", "project_manager", "p1234567");
    make_user(&state.db, "mgr2", "m2@t.com", "project_manager", "p1234567");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    let tok = auth::create_session_token("mgr2", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({ "project_id": "p1", "title": "T", "body": "Body" });
    let resp = send_post(&app, "/api/projects/updates", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn route_post_project_update_owner_notifies_subscribers() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m1@t.com", "project_manager", "p1234567");
    make_user(&state.db, "sub1", "s@t.com", "supporter", "p1234567");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    db::set_subscription(&state.db, "sub1", "p1", true).unwrap();

    let tok = auth::create_session_token("mgr1", &state.hmac_secret, 3600);
    let app = build_app(state.clone());

    let body = serde_json::json!({ "project_id": "p1", "title": "Milestone", "body": "Yay" });
    let resp = send_post(&app, "/api/projects/updates", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let notifs = db::list_notifications(&state.db, "sub1");
    assert_eq!(notifs.len(), 1);
    assert!(notifs[0].title.contains("Milestone"));
}

#[tokio::test]
async fn route_record_expense_cross_project_budget_line_rejected() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "p1234567");
    let bl1 = vec![("blA".to_string(), "A".to_string(), 10_000i64)];
    let bl2 = vec![("blB".to_string(), "B".to_string(), 10_000i64)];
    db::create_project(&state.db, "p1", "P1", "D", "health", "11111", 5000, "mgr1", &bl1).unwrap();
    db::create_project(&state.db, "p2", "P2", "D", "health", "11111", 5000, "mgr1", &bl2).unwrap();

    let tok = auth::create_session_token("mgr1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "project_id": "p1",
        "budget_line_id": "blB",
        "amount_cents": 100,
        "description": "Mismatch",
    });
    let resp = send_post(&app, "/api/projects/expenses", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ══════════════════════════════════════════════════════════════════════
// Receipts: validation errors
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn route_upload_receipt_rejects_invalid_file_type() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    let bl = vec![("bl1".to_string(), "M".to_string(), 10_000i64)];
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &bl).unwrap();
    db::create_expense(&state.db, "e1", "p1", "bl1", 5000, "Lumber", None).unwrap();
    let tok = auth::create_session_token("mgr", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "expense_id": "e1",
        "file_name": "malware.exe",
        "file_type": "application/x-executable",
        "file_size": 4,
        "file_data_base64": "dGVzdA==",
    });
    let resp = send_post(&app, "/api/receipts/upload", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_upload_receipt_rejects_oversize() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    let bl = vec![("bl1".to_string(), "M".to_string(), 10_000i64)];
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &bl).unwrap();
    db::create_expense(&state.db, "e1", "p1", "bl1", 5000, "Lumber", None).unwrap();

    let tok = auth::create_session_token("mgr", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "expense_id": "e1",
        "file_name": "big.pdf",
        "file_type": "application/pdf",
        "file_size": 11 * 1024 * 1024, // 11 MB > 10 MB cap
        "file_data_base64": "dGVzdA=="
    });
    let resp = send_post(&app, "/api/receipts/upload", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_upload_receipt_rejects_size_mismatch() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    let bl = vec![("bl1".to_string(), "M".to_string(), 10_000i64)];
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &bl).unwrap();
    db::create_expense(&state.db, "e1", "p1", "bl1", 5000, "Lumber", None).unwrap();
    let tok = auth::create_session_token("mgr", &state.hmac_secret, 3600);
    let app = build_app(state);

    // base64("test") = 4 bytes; declared 999 bytes should fail
    let body = serde_json::json!({
        "expense_id": "e1",
        "file_name": "r.pdf",
        "file_type": "application/pdf",
        "file_size": 999,
        "file_data_base64": "dGVzdA=="
    });
    let resp = send_post(&app, "/api/receipts/upload", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_upload_receipt_duplicate_fingerprint_rejected() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    let bl = vec![("bl1".to_string(), "M".to_string(), 10_000i64)];
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &bl).unwrap();
    db::create_expense(&state.db, "e1", "p1", "bl1", 5000, "Lumber", None).unwrap();

    let tok = auth::create_session_token("mgr", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "expense_id": "e1",
        "file_name": "r1.pdf",
        "file_type": "application/pdf",
        "file_size": 4,
        "file_data_base64": "dGVzdA=="  // decodes to "test"
    });
    let r1 = send_post(&app, "/api/receipts/upload", Some(&tok), &body).await;
    assert_eq!(r1.status(), StatusCode::OK);

    // Upload identical content again → duplicate
    let body2 = serde_json::json!({
        "expense_id": "e1",
        "file_name": "r2.pdf",
        "file_type": "application/pdf",
        "file_size": 4,
        "file_data_base64": "dGVzdA=="
    });
    let r2 = send_post(&app, "/api/receipts/upload", Some(&tok), &body2).await;
    assert_eq!(r2.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn route_review_receipt_rejection_without_reason_returns_400() {
    let state = test_state();
    make_user(&state.db, "fin", "f@t.com", "finance_reviewer", "p1234567");
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    let bl = vec![("bl1".to_string(), "M".to_string(), 10_000i64)];
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &bl).unwrap();
    db::create_expense(&state.db, "e1", "p1", "bl1", 5000, "Lumber", None).unwrap();
    db::create_receipt(&state.db, "r1", "e1", "f.pdf", "application/pdf", 1, b"d", "fp").unwrap();

    let tok = auth::create_session_token("fin", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "receipt_id": "r1",
        "verified": false,
        // rejection_reason missing → should be rejected
    });
    let resp = send_post(&app, "/api/receipts/review", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Empty/whitespace reason is also rejected
    let body2 = serde_json::json!({
        "receipt_id": "r1",
        "verified": false,
        "rejection_reason": "   "
    });
    let resp2 = send_post(&app, "/api/receipts/review", Some(&tok), &body2).await;
    assert_eq!(resp2.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_review_receipt_verified_happy_path() {
    let state = test_state();
    make_user(&state.db, "fin", "f@t.com", "finance_reviewer", "p1234567");
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    let bl = vec![("bl1".to_string(), "M".to_string(), 10_000i64)];
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &bl).unwrap();
    db::create_expense(&state.db, "e1", "p1", "bl1", 5000, "Lumber", None).unwrap();
    db::create_receipt(&state.db, "r1", "e1", "f.pdf", "application/pdf", 1, b"d", "fp2").unwrap();

    let tok = auth::create_session_token("fin", &state.hmac_secret, 3600);
    let app = build_app(state.clone());

    let body = serde_json::json!({ "receipt_id": "r1", "verified": true });
    let resp = send_post(&app, "/api/receipts/review", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let receipts = db::list_receipts_for_expense(&state.db, "e1");
    assert_eq!(receipts[0].status, common::ReceiptStatus::Verified);
}

#[tokio::test]
async fn route_list_receipts_finance_access() {
    let state = test_state();
    make_user(&state.db, "fin", "f@t.com", "finance_reviewer", "p1234567");
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    let bl = vec![("bl1".to_string(), "M".to_string(), 10_000i64)];
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &bl).unwrap();
    db::create_expense(&state.db, "e1", "p1", "bl1", 5000, "Lumber", None).unwrap();
    db::create_receipt(&state.db, "r1", "e1", "f.pdf", "application/pdf", 1, b"d", "fp3").unwrap();

    let fin_tok = auth::create_session_token("fin", &state.hmac_secret, 3600);
    let app = build_app(state);
    let resp = send_get(&app, "/api/expenses/e1/receipts", Some(&fin_tok)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    assert_eq!(val.as_array().unwrap().len(), 1);
}

// ══════════════════════════════════════════════════════════════════════
// Fulfillment: time consistency
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn route_fulfillment_non_owner_rejected_on_create() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m1@t.com", "project_manager", "p1234567");
    make_user(&state.db, "mgr2", "m2@t.com", "project_manager", "p1234567");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();

    let tok = auth::create_session_token("mgr2", &state.hmac_secret, 3600);
    let app = build_app(state);
    let body = serde_json::json!({ "project_id": "p1" });
    let resp = send_post(&app, "/api/fulfillments", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn route_checkpoint_invalid_code_rejected() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &[]).unwrap();
    db::create_fulfillment(&state.db, "f1", "p1").unwrap();
    let tok = auth::create_session_token("mgr", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "fulfillment_id": "f1",
        "checkpoint": "arrival",
        "code": "000000"
    });
    let resp = send_post(&app, "/api/fulfillments/checkpoint", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_checkpoint_start_without_arrival_rejected() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &[]).unwrap();
    db::create_fulfillment(&state.db, "f1", "p1").unwrap();

    // Pre-populate a valid "start" code in DB so the code check passes
    let future = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::minutes(5))
        .unwrap()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    db::store_checkpoint_code(&state.db, "c1", "f1", "start", "123456", &future).unwrap();

    let tok = auth::create_session_token("mgr", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "fulfillment_id": "f1",
        "checkpoint": "start",
        "code": "123456"
    });
    let resp = send_post(&app, "/api/fulfillments/checkpoint", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_checkpoint_end_without_start_rejected() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &[]).unwrap();
    db::create_fulfillment(&state.db, "f1", "p1").unwrap();

    let future = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::minutes(5))
        .unwrap()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    db::store_checkpoint_code(&state.db, "c1", "f1", "end", "987654", &future).unwrap();

    let tok = auth::create_session_token("mgr", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "fulfillment_id": "f1",
        "checkpoint": "end",
        "code": "987654"
    });
    let resp = send_post(&app, "/api/fulfillments/checkpoint", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_service_proof_incomplete_returns_400() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &[]).unwrap();
    db::create_fulfillment(&state.db, "f1", "p1").unwrap();

    let tok = auth::create_session_token("mgr", &state.hmac_secret, 3600);
    let app = build_app(state);
    let resp = send_get(&app, "/api/fulfillments/f1/proof", Some(&tok)).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_generate_checkpoint_code_returns_qr_svg() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "p1234567");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &[]).unwrap();
    db::create_fulfillment(&state.db, "f1", "p1").unwrap();
    let tok = auth::create_session_token("mgr", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({ "fulfillment_id": "f1", "checkpoint": "arrival" });
    let resp = send_post(&app, "/api/fulfillments/code", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    let svg = val["qr_code_svg"].as_str().unwrap();
    assert!(svg.contains("<svg"), "QR code response should include SVG markup");
}

// ══════════════════════════════════════════════════════════════════════
// Webhook URL validation — additional edge cases
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn route_webhook_https_scheme_rejected() {
    let state = test_state();
    make_user(&state.db, "adm", "a@t.com", "administrator", "adminpass");
    let tok = auth::create_session_token("adm", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "name": "TLS hook",
        "url": "https://192.168.1.1/hook",
        "event_types": ["donation.created"]
    });
    let resp = send_post(&app, "/api/webhooks", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST,
        "https scheme should be rejected (local-only policy)");
}

#[tokio::test]
async fn route_webhook_unparseable_url_rejected() {
    let state = test_state();
    make_user(&state.db, "adm", "a@t.com", "administrator", "adminpass");
    let tok = auth::create_session_token("adm", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "name": "Broken URL",
        "url": "not a url",
        "event_types": ["donation.created"]
    });
    let resp = send_post(&app, "/api/webhooks", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_webhook_localhost_accepted() {
    let state = test_state();
    make_user(&state.db, "adm", "a@t.com", "administrator", "adminpass");
    let tok = auth::create_session_token("adm", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "name": "Local",
        "url": "http://localhost:9000/hook",
        "event_types": ["donation.created"]
    });
    let resp = send_post(&app, "/api/webhooks", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn route_webhook_loopback_ipv4_accepted() {
    let state = test_state();
    make_user(&state.db, "adm", "a@t.com", "administrator", "adminpass");
    let tok = auth::create_session_token("adm", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "name": "Loopback",
        "url": "http://127.0.0.1/hook",
        "event_types": ["donation.created"]
    });
    let resp = send_post(&app, "/api/webhooks", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ══════════════════════════════════════════════════════════════════════
// Admin: role assignment validation
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn route_assign_role_wrong_password_rejected() {
    let state = test_state();
    make_user(&state.db, "adm", "a@t.com", "administrator", "adminpass");
    make_user(&state.db, "u1", "u@t.com", "supporter", "userpass");
    let adm_tok = auth::create_session_token("adm", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "user_id": "u1",
        "role": "project_manager",
        "password": "not-the-admin-password"
    });
    let resp = send_post(&app, "/api/admin/assign-role", Some(&adm_tok), &body).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn route_assign_role_invalid_role_rejected() {
    let state = test_state();
    make_user(&state.db, "adm", "a@t.com", "administrator", "adminpass");
    make_user(&state.db, "u1", "u@t.com", "supporter", "userpass");
    let adm_tok = auth::create_session_token("adm", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "user_id": "u1",
        "role": "superuser",  // not a real role
        "password": "adminpass"
    });
    let resp = send_post(&app, "/api/admin/assign-role", Some(&adm_tok), &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_assign_role_missing_user_returns_404() {
    let state = test_state();
    make_user(&state.db, "adm", "a@t.com", "administrator", "adminpass");
    let adm_tok = auth::create_session_token("adm", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({
        "user_id": "ghost",
        "role": "project_manager",
        "password": "adminpass"
    });
    let resp = send_post(&app, "/api/admin/assign-role", Some(&adm_tok), &body).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn route_unpublish_pm_forbidden() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "pmpass1234");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &[]).unwrap();
    let tok = auth::create_session_token("mgr", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({ "password": "pmpass1234" });
    let resp = send_post(&app, "/api/admin/projects/p1/unpublish", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn route_unpublish_wrong_password_rejected() {
    let state = test_state();
    make_user(&state.db, "adm", "a@t.com", "administrator", "adminpass");
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "pmpass1234");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &[]).unwrap();
    let tok = auth::create_session_token("adm", &state.hmac_secret, 3600);
    let app = build_app(state);

    let body = serde_json::json!({ "password": "wrong" });
    let resp = send_post(&app, "/api/admin/projects/p1/unpublish", Some(&tok), &body).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ══════════════════════════════════════════════════════════════════════
// Register: input validation
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn route_register_short_password_rejected() {
    let state = test_state();
    let app = build_app(state);

    let body = serde_json::json!({
        "email": "a@t.com",
        "password": "short",  // < 8 chars
        "display_name": "A",
        "role": "supporter"
    });
    let resp = send_post(&app, "/api/auth/register", None, &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_register_empty_email_rejected() {
    let state = test_state();
    let app = build_app(state);

    let body = serde_json::json!({
        "email": "",
        "password": "password123",
        "display_name": "A",
        "role": "supporter"
    });
    let resp = send_post(&app, "/api/auth/register", None, &body).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_register_duplicate_email_returns_409() {
    let state = test_state();
    let app = build_app(state);
    let body = serde_json::json!({
        "email": "dupe@t.com",
        "password": "password123",
        "display_name": "A",
        "role": "supporter"
    });
    let r1 = send_post(&app, "/api/auth/register", None, &body).await;
    assert_eq!(r1.status(), StatusCode::OK);

    let r2 = send_post(&app, "/api/auth/register", None, &body).await;
    assert_eq!(r2.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn route_register_preserves_self_selectable_roles() {
    let state = test_state();
    let app = build_app(state.clone());

    // supporter → Supporter (ignored-override scenario already covered elsewhere)
    // project_manager → ProjectManager (self-selectable per registration code)
    let body = serde_json::json!({
        "email": "pm@t.com",
        "password": "password123",
        "display_name": "PM",
        "role": "project_manager"
    });
    let resp = send_post(&app, "/api/auth/register", None, &body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    assert_eq!(val["user"]["role"], "project_manager");
}

// ══════════════════════════════════════════════════════════════════════
// Nonce middleware: GET vs POST
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn nonce_middleware_skipped_for_get_requests() {
    let state = test_state();
    let app = build_app(state);
    // No X-Nonce header — should still return 200 for GET
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn nonce_middleware_required_for_put_delete() {
    let state = test_state();
    make_user(&state.db, "u", "u@t.com", "supporter", "password123");
    let tok = auth::create_session_token("u", &state.hmac_secret, 3600);
    let app = build_app(state);

    // PUT without nonce → 400
    let r1 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/auth/dnd")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", tok))
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "dnd_start": "22:00",
                        "dnd_end": "06:00",
                        "timezone": "UTC"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r1.status(), StatusCode::BAD_REQUEST);

    // DELETE without nonce → 400
    let r2 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/webhooks/foo")
                .header("Authorization", format!("Bearer {}", tok))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r2.status(), StatusCode::BAD_REQUEST);
}

// ══════════════════════════════════════════════════════════════════════
// Previously uncovered endpoints
// ══════════════════════════════════════════════════════════════════════

/// GET /api/favorites/projects — returns full ProjectSummary objects for favorited projects.
#[tokio::test]
async fn route_list_favorite_projects_returns_summaries() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "u1", "u@t.com", "supporter", "pw12345678");
    db::create_project(&state.db, "p1", "Alpha", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    db::create_project(&state.db, "p2", "Beta", "D", "education", "22222", 5000, "mgr1", &[]).unwrap();

    // Favorite p1 only
    db::toggle_favorite(&state.db, "u1", "p1").unwrap();

    let token = auth::create_session_token("u1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_get(&app, "/api/favorites/projects", Some(&token)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    let arr = val.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "p1");
    assert_eq!(arr[0]["title"], "Alpha");
    // Verify it's a full ProjectSummary (has goal_cents etc.)
    assert!(arr[0]["goal_cents"].is_i64());
    assert!(arr[0]["status"].is_string());
}

#[tokio::test]
async fn route_list_favorite_projects_empty_when_none_favorited() {
    let state = test_state();
    make_user(&state.db, "u1", "u@t.com", "supporter", "pw12345678");
    let token = auth::create_session_token("u1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_get(&app, "/api/favorites/projects", Some(&token)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    assert_eq!(val.as_array().unwrap().len(), 0);
}

/// GET /api/projects/:id/tickets — PM/admin can list tickets for their project.
#[tokio::test]
async fn route_list_tickets_for_project_owner() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "sup1", "s@t.com", "supporter", "pw12345678");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    db::create_ticket(&state.db, "t1", "p1", "sup1", "Issue 1", "Detail 1").unwrap();
    db::create_ticket(&state.db, "t2", "p1", "sup1", "Issue 2", "Detail 2").unwrap();

    let mgr_tok = auth::create_session_token("mgr1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_get(&app, "/api/projects/p1/tickets", Some(&mgr_tok)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    let arr = val.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["subject"], "Issue 1");
    assert_eq!(arr[1]["subject"], "Issue 2");
}

#[tokio::test]
async fn route_list_tickets_non_owner_pm_rejected() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m1@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "mgr2", "m2@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "sup1", "s@t.com", "supporter", "pw12345678");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    db::create_ticket(&state.db, "t1", "p1", "sup1", "Q", "body").unwrap();

    let mgr2_tok = auth::create_session_token("mgr2", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_get(&app, "/api/projects/p1/tickets", Some(&mgr2_tok)).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn route_list_tickets_finance_reviewer_allowed() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "fin", "f@t.com", "finance_reviewer", "pw12345678");
    make_user(&state.db, "sup1", "s@t.com", "supporter", "pw12345678");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    db::create_ticket(&state.db, "t1", "p1", "sup1", "Q", "body").unwrap();

    let fin_tok = auth::create_session_token("fin", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_get(&app, "/api/projects/p1/tickets", Some(&fin_tok)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    assert_eq!(val.as_array().unwrap().len(), 1);
}

/// GET /api/projects/:id/fulfillments — list fulfillments scoped to project owner.
#[tokio::test]
async fn route_list_fulfillments_for_project_owner() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    db::create_fulfillment(&state.db, "f1", "p1").unwrap();
    db::create_fulfillment(&state.db, "f2", "p1").unwrap();

    let mgr_tok = auth::create_session_token("mgr1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_get(&app, "/api/projects/p1/fulfillments", Some(&mgr_tok)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    assert_eq!(val.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn route_list_fulfillments_non_owner_rejected() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m1@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "mgr2", "m2@t.com", "project_manager", "pw12345678");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    db::create_fulfillment(&state.db, "f1", "p1").unwrap();

    let mgr2_tok = auth::create_session_token("mgr2", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_get(&app, "/api/projects/p1/fulfillments", Some(&mgr2_tok)).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

/// GET /api/fulfillments/:id — get single fulfillment scoped to project owner.
#[tokio::test]
async fn route_get_fulfillment_by_id_owner_succeeds() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    db::create_fulfillment(&state.db, "f1", "p1").unwrap();

    let mgr_tok = auth::create_session_token("mgr1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_get(&app, "/api/fulfillments/f1", Some(&mgr_tok)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    assert_eq!(val["id"], "f1");
    assert_eq!(val["project_id"], "p1");
    assert_eq!(val["is_complete"], false);
}

#[tokio::test]
async fn route_get_fulfillment_by_id_non_owner_rejected() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m1@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "mgr2", "m2@t.com", "project_manager", "pw12345678");
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr1", &[]).unwrap();
    db::create_fulfillment(&state.db, "f1", "p1").unwrap();

    let mgr2_tok = auth::create_session_token("mgr2", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_get(&app, "/api/fulfillments/f1", Some(&mgr2_tok)).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn route_get_fulfillment_missing_returns_404() {
    let state = test_state();
    make_user(&state.db, "mgr1", "m@t.com", "project_manager", "pw12345678");
    let mgr_tok = auth::create_session_token("mgr1", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_get(&app, "/api/fulfillments/nonexistent", Some(&mgr_tok)).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// GET /api/receipts/pending — finance/admin can list all uploaded (pending) receipts.
#[tokio::test]
async fn route_pending_receipts_finance_only() {
    let state = test_state();
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "pw12345678");
    make_user(&state.db, "fin", "f@t.com", "finance_reviewer", "pw12345678");
    make_user(&state.db, "sup", "s@t.com", "supporter", "pw12345678");
    let bl = vec![("bl1".to_string(), "M".to_string(), 10_000i64)];
    db::create_project(&state.db, "p1", "P", "D", "health", "11111", 5000, "mgr", &bl).unwrap();
    db::create_expense(&state.db, "e1", "p1", "bl1", 5000, "Lumber", None).unwrap();
    db::create_receipt(&state.db, "r1", "e1", "f.pdf", "application/pdf", 1, b"d", "fp99").unwrap();

    let app = build_app(state.clone());

    // Supporter cannot view pending receipts
    let sup_tok = auth::create_session_token("sup", &state.hmac_secret, 3600);
    let r1 = send_get(&app, "/api/receipts/pending", Some(&sup_tok)).await;
    assert_eq!(r1.status(), StatusCode::FORBIDDEN);

    // PM cannot view pending receipts (finance-only)
    let mgr_tok = auth::create_session_token("mgr", &state.hmac_secret, 3600);
    let r2 = send_get(&app, "/api/receipts/pending", Some(&mgr_tok)).await;
    assert_eq!(r2.status(), StatusCode::FORBIDDEN);

    // Finance reviewer CAN view
    let fin_tok = auth::create_session_token("fin", &state.hmac_secret, 3600);
    let r3 = send_get(&app, "/api/receipts/pending", Some(&fin_tok)).await;
    assert_eq!(r3.status(), StatusCode::OK);
    let val = body_json(r3).await;
    let arr = val.as_array().unwrap();
    assert!(arr.iter().any(|r| r["id"] == "r1"), "Should include the uploaded receipt");
}

/// GET /api/webhooks/:id/deliveries — admin can view delivery log for a webhook.
#[tokio::test]
async fn route_webhook_deliveries_admin_only() {
    let state = test_state();
    make_user(&state.db, "adm", "a@t.com", "administrator", "adminpass");
    make_user(&state.db, "mgr", "m@t.com", "project_manager", "pw12345678");

    db::create_webhook(&state.db, "wh1", "TestHook", "http://10.0.0.1/hook", "secret", "[\"donation.created\"]").unwrap();
    // Insert a delivery log entry
    db::log_webhook_delivery(&state.db, "del1", "wh1", "donation.created", "{}", 1, Some(200), true, None).unwrap();

    let app = build_app(state.clone());

    // Non-admin is rejected
    let mgr_tok = auth::create_session_token("mgr", &state.hmac_secret, 3600);
    let r1 = send_get(&app, "/api/webhooks/wh1/deliveries", Some(&mgr_tok)).await;
    assert_eq!(r1.status(), StatusCode::FORBIDDEN);

    // Admin sees deliveries
    let adm_tok = auth::create_session_token("adm", &state.hmac_secret, 3600);
    let r2 = send_get(&app, "/api/webhooks/wh1/deliveries", Some(&adm_tok)).await;
    assert_eq!(r2.status(), StatusCode::OK);
    let val = body_json(r2).await;
    let arr = val.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["webhook_id"], "wh1");
    assert_eq!(arr[0]["success"], true);
    assert_eq!(arr[0]["status_code"], 200);
}

#[tokio::test]
async fn route_webhook_deliveries_empty_for_new_webhook() {
    let state = test_state();
    make_user(&state.db, "adm", "a@t.com", "administrator", "adminpass");
    db::create_webhook(&state.db, "wh2", "EmptyHook", "http://10.0.0.2/hook", "secret2", "[]").unwrap();

    let adm_tok = auth::create_session_token("adm", &state.hmac_secret, 3600);
    let app = build_app(state);

    let resp = send_get(&app, "/api/webhooks/wh2/deliveries", Some(&adm_tok)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let val = body_json(resp).await;
    assert_eq!(val.as_array().unwrap().len(), 0);
}
