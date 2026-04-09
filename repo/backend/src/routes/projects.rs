use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::db;
use crate::middleware::{require_auth, require_role};
use crate::AppState;
use common::*;

#[derive(serde::Deserialize)]
pub struct ListParams {
    pub cause: Option<String>,
    pub status: Option<String>,
    pub zip_code: Option<String>,
    pub search: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

pub async fn list_projects(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> Json<PaginatedResponse<ProjectSummary>> {
    let filter = ProjectFilter {
        cause: params.cause,
        status: params.status,
        zip_code: params.zip_code,
        search: params.search,
    };
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;

    let (items, total) = db::list_projects(&state.db, &filter, per_page, offset);
    Json(PaginatedResponse {
        items,
        total,
        page,
        per_page,
    })
}

pub async fn get_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ProjectDetail>, StatusCode> {
    db::get_project_detail(&state.db, &id)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn create_project(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ProjectDetail>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions()).map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let user = require_role(&state.db, &auth.user_id, &[Role::ProjectManager, Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden".into() })))?;

    // Parse body manually since we already consumed the request for auth
    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: CreateProjectRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    let project_id = uuid::Uuid::new_v4().to_string();
    let budget_lines: Vec<(String, String, i64)> = req
        .budget_lines
        .iter()
        .map(|bl| (uuid::Uuid::new_v4().to_string(), bl.name.clone(), bl.allocated_cents))
        .collect();

    db::create_project(
        &state.db,
        &project_id,
        &req.title,
        &req.description,
        &req.cause,
        &req.zip_code,
        req.goal_cents,
        &auth.user_id,
        &budget_lines,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    db::append_ops_log(
        &state.db,
        &auth.user_id,
        &user.display_name,
        "create_project",
        &format!("Created project: {}", req.title),
    );

    let detail = db::get_project_detail(&state.db, &project_id).unwrap();
    Ok(Json(detail))
}

pub async fn post_update(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions()).map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let user = require_role(&state.db, &auth.user_id, &[Role::ProjectManager, Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: PostUpdateRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    // IDOR: verify caller owns this project
    crate::middleware::require_project_owner(&state.db, &auth.user_id, &req.project_id)
        .map_err(|s| (s, Json(ApiError { error: "You do not manage this project".into() })))?;

    let update_id = uuid::Uuid::new_v4().to_string();
    db::create_spending_update(&state.db, &update_id, &req.project_id, &auth.user_id, &req.title, &req.body)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    // Notify subscribers
    let subscribers = db::get_project_subscribers(&state.db, &req.project_id);
    for sub_id in subscribers {
        let notif_id = uuid::Uuid::new_v4().to_string();
        let _ = db::create_notification(
            &state.db,
            &notif_id,
            &sub_id,
            &format!("New update: {}", req.title),
            &format!("{} posted an update on a project you follow", user.display_name),
        );
    }

    db::append_ops_log(&state.db, &auth.user_id, &user.display_name, "post_update", &format!("Posted update: {}", req.title));

    Ok(Json(ApiSuccess {
        message: "Update posted".into(),
    }))
}

pub async fn record_expense(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions()).map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let user = require_role(&state.db, &auth.user_id, &[Role::ProjectManager, Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: RecordExpenseRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    // IDOR: verify caller owns this project
    crate::middleware::require_project_owner(&state.db, &auth.user_id, &req.project_id)
        .map_err(|s| (s, Json(ApiError { error: "You do not manage this project".into() })))?;

    // Validate budget line belongs to this project
    let bl_project = db::get_budget_line_project_id(&state.db, &req.budget_line_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiError { error: "Budget line not found".into() })))?;
    if bl_project != req.project_id {
        return Err((StatusCode::BAD_REQUEST, Json(ApiError {
            error: "Budget line does not belong to this project".into(),
        })));
    }

    // Encrypt receipt data if present
    let encrypted_receipt = req.receipt_data.as_deref().map(|data| {
        crate::crypto::encrypt(data, &state.encryption_key).unwrap_or_default()
    });

    let expense_id = uuid::Uuid::new_v4().to_string();
    db::create_expense(
        &state.db,
        &expense_id,
        &req.project_id,
        &req.budget_line_id,
        req.amount_cents,
        &req.description,
        encrypted_receipt.as_deref(),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    db::append_ops_log(
        &state.db,
        &auth.user_id,
        &user.display_name,
        "record_expense",
        &format!("Recorded expense of {} cents", req.amount_cents),
    );

    Ok(Json(ApiSuccess {
        message: "Expense recorded".into(),
    }))
}

/// List expenses — scoped to project manager/admin/finance reviewer.
pub async fn get_expenses(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<ExpenseRecord>>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let user = db::get_user_by_id(&state.db, &auth.user_id)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ApiError { error: "User not found".into() })))?;
    if user.role != Role::Administrator && user.role != Role::FinanceReviewer {
        crate::middleware::require_project_owner(&state.db, &auth.user_id, &project_id)
            .map_err(|s| (s, Json(ApiError { error: "Not authorized".into() })))?;
    }
    Ok(Json(db::list_expenses(&state.db, &project_id, &state.encryption_key)))
}

pub async fn toggle_favorite(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    let is_fav = db::toggle_favorite(&state.db, &auth.user_id, &project_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "favorited": is_fav })))
}

pub async fn list_favorites(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    Ok(Json(db::list_favorites(&state.db, &auth.user_id)))
}

pub async fn list_favorite_projects(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<common::ProjectSummary>>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    Ok(Json(db::list_favorite_projects(&state.db, &auth.user_id)))
}

pub async fn toggle_like(
    State(state): State<Arc<AppState>>,
    Path(update_id): Path<String>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    let liked = db::toggle_like(&state.db, &auth.user_id, &update_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "liked": liked })))
}

pub async fn subscribe(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    db::set_subscription(&state.db, &auth.user_id, &project_id, true)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ApiSuccess {
        message: "Subscribed".into(),
    }))
}

pub async fn unsubscribe(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    db::set_subscription(&state.db, &auth.user_id, &project_id, false)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ApiSuccess {
        message: "Unsubscribed".into(),
    }))
}
