use leptos::*;

use crate::api;
use crate::state::AuthState;

#[component]
pub fn DashboardPage() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let (from_date, set_from) = create_signal(String::new());
    let (to_date, set_to) = create_signal(String::new());
    let (cause_filter, set_cause) = create_signal(String::new());
    let (status_filter, set_status) = create_signal(String::new());

    let stats = create_resource(
        move || (auth.get_token(), from_date.get(), to_date.get(), cause_filter.get(), status_filter.get()),
        |(token, from, to, cause, status)| async move {
            match token {
                Some(t) => {
                    let f = if from.is_empty() { None } else { Some(from) };
                    let tt = if to.is_empty() { None } else { Some(to) };
                    let c = if cause.is_empty() { None } else { Some(cause) };
                    let s = if status.is_empty() { None } else { Some(status) };
                    api::dashboard_stats(&t, f.as_deref(), tt.as_deref(), c.as_deref(), s.as_deref())
                        .await
                        .ok()
                }
                None => None,
            }
        },
    );

    let on_export = move |_| {
        if let Some(token) = auth.get_token() {
            let from = from_date.get();
            let to = to_date.get();
            let cause = cause_filter.get();
            let status = status_filter.get();
            let from_opt = if from.is_empty() { None } else { Some(from) };
            let to_opt = if to.is_empty() { None } else { Some(to) };
            let cause_opt = if cause.is_empty() { None } else { Some(cause) };
            let status_opt = if status.is_empty() { None } else { Some(status) };
            spawn_local(async move {
                match api::export_csv(&token, from_opt.as_deref(), to_opt.as_deref(), cause_opt.as_deref(), status_opt.as_deref()).await {
                    Ok(csv_text) => {
                        // Trigger download via Blob URL
                        if let Some(window) = web_sys::window() {
                            let _ = js_sys::eval(&format!(
                                "var b=new Blob([decodeURIComponent('{}')],{{type:'text/csv'}});\
                                 var a=document.createElement('a');a.href=URL.createObjectURL(b);\
                                 a.download='donations.csv';a.click();",
                                js_sys::encode_uri_component(&csv_text)
                            ));
                        }
                    }
                    Err(e) => {
                        web_sys::window().and_then(|w| w.alert_with_message(&format!("Export failed: {}", e)).ok());
                    }
                }
            });
        }
    };

    view! {
        <div class="dashboard-page">
            <h2>"Operations Dashboard"</h2>

            <div class="date-range-filters">
                <div class="form-group">
                    <label>"From"</label>
                    <input type="date" on:input=move |ev| set_from.set(event_target_value(&ev))
                        prop:value=from_date />
                </div>
                <div class="form-group">
                    <label>"To"</label>
                    <input type="date" on:input=move |ev| set_to.set(event_target_value(&ev))
                        prop:value=to_date />
                </div>
                <div class="form-group">
                    <label>"Cause"</label>
                    <select class="filter-select" on:change=move |ev| set_cause.set(event_target_value(&ev))>
                        <option value="">"All"</option>
                        <option value="education">"Education"</option>
                        <option value="health">"Health"</option>
                        <option value="environment">"Environment"</option>
                        <option value="housing">"Housing"</option>
                        <option value="food">"Food"</option>
                        <option value="youth">"Youth"</option>
                        <option value="arts">"Arts"</option>
                        <option value="other">"Other"</option>
                    </select>
                </div>
                <div class="form-group">
                    <label>"Status"</label>
                    <select class="filter-select" on:change=move |ev| set_status.set(event_target_value(&ev))>
                        <option value="">"All"</option>
                        <option value="active">"Active"</option>
                        <option value="funded">"Funded"</option>
                        <option value="closed">"Closed"</option>
                    </select>
                </div>
                <button class="btn btn-secondary" on:click=on_export>"Export CSV"</button>
            </div>

            <Suspense fallback=move || view! { <p>"Loading stats..."</p> }>
                {move || stats.get().map(|data| match data {
                    Some(s) => view! {
                        <div class="stats-grid">
                            <div class="stat-card">
                                <div class="stat-value">
                                    {format!("${:.2}", s.gmv_cents as f64 / 100.0)}
                                </div>
                                <div class="stat-label">"Gross Merchandise Value"</div>
                            </div>
                            <div class="stat-card">
                                <div class="stat-value">{s.total_donations.to_string()}</div>
                                <div class="stat-label">"Total Donations"</div>
                            </div>
                            <div class="stat-card">
                                <div class="stat-value">{s.unique_donors.to_string()}</div>
                                <div class="stat-label">"Unique Donors"</div>
                            </div>
                            <div class="stat-card">
                                <div class="stat-value">
                                    {format!("${:.2}", s.average_donation_cents as f64 / 100.0)}
                                </div>
                                <div class="stat-label">"Average Donation"</div>
                            </div>
                            <div class="stat-card">
                                <div class="stat-value">
                                    {format!("{:.1}%", s.repeat_donor_rate * 100.0)}
                                </div>
                                <div class="stat-label">"Repeat Donor Rate"</div>
                            </div>
                            <div class="stat-card">
                                <div class="stat-value">
                                    {format!("{:.1}%", s.conversion_rate * 100.0)}
                                </div>
                                <div class="stat-label">"Conversion Rate"</div>
                            </div>
                        </div>
                    }.into_view(),
                    None => view! {
                        <p class="error-msg">"Unable to load stats. Ensure you have staff access."</p>
                    }.into_view(),
                })}
            </Suspense>
        </div>
    }
}
