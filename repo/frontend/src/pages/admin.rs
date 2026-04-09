use leptos::*;

use crate::api;
use crate::state::AuthState;

#[component]
pub fn AdminPage() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();

    // Ops log
    let (log_page, set_log_page) = create_signal(1i64);
    let ops_log = create_resource(
        move || (auth.get_token(), log_page.get()),
        |(token, page)| async move {
            match token {
                Some(t) => api::ops_log(&t, page).await.ok(),
                None => None,
            }
        },
    );

    // Unpublish project
    let (unpub_id, set_unpub_id) = create_signal(String::new());
    let (unpub_pw, set_unpub_pw) = create_signal(String::new());
    let (unpub_msg, set_unpub_msg) = create_signal(Option::<String>::None);
    let (confirm_step, set_confirm_step) = create_signal(false);

    let do_unpublish = move |_| {
        if !confirm_step.get() {
            set_confirm_step.set(true);
            return;
        }
        if let Some(token) = auth.get_token() {
            let id = unpub_id.get();
            let pw = unpub_pw.get();
            spawn_local(async move {
                match api::unpublish_project(&token, &id, &pw).await {
                    Ok(r) => {
                        set_unpub_msg.set(Some(r.message));
                        set_confirm_step.set(false);
                        set_unpub_pw.set(String::new());
                    }
                    Err(e) => {
                        set_unpub_msg.set(Some(e));
                        set_confirm_step.set(false);
                    }
                }
            });
        }
    };

    view! {
        <div class="admin-page">
            <h2>"Administration Panel"</h2>

            <section class="admin-section">
                <h3>"Sensitive Actions"</h3>

                <div class="sensitive-action-card">
                    <h4>"Unpublish Project"</h4>
                    {move || unpub_msg.get().map(|m| view! { <div class="info-msg">{m}</div> })}
                    <div class="form-row">
                        <input type="text" placeholder="Project ID"
                            on:input=move |ev| set_unpub_id.set(event_target_value(&ev))
                            prop:value=unpub_id />
                    </div>
                    {move || confirm_step.get().then(|| view! {
                        <div class="confirm-box">
                            <p class="warning">"This is a sensitive action. Please re-enter your password to confirm."</p>
                            <input type="password" placeholder="Re-enter password"
                                on:input=move |ev| set_unpub_pw.set(event_target_value(&ev))
                                prop:value=unpub_pw />
                        </div>
                    })}
                    <button class="btn btn-danger" on:click=do_unpublish>
                        {move || if confirm_step.get() { "Confirm Unpublish" } else { "Unpublish" }}
                    </button>
                </div>
            </section>

            <section class="admin-section">
                <h3>"Operations Log (Immutable Audit Trail)"</h3>
                <Suspense fallback=move || view! { <p>"Loading log..."</p> }>
                    {move || ops_log.get().map(|data| match data {
                        Some(entries) => view! {
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th>"Time"</th>
                                        <th>"Actor"</th>
                                        <th>"Action"</th>
                                        <th>"Detail"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {entries.into_iter().map(|e| view! {
                                        <tr>
                                            <td>{&e.created_at}</td>
                                            <td>{&e.actor_name}</td>
                                            <td><span class="badge">{&e.action}</span></td>
                                            <td>{&e.detail}</td>
                                        </tr>
                                    }).collect::<Vec<_>>()}
                                </tbody>
                            </table>
                            <div class="pagination">
                                <button class="btn btn-sm"
                                    disabled=move || log_page.get() <= 1
                                    on:click=move |_| set_log_page.update(|p| *p -= 1)>
                                    "Previous"
                                </button>
                                <span class="page-info">{move || format!("Page {}", log_page.get())}</span>
                                <button class="btn btn-sm"
                                    on:click=move |_| set_log_page.update(|p| *p += 1)>
                                    "Next"
                                </button>
                            </div>
                        }.into_view(),
                        None => view! {
                            <p class="error-msg">"Unable to load ops log. Admin access required."</p>
                        }.into_view(),
                    })}
                </Suspense>
            </section>

            <section class="admin-section">
                <h3>"Content Moderation Settings"</h3>
                <ModerationConfigPanel />
            </section>

            <section class="admin-section">
                <h3>"Webhook Management"</h3>
                <WebhookPanel />
            </section>

            <section class="admin-section">
                <h3>"Event Data Quality"</h3>
                <DataQualityPanel />
            </section>

            <section class="admin-section">
                <h3>"Suspicious Event Review"</h3>
                <SuspiciousEventsPanel />
            </section>
        </div>
    }
}

#[component]
fn ModerationConfigPanel() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let (msg, set_msg) = create_signal(Option::<String>::None);
    let (comments_enabled, set_ce) = create_signal(true);
    let (pre_mod, set_pm) = create_signal(false);
    let (words, set_words) = create_signal(String::new());

    // Load config on mount
    create_effect(move |_| {
        if let Some(token) = auth.get_token() {
            spawn_local(async move {
                if let Ok(c) = api::get_moderation_config(&token).await {
                    set_ce.set(c.comments_enabled);
                    set_pm.set(c.require_pre_moderation);
                    set_words.set(c.sensitive_words.join(", "));
                }
            });
        }
    });

    let on_save = move |_| {
        if let Some(token) = auth.get_token() {
            let config = common::ModerationConfig {
                comments_enabled: comments_enabled.get(),
                require_pre_moderation: pre_mod.get(),
                sensitive_words: words.get().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
            };
            spawn_local(async move {
                match api::update_moderation_config(&token, &config).await {
                    Ok(_) => set_msg.set(Some("Config saved".into())),
                    Err(e) => set_msg.set(Some(e)),
                }
            });
        }
    };

    view! {
        {move || msg.get().map(|m| view! { <div class="info-msg">{m}</div> })}
        <div class="sensitive-action-card">
            <div class="form-group">
                <label>
                    <input type="checkbox" prop:checked=comments_enabled
                        on:change=move |ev| set_ce.set(event_target_checked(&ev)) />
                    " Comments Enabled"
                </label>
            </div>
            <div class="form-group">
                <label>
                    <input type="checkbox" prop:checked=pre_mod
                        on:change=move |ev| set_pm.set(event_target_checked(&ev)) />
                    " Require Pre-Moderation"
                </label>
            </div>
            <div class="form-group">
                <label>"Sensitive Words (comma-separated)"</label>
                <textarea on:input=move |ev| set_words.set(event_target_value(&ev))
                    prop:value=words placeholder="word1, word2, word3" />
            </div>
            <button class="btn btn-primary" on:click=on_save>"Save Config"</button>
        </div>
    }
}

fn event_target_checked(ev: &leptos::ev::Event) -> bool {
    use wasm_bindgen::JsCast;
    ev.target().unwrap().unchecked_into::<web_sys::HtmlInputElement>().checked()
}

#[component]
fn WebhookPanel() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let (refresh, set_refresh) = create_signal(0u32);
    let (msg, set_msg) = create_signal(Option::<String>::None);
    let (wh_name, set_wh_name) = create_signal(String::new());
    let (wh_url, set_wh_url) = create_signal(String::new());
    let (wh_events, set_wh_events) = create_signal(String::new());

    let hooks = create_resource(
        move || (auth.get_token(), refresh.get()),
        |(token, _)| async move {
            match token {
                Some(t) => api::list_webhooks(&t).await.ok(),
                None => None,
            }
        },
    );

    let on_create = move |_| {
        if let Some(token) = auth.get_token() {
            let req = common::CreateWebhookRequest {
                name: wh_name.get(),
                url: wh_url.get(),
                event_types: wh_events.get().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
            };
            spawn_local(async move {
                match api::create_webhook(&token, &req).await {
                    Ok(_) => {
                        set_msg.set(Some("Webhook created".into()));
                        set_refresh.update(|c| *c += 1);
                    }
                    Err(e) => set_msg.set(Some(e)),
                }
            });
        }
    };

    let on_delete = move |wid: String| {
        if let Some(token) = auth.get_token() {
            spawn_local(async move {
                let _ = api::delete_webhook(&token, &wid).await;
                set_refresh.update(|c| *c += 1);
            });
        }
    };

    view! {
        {move || msg.get().map(|m| view! { <div class="info-msg">{m}</div> })}
        <div class="form-row">
            <div class="form-group">
                <label>"Name"</label>
                <input type="text" on:input=move |ev| set_wh_name.set(event_target_value(&ev)) prop:value=wh_name />
            </div>
            <div class="form-group">
                <label>"URL (local network only)"</label>
                <input type="text" placeholder="http://192.168.1.100/hook"
                    on:input=move |ev| set_wh_url.set(event_target_value(&ev)) prop:value=wh_url />
            </div>
            <div class="form-group">
                <label>"Event Types (comma-sep or *)"</label>
                <input type="text" placeholder="donation.created, *"
                    on:input=move |ev| set_wh_events.set(event_target_value(&ev)) prop:value=wh_events />
            </div>
            <button class="btn btn-primary" on:click=on_create>"Create"</button>
        </div>
        <Suspense fallback=move || view! { <p>"Loading..."</p> }>
            {move || hooks.get().map(|data| match data {
                Some(list) if !list.is_empty() => {
                    view! {
                        <table class="data-table">
                            <thead><tr><th>"Name"</th><th>"URL"</th><th>"Events"</th><th>"Actions"</th></tr></thead>
                            <tbody>
                                {list.into_iter().map(|h| {
                                    let hid = h.id.clone();
                                    view! {
                                        <tr>
                                            <td>{h.name.clone()}</td>
                                            <td class="mono">{h.url.clone()}</td>
                                            <td>{h.event_types.join(", ")}</td>
                                            <td>
                                                <button class="btn btn-sm btn-danger"
                                                    on:click={let f = on_delete.clone(); move |_| f(hid.clone())}>
                                                    "Delete"
                                                </button>
                                            </td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>()}
                            </tbody>
                        </table>
                    }.into_view()
                }
                _ => view! { <p class="empty">"No webhooks configured."</p> }.into_view(),
            })}
        </Suspense>
    }
}

#[component]
fn DataQualityPanel() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();

    let metrics = create_resource(
        move || auth.get_token(),
        |token| async move {
            match token {
                Some(t) => api::data_quality_metrics(&t).await.ok(),
                None => None,
            }
        },
    );

    view! {
        <Suspense fallback=move || view! { <p>"Loading..."</p> }>
            {move || metrics.get().map(|data| match data {
                Some(m) => view! {
                    <div class="stats-grid">
                        <div class="stat-card">
                            <div class="stat-value">{m.total_events.to_string()}</div>
                            <div class="stat-label">"Total Events"</div>
                        </div>
                        <div class="stat-card">
                            <div class="stat-value">{m.duplicate_events.to_string()}</div>
                            <div class="stat-label">{format!("Duplicates ({:.1}%)", m.duplicate_rate * 100.0)}</div>
                        </div>
                        <div class="stat-card">
                            <div class="stat-value">{m.suspicious_events.to_string()}</div>
                            <div class="stat-label">{format!("Suspicious ({:.1}%)", m.suspicious_rate * 100.0)}</div>
                        </div>
                    </div>
                    <h4>"Events by Kind"</h4>
                    <table class="data-table">
                        <thead><tr><th>"Kind"</th><th>"Count"</th></tr></thead>
                        <tbody>
                            {m.events_by_kind.into_iter().map(|(kind, count)| view! {
                                <tr><td>{kind}</td><td>{count.to_string()}</td></tr>
                            }).collect::<Vec<_>>()}
                        </tbody>
                    </table>
                }.into_view(),
                None => view! { <p class="error-msg">"Unable to load metrics."</p> }.into_view(),
            })}
        </Suspense>
    }
}

#[component]
fn SuspiciousEventsPanel() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();

    let events = create_resource(
        move || auth.get_token(),
        |token| async move {
            match token {
                Some(t) => api::suspicious_events(&t).await.ok(),
                None => None,
            }
        },
    );

    view! {
        <Suspense fallback=move || view! { <p>"Loading..."</p> }>
            {move || events.get().map(|data| match data {
                Some(list) if !list.is_empty() => {
                    view! {
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>"Time"</th>
                                    <th>"Session"</th>
                                    <th>"Kind"</th>
                                    <th>"Target"</th>
                                    <th>"User"</th>
                                    <th>"Flags"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {list.into_iter().map(|e| {
                                    let flags = format!("{}{}",
                                        if e.is_duplicate { "DUP " } else { "" },
                                        if e.is_suspicious { "SUS" } else { "" },
                                    );
                                    view! {
                                        <tr>
                                            <td>{e.created_at.clone()}</td>
                                            <td class="mono">{e.session_id[..8.min(e.session_id.len())].to_string()}</td>
                                            <td><span class="badge">{e.event_kind.clone()}</span></td>
                                            <td>{format!("{}:{}", e.target_type, e.target_id)}</td>
                                            <td>{e.user_id.clone().unwrap_or_else(|| "anon".into())}</td>
                                            <td><span class="warning">{flags}</span></td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>()}
                            </tbody>
                        </table>
                    }.into_view()
                }
                Some(_) => view! { <p class="empty">"No suspicious events flagged."</p> }.into_view(),
                None => view! { <p class="error-msg">"Unable to load. Admin access required."</p> }.into_view(),
            })}
        </Suspense>
    }
}
