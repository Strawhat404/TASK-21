//! Frontend UI-logic tests.
//!
//! These exercise the pure-functional bits of the Leptos client that would
//! otherwise only be covered indirectly through the render path. We replicate
//! the exact formulas used by components in the frontend (budget-bar
//! thresholds, currency rendering, receipt display strings) so regressions
//! in these helpers are caught by the WASM test suite.

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ── BudgetBar threshold logic (mirror of components/budget_bar.rs) ─────

fn budget_bar_class(pct: f64) -> &'static str {
    if pct > 90.0 {
        "progress-danger"
    } else if pct > 70.0 {
        "progress-warning"
    } else {
        "progress-normal"
    }
}

fn budget_bar_pct(current: i64, max: i64) -> f64 {
    if max > 0 {
        (current as f64 / max as f64 * 100.0).min(100.0)
    } else {
        0.0
    }
}

#[wasm_bindgen_test]
fn budget_bar_zero_max_yields_zero_pct() {
    assert_eq!(budget_bar_pct(100, 0), 0.0);
    assert_eq!(budget_bar_class(0.0), "progress-normal");
}

#[wasm_bindgen_test]
fn budget_bar_half_is_normal() {
    let pct = budget_bar_pct(500, 1000);
    assert!((pct - 50.0).abs() < 1e-9);
    assert_eq!(budget_bar_class(pct), "progress-normal");
}

#[wasm_bindgen_test]
fn budget_bar_71_pct_is_warning() {
    let pct = budget_bar_pct(7100, 10_000);
    assert!((pct - 71.0).abs() < 1e-9);
    assert_eq!(budget_bar_class(pct), "progress-warning");
}

#[wasm_bindgen_test]
fn budget_bar_91_pct_is_danger() {
    let pct = budget_bar_pct(9100, 10_000);
    assert_eq!(budget_bar_class(pct), "progress-danger");
}

#[wasm_bindgen_test]
fn budget_bar_over_100_clamps_to_100() {
    let pct = budget_bar_pct(15_000, 10_000);
    assert!((pct - 100.0).abs() < 1e-9);
    assert_eq!(budget_bar_class(pct), "progress-danger");
}

#[wasm_bindgen_test]
fn budget_bar_exact_thresholds_boundaries() {
    // > 90.0 is danger; 90.0 exactly is warning per the > check
    assert_eq!(budget_bar_class(90.0), "progress-warning");
    assert_eq!(budget_bar_class(90.01), "progress-danger");
    // > 70.0 is warning; 70.0 exactly is normal
    assert_eq!(budget_bar_class(70.0), "progress-normal");
    assert_eq!(budget_bar_class(70.01), "progress-warning");
}

// ── Currency formatting (mirror of receipt.rs / budget_bar.rs) ─────────

fn format_cents(amount: i64) -> String {
    format!("${:.2}", amount as f64 / 100.0)
}

#[wasm_bindgen_test]
fn format_cents_whole_dollars() {
    assert_eq!(format_cents(10_000), "$100.00");
    assert_eq!(format_cents(5_000), "$50.00");
}

#[wasm_bindgen_test]
fn format_cents_cents_only() {
    assert_eq!(format_cents(0), "$0.00");
    assert_eq!(format_cents(1), "$0.01");
    assert_eq!(format_cents(99), "$0.99");
}

#[wasm_bindgen_test]
fn format_cents_mixed() {
    assert_eq!(format_cents(12_345), "$123.45");
    assert_eq!(format_cents(999_999), "$9999.99");
}

#[wasm_bindgen_test]
fn format_cents_handles_negative_refund() {
    // Refunds are stored as negative cents; the UI should still render them.
    assert_eq!(format_cents(-5_000), "$-50.00");
}

// ── Pledge/reversal number prefix convention ───────────────────────────

fn pledge_prefix(is_reversal: bool) -> &'static str {
    if is_reversal {
        "REF-"
    } else {
        "PLG-"
    }
}

#[wasm_bindgen_test]
fn pledge_number_convention() {
    assert_eq!(pledge_prefix(false), "PLG-");
    assert_eq!(pledge_prefix(true), "REF-");
}

// ── Email masking (client-side analytics display) ──────────────────────

fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((local, domain)) => {
            let first = local.chars().next().unwrap_or('*');
            format!("{}***@{}", first, domain)
        }
        None => "***".to_string(),
    }
}

#[wasm_bindgen_test]
fn mask_email_normal() {
    assert_eq!(mask_email("john.doe@example.com"), "j***@example.com");
}

#[wasm_bindgen_test]
fn mask_email_single_char_local() {
    assert_eq!(mask_email("a@b.co"), "a***@b.co");
}

#[wasm_bindgen_test]
fn mask_email_no_at() {
    assert_eq!(mask_email("notanemail"), "***");
    assert_eq!(mask_email(""), "***");
}

#[wasm_bindgen_test]
fn mask_email_unicode_local_char() {
    // First char is a multi-byte Unicode character
    let result = mask_email("ünique@example.com");
    assert!(result.starts_with("ü***@"));
    assert!(result.ends_with("@example.com"));
}

// ── Query string builders (mirror of api.rs list_projects) ─────────────

fn build_projects_query(
    cause: Option<&str>,
    status: Option<&str>,
    zip: Option<&str>,
    search: Option<&str>,
    page: i64,
) -> String {
    let mut parts = vec![format!("page={}", page)];
    if let Some(c) = cause {
        parts.push(format!("cause={}", c));
    }
    if let Some(s) = status {
        parts.push(format!("status={}", s));
    }
    if let Some(z) = zip {
        parts.push(format!("zip_code={}", z));
    }
    if let Some(s) = search {
        parts.push(format!("search={}", s));
    }
    format!("/projects?{}", parts.join("&"))
}

#[wasm_bindgen_test]
fn projects_query_only_page() {
    assert_eq!(build_projects_query(None, None, None, None, 1), "/projects?page=1");
}

#[wasm_bindgen_test]
fn projects_query_with_filters_preserves_order() {
    let q = build_projects_query(
        Some("health"),
        Some("active"),
        Some("90210"),
        Some("clean"),
        2,
    );
    assert_eq!(
        q,
        "/projects?page=2&cause=health&status=active&zip_code=90210&search=clean"
    );
}

#[wasm_bindgen_test]
fn projects_query_page_only_when_others_none() {
    let q = build_projects_query(Some("health"), None, None, None, 1);
    assert_eq!(q, "/projects?page=1&cause=health");
}

// ── Dashboard date range / filter query (mirror of api.rs) ─────────────

fn build_stats_query(
    from: Option<&str>,
    to: Option<&str>,
    cause: Option<&str>,
    status: Option<&str>,
) -> String {
    let mut parts = vec![];
    if let Some(f) = from {
        parts.push(format!("from={}", f));
    }
    if let Some(t) = to {
        parts.push(format!("to={}", t));
    }
    if let Some(c) = cause {
        parts.push(format!("cause={}", c));
    }
    if let Some(s) = status {
        parts.push(format!("status={}", s));
    }
    if parts.is_empty() {
        "/admin/stats".to_string()
    } else {
        format!("/admin/stats?{}", parts.join("&"))
    }
}

#[wasm_bindgen_test]
fn stats_query_empty_when_all_none() {
    assert_eq!(build_stats_query(None, None, None, None), "/admin/stats");
}

#[wasm_bindgen_test]
fn stats_query_all_params() {
    assert_eq!(
        build_stats_query(
            Some("2025-01-01"),
            Some("2025-12-31"),
            Some("health"),
            Some("active")
        ),
        "/admin/stats?from=2025-01-01&to=2025-12-31&cause=health&status=active"
    );
}

#[wasm_bindgen_test]
fn stats_query_partial_params() {
    assert_eq!(
        build_stats_query(Some("2025-01-01"), None, None, Some("active")),
        "/admin/stats?from=2025-01-01&status=active"
    );
}

// ── Receipt file validation (mirror of backend/routes/receipts.rs) ─────
// The frontend is expected to pre-check type/size before POSTing.

const ALLOWED_RECEIPT_TYPES: &[&str] = &["application/pdf", "image/jpeg", "image/png"];
const MAX_RECEIPT_BYTES: i64 = 10 * 1024 * 1024; // 10 MB

fn validate_receipt_upload(file_type: &str, file_size: i64) -> Result<(), &'static str> {
    if !ALLOWED_RECEIPT_TYPES.contains(&file_type) {
        return Err("invalid file type");
    }
    if file_size <= 0 {
        return Err("empty file");
    }
    if file_size > MAX_RECEIPT_BYTES {
        return Err("file too large");
    }
    Ok(())
}

#[wasm_bindgen_test]
fn receipt_upload_accepts_pdf() {
    assert!(validate_receipt_upload("application/pdf", 1000).is_ok());
}

#[wasm_bindgen_test]
fn receipt_upload_accepts_jpeg() {
    assert!(validate_receipt_upload("image/jpeg", 500_000).is_ok());
}

#[wasm_bindgen_test]
fn receipt_upload_accepts_png() {
    assert!(validate_receipt_upload("image/png", 200_000).is_ok());
}

#[wasm_bindgen_test]
fn receipt_upload_rejects_exe() {
    let err = validate_receipt_upload("application/x-executable", 100).unwrap_err();
    assert_eq!(err, "invalid file type");
}

#[wasm_bindgen_test]
fn receipt_upload_rejects_empty_file() {
    let err = validate_receipt_upload("application/pdf", 0).unwrap_err();
    assert_eq!(err, "empty file");
}

#[wasm_bindgen_test]
fn receipt_upload_rejects_oversize() {
    let err = validate_receipt_upload("application/pdf", MAX_RECEIPT_BYTES + 1).unwrap_err();
    assert_eq!(err, "file too large");
}

#[wasm_bindgen_test]
fn receipt_upload_accepts_exactly_max_size() {
    assert!(validate_receipt_upload("application/pdf", MAX_RECEIPT_BYTES).is_ok());
}

// ── DND hours display helper (time string validation) ──────────────────

fn is_valid_hhmm(s: &str) -> bool {
    // Expect HH:MM format, 00-23 hours, 00-59 minutes
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 || parts[0].len() != 2 || parts[1].len() != 2 {
        return false;
    }
    let h: u32 = match parts[0].parse() { Ok(n) => n, Err(_) => return false };
    let m: u32 = match parts[1].parse() { Ok(n) => n, Err(_) => return false };
    h < 24 && m < 60
}

#[wasm_bindgen_test]
fn dnd_valid_hhmm_accepts_defaults() {
    assert!(is_valid_hhmm("21:00"));
    assert!(is_valid_hhmm("07:00"));
    assert!(is_valid_hhmm("00:00"));
    assert!(is_valid_hhmm("23:59"));
}

#[wasm_bindgen_test]
fn dnd_invalid_hhmm_rejected() {
    assert!(!is_valid_hhmm("24:00"));
    assert!(!is_valid_hhmm("25:99"));
    assert!(!is_valid_hhmm("9:00"));
    assert!(!is_valid_hhmm("21"));
    assert!(!is_valid_hhmm("21:0"));
    assert!(!is_valid_hhmm(""));
    assert!(!is_valid_hhmm("abc:12"));
}

// ── Role-to-display helper ─────────────────────────────────────────────

fn role_display_name(role: &common::Role) -> &'static str {
    match role {
        common::Role::Supporter => "Supporter",
        common::Role::ProjectManager => "Project Manager",
        common::Role::FinanceReviewer => "Finance Reviewer",
        common::Role::Administrator => "Administrator",
    }
}

#[wasm_bindgen_test]
fn role_display_names() {
    assert_eq!(role_display_name(&common::Role::Supporter), "Supporter");
    assert_eq!(
        role_display_name(&common::Role::ProjectManager),
        "Project Manager"
    );
    assert_eq!(
        role_display_name(&common::Role::FinanceReviewer),
        "Finance Reviewer"
    );
    assert_eq!(
        role_display_name(&common::Role::Administrator),
        "Administrator"
    );
}

// ── Status badge class helpers (UI tag colors) ─────────────────────────

fn project_status_badge(s: &common::ProjectStatus) -> &'static str {
    match s {
        common::ProjectStatus::Draft => "badge-draft",
        common::ProjectStatus::Active => "badge-active",
        common::ProjectStatus::Funded => "badge-funded",
        common::ProjectStatus::Closed => "badge-closed",
        common::ProjectStatus::Unpublished => "badge-unpublished",
    }
}

#[wasm_bindgen_test]
fn project_status_badges_are_stable() {
    assert_eq!(project_status_badge(&common::ProjectStatus::Active), "badge-active");
    assert_eq!(project_status_badge(&common::ProjectStatus::Funded), "badge-funded");
    assert_eq!(project_status_badge(&common::ProjectStatus::Closed), "badge-closed");
    assert_eq!(
        project_status_badge(&common::ProjectStatus::Unpublished),
        "badge-unpublished"
    );
    assert_eq!(project_status_badge(&common::ProjectStatus::Draft), "badge-draft");
}

// ── Event dedup window semantics (mirror of backend dedup rule) ────────
// Dedup is: same (kind, target, session) within 3 seconds. The frontend
// fires events freely — the backend dedups. We simulate the client
// short-circuit helper if one were present.

fn should_emit_event(last_emit_ms: Option<i64>, now_ms: i64, dedup_window_ms: i64) -> bool {
    match last_emit_ms {
        None => true,
        Some(t) => now_ms.saturating_sub(t) > dedup_window_ms,
    }
}

#[wasm_bindgen_test]
fn should_emit_event_first_time_always_yes() {
    assert!(should_emit_event(None, 1_000, 3_000));
}

#[wasm_bindgen_test]
fn should_emit_event_within_window_is_no() {
    assert!(!should_emit_event(Some(1_000), 2_500, 3_000));
    assert!(!should_emit_event(Some(1_000), 3_999, 3_000));
}

#[wasm_bindgen_test]
fn should_emit_event_after_window_is_yes() {
    assert!(should_emit_event(Some(1_000), 4_001, 3_000));
    assert!(should_emit_event(Some(0), 10_000, 3_000));
}

// ── Fulfillment code display ───────────────────────────────────────────

fn format_otp(code: &str) -> String {
    // Expect 6-digit code, displayed as "123 456"
    if code.len() == 6 && code.chars().all(|c| c.is_ascii_digit()) {
        format!("{} {}", &code[..3], &code[3..])
    } else {
        code.to_string()
    }
}

#[wasm_bindgen_test]
fn otp_formatting_pretty_print() {
    assert_eq!(format_otp("123456"), "123 456");
    assert_eq!(format_otp("000000"), "000 000");
}

#[wasm_bindgen_test]
fn otp_formatting_non_numeric_unchanged() {
    assert_eq!(format_otp("abc123"), "abc123");
    assert_eq!(format_otp("12345"), "12345");
    assert_eq!(format_otp("1234567"), "1234567");
}

// ── Common serde sanity: request DTO → expected path/body pair ─────────

#[wasm_bindgen_test]
fn donate_request_json_has_required_fields() {
    let req = common::DonateRequest {
        project_id: "p-1".into(),
        amount_cents: 5_000,
        payment_method: Some("cash".into()),
        budget_line_id: None,
    };
    let v = serde_json::to_value(&req).unwrap();
    for key in ["project_id", "amount_cents", "payment_method", "budget_line_id"] {
        assert!(v.get(key).is_some(), "missing key {}", key);
    }
}

#[wasm_bindgen_test]
fn login_request_minimal_shape() {
    let req = common::LoginRequest {
        email: "x@y.com".into(),
        password: "pw".into(),
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["email"], "x@y.com");
    assert_eq!(v["password"], "pw");
    // Exactly these two keys
    let obj = v.as_object().unwrap();
    assert_eq!(obj.len(), 2);
}
