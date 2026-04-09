use leptos::*;

use crate::api;
use crate::state::AuthState;

#[component]
pub fn StaffPage() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();

    view! {
        <div class="staff-page">
            <h2>"Staff Operations"</h2>
            {move || {
                let user = auth.user.get();
                let is_staff = user.as_ref().map_or(false, |u|
                    matches!(u.role, common::Role::ProjectManager | common::Role::Administrator));
                if !is_staff {
                    return view! { <p class="error-msg">"Staff access required."</p> }.into_view();
                }
                view! {
                    <section class="admin-section">
                        <h3>"Create Project"</h3>
                        <CreateProjectForm />
                    </section>
                    <section class="admin-section">
                        <h3>"Post Spending Update"</h3>
                        <PostUpdateForm />
                    </section>
                    <section class="admin-section">
                        <h3>"Record Expense & Upload Receipt"</h3>
                        <RecordExpenseForm />
                    </section>
                    <section class="admin-section">
                        <h3>"Project Tickets"</h3>
                        <TicketResponsePanel />
                    </section>
                }.into_view()
            }}
        </div>
    }
}

#[component]
fn CreateProjectForm() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let (title, set_title) = create_signal(String::new());
    let (desc, set_desc) = create_signal(String::new());
    let (cause, set_cause) = create_signal("education".to_string());
    let (zip, set_zip) = create_signal(String::new());
    let (goal, set_goal) = create_signal(String::new());
    let (bl_text, set_bl_text) = create_signal(String::new());
    let (msg, set_msg) = create_signal(Option::<String>::None);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if let Some(token) = auth.get_token() {
            let goal_cents = match goal.get().parse::<f64>() {
                Ok(v) => (v * 100.0) as i64,
                Err(_) => { set_msg.set(Some("Invalid goal amount".into())); return; }
            };
            let budget_lines: Vec<common::CreateBudgetLine> = bl_text.get()
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let name = parts[0].trim().to_string();
                        let cents = parts[1].trim().parse::<f64>().ok().map(|v| (v * 100.0) as i64)?;
                        Some(common::CreateBudgetLine { name, allocated_cents: cents })
                    } else { None }
                })
                .collect();

            let req = common::CreateProjectRequest {
                title: title.get(),
                description: desc.get(),
                cause: cause.get(),
                zip_code: zip.get(),
                goal_cents,
                budget_lines,
            };
            spawn_local(async move {
                match api::create_project(&token, &req).await {
                    Ok(p) => set_msg.set(Some(format!("Project created: {}", p.id))),
                    Err(e) => set_msg.set(Some(e)),
                }
            });
        }
    };

    view! {
        {move || msg.get().map(|m| view! { <div class="info-msg">{m}</div> })}
        <form on:submit=on_submit>
            <div class="form-group">
                <label>"Title"</label>
                <input type="text" required on:input=move |ev| set_title.set(event_target_value(&ev)) prop:value=title />
            </div>
            <div class="form-group">
                <label>"Description"</label>
                <textarea required on:input=move |ev| set_desc.set(event_target_value(&ev)) prop:value=desc />
            </div>
            <div class="form-row">
                <div class="form-group">
                    <label>"Cause"</label>
                    <select on:change=move |ev| set_cause.set(event_target_value(&ev))>
                        <option value="education">"Education"</option>
                        <option value="health">"Health"</option>
                        <option value="environment">"Environment"</option>
                        <option value="housing">"Housing"</option>
                        <option value="food">"Food Security"</option>
                        <option value="youth">"Youth Programs"</option>
                        <option value="arts">"Arts & Culture"</option>
                        <option value="other">"Other"</option>
                    </select>
                </div>
                <div class="form-group">
                    <label>"ZIP Code"</label>
                    <input type="text" required on:input=move |ev| set_zip.set(event_target_value(&ev)) prop:value=zip />
                </div>
                <div class="form-group">
                    <label>"Goal ($)"</label>
                    <input type="number" step="0.01" min="1" required
                        on:input=move |ev| set_goal.set(event_target_value(&ev)) prop:value=goal />
                </div>
            </div>
            <div class="form-group">
                <label>"Budget Lines (one per line, format: Name: Amount)"</label>
                <textarea placeholder="Materials: 500.00\nLabor: 1200.00\nTransport: 300.00"
                    on:input=move |ev| set_bl_text.set(event_target_value(&ev)) prop:value=bl_text />
            </div>
            <button type="submit" class="btn btn-primary">"Create Project"</button>
        </form>
    }
}

#[component]
fn PostUpdateForm() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let (pid, set_pid) = create_signal(String::new());
    let (title, set_title) = create_signal(String::new());
    let (body, set_body) = create_signal(String::new());
    let (msg, set_msg) = create_signal(Option::<String>::None);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if let Some(token) = auth.get_token() {
            let req = common::PostUpdateRequest {
                project_id: pid.get(),
                title: title.get(),
                body: body.get(),
            };
            spawn_local(async move {
                match api::post_update(&token, &req).await {
                    Ok(r) => {
                        set_msg.set(Some(r.message));
                        set_title.set(String::new());
                        set_body.set(String::new());
                    }
                    Err(e) => set_msg.set(Some(e)),
                }
            });
        }
    };

    view! {
        {move || msg.get().map(|m| view! { <div class="info-msg">{m}</div> })}
        <form on:submit=on_submit>
            <div class="form-group">
                <label>"Project ID"</label>
                <input type="text" required on:input=move |ev| set_pid.set(event_target_value(&ev)) prop:value=pid />
            </div>
            <div class="form-group">
                <label>"Update Title"</label>
                <input type="text" required on:input=move |ev| set_title.set(event_target_value(&ev)) prop:value=title />
            </div>
            <div class="form-group">
                <label>"Update Body"</label>
                <textarea required on:input=move |ev| set_body.set(event_target_value(&ev)) prop:value=body />
            </div>
            <button type="submit" class="btn btn-primary">"Post Update"</button>
        </form>
    }
}

#[component]
fn RecordExpenseForm() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let (pid, set_pid) = create_signal(String::new());
    let (blid, set_blid) = create_signal(String::new());
    let (amount, set_amount) = create_signal(String::new());
    let (desc, set_desc) = create_signal(String::new());
    let (msg, set_msg) = create_signal(Option::<String>::None);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if let Some(token) = auth.get_token() {
            let cents = match amount.get().parse::<f64>() {
                Ok(v) => (v * 100.0) as i64,
                Err(_) => { set_msg.set(Some("Invalid amount".into())); return; }
            };
            let req = common::RecordExpenseRequest {
                project_id: pid.get(),
                budget_line_id: blid.get(),
                amount_cents: cents,
                description: desc.get(),
                receipt_data: None,
            };
            spawn_local(async move {
                match api::record_expense(&token, &req).await {
                    Ok(r) => {
                        set_msg.set(Some(r.message));
                        set_amount.set(String::new());
                        set_desc.set(String::new());
                    }
                    Err(e) => set_msg.set(Some(e)),
                }
            });
        }
    };

    view! {
        {move || msg.get().map(|m| view! { <div class="info-msg">{m}</div> })}
        <form on:submit=on_submit>
            <div class="form-row">
                <div class="form-group">
                    <label>"Project ID"</label>
                    <input type="text" required on:input=move |ev| set_pid.set(event_target_value(&ev)) prop:value=pid />
                </div>
                <div class="form-group">
                    <label>"Budget Line ID"</label>
                    <input type="text" required on:input=move |ev| set_blid.set(event_target_value(&ev)) prop:value=blid />
                </div>
            </div>
            <div class="form-row">
                <div class="form-group">
                    <label>"Amount ($)"</label>
                    <input type="number" step="0.01" min="0.01" required
                        on:input=move |ev| set_amount.set(event_target_value(&ev)) prop:value=amount />
                </div>
                <div class="form-group">
                    <label>"Description"</label>
                    <input type="text" required on:input=move |ev| set_desc.set(event_target_value(&ev)) prop:value=desc />
                </div>
            </div>
            <button type="submit" class="btn btn-primary">"Record Expense"</button>
        </form>

        <h4>"Upload Receipt / Voucher"</h4>
        <ReceiptUploadForm />
    }
}

#[component]
fn ReceiptUploadForm() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let (expense_id, set_expense_id) = create_signal(String::new());
    let (file_data, set_file_data) = create_signal(Option::<(String, String, i64, String)>::None); // (name, type, size, base64)
    let (upload_msg, set_upload_msg) = create_signal(Option::<String>::None);

    let on_file_change = move |ev: leptos::ev::Event| {
        use wasm_bindgen::JsCast;
        let input: web_sys::HtmlInputElement = ev.target().unwrap().unchecked_into();
        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                let name = file.name();
                let mime = file.type_();
                let size = file.size() as i64;

                // Validate type
                let allowed = ["application/pdf", "image/jpeg", "image/png"];
                if !allowed.contains(&mime.as_str()) {
                    set_upload_msg.set(Some(format!("Invalid file type: {}. Use PDF, JPG, or PNG.", mime)));
                    return;
                }
                // Validate size (10 MB)
                if size > 10 * 1024 * 1024 {
                    set_upload_msg.set(Some("File too large. Maximum 10 MB.".into()));
                    return;
                }

                let reader = web_sys::FileReader::new().unwrap();
                let reader_clone = reader.clone();
                let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::Event| {
                    if let Ok(result) = reader_clone.result() {
                        let data_url: String = result.as_string().unwrap_or_default();
                        // Strip data URL prefix to get base64
                        let base64 = data_url.split(',').nth(1).unwrap_or("").to_string();
                        set_file_data.set(Some((name.clone(), mime.clone(), size, base64)));
                    }
                }) as Box<dyn FnMut(_)>);
                reader.set_onload(Some(closure.as_ref().unchecked_ref()));
                closure.forget();
                let _ = reader.read_as_data_url(&file);
            }
        }
    };

    let on_upload = move |_| {
        if let (Some(token), Some((name, mime, size, b64))) = (auth.get_token(), file_data.get()) {
            let eid = expense_id.get();
            if eid.is_empty() {
                set_upload_msg.set(Some("Enter expense ID".into()));
                return;
            }
            let req = common::UploadReceiptRequest {
                expense_id: eid,
                file_name: name,
                file_type: mime,
                file_size: size,
                file_data_base64: b64,
            };
            spawn_local(async move {
                match api::upload_receipt(&token, &req).await {
                    Ok(r) => set_upload_msg.set(Some(format!("Receipt uploaded: {} (SHA256: {})", r.file_name, &r.sha256_fingerprint[..16]))),
                    Err(e) => set_upload_msg.set(Some(e)),
                }
            });
        } else {
            set_upload_msg.set(Some("Select a file first".into()));
        }
    };

    view! {
        {move || upload_msg.get().map(|m| view! { <div class="info-msg">{m}</div> })}
        <div class="form-row">
            <div class="form-group">
                <label>"Expense ID"</label>
                <input type="text" on:input=move |ev| set_expense_id.set(event_target_value(&ev)) prop:value=expense_id />
            </div>
            <div class="form-group">
                <label>"Receipt File (PDF/JPG/PNG, max 10 MB)"</label>
                <input type="file" accept=".pdf,.jpg,.jpeg,.png" on:change=on_file_change />
            </div>
            <button class="btn btn-secondary" on:click=on_upload>"Upload Receipt"</button>
        </div>
    }
}

#[component]
fn TicketResponsePanel() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let (project_id, set_project_id) = create_signal(String::new());
    let (refresh, set_refresh) = create_signal(0u32);
    let (resp_text, set_resp_text) = create_signal(String::new());
    let (msg, set_msg) = create_signal(Option::<String>::None);

    let tickets = create_resource(
        move || (auth.get_token(), project_id.get(), refresh.get()),
        |(token, pid, _)| async move {
            if pid.is_empty() { return None; }
            match token {
                Some(t) => api::list_tickets(&t, &pid).await.ok(),
                None => None,
            }
        },
    );

    let on_respond = move |ticket_id: String| {
        if let Some(token) = auth.get_token() {
            let response = resp_text.get();
            if response.is_empty() {
                set_msg.set(Some("Enter a response".into()));
                return;
            }
            spawn_local(async move {
                match api::respond_ticket(&token, &ticket_id, &response).await {
                    Ok(r) => {
                        set_msg.set(Some(r.message));
                        set_resp_text.set(String::new());
                        set_refresh.update(|c| *c += 1);
                    }
                    Err(e) => set_msg.set(Some(e)),
                }
            });
        }
    };

    view! {
        {move || msg.get().map(|m| view! { <div class="info-msg">{m}</div> })}
        <div class="form-row">
            <div class="form-group">
                <label>"Project ID"</label>
                <input type="text" placeholder="Enter project ID to load tickets"
                    on:input=move |ev| { set_project_id.set(event_target_value(&ev)); set_refresh.update(|c| *c += 1); }
                    prop:value=project_id />
            </div>
        </div>
        <div class="form-group">
            <label>"Response text"</label>
            <textarea placeholder="Type your response..."
                on:input=move |ev| set_resp_text.set(event_target_value(&ev))
                prop:value=resp_text />
        </div>
        <Suspense fallback=move || view! { <p>"Enter a project ID above..."</p> }>
            {move || tickets.get().map(|data| match data {
                Some(list) if !list.is_empty() => {
                    view! {
                        <div class="expense-review-list">
                            {list.into_iter().map(|t| {
                                let tid = t.id.clone();
                                let has_response = t.response.is_some();
                                view! {
                                    <div class="expense-card">
                                        <div class="expense-header">
                                            <strong>{t.subject.clone()}</strong>
                                            <span class="badge">{t.status.clone()}</span>
                                        </div>
                                        <p>{t.body.clone()}</p>
                                        <div class="expense-meta">
                                            <span>"From: " {t.submitter_name.clone()}</span>
                                            <span>{t.created_at.clone()}</span>
                                        </div>
                                        {t.response.as_ref().map(|r| view! {
                                            <div class="info-msg">"Response: " {r.clone()}</div>
                                        })}
                                        {(!has_response).then(|| {
                                            let on_respond = on_respond.clone();
                                            view! {
                                                <button class="btn btn-primary btn-sm"
                                                    on:click=move |_| on_respond(tid.clone())>
                                                    "Send Response"
                                                </button>
                                            }
                                        })}
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_view()
                }
                _ => view! { <p class="empty">"No tickets found."</p> }.into_view(),
            })}
        </Suspense>
    }
}
