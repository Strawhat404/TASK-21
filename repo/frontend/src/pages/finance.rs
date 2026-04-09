use leptos::*;

use crate::api;
use crate::state::AuthState;

#[component]
pub fn FinancePage() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let (refresh_counter, set_refresh) = create_signal(0u32);

    let expenses = create_resource(
        move || (auth.get_token(), refresh_counter.get()),
        |(token, _)| async move {
            match token {
                Some(t) => api::pending_expenses(&t).await.ok(),
                None => None,
            }
        },
    );

    let (review_note, set_review_note) = create_signal(String::new());
    let (review_msg, set_review_msg) = create_signal(Option::<String>::None);

    let do_review = move |expense_id: String, approved: bool| {
        if let Some(token) = auth.get_token() {
            let note = review_note.get();
            let note_opt = if note.is_empty() { None } else { Some(note) };
            spawn_local(async move {
                match api::review_expense(&token, &expense_id, approved, note_opt.as_deref()).await {
                    Ok(r) => {
                        set_review_msg.set(Some(r.message));
                        set_refresh.update(|c| *c += 1);
                    }
                    Err(e) => set_review_msg.set(Some(e)),
                }
            });
        }
    };

    view! {
        <div class="finance-page">
            <h2>"Finance Review"</h2>
            <p>"Verify receipts and approve or reject expense disclosures."</p>

            {move || review_msg.get().map(|m| view! { <div class="info-msg">{m}</div> })}

            <div class="form-group">
                <label>"Review Note (optional)"</label>
                <textarea placeholder="Add a note for the review..."
                    on:input=move |ev| set_review_note.set(event_target_value(&ev))
                    prop:value=review_note />
            </div>

            <Suspense fallback=move || view! { <p>"Loading pending expenses..."</p> }>
                {move || expenses.get().map(|data| match data {
                    Some(list) if !list.is_empty() => {
                        view! {
                            <div class="expense-review-list">
                                {list.into_iter().map(|exp| {
                                    let eid_approve = exp.id.clone();
                                    let eid_reject = exp.id.clone();
                                    view! {
                                        <div class="expense-card">
                                            <div class="expense-header">
                                                <strong>{&exp.budget_line_name}</strong>
                                                <span class="amount">
                                                    {format!("${:.2}", exp.amount_cents as f64 / 100.0)}
                                                </span>
                                            </div>
                                            <p>{&exp.description}</p>
                                            <div class="expense-meta">
                                                <span>"Project: " {&exp.project_id}</span>
                                                <span>"Date: " {&exp.created_at}</span>
                                                {exp.receipt_url.as_ref().map(|_| view! {
                                                    <span class="receipt-badge">"Has Receipt"</span>
                                                })}
                                            </div>
                                            <div class="review-actions">
                                                <button class="btn btn-success"
                                                    on:click={
                                                        let do_review = do_review.clone();
                                                        move |_| do_review(eid_approve.clone(), true)
                                                    }>
                                                    "Approve"
                                                </button>
                                                <button class="btn btn-danger"
                                                    on:click={
                                                        let do_review = do_review.clone();
                                                        move |_| do_review(eid_reject.clone(), false)
                                                    }>
                                                    "Reject"
                                                </button>
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_view()
                    }
                    Some(_) => view! {
                        <p class="empty">"No pending expenses to review."</p>
                    }.into_view(),
                    None => view! {
                        <p class="error-msg">"Unable to load. Finance reviewer access required."</p>
                    }.into_view(),
                })}
            </Suspense>

            <section class="admin-section">
                <h3>"Pending Receipt Verifications"</h3>
                <PendingReceipts />
            </section>

            <section class="admin-section">
                <h3>"Pending Refund Approvals"</h3>
                <PendingRefunds />
            </section>
        </div>
    }
}

#[component]
fn PendingReceipts() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let (refresh, set_refresh) = create_signal(0u32);
    let (rej_reason, set_rej_reason) = create_signal(String::new());
    let (msg, set_msg) = create_signal(Option::<String>::None);

    let receipts = create_resource(
        move || (auth.get_token(), refresh.get()),
        |(token, _)| async move {
            match token {
                Some(t) => api::pending_receipts(&t).await.ok(),
                None => None,
            }
        },
    );

    let do_review = move |receipt_id: String, verified: bool| {
        if let Some(token) = auth.get_token() {
            let reason = if verified { None } else { Some(rej_reason.get()) };
            spawn_local(async move {
                match api::review_receipt(&token, &receipt_id, verified, reason.as_deref()).await {
                    Ok(r) => {
                        set_msg.set(Some(r.message));
                        set_refresh.update(|c| *c += 1);
                    }
                    Err(e) => set_msg.set(Some(e)),
                }
            });
        }
    };

    view! {
        {move || msg.get().map(|m| view! { <div class="info-msg">{m}</div> })}
        <div class="form-group">
            <label>"Rejection Reason (required when rejecting)"</label>
            <input type="text" placeholder="Reason for rejection..."
                on:input=move |ev| set_rej_reason.set(event_target_value(&ev))
                prop:value=rej_reason />
        </div>
        <Suspense fallback=move || view! { <p>"Loading..."</p> }>
            {move || receipts.get().map(|data| match data {
                Some(list) if !list.is_empty() => {
                    view! {
                        <div class="expense-review-list">
                            {list.into_iter().map(|r| {
                                let rid_v = r.id.clone();
                                let rid_r = r.id.clone();
                                view! {
                                    <div class="expense-card">
                                        <div class="expense-header">
                                            <strong>{r.file_name.clone()}</strong>
                                            <span>{format!("{} ({} bytes)", r.file_type, r.file_size)}</span>
                                        </div>
                                        <div class="expense-meta">
                                            <span>"Expense: " {r.expense_id.clone()}</span>
                                            <span class="mono">"SHA256: " {r.sha256_fingerprint[..16].to_string()} "..."</span>
                                        </div>
                                        <div class="review-actions">
                                            <button class="btn btn-success"
                                                on:click={let do_review = do_review.clone(); move |_| do_review(rid_v.clone(), true)}>
                                                "Verify"
                                            </button>
                                            <button class="btn btn-danger"
                                                on:click={let do_review = do_review.clone(); move |_| do_review(rid_r.clone(), false)}>
                                                "Reject"
                                            </button>
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_view()
                }
                _ => view! { <p class="empty">"No pending receipts."</p> }.into_view(),
            })}
        </Suspense>
    }
}

#[component]
fn PendingRefunds() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let (refresh, set_refresh) = create_signal(0u32);
    let (pw, set_pw) = create_signal(String::new());
    let (msg, set_msg) = create_signal(Option::<String>::None);

    let refunds = create_resource(
        move || (auth.get_token(), refresh.get()),
        |(token, _)| async move {
            match token {
                Some(t) => api::pending_refunds(&t).await.ok(),
                None => None,
            }
        },
    );

    // Two-step confirmation state: stores (donation_id, approved) pending confirmation
    let (confirm_action, set_confirm_action) = create_signal(Option::<(String, bool)>::None);

    let do_approve = move |donation_id: String, approved: bool| {
        // Step 1: require user to click first to enter confirmation state
        let current = confirm_action.get();
        if current.as_ref().map_or(true, |c| c.0 != donation_id || c.1 != approved) {
            set_confirm_action.set(Some((donation_id, approved)));
            set_msg.set(Some("Please re-enter your password and click again to confirm.".into()));
            return;
        }

        // Step 2: actually submit with password
        if let Some(token) = auth.get_token() {
            let password = pw.get();
            if password.is_empty() {
                set_msg.set(Some("Password is required to confirm this action".into()));
                return;
            }
            set_confirm_action.set(None);
            spawn_local(async move {
                match api::approve_refund(&token, &donation_id, approved, &password).await {
                    Ok(r) => {
                        set_msg.set(Some(r.message));
                        set_refresh.update(|c| *c += 1);
                    }
                    Err(e) => set_msg.set(Some(e)),
                }
            });
        }
    };

    view! {
        {move || msg.get().map(|m| view! { <div class="info-msg">{m}</div> })}
        {move || confirm_action.get().map(|(ref id, approved)| {
            let action = if approved { "approve" } else { "reject" };
            view! {
                <div class="confirm-box">
                    <p class="warning">{format!("You are about to {} refund {}. Enter your password and click the button again to confirm.", action, id)}</p>
                </div>
            }
        })}
        <div class="form-group">
            <label>"Password (required for refund approval)"</label>
            <input type="password" placeholder="Re-enter password"
                on:input=move |ev| set_pw.set(event_target_value(&ev))
                prop:value=pw />
        </div>
        <Suspense fallback=move || view! { <p>"Loading..."</p> }>
            {move || refunds.get().map(|data| match data {
                Some(list) if !list.is_empty() => {
                    view! {
                        <div class="expense-review-list">
                            {list.into_iter().map(|d| {
                                let did_a = d.id.clone();
                                let did_r = d.id.clone();
                                let is_confirming_approve = {
                                    let did = d.id.clone();
                                    move || confirm_action.get().map_or(false, |c| c.0 == did && c.1)
                                };
                                let is_confirming_reject = {
                                    let did = d.id.clone();
                                    move || confirm_action.get().map_or(false, |c| c.0 == did && !c.1)
                                };
                                view! {
                                    <div class="expense-card">
                                        <div class="expense-header">
                                            <strong>{d.pledge_number.clone()}</strong>
                                            <span class="amount">{format!("${:.2}", d.amount_cents as f64 / 100.0)}</span>
                                        </div>
                                        <div class="expense-meta">
                                            <span>"Project: " {d.project_title.clone()}</span>
                                            <span>"Method: " {d.payment_method.clone()}</span>
                                            <span>"Original: " {d.reversal_of.clone().unwrap_or_default()}</span>
                                        </div>
                                        <div class="review-actions">
                                            <button class="btn btn-success"
                                                on:click={let f = do_approve.clone(); move |_| f(did_a.clone(), true)}>
                                                {move || if is_confirming_approve() { "Confirm Approve" } else { "Approve Refund" }}
                                            </button>
                                            <button class="btn btn-danger"
                                                on:click={let f = do_approve.clone(); move |_| f(did_r.clone(), false)}>
                                                {move || if is_confirming_reject() { "Confirm Reject" } else { "Reject" }}
                                            </button>
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_view()
                }
                _ => view! { <p class="empty">"No pending refunds."</p> }.into_view(),
            })}
        </Suspense>
    }
}
