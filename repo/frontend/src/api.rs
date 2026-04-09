use gloo_net::http::Request;
use serde::{de::DeserializeOwned, Serialize};

const BASE: &str = "/api";

/// Fetch a fresh nonce from the server (required for every mutating request).
async fn fetch_nonce() -> Result<String, String> {
    let resp = Request::get(&format!("{}/auth/nonce", BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let val: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    val["nonce"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Failed to obtain nonce".into())
}

async fn get_json<T: DeserializeOwned>(path: &str, token: Option<&str>) -> Result<T, String> {
    let mut req = Request::get(&format!("{}{}", BASE, path));
    if let Some(t) = token {
        req = req.header("Authorization", &format!("Bearer {}", t));
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    resp.json().await.map_err(|e| e.to_string())
}

async fn post_json<T: DeserializeOwned, B: Serialize>(
    path: &str,
    body: &B,
    token: Option<&str>,
) -> Result<T, String> {
    let nonce = fetch_nonce().await?;
    let mut req = Request::post(&format!("{}{}", BASE, path))
        .header("X-Nonce", &nonce);
    if let Some(t) = token {
        req = req.header("Authorization", &format!("Bearer {}", t));
    }
    let resp = req
        .json(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    resp.json().await.map_err(|e| e.to_string())
}

async fn put_json<T: DeserializeOwned, B: Serialize>(
    path: &str,
    body: &B,
    token: Option<&str>,
) -> Result<T, String> {
    let nonce = fetch_nonce().await?;
    let mut req = Request::put(&format!("{}{}", BASE, path))
        .header("X-Nonce", &nonce);
    if let Some(t) = token {
        req = req.header("Authorization", &format!("Bearer {}", t));
    }
    let resp = req
        .json(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    resp.json().await.map_err(|e| e.to_string())
}

async fn delete_req<T: DeserializeOwned>(path: &str, token: Option<&str>) -> Result<T, String> {
    let nonce = fetch_nonce().await?;
    let mut req = Request::delete(&format!("{}{}", BASE, path))
        .header("X-Nonce", &nonce);
    if let Some(t) = token {
        req = req.header("Authorization", &format!("Bearer {}", t));
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        let text = resp.text().await.unwrap_or_default();
        return Err(text);
    }
    resp.json().await.map_err(|e| e.to_string())
}

// ── Auth ──

pub async fn login(email: &str, password: &str) -> Result<common::AuthResponse, String> {
    post_json(
        "/auth/login",
        &common::LoginRequest {
            email: email.to_string(),
            password: password.to_string(),
        },
        None,
    )
    .await
}

pub async fn register(
    email: &str,
    password: &str,
    display_name: &str,
    role: common::Role,
) -> Result<common::AuthResponse, String> {
    post_json(
        "/auth/register",
        &common::RegisterRequest {
            email: email.to_string(),
            password: password.to_string(),
            display_name: display_name.to_string(),
            role,
        },
        None,
    )
    .await
}

pub async fn get_me(token: &str) -> Result<common::UserProfile, String> {
    get_json("/auth/me", Some(token)).await
}

pub async fn update_dnd(
    token: &str,
    start: &str,
    end: &str,
    timezone: &str,
) -> Result<common::ApiSuccess, String> {
    put_json(
        "/auth/dnd",
        &common::DndSettings {
            dnd_start: start.to_string(),
            dnd_end: end.to_string(),
            timezone: timezone.to_string(),
        },
        Some(token),
    )
    .await
}

// ── Projects ──

pub async fn list_projects(
    cause: Option<&str>,
    status: Option<&str>,
    zip: Option<&str>,
    search: Option<&str>,
    page: i64,
) -> Result<common::PaginatedResponse<common::ProjectSummary>, String> {
    let mut query_parts = vec![format!("page={}", page)];
    if let Some(c) = cause {
        query_parts.push(format!("cause={}", c));
    }
    if let Some(s) = status {
        query_parts.push(format!("status={}", s));
    }
    if let Some(z) = zip {
        query_parts.push(format!("zip_code={}", z));
    }
    if let Some(s) = search {
        query_parts.push(format!("search={}", s));
    }
    let path = format!("/projects?{}", query_parts.join("&"));
    get_json(&path, None).await
}

pub async fn get_project(id: &str) -> Result<common::ProjectDetail, String> {
    get_json(&format!("/projects/{}", id), None).await
}

pub async fn create_project(
    token: &str,
    req: &common::CreateProjectRequest,
) -> Result<common::ProjectDetail, String> {
    post_json("/projects", req, Some(token)).await
}

pub async fn post_update(
    token: &str,
    req: &common::PostUpdateRequest,
) -> Result<common::ApiSuccess, String> {
    post_json("/projects/updates", req, Some(token)).await
}

pub async fn record_expense(
    token: &str,
    req: &common::RecordExpenseRequest,
) -> Result<common::ApiSuccess, String> {
    post_json("/projects/expenses", req, Some(token)).await
}

// ── Donations ──

pub async fn donate(
    token: &str,
    req: &common::DonateRequest,
) -> Result<common::DonateResponse, String> {
    post_json("/donations", req, Some(token)).await
}

pub async fn my_donations(token: &str) -> Result<Vec<common::DonationRecord>, String> {
    get_json("/donations/mine", Some(token)).await
}

// ── Comments ──

pub async fn list_comments(project_id: &str) -> Result<Vec<common::Comment>, String> {
    get_json(&format!("/projects/{}/comments", project_id), None).await
}

pub async fn post_comment(
    token: &str,
    project_id: &str,
    body: &str,
) -> Result<common::ApiSuccess, String> {
    post_json(
        "/comments",
        &common::CreateCommentRequest {
            project_id: project_id.to_string(),
            body: body.to_string(),
        },
        Some(token),
    )
    .await
}

pub async fn delete_comment(token: &str, comment_id: &str, password: &str) -> Result<common::ApiSuccess, String> {
    post_json(
        &format!("/comments/{}/delete", comment_id),
        &common::SensitiveActionConfirm { password: password.to_string(), confirmation_token: None },
        Some(token),
    )
    .await
}

/// Download CSV export as a string (fetched with auth token).
pub async fn export_csv(
    token: &str,
    from: Option<&str>,
    to: Option<&str>,
    cause: Option<&str>,
    status: Option<&str>,
) -> Result<String, String> {
    let mut parts = vec![];
    if let Some(f) = from { parts.push(format!("from={}", f)); }
    if let Some(t) = to { parts.push(format!("to={}", t)); }
    if let Some(c) = cause { parts.push(format!("cause={}", c)); }
    if let Some(s) = status { parts.push(format!("status={}", s)); }
    let q = if parts.is_empty() { String::new() } else { format!("?{}", parts.join("&")) };
    let mut req = Request::get(&format!("{}/admin/export/csv{}", BASE, q));
    req = req.header("Authorization", &format!("Bearer {}", token));
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(resp.text().await.unwrap_or_default());
    }
    resp.text().await.map_err(|e| e.to_string())
}

// ── Favorites & Likes ──

pub async fn toggle_favorite(
    token: &str,
    project_id: &str,
) -> Result<serde_json::Value, String> {
    post_json(
        &format!("/projects/{}/favorite", project_id),
        &serde_json::json!({}),
        Some(token),
    )
    .await
}

pub async fn list_favorites(token: &str) -> Result<Vec<String>, String> {
    get_json("/favorites", Some(token)).await
}

pub async fn list_favorite_projects(token: &str) -> Result<Vec<common::ProjectSummary>, String> {
    get_json("/favorites/projects", Some(token)).await
}

pub async fn toggle_like(token: &str, update_id: &str) -> Result<serde_json::Value, String> {
    post_json(
        &format!("/updates/{}/like", update_id),
        &serde_json::json!({}),
        Some(token),
    )
    .await
}

// ── Subscriptions ──

pub async fn subscribe(token: &str, project_id: &str) -> Result<common::ApiSuccess, String> {
    post_json(
        &format!("/projects/{}/subscribe", project_id),
        &serde_json::json!({}),
        Some(token),
    )
    .await
}

pub async fn unsubscribe(token: &str, project_id: &str) -> Result<common::ApiSuccess, String> {
    post_json(
        &format!("/projects/{}/unsubscribe", project_id),
        &serde_json::json!({}),
        Some(token),
    )
    .await
}

// ── Notifications ──

pub async fn list_notifications(token: &str) -> Result<Vec<common::Notification>, String> {
    get_json("/notifications", Some(token)).await
}

pub async fn mark_notification_read(
    token: &str,
    notif_id: &str,
) -> Result<common::ApiSuccess, String> {
    post_json(
        &format!("/notifications/{}/read", notif_id),
        &serde_json::json!({}),
        Some(token),
    )
    .await
}

pub async fn mark_all_read(token: &str) -> Result<common::ApiSuccess, String> {
    post_json(
        "/notifications/read-all",
        &serde_json::json!({}),
        Some(token),
    )
    .await
}

// ── Tickets ──

pub async fn submit_ticket(
    token: &str,
    project_id: &str,
    subject: &str,
    body: &str,
) -> Result<common::ApiSuccess, String> {
    post_json(
        "/tickets",
        &common::SubmitTicketRequest {
            project_id: project_id.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
        },
        Some(token),
    )
    .await
}

pub async fn list_tickets(token: &str, project_id: &str) -> Result<Vec<common::Ticket>, String> {
    get_json(&format!("/projects/{}/tickets", project_id), Some(token)).await
}

pub async fn respond_ticket(
    token: &str,
    ticket_id: &str,
    response: &str,
) -> Result<common::ApiSuccess, String> {
    post_json(
        "/tickets/respond",
        &common::RespondTicketRequest {
            ticket_id: ticket_id.to_string(),
            response: response.to_string(),
        },
        Some(token),
    )
    .await
}

// ── Finance ──

pub async fn pending_expenses(token: &str) -> Result<Vec<common::ExpenseRecord>, String> {
    get_json("/finance/pending", Some(token)).await
}

pub async fn review_expense(
    token: &str,
    expense_id: &str,
    approved: bool,
    note: Option<&str>,
) -> Result<common::ApiSuccess, String> {
    post_json(
        "/finance/review",
        &common::ReviewExpenseRequest {
            expense_id: expense_id.to_string(),
            approved,
            note: note.map(|s| s.to_string()),
        },
        Some(token),
    )
    .await
}

// ── Admin ──

pub async fn dashboard_stats(
    token: &str,
    from: Option<&str>,
    to: Option<&str>,
    cause: Option<&str>,
    status: Option<&str>,
) -> Result<common::DashboardStats, String> {
    let mut query_parts = vec![];
    if let Some(f) = from { query_parts.push(format!("from={}", f)); }
    if let Some(t) = to { query_parts.push(format!("to={}", t)); }
    if let Some(c) = cause { query_parts.push(format!("cause={}", c)); }
    if let Some(s) = status { query_parts.push(format!("status={}", s)); }
    let q = if query_parts.is_empty() { String::new() } else { format!("?{}", query_parts.join("&")) };
    get_json(&format!("/admin/stats{}", q), Some(token)).await
}

pub async fn ops_log(
    token: &str,
    page: i64,
) -> Result<Vec<common::OpsLogEntry>, String> {
    get_json(&format!("/admin/ops-log?page={}", page), Some(token)).await
}

pub async fn unpublish_project(
    token: &str,
    project_id: &str,
    password: &str,
) -> Result<common::ApiSuccess, String> {
    post_json(
        &format!("/admin/projects/{}/unpublish", project_id),
        &common::SensitiveActionConfirm {
            password: password.to_string(),
            confirmation_token: None,
        },
        Some(token),
    )
    .await
}

// ── Role Management ──

pub async fn assign_role(
    token: &str,
    user_id: &str,
    role: &str,
    password: &str,
) -> Result<common::ApiSuccess, String> {
    post_json(
        "/admin/assign-role",
        &common::AssignRoleRequest {
            user_id: user_id.to_string(),
            role: role.to_string(),
            password: password.to_string(),
        },
        Some(token),
    )
    .await
}

pub async fn bootstrap_admin(
    token: &str,
    password: &str,
) -> Result<common::ApiSuccess, String> {
    post_json(
        "/admin/bootstrap",
        &common::SensitiveActionConfirm {
            password: password.to_string(),
            confirmation_token: None,
        },
        Some(token),
    )
    .await
}

// ── Refunds ──

pub async fn request_refund(
    token: &str,
    donation_id: &str,
    reason: &str,
) -> Result<common::DonateResponse, String> {
    post_json(
        "/donations/refund",
        &common::RefundRequest {
            donation_id: donation_id.to_string(),
            reason: reason.to_string(),
        },
        Some(token),
    )
    .await
}

pub async fn pending_refunds(token: &str) -> Result<Vec<common::DonationRecord>, String> {
    get_json("/donations/refund/pending", Some(token)).await
}

pub async fn approve_refund(
    token: &str,
    donation_id: &str,
    approved: bool,
    password: &str,
) -> Result<common::ApiSuccess, String> {
    post_json(
        "/donations/refund/approve",
        &common::ApproveRefundRequest {
            donation_id: donation_id.to_string(),
            approved,
            password: password.to_string(),
            confirmation_token: None,
        },
        Some(token),
    )
    .await
}

// ── Receipts ──

pub async fn upload_receipt(
    token: &str,
    req: &common::UploadReceiptRequest,
) -> Result<common::ReceiptRecord, String> {
    post_json("/receipts/upload", req, Some(token)).await
}

pub async fn review_receipt(
    token: &str,
    receipt_id: &str,
    verified: bool,
    rejection_reason: Option<&str>,
) -> Result<common::ApiSuccess, String> {
    post_json(
        "/receipts/review",
        &common::ReviewReceiptRequest {
            receipt_id: receipt_id.to_string(),
            verified,
            rejection_reason: rejection_reason.map(|s| s.to_string()),
        },
        Some(token),
    )
    .await
}

pub async fn pending_receipts(token: &str) -> Result<Vec<common::ReceiptRecord>, String> {
    get_json("/receipts/pending", Some(token)).await
}

// ── Moderation ──

pub async fn get_moderation_config(token: &str) -> Result<common::ModerationConfig, String> {
    get_json("/moderation/config", Some(token)).await
}

pub async fn update_moderation_config(
    token: &str,
    config: &common::ModerationConfig,
) -> Result<common::ApiSuccess, String> {
    put_json("/moderation/config", config, Some(token)).await
}

pub async fn pending_moderation_comments(token: &str) -> Result<Vec<common::Comment>, String> {
    get_json("/moderation/comments/pending", Some(token)).await
}

pub async fn moderate_comment(
    token: &str,
    comment_id: &str,
    approved: bool,
) -> Result<common::ApiSuccess, String> {
    post_json(
        "/moderation/comments/review",
        &common::ModerateCommentRequest {
            comment_id: comment_id.to_string(),
            approved,
        },
        Some(token),
    )
    .await
}

// ── Fulfillment ──

pub async fn create_fulfillment(
    token: &str,
    project_id: &str,
) -> Result<common::FulfillmentRecord, String> {
    post_json(
        "/fulfillments",
        &serde_json::json!({ "project_id": project_id }),
        Some(token),
    )
    .await
}

pub async fn list_fulfillments(token: &str, project_id: &str) -> Result<Vec<common::FulfillmentRecord>, String> {
    get_json(&format!("/projects/{}/fulfillments", project_id), Some(token)).await
}

pub async fn generate_checkpoint_code(
    token: &str,
    fulfillment_id: &str,
    checkpoint: &str,
) -> Result<common::CheckpointCodeResponse, String> {
    post_json(
        "/fulfillments/code",
        &serde_json::json!({
            "fulfillment_id": fulfillment_id,
            "checkpoint": checkpoint,
        }),
        Some(token),
    )
    .await
}

pub async fn record_checkpoint(
    token: &str,
    fulfillment_id: &str,
    checkpoint: &str,
    code: &str,
) -> Result<common::ApiSuccess, String> {
    post_json(
        "/fulfillments/checkpoint",
        &serde_json::json!({
            "fulfillment_id": fulfillment_id,
            "checkpoint": checkpoint,
            "code": code,
        }),
        Some(token),
    )
    .await
}

pub async fn service_proof(token: &str, fulfillment_id: &str) -> Result<common::ServiceProof, String> {
    get_json(&format!("/fulfillments/{}/proof", fulfillment_id), Some(token)).await
}

// ── Events ──

pub async fn track_event(req: &common::TrackEventRequest) -> Result<common::ApiSuccess, String> {
    post_json("/events/track", req, None).await
}

pub async fn data_quality_metrics(token: &str) -> Result<common::DataQualityMetrics, String> {
    get_json("/events/quality", Some(token)).await
}

pub async fn suspicious_events(token: &str) -> Result<Vec<common::AnalyticsEvent>, String> {
    get_json("/events/suspicious", Some(token)).await
}

// ── Webhooks ──

pub async fn create_webhook(
    token: &str,
    req: &common::CreateWebhookRequest,
) -> Result<common::WebhookConfig, String> {
    post_json("/webhooks", req, Some(token)).await
}

pub async fn list_webhooks(token: &str) -> Result<Vec<common::WebhookConfig>, String> {
    get_json("/webhooks", Some(token)).await
}

pub async fn delete_webhook(token: &str, webhook_id: &str) -> Result<common::ApiSuccess, String> {
    delete_req(&format!("/webhooks/{}", webhook_id), Some(token)).await
}

pub async fn webhook_deliveries(
    token: &str,
    webhook_id: &str,
) -> Result<Vec<common::WebhookDeliveryLog>, String> {
    get_json(
        &format!("/webhooks/{}/deliveries", webhook_id),
        Some(token),
    )
    .await
}
