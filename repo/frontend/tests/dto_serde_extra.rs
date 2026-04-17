//! Extended frontend WASM tests for shared DTOs.
//!
//! These verify serde wire-format compatibility between the Axum backend
//! (which produces snake_case JSON) and the Leptos frontend (which consumes
//! it). Broken here → broken in production.
//!
//! Complements `common_types.rs` with the remaining DTOs not yet covered:
//! ProjectDetail, BudgetLine, SpendingUpdate, ExpenseRecord, ReceiptRecord,
//! Ticket, Notification, DashboardStats, OpsLogEntry, DataQualityMetrics,
//! WebhookConfig, WebhookDeliveryLog, ServiceProof, AnalyticsEvent,
//! FulfillmentRecord, ReviewExpenseRequest, ReviewReceiptRequest,
//! UploadReceiptRequest, ApproveRefundRequest, AssignRoleRequest,
//! ApiError, ApiSuccess, ConfirmationRequired, PaymentMethod,
//! DisclosureStatus, ReceiptStatus, ModerateCommentRequest,
//! DonateRequest, RefundRequest, SubmitTicketRequest, RespondTicketRequest,
//! PostUpdateRequest, RecordExpenseRequest, CommentWithModeration.

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

use common::*;

// ══════════════════════════════════════════════════════════════════════
// ProjectDetail + BudgetLine + SpendingUpdate
// ══════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn project_detail_full_deserialization() {
    let json = r#"{
        "id": "p-1",
        "title": "Clean Water",
        "description": "Build wells",
        "cause": "water",
        "zip_code": "30301",
        "status": "active",
        "goal_cents": 1000000,
        "raised_cents": 250000,
        "spent_cents": 50000,
        "manager_id": "u-mgr",
        "manager_name": "Mgr",
        "budget_lines": [
            { "id": "bl-1", "project_id": "p-1", "name": "Pipes", "allocated_cents": 500000, "spent_cents": 25000 }
        ],
        "updates": [
            { "id": "up-1", "project_id": "p-1", "title": "Kickoff", "body": "We began today",
              "author_name": "Alice", "like_count": 3, "created_at": "2025-03-01T00:00:00Z" }
        ],
        "created_at": "2025-01-01T00:00:00Z"
    }"#;
    let detail: ProjectDetail = serde_json::from_str(json).unwrap();
    assert_eq!(detail.id, "p-1");
    assert_eq!(detail.budget_lines.len(), 1);
    assert_eq!(detail.budget_lines[0].name, "Pipes");
    assert_eq!(detail.budget_lines[0].spent_cents, 25_000);
    assert_eq!(detail.updates.len(), 1);
    assert_eq!(detail.updates[0].like_count, 3);
    assert_eq!(detail.status, ProjectStatus::Active);
}

#[wasm_bindgen_test]
fn budget_line_round_trip() {
    let bl = BudgetLine {
        id: "bl-1".into(),
        project_id: "p-1".into(),
        name: "Materials".into(),
        allocated_cents: 80_000,
        spent_cents: 5_000,
    };
    let j = serde_json::to_string(&bl).unwrap();
    let r: BudgetLine = serde_json::from_str(&j).unwrap();
    assert_eq!(r.name, "Materials");
    assert_eq!(r.allocated_cents, 80_000);
    assert_eq!(r.spent_cents, 5_000);
}

#[wasm_bindgen_test]
fn spending_update_round_trip() {
    let u = SpendingUpdate {
        id: "up-1".into(),
        project_id: "p-1".into(),
        title: "Milestone".into(),
        body: "Week 2 recap".into(),
        author_name: "Bob".into(),
        like_count: 7,
        created_at: "2025-04-01T00:00:00Z".into(),
    };
    let j = serde_json::to_string(&u).unwrap();
    let r: SpendingUpdate = serde_json::from_str(&j).unwrap();
    assert_eq!(r.like_count, 7);
    assert_eq!(r.author_name, "Bob");
}

// ══════════════════════════════════════════════════════════════════════
// ExpenseRecord with enum DisclosureStatus
// ══════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn expense_record_pending_serialization() {
    let e = ExpenseRecord {
        id: "e-1".into(),
        project_id: "p-1".into(),
        budget_line_id: "bl-1".into(),
        budget_line_name: "Materials".into(),
        amount_cents: 5_000,
        description: "Lumber".into(),
        receipt_url: None,
        disclosure_status: DisclosureStatus::Pending,
        reviewer_note: None,
        created_at: "2025-03-01T00:00:00Z".into(),
    };
    let v: serde_json::Value = serde_json::to_value(&e).unwrap();
    assert_eq!(v["disclosure_status"], "pending");
    assert!(v["receipt_url"].is_null());
    assert!(v["reviewer_note"].is_null());

    let r: ExpenseRecord = serde_json::from_value(v).unwrap();
    assert_eq!(r.disclosure_status, DisclosureStatus::Pending);
}

#[wasm_bindgen_test]
fn expense_record_approved_with_note() {
    let json = r#"{
        "id": "e-2",
        "project_id": "p-1",
        "budget_line_id": "bl-1",
        "budget_line_name": "Labor",
        "amount_cents": 15000,
        "description": "Contractor",
        "receipt_url": "/receipts/r-1",
        "disclosure_status": "approved",
        "reviewer_note": "Looks good",
        "created_at": "2025-03-02T00:00:00Z"
    }"#;
    let r: ExpenseRecord = serde_json::from_str(json).unwrap();
    assert_eq!(r.disclosure_status, DisclosureStatus::Approved);
    assert_eq!(r.reviewer_note.as_deref(), Some("Looks good"));
    assert_eq!(r.receipt_url.as_deref(), Some("/receipts/r-1"));
}

#[wasm_bindgen_test]
fn disclosure_status_variants_round_trip() {
    for s in &[
        DisclosureStatus::Pending,
        DisclosureStatus::Approved,
        DisclosureStatus::Rejected,
    ] {
        let j = serde_json::to_string(s).unwrap();
        let r: DisclosureStatus = serde_json::from_str(&j).unwrap();
        assert_eq!(*s, r);
        assert_eq!(DisclosureStatus::from_str(s.as_str()), Some(*s));
    }
    assert!(DisclosureStatus::from_str("mystery").is_none());
}

// ══════════════════════════════════════════════════════════════════════
// ReceiptRecord with enum ReceiptStatus
// ══════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn receipt_record_uploaded_state() {
    let r = ReceiptRecord {
        id: "r-1".into(),
        expense_id: "e-1".into(),
        file_name: "invoice.pdf".into(),
        file_type: "application/pdf".into(),
        file_size: 10_000,
        sha256_fingerprint: "a".repeat(64),
        status: ReceiptStatus::Uploaded,
        rejection_reason: None,
        reviewer_id: None,
        created_at: "2025-03-01T00:00:00Z".into(),
    };
    let j = serde_json::to_string(&r).unwrap();
    let v: serde_json::Value = serde_json::from_str(&j).unwrap();
    assert_eq!(v["status"], "uploaded");
    assert_eq!(v["file_size"], 10_000);
    assert!(v["rejection_reason"].is_null());
}

#[wasm_bindgen_test]
fn receipt_record_rejected_with_reason() {
    let json = r#"{
        "id": "r-2",
        "expense_id": "e-1",
        "file_name": "blurry.jpg",
        "file_type": "image/jpeg",
        "file_size": 5000,
        "sha256_fingerprint": "deadbeef",
        "status": "rejected",
        "rejection_reason": "Image is unreadable",
        "reviewer_id": "u-fin",
        "created_at": "2025-03-01T00:00:00Z"
    }"#;
    let r: ReceiptRecord = serde_json::from_str(json).unwrap();
    assert_eq!(r.status, ReceiptStatus::Rejected);
    assert_eq!(r.rejection_reason.as_deref(), Some("Image is unreadable"));
    assert_eq!(r.reviewer_id.as_deref(), Some("u-fin"));
}

#[wasm_bindgen_test]
fn receipt_status_variants_round_trip() {
    for s in &[
        ReceiptStatus::Uploaded,
        ReceiptStatus::Verified,
        ReceiptStatus::Rejected,
    ] {
        let j = serde_json::to_string(s).unwrap();
        let r: ReceiptStatus = serde_json::from_str(&j).unwrap();
        assert_eq!(*s, r);
        assert_eq!(ReceiptStatus::from_str(s.as_str()), Some(*s));
    }
    assert!(ReceiptStatus::from_str("nope").is_none());
}

// ══════════════════════════════════════════════════════════════════════
// Ticket with optional response
// ══════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn ticket_open_state() {
    let t = Ticket {
        id: "t-1".into(),
        project_id: "p-1".into(),
        submitter_id: "u-1".into(),
        submitter_name: "Alice".into(),
        subject: "Question".into(),
        body: "When start?".into(),
        status: "open".into(),
        response: None,
        created_at: "2025-03-01T00:00:00Z".into(),
    };
    let v = serde_json::to_value(&t).unwrap();
    assert_eq!(v["status"], "open");
    assert!(v["response"].is_null());
    let r: Ticket = serde_json::from_value(v).unwrap();
    assert!(r.response.is_none());
}

#[wasm_bindgen_test]
fn ticket_with_response_round_trip() {
    let json = r#"{
        "id": "t-2",
        "project_id": "p-1",
        "submitter_id": "u-2",
        "submitter_name": "Bob",
        "subject": "Update",
        "body": "Latest news?",
        "status": "answered",
        "response": "Next week",
        "created_at": "2025-03-02T00:00:00Z"
    }"#;
    let t: Ticket = serde_json::from_str(json).unwrap();
    assert_eq!(t.status, "answered");
    assert_eq!(t.response.as_deref(), Some("Next week"));
}

// ══════════════════════════════════════════════════════════════════════
// Notification flags
// ══════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn notification_flags_round_trip() {
    let n = Notification {
        id: "n-1".into(),
        user_id: "u-1".into(),
        title: "Hi".into(),
        body: "new update".into(),
        is_read: false,
        is_deferred: true,
        created_at: "2025-01-01T00:00:00Z".into(),
    };
    let j = serde_json::to_string(&n).unwrap();
    let v: serde_json::Value = serde_json::from_str(&j).unwrap();
    assert_eq!(v["is_read"], false);
    assert_eq!(v["is_deferred"], true);
    let r: Notification = serde_json::from_value(v).unwrap();
    assert!(!r.is_read);
    assert!(r.is_deferred);
}

// ══════════════════════════════════════════════════════════════════════
// DashboardStats + OpsLogEntry + DataQualityMetrics
// ══════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn dashboard_stats_round_trip() {
    let s = DashboardStats {
        gmv_cents: 123_456,
        total_donations: 20,
        unique_donors: 12,
        average_donation_cents: 6_000,
        repeat_donor_rate: 0.35,
        conversion_rate: 0.12,
    };
    let j = serde_json::to_string(&s).unwrap();
    let r: DashboardStats = serde_json::from_str(&j).unwrap();
    assert_eq!(r.gmv_cents, 123_456);
    assert_eq!(r.total_donations, 20);
    assert!((r.repeat_donor_rate - 0.35).abs() < 1e-9);
    assert!((r.conversion_rate - 0.12).abs() < 1e-9);
}

#[wasm_bindgen_test]
fn ops_log_entry_round_trip() {
    let e = OpsLogEntry {
        id: "log-1".into(),
        actor_id: "u-1".into(),
        actor_name: "Alice".into(),
        action: "delete_comment".into(),
        detail: "comment c1 removed".into(),
        created_at: "2025-04-01T00:00:00Z".into(),
    };
    let j = serde_json::to_string(&e).unwrap();
    let r: OpsLogEntry = serde_json::from_str(&j).unwrap();
    assert_eq!(r.action, "delete_comment");
    assert_eq!(r.actor_name, "Alice");
}

#[wasm_bindgen_test]
fn data_quality_metrics_with_events_by_kind() {
    let json = r#"{
        "total_events": 1000,
        "duplicate_events": 12,
        "suspicious_events": 3,
        "duplicate_rate": 0.012,
        "suspicious_rate": 0.003,
        "events_by_kind": [["click", 600], ["impression", 350], ["dwell_time", 50]]
    }"#;
    let m: DataQualityMetrics = serde_json::from_str(json).unwrap();
    assert_eq!(m.total_events, 1000);
    assert_eq!(m.events_by_kind.len(), 3);
    assert_eq!(m.events_by_kind[0].0, "click");
    assert_eq!(m.events_by_kind[0].1, 600);
    assert!((m.suspicious_rate - 0.003).abs() < 1e-9);
}

// ══════════════════════════════════════════════════════════════════════
// Webhook DTOs
// ══════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn webhook_config_round_trip() {
    let w = WebhookConfig {
        id: "w-1".into(),
        name: "Door Controller".into(),
        url: "http://10.0.0.5/hook".into(),
        secret: "abcd".into(),
        event_types: vec!["donation.created".into(), "event.click".into()],
        enabled: true,
        created_at: "2025-05-01T00:00:00Z".into(),
    };
    let j = serde_json::to_string(&w).unwrap();
    let r: WebhookConfig = serde_json::from_str(&j).unwrap();
    assert_eq!(r.event_types.len(), 2);
    assert!(r.event_types.contains(&"donation.created".to_string()));
    assert!(r.enabled);
}

#[wasm_bindgen_test]
fn webhook_delivery_log_success_and_failure() {
    let success = WebhookDeliveryLog {
        id: "log-1".into(),
        webhook_id: "w-1".into(),
        event_type: "donation.created".into(),
        payload_summary: "{\"amount\":100}".into(),
        attempt: 1,
        status_code: Some(200),
        success: true,
        error_message: None,
        created_at: "2025-05-01T00:00:00Z".into(),
    };
    let j = serde_json::to_string(&success).unwrap();
    let r: WebhookDeliveryLog = serde_json::from_str(&j).unwrap();
    assert_eq!(r.status_code, Some(200));
    assert!(r.success);

    let failed_json = r#"{
        "id": "log-2",
        "webhook_id": "w-1",
        "event_type": "donation.created",
        "payload_summary": "{}",
        "attempt": 3,
        "status_code": null,
        "success": false,
        "error_message": "connection refused",
        "created_at": "2025-05-01T00:00:00Z"
    }"#;
    let r: WebhookDeliveryLog = serde_json::from_str(failed_json).unwrap();
    assert_eq!(r.attempt, 3);
    assert!(r.status_code.is_none());
    assert_eq!(r.error_message.as_deref(), Some("connection refused"));
}

#[wasm_bindgen_test]
fn create_webhook_request_empty_and_populated() {
    let empty = CreateWebhookRequest {
        name: "t".into(),
        url: "http://localhost/x".into(),
        event_types: vec![],
    };
    let j = serde_json::to_string(&empty).unwrap();
    let v: serde_json::Value = serde_json::from_str(&j).unwrap();
    assert_eq!(v["event_types"].as_array().unwrap().len(), 0);

    let populated_json = r#"{
        "name": "n",
        "url": "http://10.0.0.5/x",
        "event_types": ["donation.created", "event.click"]
    }"#;
    let p: CreateWebhookRequest = serde_json::from_str(populated_json).unwrap();
    assert_eq!(p.event_types.len(), 2);
}

// ══════════════════════════════════════════════════════════════════════
// ServiceProof, AnalyticsEvent, FulfillmentRecord
// ══════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn service_proof_round_trip() {
    let p = ServiceProof {
        fulfillment_id: "f-1".into(),
        project_id: "p-1".into(),
        project_title: "Proj".into(),
        arrival_at: "2025-04-01 09:00:00".into(),
        start_at: "2025-04-01 09:10:00".into(),
        end_at: "2025-04-01 12:00:00".into(),
        service_record_hash: "ab".repeat(32),
        generated_at: "2025-04-01 12:05:00".into(),
    };
    let j = serde_json::to_string(&p).unwrap();
    let r: ServiceProof = serde_json::from_str(&j).unwrap();
    assert_eq!(r.service_record_hash.len(), 64);
}

#[wasm_bindgen_test]
fn analytics_event_round_trip() {
    let e = AnalyticsEvent {
        id: "e-1".into(),
        event_kind: "click".into(),
        target_type: "button".into(),
        target_id: "donate".into(),
        session_id: "sess-1".into(),
        user_id: Some("u-1".into()),
        dwell_ms: Some(1_500),
        is_duplicate: false,
        is_suspicious: false,
        created_at: "2025-04-01T00:00:00Z".into(),
    };
    let j = serde_json::to_string(&e).unwrap();
    let v: serde_json::Value = serde_json::from_str(&j).unwrap();
    assert_eq!(v["event_kind"], "click");
    assert_eq!(v["dwell_ms"], 1_500);
    assert_eq!(v["is_duplicate"], false);
    let r: AnalyticsEvent = serde_json::from_value(v).unwrap();
    assert_eq!(r.user_id.as_deref(), Some("u-1"));
}

#[wasm_bindgen_test]
fn fulfillment_record_incomplete() {
    let f = FulfillmentRecord {
        id: "f-1".into(),
        project_id: "p-1".into(),
        arrival_at: None,
        start_at: None,
        end_at: None,
        arrival_code: None,
        start_code: None,
        end_code: None,
        is_complete: false,
        service_record_hash: None,
        created_at: "2025-04-01T00:00:00Z".into(),
    };
    let v = serde_json::to_value(&f).unwrap();
    assert!(v["arrival_at"].is_null());
    assert_eq!(v["is_complete"], false);
    assert!(v["service_record_hash"].is_null());
}

#[wasm_bindgen_test]
fn fulfillment_record_complete_round_trip() {
    let hash_hex = "f".repeat(64);
    let original = FulfillmentRecord {
        id: "f-2".into(),
        project_id: "p-1".into(),
        arrival_at: Some("2025-04-01 09:00:00".into()),
        start_at: Some("2025-04-01 09:10:00".into()),
        end_at: Some("2025-04-01 12:00:00".into()),
        arrival_code: None,
        start_code: None,
        end_code: None,
        is_complete: true,
        service_record_hash: Some(hash_hex.clone()),
        created_at: "2025-04-01T00:00:00Z".into(),
    };
    let j = serde_json::to_string(&original).unwrap();
    let f: FulfillmentRecord = serde_json::from_str(&j).unwrap();
    assert!(f.is_complete);
    assert_eq!(f.service_record_hash.as_deref(), Some(hash_hex.as_str()));
    assert_eq!(f.arrival_at.as_deref(), Some("2025-04-01 09:00:00"));
}

#[wasm_bindgen_test]
fn checkpoint_code_response_with_qr() {
    let json = r#"{
        "code": "123456",
        "expires_at": "2025-04-01 09:10:00",
        "checkpoint": "arrival",
        "qr_code_svg": "<svg></svg>"
    }"#;
    let r: CheckpointCodeResponse = serde_json::from_str(json).unwrap();
    assert_eq!(r.code, "123456");
    assert_eq!(r.qr_code_svg.as_deref(), Some("<svg></svg>"));
}

#[wasm_bindgen_test]
fn checkpoint_code_response_without_qr() {
    let json = r#"{
        "code": "999999",
        "expires_at": "2025-04-01 09:10:00",
        "checkpoint": "end"
    }"#;
    let r: CheckpointCodeResponse = serde_json::from_str(json).unwrap();
    assert!(r.qr_code_svg.is_none());
}

#[wasm_bindgen_test]
fn record_checkpoint_request_serializes_checkpoint_kind() {
    let req = RecordCheckpointRequest {
        fulfillment_id: "f-1".into(),
        checkpoint: CheckpointKind::Start,
        code: "123456".into(),
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["checkpoint"], "start");
    assert_eq!(v["code"], "123456");
}

// ══════════════════════════════════════════════════════════════════════
// Request DTOs
// ══════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn donate_request_with_optional_fields() {
    // Designated donation with budget_line_id
    let req = DonateRequest {
        project_id: "p-1".into(),
        amount_cents: 5_000,
        payment_method: Some("check".into()),
        budget_line_id: Some("bl-1".into()),
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["payment_method"], "check");
    assert_eq!(v["budget_line_id"], "bl-1");

    // General donation (no budget line)
    let general = DonateRequest {
        project_id: "p-1".into(),
        amount_cents: 1_000,
        payment_method: None,
        budget_line_id: None,
    };
    let v2 = serde_json::to_value(&general).unwrap();
    assert!(v2["payment_method"].is_null());
    assert!(v2["budget_line_id"].is_null());
}

#[wasm_bindgen_test]
fn refund_request_round_trip() {
    let r = RefundRequest {
        donation_id: "d-1".into(),
        reason: "duplicate charge".into(),
    };
    let j = serde_json::to_string(&r).unwrap();
    let back: RefundRequest = serde_json::from_str(&j).unwrap();
    assert_eq!(back.donation_id, "d-1");
    assert_eq!(back.reason, "duplicate charge");
}

#[wasm_bindgen_test]
fn approve_refund_request_with_and_without_token() {
    let without = ApproveRefundRequest {
        donation_id: "d-1".into(),
        approved: true,
        password: "pw".into(),
        confirmation_token: None,
    };
    let v = serde_json::to_value(&without).unwrap();
    assert!(v["confirmation_token"].is_null());

    let with = ApproveRefundRequest {
        donation_id: "d-1".into(),
        approved: true,
        password: "pw".into(),
        confirmation_token: Some("tok-xyz".into()),
    };
    let v2 = serde_json::to_value(&with).unwrap();
    assert_eq!(v2["confirmation_token"], "tok-xyz");
}

#[wasm_bindgen_test]
fn assign_role_request_round_trip() {
    let r = AssignRoleRequest {
        user_id: "u-1".into(),
        role: "project_manager".into(),
        password: "adminpass".into(),
    };
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["role"], "project_manager");
    assert_eq!(v["password"], "adminpass");
}

#[wasm_bindgen_test]
fn submit_and_respond_ticket_request_round_trip() {
    let s = SubmitTicketRequest {
        project_id: "p-1".into(),
        subject: "Hi".into(),
        body: "Question".into(),
    };
    let j = serde_json::to_string(&s).unwrap();
    let b: SubmitTicketRequest = serde_json::from_str(&j).unwrap();
    assert_eq!(b.subject, "Hi");

    let r = RespondTicketRequest {
        ticket_id: "t-1".into(),
        response: "Thanks!".into(),
    };
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["response"], "Thanks!");
}

#[wasm_bindgen_test]
fn post_update_and_record_expense_requests() {
    let upd = PostUpdateRequest {
        project_id: "p-1".into(),
        title: "T".into(),
        body: "B".into(),
    };
    let v = serde_json::to_value(&upd).unwrap();
    assert_eq!(v["title"], "T");

    let exp = RecordExpenseRequest {
        project_id: "p-1".into(),
        budget_line_id: "bl-1".into(),
        amount_cents: 500,
        description: "Pencils".into(),
        receipt_data: Some("raw note".into()),
    };
    let v = serde_json::to_value(&exp).unwrap();
    assert_eq!(v["amount_cents"], 500);
    assert_eq!(v["receipt_data"], "raw note");
}

#[wasm_bindgen_test]
fn review_expense_request_has_optional_note() {
    let with = ReviewExpenseRequest {
        expense_id: "e-1".into(),
        approved: false,
        note: Some("Missing receipt".into()),
    };
    let v = serde_json::to_value(&with).unwrap();
    assert_eq!(v["approved"], false);
    assert_eq!(v["note"], "Missing receipt");

    let without = ReviewExpenseRequest {
        expense_id: "e-2".into(),
        approved: true,
        note: None,
    };
    let v2 = serde_json::to_value(&without).unwrap();
    assert!(v2["note"].is_null());
}

#[wasm_bindgen_test]
fn review_receipt_request_rejection_reason() {
    let reject = ReviewReceiptRequest {
        receipt_id: "r-1".into(),
        verified: false,
        rejection_reason: Some("Blurry".into()),
    };
    let v = serde_json::to_value(&reject).unwrap();
    assert_eq!(v["verified"], false);
    assert_eq!(v["rejection_reason"], "Blurry");

    let accept = ReviewReceiptRequest {
        receipt_id: "r-2".into(),
        verified: true,
        rejection_reason: None,
    };
    let v = serde_json::to_value(&accept).unwrap();
    assert_eq!(v["verified"], true);
    assert!(v["rejection_reason"].is_null());
}

#[wasm_bindgen_test]
fn upload_receipt_request_base64_field() {
    let r = UploadReceiptRequest {
        expense_id: "e-1".into(),
        file_name: "r.pdf".into(),
        file_type: "application/pdf".into(),
        file_size: 4,
        file_data_base64: "dGVzdA==".into(),
    };
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["file_type"], "application/pdf");
    assert_eq!(v["file_data_base64"], "dGVzdA==");
}

#[wasm_bindgen_test]
fn moderate_comment_request_round_trip() {
    let r = ModerateCommentRequest {
        comment_id: "c-1".into(),
        approved: true,
    };
    let j = serde_json::to_string(&r).unwrap();
    let b: ModerateCommentRequest = serde_json::from_str(&j).unwrap();
    assert_eq!(b.comment_id, "c-1");
    assert!(b.approved);
}

#[wasm_bindgen_test]
fn track_event_request_with_and_without_dwell() {
    let with_dwell = TrackEventRequest {
        event_kind: EventKind::DwellTime,
        target_type: "project".into(),
        target_id: "p-1".into(),
        session_id: "sess-1".into(),
        dwell_ms: Some(5_000),
        metadata: None,
    };
    let v = serde_json::to_value(&with_dwell).unwrap();
    assert_eq!(v["event_kind"], "dwell_time");
    assert_eq!(v["dwell_ms"], 5_000);
    assert!(v["metadata"].is_null());

    let click = TrackEventRequest {
        event_kind: EventKind::Click,
        target_type: "button".into(),
        target_id: "donate".into(),
        session_id: "sess-1".into(),
        dwell_ms: None,
        metadata: Some("{\"area\":\"hero\"}".into()),
    };
    let v2 = serde_json::to_value(&click).unwrap();
    assert_eq!(v2["event_kind"], "click");
    assert!(v2["dwell_ms"].is_null());
    assert_eq!(v2["metadata"], "{\"area\":\"hero\"}");
}

#[wasm_bindgen_test]
fn generate_checkpoint_code_request_round_trip() {
    let r = GenerateCheckpointCodeRequest {
        fulfillment_id: "f-1".into(),
        checkpoint: CheckpointKind::End,
    };
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["checkpoint"], "end");
}

// ══════════════════════════════════════════════════════════════════════
// Envelope types
// ══════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn api_error_round_trip() {
    let e = ApiError {
        error: "Bad request".into(),
    };
    let j = serde_json::to_string(&e).unwrap();
    let b: ApiError = serde_json::from_str(&j).unwrap();
    assert_eq!(b.error, "Bad request");
}

#[wasm_bindgen_test]
fn api_success_round_trip() {
    let s = ApiSuccess {
        message: "Done".into(),
    };
    let j = serde_json::to_string(&s).unwrap();
    let b: ApiSuccess = serde_json::from_str(&j).unwrap();
    assert_eq!(b.message, "Done");
}

#[wasm_bindgen_test]
fn confirmation_required_round_trip() {
    let c = ConfirmationRequired {
        confirmation_token: "tok-abc".into(),
        message: "Confirm required".into(),
    };
    let j = serde_json::to_string(&c).unwrap();
    let b: ConfirmationRequired = serde_json::from_str(&j).unwrap();
    assert_eq!(b.confirmation_token, "tok-abc");
}

#[wasm_bindgen_test]
fn paginated_response_generic_over_comment() {
    let json = r#"{
        "items": [
            { "id": "c1", "project_id": "p1", "author_id": "u1", "author_name": "A",
              "body": "Nice", "moderation_status": "approved", "created_at": "2025-01-01T00:00:00Z" }
        ],
        "total": 1, "page": 1, "per_page": 10
    }"#;
    let r: PaginatedResponse<Comment> = serde_json::from_str(json).unwrap();
    assert_eq!(r.items.len(), 1);
    assert_eq!(r.items[0].body, "Nice");
    assert_eq!(r.total, 1);
}

// ══════════════════════════════════════════════════════════════════════
// Enum round-trips not yet covered
// ══════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn payment_method_round_trip() {
    for m in &[
        PaymentMethod::Cash,
        PaymentMethod::Check,
        PaymentMethod::CardTerminal,
    ] {
        let s = m.as_str();
        assert_eq!(PaymentMethod::from_str(s), Some(*m));
        let j = serde_json::to_string(m).unwrap();
        let back: PaymentMethod = serde_json::from_str(&j).unwrap();
        assert_eq!(back, *m);
    }
    assert!(PaymentMethod::from_str("crypto").is_none());
}

#[wasm_bindgen_test]
fn comment_with_moderation_round_trip() {
    let c = CommentWithModeration {
        id: "c1".into(),
        project_id: "p1".into(),
        author_id: "u1".into(),
        author_name: "A".into(),
        body: "text".into(),
        moderation_status: ModerationStatus::PendingReview,
        created_at: "2025-01-01T00:00:00Z".into(),
    };
    let v = serde_json::to_value(&c).unwrap();
    assert_eq!(v["moderation_status"], "pending_review");
    let r: CommentWithModeration = serde_json::from_value(v).unwrap();
    assert_eq!(r.moderation_status, ModerationStatus::PendingReview);
}

// ══════════════════════════════════════════════════════════════════════
// ProjectFilter + DateRange
// ══════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn project_filter_partial_applies() {
    // Only cause specified
    let only_cause = ProjectFilter {
        cause: Some("health".into()),
        status: None,
        zip_code: None,
        search: None,
    };
    let v = serde_json::to_value(&only_cause).unwrap();
    assert_eq!(v["cause"], "health");
    assert!(v["status"].is_null());
    assert!(v["zip_code"].is_null());
    assert!(v["search"].is_null());
}

#[wasm_bindgen_test]
fn date_range_open_ended() {
    let only_from = DateRange {
        from: Some("2025-01-01".into()),
        to: None,
    };
    let v = serde_json::to_value(&only_from).unwrap();
    assert_eq!(v["from"], "2025-01-01");
    assert!(v["to"].is_null());
}
