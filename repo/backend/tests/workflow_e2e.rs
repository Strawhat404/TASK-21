//! End-to-end workflow tests exercising multi-step user flows through
//! the real Axum router. Each test walks through a realistic scenario
//! (register → promote → act → verify) using nonces, tokens, and all
//! enforcement middleware in the loop.
//!
//! These complement the narrower per-route tests in `integration.rs` and
//! `api_extended.rs` by catching regressions that only manifest when
//! endpoints are chained together.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware as axum_mw,
    routing::{get, post, put},
    Router,
};
use server::{auth, db, middleware, routes, AppState};
use std::sync::Arc;
use tower::util::ServiceExt;

fn test_state() -> Arc<AppState> {
    let key: [u8; 32] = [0xCD; 32];
    Arc::new(AppState {
        db: db::init_db(":memory:"),
        hmac_secret: b"workflow-tests-hmac-secret-key!!".to_vec(),
        encryption_key: key,
        rate_limiter: middleware::RateLimitState::new(10_000, 60),
    })
}

fn build_app(state: Arc<AppState>) -> Router {
    use axum::routing::delete;
    let public = Router::new()
        .route("/api/auth/register", post(routes::auth_routes::register))
        .route("/api/auth/login", post(routes::auth_routes::login))
        .route("/api/auth/nonce", get(routes::auth_routes::get_nonce))
        .route("/api/projects", get(routes::projects::list_projects))
        .route("/api/projects/:id", get(routes::projects::get_project))
        .route("/api/projects/:id/comments", get(routes::comments::list_comments))
        .route("/api/events/track", post(routes::events::track_event));

    let authed = Router::new()
        .route("/api/auth/me", get(routes::auth_routes::me))
        .route("/api/auth/dnd", put(routes::auth_routes::update_dnd))
        .route("/api/projects", post(routes::projects::create_project))
        .route("/api/projects/updates", post(routes::projects::post_update))
        .route("/api/projects/expenses", post(routes::projects::record_expense))
        .route("/api/projects/:id/favorite", post(routes::projects::toggle_favorite))
        .route("/api/projects/:id/subscribe", post(routes::projects::subscribe))
        .route("/api/updates/:id/like", post(routes::projects::toggle_like))
        .route("/api/favorites", get(routes::projects::list_favorites))
        .route("/api/projects/:id/expenses", get(routes::projects::get_expenses))
        .route("/api/projects/:id/tickets", get(routes::comments::list_tickets))
        .route("/api/projects/:id/fulfillments", get(routes::fulfillment::list_fulfillments))
        .route("/api/fulfillments/:id", get(routes::fulfillment::get_fulfillment))
        .route("/api/fulfillments/:id/proof", get(routes::fulfillment::service_proof))
        .route("/api/expenses/:id/receipts", get(routes::receipts::list_receipts))
        .route("/api/donations", post(routes::donations::donate))
        .route("/api/donations/mine", get(routes::donations::my_donations))
        .route("/api/donations/refund", post(routes::donations::request_refund))
        .route("/api/donations/refund/approve", post(routes::donations::approve_refund))
        .route("/api/donations/refund/pending", get(routes::donations::pending_refunds))
        .route("/api/comments", post(routes::comments::create_comment))
        .route("/api/tickets", post(routes::comments::submit_ticket))
        .route("/api/tickets/respond", post(routes::comments::respond_ticket))
        .route("/api/notifications", get(routes::notifications::list_notifications))
        .route("/api/notifications/:id/read", post(routes::notifications::mark_read))
        .route("/api/notifications/read-all", post(routes::notifications::mark_all_read))
        .route("/api/receipts/upload", post(routes::receipts::upload_receipt))
        .route("/api/receipts/review", post(routes::receipts::review_receipt))
        .route("/api/receipts/pending", get(routes::receipts::pending_receipts))
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
        .route("/api/fulfillments", post(routes::fulfillment::create_fulfillment))
        .route("/api/fulfillments/code", post(routes::fulfillment::generate_code))
        .route("/api/fulfillments/checkpoint", post(routes::fulfillment::record_checkpoint))
        .route("/api/events/quality", get(routes::events::data_quality))
        .route("/api/events/suspicious", get(routes::events::suspicious_events))
        .route("/api/admin/stats", get(routes::admin::dashboard_stats))
        .route("/api/admin/ops-log", get(routes::admin::ops_log))
        .route(
            "/api/admin/projects/:id/unpublish",
            post(routes::admin::unpublish_project),
        )
        .route("/api/admin/export/csv", get(routes::admin::export_csv))
        .route("/api/admin/assign-role", post(routes::admin::assign_role))
        .route("/api/admin/bootstrap", post(routes::admin::bootstrap_admin))
        .route("/api/finance/pending", get(routes::finance::pending_expenses))
        .route("/api/finance/review", post(routes::finance::review_expense))
        .route(
            "/api/webhooks",
            post(routes::webhooks::create_webhook).get(routes::webhooks::list_webhooks),
        )
        .route("/api/webhooks/:id", delete(routes::webhooks::delete_webhook))
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

async fn get_nonce(app: &Router) -> String {
    let resp = app
        .clone()
        .oneshot(Request::builder().uri("/api/auth/nonce").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(resp.into_body(), 1_048_576).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v["nonce"].as_str().unwrap().to_string()
}

async fn post_json(
    app: &Router,
    uri: &str,
    token: Option<&str>,
    body: &serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let nonce = get_nonce(app).await;
    let mut req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("Content-Type", "application/json")
        .header("X-Nonce", &nonce);
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }
    let resp = app
        .clone()
        .oneshot(req.body(Body::from(serde_json::to_vec(body).unwrap())).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1_048_576).await.unwrap();
    let val = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, val)
}

async fn get_json(
    app: &Router,
    uri: &str,
    token: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let mut req = Request::builder().uri(uri);
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }
    let resp = app.clone().oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1_048_576).await.unwrap();
    let val = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, val)
}

async fn register_user(
    app: &Router,
    email: &str,
    password: &str,
    name: &str,
    role: &str,
) -> String {
    let (status, val) = post_json(
        app,
        "/api/auth/register",
        None,
        &serde_json::json!({
            "email": email,
            "password": password,
            "display_name": name,
            "role": role,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "register failed for {}: {}", email, val);
    val["token"].as_str().unwrap().to_string()
}

// ══════════════════════════════════════════════════════════════════════
// FLOW 1: Donor lifecycle
//   register → /me → donate → /donations/mine → receipt → refund request
//   → finance approve (two-step) → verify ledger updated
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn flow_donor_lifecycle_with_refund_approval() {
    let state = test_state();

    // Seed: project manager + project to donate to
    let hash = auth::hash_password("managerpass").unwrap();
    db::create_user(&state.db, "mgr", "m@t.com", "Mgr", &hash, "project_manager").unwrap();
    db::create_project(
        &state.db, "p1", "Proj", "Desc", "health", "11111", 100_000, "mgr", &[],
    )
    .unwrap();

    // Seed: finance reviewer (must exist before refund flow)
    db::create_user(&state.db, "fin", "f@t.com", "Fin",
        &auth::hash_password("finpass12").unwrap(), "finance_reviewer").unwrap();

    let app = build_app(state.clone());

    // Step 1: donor registers (as supporter)
    let donor_token = register_user(&app, "donor@t.com", "donorpass12", "Donor", "supporter").await;

    // Step 2: /me returns profile
    let (st, me) = get_json(&app, "/api/auth/me", Some(&donor_token)).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(me["email"], "donor@t.com");
    assert_eq!(me["role"], "supporter");

    // Step 3: donor makes a $50 donation (5000 cents) via check
    let (st, d) = post_json(
        &app,
        "/api/donations",
        Some(&donor_token),
        &serde_json::json!({
            "project_id": "p1",
            "amount_cents": 5000,
            "payment_method": "check"
        }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(d["donation"]["pledge_number"].as_str().unwrap().starts_with("PLG-"));
    assert_eq!(d["donation"]["amount_cents"], 5000);
    let donation_id = d["donation"]["id"].as_str().unwrap().to_string();

    // Step 4: /donations/mine shows this single record
    let (st, mine) = get_json(&app, "/api/donations/mine", Some(&donor_token)).await;
    assert_eq!(st, StatusCode::OK);
    let arr = mine.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], donation_id);

    // Step 5: donation should have auto-subscribed donor to project updates
    let donor_uid = auth::validate_session_token(&donor_token, &state.hmac_secret).unwrap();
    assert!(db::get_project_subscribers(&state.db, "p1").contains(&donor_uid));

    // Step 6: donor requests a refund
    let (st, _ref) = post_json(
        &app,
        "/api/donations/refund",
        Some(&donor_token),
        &serde_json::json!({ "donation_id": donation_id, "reason": "duplicate" }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // Step 7: finance reviewer calls approve_refund without confirmation token → 428
    let fin_tok = auth::create_session_token("fin", &state.hmac_secret, 3600);
    let pending = db::list_pending_reversals(&state.db);
    assert_eq!(pending.len(), 1);
    let reversal_id = pending[0].id.clone();

    let (st1, v1) = post_json(
        &app,
        "/api/donations/refund/approve",
        Some(&fin_tok),
        &serde_json::json!({
            "donation_id": reversal_id,
            "approved": true,
            "password": "finpass12"
        }),
    )
    .await;
    assert_eq!(st1.as_u16(), 428, "first approve call must return 428 Precondition Required");
    let inner: serde_json::Value = serde_json::from_str(v1["error"].as_str().unwrap()).unwrap();
    let confirm_token = inner["confirmation_token"].as_str().unwrap().to_string();

    // Step 8: resubmit with confirmation token → 200
    let (st2, _v2) = post_json(
        &app,
        "/api/donations/refund/approve",
        Some(&fin_tok),
        &serde_json::json!({
            "donation_id": reversal_id,
            "approved": true,
            "password": "finpass12",
            "confirmation_token": confirm_token
        }),
    )
    .await;
    assert_eq!(st2, StatusCode::OK);

    // Step 9: ledger reflects approved refund (net donations = 0)
    let detail = db::get_project_detail(&state.db, "p1").unwrap();
    assert_eq!(detail.raised_cents, 0, "Approved refund should net to zero raised");
}

// ══════════════════════════════════════════════════════════════════════
// FLOW 2: Project manager lifecycle
//   admin bootstraps → admin assigns PM → PM creates project →
//   posts update (notifies subscribers) → records expense → uploads receipt
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn flow_project_manager_end_to_end() {
    let state = test_state();
    let app = build_app(state.clone());

    // Admin registers + bootstraps
    let admin_token = register_user(&app, "admin@t.com", "adminpass", "Admin", "supporter").await;
    let (st, _) = post_json(
        &app,
        "/api/admin/bootstrap",
        Some(&admin_token),
        &serde_json::json!({ "password": "adminpass" }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // PM registers (will be promoted below)
    let pm_token = register_user(&app, "pm@t.com", "pmpass1234", "PM", "supporter").await;
    let pm_uid = auth::validate_session_token(&pm_token, &state.hmac_secret).unwrap();

    // Admin assigns project_manager role to PM
    let (st, _) = post_json(
        &app,
        "/api/admin/assign-role",
        Some(&admin_token),
        &serde_json::json!({
            "user_id": pm_uid,
            "role": "project_manager",
            "password": "adminpass"
        }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // PM creates a project with two budget lines
    let (st, proj) = post_json(
        &app,
        "/api/projects",
        Some(&pm_token),
        &serde_json::json!({
            "title": "New Playground",
            "description": "Accessible play equipment",
            "cause": "environment",
            "zip_code": "60614",
            "goal_cents": 500_000,
            "budget_lines": [
                { "name": "Equipment", "allocated_cents": 300_000 },
                { "name": "Labor",     "allocated_cents": 200_000 }
            ]
        }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let project_id = proj["id"].as_str().unwrap().to_string();
    let bl0_id = proj["budget_lines"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(proj["budget_lines"][0]["name"], "Equipment");

    // Supporter subscribes to the project
    let sup_token = register_user(&app, "sup@t.com", "suppass12", "Sup", "supporter").await;
    let (st, _) = post_json(
        &app,
        &format!("/api/projects/{}/subscribe", project_id),
        Some(&sup_token),
        &serde_json::json!({}),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // PM posts a spending update → subscriber gets a notification
    let (st, _) = post_json(
        &app,
        "/api/projects/updates",
        Some(&pm_token),
        &serde_json::json!({
            "project_id": project_id,
            "title": "Equipment ordered",
            "body": "We placed the order with local supplier today.",
        }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    let (st, notifs) = get_json(&app, "/api/notifications", Some(&sup_token)).await;
    assert_eq!(st, StatusCode::OK);
    let arr = notifs.as_array().unwrap();
    assert!(arr.len() >= 1, "subscriber should have received a notification");
    assert!(arr[0]["title"]
        .as_str()
        .unwrap()
        .contains("Equipment ordered"));

    // PM records an expense
    let (st, _) = post_json(
        &app,
        "/api/projects/expenses",
        Some(&pm_token),
        &serde_json::json!({
            "project_id": project_id,
            "budget_line_id": bl0_id,
            "amount_cents": 15000,
            "description": "Slide set",
            "receipt_data": "purchase order 12345"
        }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // PM uploads a receipt PDF
    // base64("minimal-pdf-content") = bWluaW1hbC1wZGYtY29udGVudA==
    use base64::Engine as _;
    let file_data = b"minimal-pdf-content";
    let b64 = base64::engine::general_purpose::STANDARD.encode(file_data);
    // Find the expense id
    let expenses = db::list_expenses(&state.db, &project_id, &state.encryption_key);
    assert_eq!(expenses.len(), 1);
    let expense_id = expenses[0].id.clone();

    let (st, receipt) = post_json(
        &app,
        "/api/receipts/upload",
        Some(&pm_token),
        &serde_json::json!({
            "expense_id": expense_id,
            "file_name": "invoice.pdf",
            "file_type": "application/pdf",
            "file_size": file_data.len() as i64,
            "file_data_base64": b64
        }),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "receipt upload failed: {}", receipt);
    assert_eq!(receipt["status"], "uploaded");
    assert!(receipt["sha256_fingerprint"].as_str().unwrap().len() == 64);

    // PM can list their expenses via the scoped route
    let (st, exp_list) = get_json(
        &app,
        &format!("/api/projects/{}/expenses", project_id),
        Some(&pm_token),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(exp_list.as_array().unwrap().len(), 1);
}

// ══════════════════════════════════════════════════════════════════════
// FLOW 3: Finance reviews an expense; CSV export reflects approved state
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn flow_finance_reviews_expense_and_exports_csv() {
    let state = test_state();
    let hash = auth::hash_password("p1234567").unwrap();
    db::create_user(&state.db, "fin", "f@t.com", "F",
        &auth::hash_password("finpass123").unwrap(), "finance_reviewer").unwrap();
    db::create_user(&state.db, "mgr", "m@t.com", "M", &hash, "project_manager").unwrap();
    db::create_user(&state.db, "adm", "a@t.com", "A",
        &auth::hash_password("adminpass").unwrap(), "administrator").unwrap();
    db::create_user(&state.db, "d", "d@t.com", "D", &hash, "supporter").unwrap();

    let bl = vec![("bl1".to_string(), "Lumber".to_string(), 50_000i64)];
    db::create_project(&state.db, "p1", "P", "D", "education", "11111", 100_000, "mgr", &bl).unwrap();
    db::create_expense(&state.db, "e1", "p1", "bl1", 5000, "Boards", None).unwrap();
    db::create_donation(&state.db, "d1", "PLG-ABC", "p1", "d", 10_000, "cash", false, None, None).unwrap();

    let fin_tok = auth::create_session_token("fin", &state.hmac_secret, 3600);
    let adm_tok = auth::create_session_token("adm", &state.hmac_secret, 3600);

    let app = build_app(state.clone());

    // Finance views pending expenses
    let (st, pend) = get_json(&app, "/api/finance/pending", Some(&fin_tok)).await;
    assert_eq!(st, StatusCode::OK);
    assert!(pend.as_array().unwrap().iter().any(|e| e["id"] == "e1"));

    // Finance approves the expense
    let (st, _) = post_json(
        &app,
        "/api/finance/review",
        Some(&fin_tok),
        &serde_json::json!({ "expense_id": "e1", "approved": true, "note": "OK" }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    let expenses = db::list_expenses(&state.db, "p1", &state.encryption_key);
    assert_eq!(expenses[0].disclosure_status, common::DisclosureStatus::Approved);

    // Admin exports CSV and confirms donation line appears
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/admin/export/csv")
                .header("Authorization", format!("Bearer {}", adm_tok))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers()["content-type"], "text/csv");
    let bytes = axum::body::to_bytes(resp.into_body(), 1_048_576).await.unwrap();
    let csv = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(csv.contains("PLG-ABC"));
}

// ══════════════════════════════════════════════════════════════════════
// FLOW 4: Fulfillment OTP lifecycle → tamper-evident service proof
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn flow_fulfillment_arrival_start_end_produces_proof() {
    let state = test_state();
    let hash = auth::hash_password("p1234567").unwrap();
    db::create_user(&state.db, "mgr", "m@t.com", "M", &hash, "project_manager").unwrap();
    db::create_project(&state.db, "p1", "Proj", "D", "health", "11111", 50_000, "mgr", &[]).unwrap();

    let mgr_tok = auth::create_session_token("mgr", &state.hmac_secret, 3600);
    let app = build_app(state.clone());

    // Create the fulfillment
    let (st, fulf) = post_json(
        &app,
        "/api/fulfillments",
        Some(&mgr_tok),
        &serde_json::json!({ "project_id": "p1" }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let fid = fulf["id"].as_str().unwrap().to_string();

    // Helper: generate code, then record checkpoint with it
    async fn do_checkpoint(
        app: &Router,
        tok: &str,
        fid: &str,
        checkpoint: &str,
    ) {
        let (st, gen) = post_json(
            app,
            "/api/fulfillments/code",
            Some(tok),
            &serde_json::json!({ "fulfillment_id": fid, "checkpoint": checkpoint }),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "code gen failed for {}: {}", checkpoint, gen);
        let code = gen["code"].as_str().unwrap().to_string();

        let (st, _) = post_json(
            app,
            "/api/fulfillments/checkpoint",
            Some(tok),
            &serde_json::json!({
                "fulfillment_id": fid,
                "checkpoint": checkpoint,
                "code": code
            }),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "record checkpoint failed for {}", checkpoint);
    }

    do_checkpoint(&app, &mgr_tok, &fid, "arrival").await;
    // Ensure `end` is strictly after `start` (DB stores with 1-second resolution).
    do_checkpoint(&app, &mgr_tok, &fid, "start").await;
    std::thread::sleep(std::time::Duration::from_millis(1100));
    do_checkpoint(&app, &mgr_tok, &fid, "end").await;

    // Proof endpoint should now work and include a non-empty hash
    let (st, proof) = get_json(
        &app,
        &format!("/api/fulfillments/{}/proof", fid),
        Some(&mgr_tok),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(proof["fulfillment_id"], fid);
    let hash = proof["service_record_hash"].as_str().unwrap();
    assert_eq!(hash.len(), 64, "SHA-256 hex hash should be 64 chars");
}

// ══════════════════════════════════════════════════════════════════════
// FLOW 5: Moderation pre-moderation round trip
//   admin enables pre-mod → supporter posts comment (pending) →
//   PM approves → comment appears publicly
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn flow_pre_moderation_approval_reveals_comment() {
    let state = test_state();
    let hash = auth::hash_password("pw1234567").unwrap();
    db::create_user(&state.db, "adm", "a@t.com", "A",
        &auth::hash_password("adminpass").unwrap(), "administrator").unwrap();
    db::create_user(&state.db, "mgr", "m@t.com", "M", &hash, "project_manager").unwrap();
    db::create_user(&state.db, "sup", "s@t.com", "S", &hash, "supporter").unwrap();
    db::create_project(&state.db, "p1", "P", "D", "education", "11111", 5000, "mgr", &[]).unwrap();

    let adm_tok = auth::create_session_token("adm", &state.hmac_secret, 3600);
    let mgr_tok = auth::create_session_token("mgr", &state.hmac_secret, 3600);
    let sup_tok = auth::create_session_token("sup", &state.hmac_secret, 3600);
    let app = build_app(state.clone());

    // Admin enables pre-moderation
    let nonce = get_nonce(&app).await;
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/moderation/config")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", adm_tok))
                .header("X-Nonce", &nonce)
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "comments_enabled": true,
                        "require_pre_moderation": true,
                        "sensitive_words": []
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Supporter posts a comment → goes to pending_review
    let (st, _) = post_json(
        &app,
        "/api/comments",
        Some(&sup_tok),
        &serde_json::json!({ "project_id": "p1", "body": "Awesome work!" }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // Public list is empty (pending comments hidden)
    let (_st, pub_list) = get_json(&app, "/api/projects/p1/comments", None).await;
    assert_eq!(pub_list.as_array().unwrap().len(), 0);

    // PM sees the pending comment in their moderation queue
    let (st, pending) = get_json(&app, "/api/moderation/comments/pending", Some(&mgr_tok)).await;
    assert_eq!(st, StatusCode::OK);
    let parr = pending.as_array().unwrap();
    assert_eq!(parr.len(), 1);
    let cid = parr[0]["id"].as_str().unwrap().to_string();

    // PM approves the comment
    let (st, _) = post_json(
        &app,
        "/api/moderation/comments/review",
        Some(&mgr_tok),
        &serde_json::json!({ "comment_id": cid, "approved": true }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // Now it should appear on the public list
    let (_st, pub_list) = get_json(&app, "/api/projects/p1/comments", None).await;
    assert_eq!(pub_list.as_array().unwrap().len(), 1);
    assert_eq!(pub_list[0]["body"], "Awesome work!");
}

// ══════════════════════════════════════════════════════════════════════
// FLOW 6: Supporter browses projects with filters, favorites one,
//         views their favorites list. Covers the supporter UI read path.
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn flow_supporter_browse_filter_and_favorite() {
    let state = test_state();
    let hash = auth::hash_password("p1234567").unwrap();
    db::create_user(&state.db, "mgr", "m@t.com", "M", &hash, "project_manager").unwrap();
    db::create_project(&state.db, "ph", "Health A", "D", "health", "10001", 5000, "mgr", &[]).unwrap();
    db::create_project(&state.db, "pe", "Edu A", "D", "education", "10001", 5000, "mgr", &[]).unwrap();
    db::create_project(&state.db, "pz", "Edu Other", "D", "education", "20002", 5000, "mgr", &[]).unwrap();

    let app = build_app(state.clone());

    // Register a supporter
    let tok = register_user(&app, "browser@t.com", "browsepass1", "Browser", "supporter").await;

    // Filter: cause=education AND zip=10001 → only "Edu A"
    let (st, list) = get_json(
        &app,
        "/api/projects?cause=education&zip_code=10001",
        Some(&tok),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let items = list["items"].as_array().unwrap();
    let titles: Vec<&str> = items.iter().filter_map(|i| i["title"].as_str()).collect();
    assert!(titles.contains(&"Edu A"));
    assert!(!titles.contains(&"Edu Other"));
    assert!(!titles.contains(&"Health A"));

    // Search: should match title
    let (_st, list) = get_json(&app, "/api/projects?search=Health", Some(&tok)).await;
    let found: Vec<&str> = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|i| i["title"].as_str())
        .collect();
    assert!(found.iter().any(|t| t.contains("Health")));

    // Favorite the Edu A project
    let (st, _) = post_json(
        &app,
        "/api/projects/pe/favorite",
        Some(&tok),
        &serde_json::json!({}),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // List favorites
    let (_st, favs) = get_json(&app, "/api/favorites", Some(&tok)).await;
    let ids: Vec<&str> = favs.as_array().unwrap().iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(ids, vec!["pe"]);
}

// ══════════════════════════════════════════════════════════════════════
// FLOW 7: Admin unpublish two-step protocol and post-state
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn flow_admin_unpublish_two_step_changes_visibility() {
    let state = test_state();
    db::create_user(&state.db, "adm", "a@t.com", "A",
        &auth::hash_password("adminpass").unwrap(), "administrator").unwrap();
    db::create_user(&state.db, "mgr", "m@t.com", "M",
        &auth::hash_password("pw1234567").unwrap(), "project_manager").unwrap();
    db::create_project(&state.db, "p1", "Visible", "D", "health", "11111", 5000, "mgr", &[]).unwrap();

    let adm_tok = auth::create_session_token("adm", &state.hmac_secret, 3600);
    let app = build_app(state.clone());

    // Step 1: without confirmation token → 428 with a token embedded
    let (st1, v1) = post_json(
        &app,
        "/api/admin/projects/p1/unpublish",
        Some(&adm_tok),
        &serde_json::json!({ "password": "adminpass" }),
    )
    .await;
    assert_eq!(st1.as_u16(), 428);
    let inner: serde_json::Value = serde_json::from_str(v1["error"].as_str().unwrap()).unwrap();
    let confirm = inner["confirmation_token"].as_str().unwrap().to_string();

    // Step 2: with confirmation token → success
    let (st2, _v2) = post_json(
        &app,
        "/api/admin/projects/p1/unpublish",
        Some(&adm_tok),
        &serde_json::json!({
            "password": "adminpass",
            "confirmation_token": confirm
        }),
    )
    .await;
    assert_eq!(st2, StatusCode::OK);

    // Verify the project's status changed
    let d = db::get_project_detail(&state.db, "p1").unwrap();
    assert_eq!(d.status, common::ProjectStatus::Unpublished);
}

// ══════════════════════════════════════════════════════════════════════
// FLOW 8: Webhook create → list → delete lifecycle (admin only)
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn flow_webhook_crud_lifecycle() {
    let state = test_state();
    db::create_user(&state.db, "adm", "a@t.com", "A",
        &auth::hash_password("adminpass").unwrap(), "administrator").unwrap();
    let tok = auth::create_session_token("adm", &state.hmac_secret, 3600);
    let app = build_app(state.clone());

    // Create with local URL
    let (st, created) = post_json(
        &app,
        "/api/webhooks",
        Some(&tok),
        &serde_json::json!({
            "name": "Door Controller",
            "url": "http://10.0.0.5/incoming",
            "event_types": ["donation.created", "event.click"]
        }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let wid = created["id"].as_str().unwrap().to_string();

    // List
    let (st, list) = get_json(&app, "/api/webhooks", Some(&tok)).await;
    assert_eq!(st, StatusCode::OK);
    assert!(list.as_array().unwrap().iter().any(|h| h["id"] == wid));

    // Delete
    let nonce = get_nonce(&app).await;
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/webhooks/{}", wid))
                .header("Authorization", format!("Bearer {}", tok))
                .header("X-Nonce", &nonce)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // List shows 0 now
    let (_st, list2) = get_json(&app, "/api/webhooks", Some(&tok)).await;
    assert_eq!(list2.as_array().unwrap().len(), 0);
}

// ══════════════════════════════════════════════════════════════════════
// FLOW 9: Rate limiter: session key is burstable up to the cap, then 429
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn flow_rate_limiter_returns_429_after_exhaustion() {
    // Build an AppState with a very small limit
    let state = Arc::new(AppState {
        db: db::init_db(":memory:"),
        hmac_secret: b"rate-test-key".to_vec(),
        encryption_key: [0xAA; 32],
        rate_limiter: middleware::RateLimitState::new(3, 60),
    });
    let app = build_app(state);

    // 3 GETs allowed
    for _ in 0..3 {
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
    // 4th → 429
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
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
}

// ══════════════════════════════════════════════════════════════════════
// FLOW 10: Session token login flow: register → logout → login → /me
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn flow_login_after_register_returns_valid_token() {
    let state = test_state();
    let app = build_app(state.clone());

    // Register
    let reg_token = register_user(&app, "loop@t.com", "looppass12", "LoopUser", "supporter").await;
    // /me works with reg token
    let (st1, _v1) = get_json(&app, "/api/auth/me", Some(&reg_token)).await;
    assert_eq!(st1, StatusCode::OK);

    // Now "logout" → client discards token → login again
    let (st, val) = post_json(
        &app,
        "/api/auth/login",
        None,
        &serde_json::json!({ "email": "loop@t.com", "password": "looppass12" }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let new_token = val["token"].as_str().unwrap().to_string();
    assert_ne!(new_token, reg_token, "second login should issue a new token");

    // New token resolves /me correctly
    let (st2, me) = get_json(&app, "/api/auth/me", Some(&new_token)).await;
    assert_eq!(st2, StatusCode::OK);
    assert_eq!(me["email"], "loop@t.com");
}
