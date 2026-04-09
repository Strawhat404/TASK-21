use leptos::*;
use leptos_router::*;

use crate::analytics;
use crate::api;
use crate::components::budget_bar::BudgetBar;
use crate::components::comment_section::CommentSection;
use crate::state::AuthState;

#[component]
pub fn ProjectDetailPage() -> impl IntoView {
    let params = use_params_map();
    let auth = use_context::<AuthState>().unwrap();

    let project_id = move || params.with(|p| p.get("id").cloned().unwrap_or_default());

    // Track impression and dwell time
    let entered_at = std::rc::Rc::new(std::cell::Cell::new(js_sys::Date::now() as i64));
    {
        let entered_at = entered_at.clone();
        let project_id = project_id.clone();
        create_effect(move |_| {
            let pid = project_id();
            if !pid.is_empty() {
                analytics::emit(common::EventKind::Impression, "project", &pid);
                entered_at.set(js_sys::Date::now() as i64);
            }
        });
    }
    {
        let entered_at = entered_at.clone();
        on_cleanup(move || {
            let pid = project_id();
            if !pid.is_empty() {
                let dwell = js_sys::Date::now() as i64 - entered_at.get();
                analytics::emit_dwell(common::EventKind::DwellTime, "project", &pid, dwell);
            }
        });
    }

    // Auto-refresh counter for live ledger updates (refunds, donations)
    let (refresh, set_refresh) = create_signal(0u32);

    let project = create_resource(
        move || (project_id(), refresh.get()),
        |(id, _)| async move {
            api::get_project(&id).await.ok()
        },
    );

    // Poll every 30 seconds so approved refunds appear without manual refresh
    create_effect(move |_| {
        let handle = gloo_timers::callback::Interval::new(30_000, move || {
            set_refresh.update(|c| *c += 1);
        });
        std::mem::forget(handle);
    });

    let on_favorite = move |_| {
        let id = project_id();
        analytics::emit(common::EventKind::Click, "favorite", &id);
        if let Some(token) = auth.get_token() {
            spawn_local(async move {
                let _ = api::toggle_favorite(&token, &id).await;
            });
        }
    };

    let on_subscribe = move |_| {
        if let Some(token) = auth.get_token() {
            let id = project_id();
            spawn_local(async move {
                let _ = api::subscribe(&token, &id).await;
            });
        }
    };

    let on_unsubscribe = move |_| {
        if let Some(token) = auth.get_token() {
            let id = project_id();
            spawn_local(async move {
                let _ = api::unsubscribe(&token, &id).await;
            });
        }
    };

    view! {
        <Suspense fallback=move || view! { <p>"Loading project..."</p> }>
            {move || project.get().map(|data| match data {
                Some(p) => {
                    let pid = p.id.clone();
                    let budget_lines = p.budget_lines.clone();
                    let updates = p.updates.clone();
                    view! {
                        <div class="project-detail">
                            <div class="project-header">
                                <h2>{&p.title}</h2>
                                <div class="project-meta">
                                    <span class="badge">{p.status.as_str()}</span>
                                    <span class="cause-tag">{&p.cause}</span>
                                    <span class="zip-tag">{&p.zip_code}</span>
                                    <span class="manager">"By: " {&p.manager_name}</span>
                                </div>
                            </div>

                            <div class="project-body">
                                <p class="description">{&p.description}</p>

                                <div class="overall-progress">
                                    <h3>"Fundraising Progress"</h3>
                                    <BudgetBar label="Total Raised".to_string()
                                        current=p.raised_cents max=p.goal_cents />
                                    <BudgetBar label="Total Spent (Verified)".to_string()
                                        current=p.spent_cents max=p.raised_cents.max(1) />
                                </div>

                                <div class="budget-section">
                                    <h3>"Budget Breakdown"</h3>
                                    {budget_lines.into_iter().map(|bl| {
                                        let name = bl.name.clone();
                                        view! {
                                            <BudgetBar label=name
                                                current=bl.spent_cents max=bl.allocated_cents />
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>

                                <div class="action-buttons">
                                    <a href=format!("/donate/{}", &pid) class="btn btn-primary">
                                        "Donate to This Project"
                                    </a>
                                    <button class="btn btn-secondary" on:click=on_favorite>
                                        "Favorite"
                                    </button>
                                    <button class="btn btn-secondary" on:click=on_subscribe>
                                        "Subscribe"
                                    </button>
                                    <button class="btn btn-secondary" on:click=on_unsubscribe>
                                        "Unsubscribe"
                                    </button>
                                </div>

                                <div class="updates-section">
                                    <h3>"Spending Updates"</h3>
                                    {if updates.is_empty() {
                                        view! { <p class="empty">"No updates yet."</p> }.into_view()
                                    } else {
                                        updates.into_iter().map(|u| {
                                            let uid = u.id.clone();
                                            let on_like = move |_| {
                                                let uid = uid.clone();
                                                if let Some(token) = auth.get_token() {
                                                    spawn_local(async move {
                                                        let _ = api::toggle_like(&token, &uid).await;
                                                    });
                                                }
                                            };
                                            view! {
                                                <div class="update-card">
                                                    <h4>{&u.title}</h4>
                                                    <p>{&u.body}</p>
                                                    <div class="update-meta">
                                                        <span>"By " {&u.author_name}</span>
                                                        <span>{&u.created_at}</span>
                                                        <button class="btn-like" on:click=on_like>
                                                            {format!("Like ({})", u.like_count)}
                                                        </button>
                                                    </div>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>().into_view()
                                    }}
                                </div>

                                <CommentSection project_id=pid />
                            </div>
                        </div>
                    }.into_view()
                }
                None => view! { <p>"Project not found."</p> }.into_view(),
            })}
        </Suspense>
    }
}
