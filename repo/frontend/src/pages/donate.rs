use leptos::*;
use leptos_router::*;

use crate::api;
use crate::components::receipt::ReceiptDisplay;
use crate::state::AuthState;

#[component]
pub fn DonatePage() -> impl IntoView {
    let params = use_params_map();
    let auth = use_context::<AuthState>().unwrap();
    let project_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    let project = create_resource(project_id, |id| async move {
        api::get_project(&id).await.ok()
    });

    let (amount, set_amount) = create_signal(String::new());
    let (selected_line, set_selected_line) = create_signal(String::new());
    let (payment_method, set_payment_method) = create_signal("cash".to_string());
    let (donation_result, set_donation_result) = create_signal(Option::<common::DonationRecord>::None);
    let (error, set_error) = create_signal(Option::<String>::None);

    let on_donate = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let token = match auth.get_token() {
            Some(t) => t,
            None => {
                set_error.set(Some("Please sign in to donate".to_string()));
                return;
            }
        };
        let pid = project_id();
        let amount_str = amount.get();
        let cents: i64 = match amount_str.parse::<f64>() {
            Ok(v) => (v * 100.0) as i64,
            Err(_) => {
                set_error.set(Some("Invalid amount".to_string()));
                return;
            }
        };
        let line = selected_line.get();
        let budget_line_id = if line.is_empty() { None } else { Some(line) };
        let pm = payment_method.get();

        spawn_local(async move {
            let req = common::DonateRequest {
                project_id: pid,
                amount_cents: cents,
                payment_method: Some(pm),
                budget_line_id,
            };
            match api::donate(&token, &req).await {
                Ok(resp) => {
                    set_donation_result.set(Some(resp.donation));
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    view! {
        <div class="donate-page">
            <Suspense fallback=move || view! { <p>"Loading..."</p> }>
                {move || project.get().map(|data| match data {
                    Some(p) => {
                        let budget_lines = p.budget_lines.clone();
                        view! {
                            <h2>{format!("Donate to: {}", p.title)}</h2>
                            <div class="project-summary">
                                <p>{format!("Goal: ${:.2} | Raised: ${:.2}",
                                    p.goal_cents as f64 / 100.0,
                                    p.raised_cents as f64 / 100.0)}</p>
                            </div>

                            {move || {
                                if let Some(ref donation) = donation_result.get() {
                                    view! {
                                        <ReceiptDisplay donation=donation.clone() />
                                    }.into_view()
                                } else {
                                    view! {
                                        <form class="donate-form" on:submit=on_donate>
                                            {move || error.get().map(|e| view! { <div class="error-msg">{e}</div> })}
                                            <div class="form-group">
                                                <label>"Donation Amount ($)"</label>
                                                <input type="number" step="0.01" min="1" required
                                                    placeholder="25.00"
                                                    on:input=move |ev| set_amount.set(event_target_value(&ev))
                                                    prop:value=amount />
                                            </div>
                                            <div class="form-group">
                                                <label>"Payment Method"</label>
                                                <select on:change=move |ev| set_payment_method.set(event_target_value(&ev))>
                                                    <option value="cash">"Cash"</option>
                                                    <option value="check">"Check"</option>
                                                    <option value="card_terminal">"Card Terminal"</option>
                                                </select>
                                            </div>
                                            <div class="form-group">
                                                <label>"Designate to Budget Line (optional)"</label>
                                                <select on:change=move |ev| set_selected_line.set(event_target_value(&ev))>
                                                    <option value="">"General (no designation)"</option>
                                                    {budget_lines.clone().into_iter().map(|bl| {
                                                        let id = bl.id.clone();
                                                        let label = format!("{} (${:.2} allocated)",
                                                            bl.name, bl.allocated_cents as f64 / 100.0);
                                                        view! {
                                                            <option value=id>{label}</option>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </select>
                                            </div>
                                            <button type="submit" class="btn btn-primary btn-lg">
                                                "Complete Donation"
                                            </button>
                                        </form>
                                    }.into_view()
                                }
                            }}
                        }.into_view()
                    }
                    None => view! { <p>"Project not found."</p> }.into_view(),
                })}
            </Suspense>
        </div>
    }
}
