use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;

use crate::db;
use crate::middleware::{require_auth, require_role};
use crate::AppState;
use common::*;

pub async fn donate(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<DonateResponse>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: DonateRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    if req.amount_cents <= 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError { error: "Donation amount must be positive".into() }),
        ));
    }

    // Verify project exists and is active
    let project = db::get_project_detail(&state.db, &req.project_id).ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(ApiError { error: "Project not found".into() }))
    })?;
    if project.status != ProjectStatus::Active {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError { error: "Project is not accepting donations".into() }),
        ));
    }

    let payment_method = req.payment_method.as_deref().unwrap_or("cash");
    // Validate payment method
    if PaymentMethod::from_str(payment_method).is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError { error: "Invalid payment method. Use: cash, check, card_terminal".into() }),
        ));
    }

    // Validate budget line belongs to this project if specified
    if let Some(ref bl_id) = req.budget_line_id {
        let bl_project = db::get_budget_line_project_id(&state.db, bl_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, Json(ApiError { error: "Budget line not found".into() })))?;
        if bl_project != req.project_id {
            return Err((StatusCode::BAD_REQUEST, Json(ApiError {
                error: "Budget line does not belong to this project".into(),
            })));
        }
    }

    let donation_id = uuid::Uuid::new_v4().to_string();
    let pledge_number = format!("PLG-{}", &donation_id[..8].to_uppercase());

    db::create_donation(
        &state.db,
        &donation_id,
        &pledge_number,
        &req.project_id,
        &auth.user_id,
        req.amount_cents,
        payment_method,
        false,
        None,
        req.budget_line_id.as_deref(),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    let donation = db::get_donation(&state.db, &donation_id).unwrap();

    // Auto-subscribe donor to project notifications
    let _ = db::set_subscription(&state.db, &auth.user_id, &req.project_id, true);

    db::append_ops_log(
        &state.db,
        &auth.user_id,
        "donor",
        "donate",
        &format!("Donated {} cents ({}) to project {}", req.amount_cents, payment_method, req.project_id),
    );

    // Fire webhooks
    crate::routes::webhooks::fire_webhooks(
        &state,
        "donation.created",
        &serde_json::json!({
            "donation_id": donation_id,
            "project_id": req.project_id,
            "amount_cents": req.amount_cents,
            "payment_method": payment_method,
        }),
    )
    .await;

    Ok(Json(DonateResponse { donation }))
}

/// Request a refund (creates a negative reversal record pending Finance Reviewer approval).
pub async fn request_refund(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<DonateResponse>, (StatusCode, Json<ApiError>)> {
    let auth = require_auth(request.extensions())
        .map_err(|s| (s, Json(ApiError { error: "Unauthorized".into() })))?;

    let body = axum::body::to_bytes(request.into_body(), 1_048_576)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ApiError { error: "Invalid body".into() })))?;
    let req: RefundRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    // Get original donation
    let original = db::get_donation(&state.db, &req.donation_id).ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(ApiError { error: "Original donation not found".into() }))
    })?;

    // Only the original donor may request a refund
    if original.donor_id != auth.user_id {
        return Err((StatusCode::FORBIDDEN, Json(ApiError {
            error: "Only the original donor can request a refund for this donation".into(),
        })));
    }

    let reversal_id = uuid::Uuid::new_v4().to_string();
    let pledge_number = format!("REF-{}", &reversal_id[..8].to_uppercase());

    db::create_donation(
        &state.db,
        &reversal_id,
        &pledge_number,
        &original.project_id,
        &original.donor_id,
        -original.amount_cents, // Negative amount
        &original.payment_method,
        true,
        Some(&req.donation_id),
        original.budget_line_id.as_deref(),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;

    let reversal = db::get_donation(&state.db, &reversal_id).unwrap();

    db::append_ops_log(
        &state.db,
        &auth.user_id,
        "user",
        "request_refund",
        &format!("Requested refund for donation {} (reason: {})", req.donation_id, req.reason),
    );

    Ok(Json(DonateResponse { donation: reversal }))
}

/// Approve or reject a pending refund (Finance Reviewer only, requires password).
pub async fn approve_refund(
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
    let req: ApproveRefundRequest = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })))?;

    // Verify password (sensitive action)
    let hash = db::get_user_password_hash(&state.db, &auth.user_id)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ApiError { error: "User not found".into() })))?;
    if !crate::auth::verify_password(&req.password, &hash) {
        return Err((StatusCode::UNAUTHORIZED, Json(ApiError { error: "Invalid password".into() })));
    }

    // Server-side two-step confirmation protocol
    let action_name = if req.approved { "approve_refund" } else { "reject_refund" };
    match req.confirmation_token {
        None => {
            // Step 1: issue a confirmation token
            let token = db::create_confirmation_token(&state.db, &auth.user_id, action_name, &req.donation_id)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;
            return Err((StatusCode::from_u16(428).unwrap(), Json(ApiError {
                error: format!("{{\"confirmation_token\":\"{}\",\"message\":\"Confirmation required. Resubmit with this token to execute.\"}}", token),
            })));
        }
        Some(ref token) => {
            if !db::consume_confirmation_token(&state.db, token, &auth.user_id, action_name, &req.donation_id) {
                return Err((StatusCode::CONFLICT, Json(ApiError {
                    error: "Invalid, expired, or already-used confirmation token".into(),
                })));
            }
        }
    }

    let rows = db::approve_reversal(&state.db, &req.donation_id, req.approved)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })))?;
    if rows == 0 {
        return Err((StatusCode::NOT_FOUND, Json(ApiError {
            error: "Refund not found or not a pending reversal".into(),
        })));
    }

    let action = if req.approved { "approved" } else { "rejected" };
    db::append_ops_log(&state.db, &auth.user_id, &user.display_name, "approve_refund",
        &format!("Refund {} {}", req.donation_id, action));

    Ok(Json(ApiSuccess { message: format!("Refund {}", action) }))
}

/// List pending refund reversals.
pub async fn pending_refunds(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<DonationRecord>>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    require_role(&state.db, &auth.user_id, &[Role::FinanceReviewer, Role::Administrator])?;
    Ok(Json(db::list_pending_reversals(&state.db)))
}

pub async fn my_donations(
    State(state): State<Arc<AppState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Result<Json<Vec<DonationRecord>>, StatusCode> {
    let auth = require_auth(request.extensions())?;
    Ok(Json(db::list_user_donations(&state.db, &auth.user_id)))
}
