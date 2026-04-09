use leptos::*;
use leptos_router::*;

use crate::api;
use crate::state::AuthState;

#[component]
pub fn FulfillmentPage() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let params = use_params_map();
    let project_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let (refresh, set_refresh) = create_signal(0u32);
    let fulfillments = create_resource(
        move || (auth.get_token(), project_id(), refresh.get()),
        |(token, pid, _)| async move {
            match token {
                Some(t) => api::list_fulfillments(&t, &pid).await.ok(),
                None => None,
            }
        },
    );

    let (msg, set_msg) = create_signal(Option::<String>::None);

    // Create new fulfillment
    let on_create = move |_| {
        if let Some(token) = auth.get_token() {
            let pid = project_id();
            spawn_local(async move {
                match api::create_fulfillment(&token, &pid).await {
                    Ok(_) => {
                        set_msg.set(Some("Fulfillment record created".into()));
                        set_refresh.update(|c| *c += 1);
                    }
                    Err(e) => set_msg.set(Some(e)),
                }
            });
        }
    };

    // Generate checkpoint code
    let (gen_fid, set_gen_fid) = create_signal(String::new());
    let (gen_cp, set_gen_cp) = create_signal("arrival".to_string());
    let (code_result, set_code_result) = create_signal(Option::<String>::None);
    let (qr_svg, set_qr_svg) = create_signal(Option::<String>::None);

    let on_generate = move |_| {
        if let Some(token) = auth.get_token() {
            let fid = gen_fid.get();
            let cp = gen_cp.get();
            spawn_local(async move {
                match api::generate_checkpoint_code(&token, &fid, &cp).await {
                    Ok(resp) => {
                        set_code_result.set(Some(format!("Code: {} (expires: {})", resp.code, resp.expires_at)));
                        set_qr_svg.set(resp.qr_code_svg);
                    }
                    Err(e) => {
                        set_code_result.set(Some(e));
                        set_qr_svg.set(None);
                    }
                }
            });
        }
    };

    // Record checkpoint
    let (rec_fid, set_rec_fid) = create_signal(String::new());
    let (rec_cp, set_rec_cp) = create_signal("arrival".to_string());
    let (rec_code, set_rec_code) = create_signal(String::new());
    let (rec_msg, set_rec_msg) = create_signal(Option::<String>::None);

    let on_record = move |_| {
        if let Some(token) = auth.get_token() {
            let fid = rec_fid.get();
            let cp = rec_cp.get();
            let code = rec_code.get();
            spawn_local(async move {
                match api::record_checkpoint(&token, &fid, &cp, &code).await {
                    Ok(r) => {
                        set_rec_msg.set(Some(r.message));
                        set_refresh.update(|c| *c += 1);
                    }
                    Err(e) => set_rec_msg.set(Some(e)),
                }
            });
        }
    };

    view! {
        <div class="fulfillment-page">
            <h2>"Fulfillment Verification"</h2>
            {move || msg.get().map(|m| view! { <div class="info-msg">{m}</div> })}

            <button class="btn btn-primary" on:click=on_create>"Create New Fulfillment Record"</button>

            <section class="admin-section">
                <h3>"Generate Checkpoint Code (OTP/QR)"</h3>
                {move || code_result.get().map(|m| view! { <div class="info-msg">{m}</div> })}
                <div class="form-row">
                    <div class="form-group">
                        <label>"Fulfillment ID"</label>
                        <input type="text" on:input=move |ev| set_gen_fid.set(event_target_value(&ev)) prop:value=gen_fid />
                    </div>
                    <div class="form-group">
                        <label>"Checkpoint"</label>
                        <select on:change=move |ev| set_gen_cp.set(event_target_value(&ev))>
                            <option value="arrival">"Arrival"</option>
                            <option value="start">"Start"</option>
                            <option value="end">"End"</option>
                        </select>
                    </div>
                    <button class="btn btn-secondary" on:click=on_generate>"Generate Code"</button>
                </div>
                {move || qr_svg.get().map(|svg| view! {
                    <div class="qr-code-display" inner_html=svg></div>
                })}
            </section>

            <section class="admin-section">
                <h3>"Record Checkpoint"</h3>
                {move || rec_msg.get().map(|m| view! { <div class="info-msg">{m}</div> })}
                <div class="form-row">
                    <div class="form-group">
                        <label>"Fulfillment ID"</label>
                        <input type="text" on:input=move |ev| set_rec_fid.set(event_target_value(&ev)) prop:value=rec_fid />
                    </div>
                    <div class="form-group">
                        <label>"Checkpoint"</label>
                        <select on:change=move |ev| set_rec_cp.set(event_target_value(&ev))>
                            <option value="arrival">"Arrival"</option>
                            <option value="start">"Start"</option>
                            <option value="end">"End"</option>
                        </select>
                    </div>
                    <div class="form-group">
                        <label>"Code"</label>
                        <input type="text" placeholder="6-digit code"
                            on:input=move |ev| set_rec_code.set(event_target_value(&ev)) prop:value=rec_code />
                    </div>
                    <button class="btn btn-primary" on:click=on_record>"Record"</button>
                </div>
            </section>

            <section class="admin-section">
                <h3>"Fulfillment Records"</h3>
                <Suspense fallback=move || view! { <p>"Loading..."</p> }>
                    {move || fulfillments.get().map(|data| match data {
                        Some(list) if !list.is_empty() => {
                            view! {
                                <table class="data-table">
                                    <thead>
                                        <tr>
                                            <th>"ID"</th>
                                            <th>"Arrival"</th>
                                            <th>"Start"</th>
                                            <th>"End"</th>
                                            <th>"Status"</th>
                                            <th>"Proof"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {list.into_iter().map(|f| {
                                            let fid = f.id.clone();
                                            let id_short = f.id[..8].to_string();
                                            let arrival = f.arrival_at.clone().unwrap_or_else(|| "-".into());
                                            let start = f.start_at.clone().unwrap_or_else(|| "-".into());
                                            let end = f.end_at.clone().unwrap_or_else(|| "-".into());
                                            let complete = f.is_complete;
                                            view! {
                                                <tr>
                                                    <td class="mono">{id_short}</td>
                                                    <td>{arrival}</td>
                                                    <td>{start}</td>
                                                    <td>{end}</td>
                                                    <td>{if complete { "Complete" } else { "In Progress" }}</td>
                                                    <td>{complete.then(|| view! {
                                                        <a href=format!("/fulfillment/{}/proof", fid) class="btn btn-sm btn-secondary">"Download"</a>
                                                    })}</td>
                                                </tr>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </tbody>
                                </table>
                            }.into_view()
                        }
                        _ => view! { <p class="empty">"No fulfillment records yet."</p> }.into_view(),
                    })}
                </Suspense>
            </section>
        </div>
    }
}

#[component]
pub fn ServiceProofPage() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let params = use_params_map();
    let fid = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let proof = create_resource(
        move || (auth.get_token(), fid()),
        |(token, id)| async move {
            match token {
                Some(t) => api::service_proof(&t, &id).await.ok(),
                None => None,
            }
        },
    );

    let on_print = |_| {
        if let Some(w) = web_sys::window() { let _ = w.print(); }
    };

    view! {
        <div class="service-proof-page">
            <Suspense fallback=move || view! { <p>"Loading proof..."</p> }>
                {move || proof.get().map(|data| match data {
                    Some(p) => view! {
                        <div class="receipt">
                            <div class="receipt-header">
                                <h2>"Service Completion Proof"</h2>
                                <p class="receipt-subtitle">"Tamper-evident fulfillment record"</p>
                            </div>
                            <div class="receipt-body">
                                <div class="receipt-row">
                                    <span class="receipt-label">"Project:"</span>
                                    <span class="receipt-value">{&p.project_title}</span>
                                </div>
                                <div class="receipt-row">
                                    <span class="receipt-label">"Arrival:"</span>
                                    <span class="receipt-value">{&p.arrival_at}</span>
                                </div>
                                <div class="receipt-row">
                                    <span class="receipt-label">"Start:"</span>
                                    <span class="receipt-value">{&p.start_at}</span>
                                </div>
                                <div class="receipt-row">
                                    <span class="receipt-label">"End:"</span>
                                    <span class="receipt-value">{&p.end_at}</span>
                                </div>
                                <div class="receipt-row">
                                    <span class="receipt-label">"Record Hash:"</span>
                                    <span class="receipt-value mono">{&p.service_record_hash}</span>
                                </div>
                                <div class="receipt-row">
                                    <span class="receipt-label">"Generated:"</span>
                                    <span class="receipt-value">{&p.generated_at}</span>
                                </div>
                            </div>
                            <div class="receipt-footer">
                                <button class="btn btn-secondary" on:click=on_print>"Print Proof"</button>
                            </div>
                        </div>
                    }.into_view(),
                    None => view! { <p>"Proof not available."</p> }.into_view(),
                })}
            </Suspense>
        </div>
    }
}
