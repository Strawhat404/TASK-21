use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

use common::{
    CheckpointKind, CreateBudgetLine, CreateProjectRequest, DndSettings, DonationRecord,
    EventKind, ModerationConfig, ModerationStatus, PaginatedResponse, ProjectStatus,
    ProjectSummary, RegisterRequest, Role, SensitiveActionConfirm, UserProfile,
};

// ── Role serialization / deserialization ──

#[wasm_bindgen_test]
fn role_serialize_to_snake_case() {
    assert_eq!(serde_json::to_string(&Role::Supporter).unwrap(), "\"supporter\"");
    assert_eq!(serde_json::to_string(&Role::ProjectManager).unwrap(), "\"project_manager\"");
    assert_eq!(serde_json::to_string(&Role::FinanceReviewer).unwrap(), "\"finance_reviewer\"");
    assert_eq!(serde_json::to_string(&Role::Administrator).unwrap(), "\"administrator\"");
}

#[wasm_bindgen_test]
fn role_deserialize_from_snake_case() {
    assert_eq!(serde_json::from_str::<Role>("\"supporter\"").unwrap(), Role::Supporter);
    assert_eq!(serde_json::from_str::<Role>("\"project_manager\"").unwrap(), Role::ProjectManager);
    assert_eq!(serde_json::from_str::<Role>("\"finance_reviewer\"").unwrap(), Role::FinanceReviewer);
    assert_eq!(serde_json::from_str::<Role>("\"administrator\"").unwrap(), Role::Administrator);
}

#[wasm_bindgen_test]
fn role_as_str_from_str_round_trip() {
    let variants = [
        Role::Supporter,
        Role::ProjectManager,
        Role::FinanceReviewer,
        Role::Administrator,
    ];
    for role in &variants {
        let s = role.as_str();
        let recovered = Role::from_str(s).expect(&format!("from_str failed for {:?}", role));
        assert_eq!(*role, recovered);
    }
}

#[wasm_bindgen_test]
fn role_from_str_returns_none_for_unknown() {
    assert!(Role::from_str("unknown_role").is_none());
}

// ── ProjectStatus serialization / deserialization ──

#[wasm_bindgen_test]
fn project_status_serialize_to_snake_case() {
    assert_eq!(serde_json::to_string(&ProjectStatus::Draft).unwrap(), "\"draft\"");
    assert_eq!(serde_json::to_string(&ProjectStatus::Active).unwrap(), "\"active\"");
    assert_eq!(serde_json::to_string(&ProjectStatus::Funded).unwrap(), "\"funded\"");
    assert_eq!(serde_json::to_string(&ProjectStatus::Closed).unwrap(), "\"closed\"");
    assert_eq!(serde_json::to_string(&ProjectStatus::Unpublished).unwrap(), "\"unpublished\"");
}

#[wasm_bindgen_test]
fn project_status_deserialize_from_snake_case() {
    assert_eq!(serde_json::from_str::<ProjectStatus>("\"draft\"").unwrap(), ProjectStatus::Draft);
    assert_eq!(serde_json::from_str::<ProjectStatus>("\"active\"").unwrap(), ProjectStatus::Active);
    assert_eq!(serde_json::from_str::<ProjectStatus>("\"funded\"").unwrap(), ProjectStatus::Funded);
    assert_eq!(serde_json::from_str::<ProjectStatus>("\"closed\"").unwrap(), ProjectStatus::Closed);
    assert_eq!(
        serde_json::from_str::<ProjectStatus>("\"unpublished\"").unwrap(),
        ProjectStatus::Unpublished
    );
}

#[wasm_bindgen_test]
fn project_status_as_str_from_str_round_trip() {
    let variants = [
        ProjectStatus::Draft,
        ProjectStatus::Active,
        ProjectStatus::Funded,
        ProjectStatus::Closed,
        ProjectStatus::Unpublished,
    ];
    for status in &variants {
        let s = status.as_str();
        let recovered =
            ProjectStatus::from_str(s).expect(&format!("from_str failed for {:?}", status));
        assert_eq!(*status, recovered);
    }
}

#[wasm_bindgen_test]
fn project_status_from_str_returns_none_for_unknown() {
    assert!(ProjectStatus::from_str("bogus").is_none());
}

// ── UserProfile serialization round-trip ──

#[wasm_bindgen_test]
fn user_profile_serde_round_trip() {
    let profile = UserProfile {
        id: "u-1".into(),
        email: "alice@example.com".into(),
        display_name: "Alice".into(),
        role: Role::Supporter,
        dnd_start: "22:00".into(),
        dnd_end: "08:00".into(),
        timezone: "UTC".into(),
        created_at: "2025-01-01T00:00:00Z".into(),
    };
    let json = serde_json::to_string(&profile).unwrap();
    let recovered: UserProfile = serde_json::from_str(&json).unwrap();
    assert_eq!(recovered.id, "u-1");
    assert_eq!(recovered.email, "alice@example.com");
    assert_eq!(recovered.display_name, "Alice");
    assert_eq!(recovered.role, Role::Supporter);
    assert_eq!(recovered.timezone, "UTC");
}

// ── ProjectSummary serialization round-trip ──

#[wasm_bindgen_test]
fn project_summary_serde_round_trip() {
    let summary = ProjectSummary {
        id: "p-1".into(),
        title: "Build a Well".into(),
        cause: "water".into(),
        zip_code: "90210".into(),
        status: ProjectStatus::Active,
        goal_cents: 500_000,
        raised_cents: 123_000,
        spent_cents: 45_000,
        manager_name: "Bob".into(),
        created_at: "2025-06-01T12:00:00Z".into(),
    };
    let json = serde_json::to_string(&summary).unwrap();
    let recovered: ProjectSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(recovered.id, "p-1");
    assert_eq!(recovered.title, "Build a Well");
    assert_eq!(recovered.status, ProjectStatus::Active);
    assert_eq!(recovered.goal_cents, 500_000);
    assert_eq!(recovered.raised_cents, 123_000);
    assert_eq!(recovered.spent_cents, 45_000);
}

// ── DonationRecord optional fields serialize as null ──

#[wasm_bindgen_test]
fn donation_record_optional_fields_null() {
    let record = DonationRecord {
        id: "d-1".into(),
        pledge_number: "PLG-001".into(),
        project_id: "p-1".into(),
        project_title: "Build a Well".into(),
        donor_id: "u-2".into(),
        amount_cents: 5000,
        payment_method: "cash".into(),
        is_reversal: false,
        reversal_of: None,
        reversal_approved: None,
        budget_line_id: None,
        budget_line_name: None,
        created_at: "2025-06-15T10:00:00Z".into(),
    };
    let json = serde_json::to_string(&record).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(value["reversal_of"].is_null());
    assert!(value["budget_line_id"].is_null());
    assert!(value["budget_line_name"].is_null());
    assert!(value["reversal_approved"].is_null());
}

#[wasm_bindgen_test]
fn donation_record_optional_fields_present() {
    let record = DonationRecord {
        id: "d-2".into(),
        pledge_number: "PLG-002".into(),
        project_id: "p-1".into(),
        project_title: "Build a Well".into(),
        donor_id: "u-3".into(),
        amount_cents: 3000,
        payment_method: "check".into(),
        is_reversal: true,
        reversal_of: Some("d-1".into()),
        reversal_approved: Some(true),
        budget_line_id: Some("bl-1".into()),
        budget_line_name: Some("Materials".into()),
        created_at: "2025-06-16T10:00:00Z".into(),
    };
    let json = serde_json::to_string(&record).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["reversal_of"], "d-1");
    assert_eq!(value["budget_line_id"], "bl-1");
    assert_eq!(value["reversal_approved"], true);
}

// ── RegisterRequest serialization ──

#[wasm_bindgen_test]
fn register_request_json_structure() {
    let req = RegisterRequest {
        email: "newuser@example.com".into(),
        password: "secret123".into(),
        display_name: "New User".into(),
        role: Role::Supporter,
    };
    let value: serde_json::Value = serde_json::to_value(&req).unwrap();
    assert_eq!(value["email"], "newuser@example.com");
    assert_eq!(value["password"], "secret123");
    assert_eq!(value["display_name"], "New User");
    assert_eq!(value["role"], "supporter");
}

// ── CreateProjectRequest with budget lines ──

#[wasm_bindgen_test]
fn create_project_request_nested_serialization() {
    let req = CreateProjectRequest {
        title: "School Supplies".into(),
        description: "Provide supplies to local school".into(),
        cause: "education".into(),
        zip_code: "10001".into(),
        goal_cents: 200_000,
        budget_lines: vec![
            CreateBudgetLine {
                name: "Notebooks".into(),
                allocated_cents: 80_000,
            },
            CreateBudgetLine {
                name: "Pencils".into(),
                allocated_cents: 20_000,
            },
        ],
    };
    let value: serde_json::Value = serde_json::to_value(&req).unwrap();
    assert_eq!(value["title"], "School Supplies");
    assert_eq!(value["goal_cents"], 200_000);

    let lines = value["budget_lines"].as_array().unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["name"], "Notebooks");
    assert_eq!(lines[0]["allocated_cents"], 80_000);
    assert_eq!(lines[1]["name"], "Pencils");
    assert_eq!(lines[1]["allocated_cents"], 20_000);

    // round-trip
    let recovered: CreateProjectRequest = serde_json::from_value(value).unwrap();
    assert_eq!(recovered.budget_lines.len(), 2);
    assert_eq!(recovered.budget_lines[0].name, "Notebooks");
}

// ── PaginatedResponse<ProjectSummary> deserialization ──

#[wasm_bindgen_test]
fn paginated_response_deserialization() {
    let json = r#"{
        "items": [
            {
                "id": "p-10",
                "title": "Clean Water",
                "cause": "water",
                "zip_code": "30301",
                "status": "active",
                "goal_cents": 1000000,
                "raised_cents": 400000,
                "spent_cents": 100000,
                "manager_name": "Charlie",
                "created_at": "2025-03-01T00:00:00Z"
            }
        ],
        "total": 42,
        "page": 1,
        "per_page": 10
    }"#;
    let resp: PaginatedResponse<ProjectSummary> = serde_json::from_str(json).unwrap();
    assert_eq!(resp.total, 42);
    assert_eq!(resp.page, 1);
    assert_eq!(resp.per_page, 10);
    assert_eq!(resp.items.len(), 1);
    assert_eq!(resp.items[0].id, "p-10");
    assert_eq!(resp.items[0].title, "Clean Water");
    assert_eq!(resp.items[0].status, ProjectStatus::Active);
    assert_eq!(resp.items[0].goal_cents, 1_000_000);
}

// ── DndSettings default timezone ──

#[wasm_bindgen_test]
fn dnd_settings_default_timezone() {
    let json = r#"{"dnd_start": "22:00", "dnd_end": "08:00"}"#;
    let settings: DndSettings = serde_json::from_str(json).unwrap();
    assert_eq!(settings.timezone, "UTC");
    assert_eq!(settings.dnd_start, "22:00");
    assert_eq!(settings.dnd_end, "08:00");
}

#[wasm_bindgen_test]
fn dnd_settings_explicit_timezone() {
    let json = r#"{"dnd_start": "22:00", "dnd_end": "08:00", "timezone": "+05:30"}"#;
    let settings: DndSettings = serde_json::from_str(json).unwrap();
    assert_eq!(settings.timezone, "+05:30");
}

// ── SensitiveActionConfirm optional token ──

#[wasm_bindgen_test]
fn sensitive_action_confirm_without_token() {
    let json = r#"{"password": "mypass"}"#;
    let confirm: SensitiveActionConfirm = serde_json::from_str(json).unwrap();
    assert_eq!(confirm.password, "mypass");
    assert!(confirm.confirmation_token.is_none());
}

#[wasm_bindgen_test]
fn sensitive_action_confirm_with_token() {
    let json = r#"{"password": "mypass", "confirmation_token": "tok-abc"}"#;
    let confirm: SensitiveActionConfirm = serde_json::from_str(json).unwrap();
    assert_eq!(confirm.password, "mypass");
    assert_eq!(confirm.confirmation_token.as_deref(), Some("tok-abc"));
}

// ── EventKind as_str / from_str round-trips ──

#[wasm_bindgen_test]
fn event_kind_round_trip() {
    let variants = [
        EventKind::Impression,
        EventKind::Click,
        EventKind::DwellTime,
        EventKind::SessionStart,
        EventKind::SessionEnd,
    ];
    for kind in &variants {
        let s = kind.as_str();
        let recovered =
            EventKind::from_str(s).expect(&format!("from_str failed for {:?}", kind));
        assert_eq!(*kind, recovered);
    }
}

#[wasm_bindgen_test]
fn event_kind_from_str_returns_none_for_unknown() {
    assert!(EventKind::from_str("page_view").is_none());
}

// ── CheckpointKind as_str / from_str round-trips ──

#[wasm_bindgen_test]
fn checkpoint_kind_round_trip() {
    let variants = [
        CheckpointKind::Arrival,
        CheckpointKind::Start,
        CheckpointKind::End,
    ];
    for kind in &variants {
        let s = kind.as_str();
        let recovered =
            CheckpointKind::from_str(s).expect(&format!("from_str failed for {:?}", kind));
        assert_eq!(*kind, recovered);
    }
}

#[wasm_bindgen_test]
fn checkpoint_kind_from_str_returns_none_for_unknown() {
    assert!(CheckpointKind::from_str("middle").is_none());
}

// ── ModerationConfig serialize / deserialize with sensitive_words ──

#[wasm_bindgen_test]
fn moderation_config_serde_round_trip() {
    let config = ModerationConfig {
        comments_enabled: true,
        require_pre_moderation: false,
        sensitive_words: vec!["spam".into(), "scam".into(), "fraud".into()],
    };
    let json = serde_json::to_string(&config).unwrap();
    let recovered: ModerationConfig = serde_json::from_str(&json).unwrap();
    assert!(recovered.comments_enabled);
    assert!(!recovered.require_pre_moderation);
    assert_eq!(recovered.sensitive_words.len(), 3);
    assert_eq!(recovered.sensitive_words[0], "spam");
    assert_eq!(recovered.sensitive_words[1], "scam");
    assert_eq!(recovered.sensitive_words[2], "fraud");
}

#[wasm_bindgen_test]
fn moderation_config_empty_sensitive_words() {
    let json = r#"{"comments_enabled": false, "require_pre_moderation": true, "sensitive_words": []}"#;
    let config: ModerationConfig = serde_json::from_str(json).unwrap();
    assert!(!config.comments_enabled);
    assert!(config.require_pre_moderation);
    assert!(config.sensitive_words.is_empty());
}

// ── ModerationStatus as_str / from_str ──

#[wasm_bindgen_test]
fn moderation_status_round_trip() {
    let variants = [
        ModerationStatus::Approved,
        ModerationStatus::PendingReview,
        ModerationStatus::Rejected,
    ];
    for status in &variants {
        let s = status.as_str();
        let recovered =
            ModerationStatus::from_str(s).expect(&format!("from_str failed for {:?}", status));
        assert_eq!(*status, recovered);
    }
}
