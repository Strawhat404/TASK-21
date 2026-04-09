use serde::{Deserialize, Serialize};

// ── Roles ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Supporter,
    ProjectManager,
    FinanceReviewer,
    Administrator,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Supporter => "supporter",
            Role::ProjectManager => "project_manager",
            Role::FinanceReviewer => "finance_reviewer",
            Role::Administrator => "administrator",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "supporter" => Some(Role::Supporter),
            "project_manager" => Some(Role::ProjectManager),
            "finance_reviewer" => Some(Role::FinanceReviewer),
            "administrator" => Some(Role::Administrator),
            _ => None,
        }
    }
}

// ── Project Status ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Draft,
    Active,
    Funded,
    Closed,
    Unpublished,
}

impl ProjectStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectStatus::Draft => "draft",
            ProjectStatus::Active => "active",
            ProjectStatus::Funded => "funded",
            ProjectStatus::Closed => "closed",
            ProjectStatus::Unpublished => "unpublished",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(ProjectStatus::Draft),
            "active" => Some(ProjectStatus::Active),
            "funded" => Some(ProjectStatus::Funded),
            "closed" => Some(ProjectStatus::Closed),
            "unpublished" => Some(ProjectStatus::Unpublished),
            _ => None,
        }
    }
}

// ── Disclosure Status ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisclosureStatus {
    Pending,
    Approved,
    Rejected,
}

impl DisclosureStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DisclosureStatus::Pending => "pending",
            DisclosureStatus::Approved => "approved",
            DisclosureStatus::Rejected => "rejected",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(DisclosureStatus::Pending),
            "approved" => Some(DisclosureStatus::Approved),
            "rejected" => Some(DisclosureStatus::Rejected),
            _ => None,
        }
    }
}

// ── Payment Method ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    Cash,
    Check,
    CardTerminal,
}

impl PaymentMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            PaymentMethod::Cash => "cash",
            PaymentMethod::Check => "check",
            PaymentMethod::CardTerminal => "card_terminal",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "cash" => Some(PaymentMethod::Cash),
            "check" => Some(PaymentMethod::Check),
            "card_terminal" => Some(PaymentMethod::CardTerminal),
            _ => None,
        }
    }
}

// ── Receipt Verification Status ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptStatus {
    Uploaded,
    Verified,
    Rejected,
}

impl ReceiptStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReceiptStatus::Uploaded => "uploaded",
            ReceiptStatus::Verified => "verified",
            ReceiptStatus::Rejected => "rejected",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "uploaded" => Some(ReceiptStatus::Uploaded),
            "verified" => Some(ReceiptStatus::Verified),
            "rejected" => Some(ReceiptStatus::Rejected),
            _ => None,
        }
    }
}

// ── Fulfillment Checkpoint Kind ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointKind {
    Arrival,
    Start,
    End,
}

impl CheckpointKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            CheckpointKind::Arrival => "arrival",
            CheckpointKind::Start => "start",
            CheckpointKind::End => "end",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "arrival" => Some(CheckpointKind::Arrival),
            "start" => Some(CheckpointKind::Start),
            "end" => Some(CheckpointKind::End),
            _ => None,
        }
    }
}

// ── Event Kind ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Impression,
    Click,
    DwellTime,
    SessionStart,
    SessionEnd,
}

impl EventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventKind::Impression => "impression",
            EventKind::Click => "click",
            EventKind::DwellTime => "dwell_time",
            EventKind::SessionStart => "session_start",
            EventKind::SessionEnd => "session_end",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "impression" => Some(EventKind::Impression),
            "click" => Some(EventKind::Click),
            "dwell_time" => Some(EventKind::DwellTime),
            "session_start" => Some(EventKind::SessionStart),
            "session_end" => Some(EventKind::SessionEnd),
            _ => None,
        }
    }
}

// ── Comment Moderation Status ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationStatus {
    Approved,
    PendingReview,
    Rejected,
}

impl ModerationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModerationStatus::Approved => "approved",
            ModerationStatus::PendingReview => "pending_review",
            ModerationStatus::Rejected => "rejected",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "approved" => Some(ModerationStatus::Approved),
            "pending_review" => Some(ModerationStatus::PendingReview),
            "rejected" => Some(ModerationStatus::Rejected),
            _ => None,
        }
    }
}

// ── Data Transfer Objects ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub role: Role,
    pub dnd_start: String,
    pub dnd_end: String,
    pub timezone: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: String,
    pub title: String,
    pub cause: String,
    pub zip_code: String,
    pub status: ProjectStatus,
    pub goal_cents: i64,
    pub raised_cents: i64,
    pub spent_cents: i64,
    pub manager_name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDetail {
    pub id: String,
    pub title: String,
    pub description: String,
    pub cause: String,
    pub zip_code: String,
    pub status: ProjectStatus,
    pub goal_cents: i64,
    pub raised_cents: i64,
    pub spent_cents: i64,
    pub manager_id: String,
    pub manager_name: String,
    pub budget_lines: Vec<BudgetLine>,
    pub updates: Vec<SpendingUpdate>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetLine {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub allocated_cents: i64,
    pub spent_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingUpdate {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub body: String,
    pub author_name: String,
    pub like_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DonationRecord {
    pub id: String,
    pub pledge_number: String,
    pub project_id: String,
    pub project_title: String,
    pub donor_id: String,
    pub amount_cents: i64,
    pub payment_method: String,
    pub is_reversal: bool,
    pub reversal_of: Option<String>,
    pub reversal_approved: Option<bool>,
    pub budget_line_id: Option<String>,
    pub budget_line_name: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpenseRecord {
    pub id: String,
    pub project_id: String,
    pub budget_line_id: String,
    pub budget_line_name: String,
    pub amount_cents: i64,
    pub description: String,
    pub receipt_url: Option<String>,
    pub disclosure_status: DisclosureStatus,
    pub reviewer_note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub project_id: String,
    pub author_id: String,
    pub author_name: String,
    pub body: String,
    pub moderation_status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub body: String,
    pub is_read: bool,
    pub is_deferred: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub id: String,
    pub project_id: String,
    pub submitter_id: String,
    pub submitter_name: String,
    pub subject: String,
    pub body: String,
    pub status: String,
    pub response: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsLogEntry {
    pub id: String,
    pub actor_id: String,
    pub actor_name: String,
    pub action: String,
    pub detail: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub gmv_cents: i64,
    pub total_donations: i64,
    pub unique_donors: i64,
    pub average_donation_cents: i64,
    pub repeat_donor_rate: f64,
    pub conversion_rate: f64,
}

// ── Request / Response DTOs ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: String,
    pub role: Role,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub title: String,
    pub description: String,
    pub cause: String,
    pub zip_code: String,
    pub goal_cents: i64,
    pub budget_lines: Vec<CreateBudgetLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBudgetLine {
    pub name: String,
    pub allocated_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DonateRequest {
    pub project_id: String,
    pub amount_cents: i64,
    pub payment_method: Option<String>,
    pub budget_line_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundRequest {
    pub donation_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DonateResponse {
    pub donation: DonationRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostUpdateRequest {
    pub project_id: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordExpenseRequest {
    pub project_id: String,
    pub budget_line_id: String,
    pub amount_cents: i64,
    pub description: String,
    pub receipt_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewExpenseRequest {
    pub expense_id: String,
    pub approved: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommentRequest {
    pub project_id: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitTicketRequest {
    pub project_id: String,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespondTicketRequest {
    pub ticket_id: String,
    pub response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFilter {
    pub cause: Option<String>,
    pub status: Option<String>,
    pub zip_code: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DndSettings {
    pub dnd_start: String,
    pub dnd_end: String,
    /// User timezone as UTC offset (e.g., "+05:30", "-08:00") or "UTC". Defaults to "UTC".
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveActionConfirm {
    pub password: String,
    /// Server-issued confirmation token from the first step. If absent, the
    /// server returns a token instead of executing the action.
    #[serde(default)]
    pub confirmation_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationRequired {
    pub confirmation_token: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSuccess {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

// ── Receipt / Voucher ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptRecord {
    pub id: String,
    pub expense_id: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size: i64,
    pub sha256_fingerprint: String,
    pub status: ReceiptStatus,
    pub rejection_reason: Option<String>,
    pub reviewer_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadReceiptRequest {
    pub expense_id: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size: i64,
    pub file_data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewReceiptRequest {
    pub receipt_id: String,
    pub verified: bool,
    pub rejection_reason: Option<String>,
}

// ── Content Moderation ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationConfig {
    pub comments_enabled: bool,
    pub require_pre_moderation: bool,
    pub sensitive_words: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerateCommentRequest {
    pub comment_id: String,
    pub approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentWithModeration {
    pub id: String,
    pub project_id: String,
    pub author_id: String,
    pub author_name: String,
    pub body: String,
    pub moderation_status: ModerationStatus,
    pub created_at: String,
}

// ── Fulfillment Verification ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FulfillmentRecord {
    pub id: String,
    pub project_id: String,
    pub arrival_at: Option<String>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub arrival_code: Option<String>,
    pub start_code: Option<String>,
    pub end_code: Option<String>,
    pub is_complete: bool,
    pub service_record_hash: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateCheckpointCodeRequest {
    pub fulfillment_id: String,
    pub checkpoint: CheckpointKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointCodeResponse {
    pub code: String,
    pub expires_at: String,
    pub checkpoint: String,
    /// QR code rendered as an SVG string encoding the OTP code.
    #[serde(default)]
    pub qr_code_svg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordCheckpointRequest {
    pub fulfillment_id: String,
    pub checkpoint: CheckpointKind,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceProof {
    pub fulfillment_id: String,
    pub project_id: String,
    pub project_title: String,
    pub arrival_at: String,
    pub start_at: String,
    pub end_at: String,
    pub service_record_hash: String,
    pub generated_at: String,
}

// ── Event Instrumentation ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackEventRequest {
    pub event_kind: EventKind,
    pub target_type: String,
    pub target_id: String,
    pub session_id: String,
    pub dwell_ms: Option<i64>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub id: String,
    pub event_kind: String,
    pub target_type: String,
    pub target_id: String,
    pub session_id: String,
    pub user_id: Option<String>,
    pub dwell_ms: Option<i64>,
    pub is_duplicate: bool,
    pub is_suspicious: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQualityMetrics {
    pub total_events: i64,
    pub duplicate_events: i64,
    pub suspicious_events: i64,
    pub duplicate_rate: f64,
    pub suspicious_rate: f64,
    pub events_by_kind: Vec<(String, i64)>,
}

// ── Webhooks ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub id: String,
    pub name: String,
    pub url: String,
    pub secret: String,
    pub event_types: Vec<String>,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWebhookRequest {
    pub name: String,
    pub url: String,
    pub event_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDeliveryLog {
    pub id: String,
    pub webhook_id: String,
    pub event_type: String,
    pub payload_summary: String,
    pub attempt: i32,
    pub status_code: Option<i32>,
    pub success: bool,
    pub error_message: Option<String>,
    pub created_at: String,
}

// ── Refund Approval ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveRefundRequest {
    pub donation_id: String,
    pub approved: bool,
    pub password: String,
    #[serde(default)]
    pub confirmation_token: Option<String>,
}

// ── Admin Role Assignment ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignRoleRequest {
    pub user_id: String,
    pub role: String,
    pub password: String,
}
