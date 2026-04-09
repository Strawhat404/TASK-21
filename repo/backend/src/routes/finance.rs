use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;

use crate::db;
use crate::middleware::{require_auth, require_role};
use crate::AppState;
use common::*;

pub async fn pending_expenses(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<ExpenseRecord>>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    require_role(&state.db, &auth.user_id, &[Role::FinanceReviewer, Role::Administrator])?;
    Ok(Json(db::list_pending_expenses(&state.db, &state.encryption_key)))
}

pub async fn review_expense(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<ApiSuccess>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;
    let user = require_role(&state.db, &auth.user_id, &[Role::FinanceReviewer, Role::Administrator])
        .map_err(|s| (s, Json(ApiError { error: "Forbidden".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: ReviewExpenseRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    let rows = db::review_expense(
        &state.db,
        &req.expense_id,
        req.approved,
        &auth.user_id,
        req.note.as_deref(),
        &state.encryption_key,
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;
    if rows == 0 {
        return Err((StatusCode::NOT_FOUND, Json(ApiError {
            error: "Expense not found".into(),
        })));
    }

    let action = if req.approved { "approved" } else { "rejected" };
    db::append_ops_log(
        &state.db,
        &auth.user_id,
        &user.display_name,
        "review_expense",
        &format!("Expense {} {}", req.expense_id, action),
    );

    Ok(Json(ApiSuccess {
        message: format!("Expense {}", action),
    }))
}
