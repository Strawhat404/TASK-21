use leptos::*;

use crate::api;
use crate::components::notification_center::NotificationCenter;
use crate::components::project_card::ProjectCard;
use crate::state::AuthState;

#[component]
pub fn ProfilePage() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();

    let (dnd_start, set_dnd_start) = create_signal("21:00".to_string());
    let (dnd_end, set_dnd_end) = create_signal("07:00".to_string());
    let (dnd_tz, set_dnd_tz) = create_signal("UTC".to_string());
    let (msg, set_msg) = create_signal(Option::<String>::None);

    // Hydrate DND form from persisted user profile
    create_effect(move |_| {
        if let Some(user) = auth.user.get() {
            set_dnd_start.set(user.dnd_start.clone());
            set_dnd_end.set(user.dnd_end.clone());
            set_dnd_tz.set(user.timezone.clone());
        }
    });

    // Load user's donations
    let donations = create_resource(
        move || auth.get_token(),
        |token| async move {
            match token {
                Some(t) => api::my_donations(&t).await.ok(),
                None => None,
            }
        },
    );

    let favorites = create_resource(
        move || auth.get_token(),
        |token| async move {
            match token {
                Some(t) => api::list_favorite_projects(&t).await.ok(),
                None => None,
            }
        },
    );

    let save_dnd = move |_| {
        if let Some(token) = auth.get_token() {
            let start = dnd_start.get();
            let end = dnd_end.get();
            let tz = dnd_tz.get();
            spawn_local(async move {
                match api::update_dnd(&token, &start, &end, &tz).await {
                    Ok(_) => set_msg.set(Some("DND settings saved".to_string())),
                    Err(e) => set_msg.set(Some(e)),
                }
            });
        }
    };

    view! {
        <div class="profile-page">
            {move || auth.user.get().map(|u| view! {
                <h2>{format!("Welcome, {}", u.display_name)}</h2>
                <div class="profile-info">
                    <p><strong>"Email: "</strong>{&u.email}</p>
                    <p><strong>"Role: "</strong>{u.role.as_str()}</p>
                    <p><strong>"Member since: "</strong>{&u.created_at}</p>
                </div>
            })}

            <section class="dnd-settings">
                <h3>"Do Not Disturb Hours"</h3>
                {move || msg.get().map(|m| view! { <div class="info-msg">{m}</div> })}
                <div class="form-row">
                    <div class="form-group">
                        <label>"Start"</label>
                        <input type="time" prop:value=dnd_start
                            on:input=move |ev| set_dnd_start.set(event_target_value(&ev)) />
                    </div>
                    <div class="form-group">
                        <label>"End"</label>
                        <input type="time" prop:value=dnd_end
                            on:input=move |ev| set_dnd_end.set(event_target_value(&ev)) />
                    </div>
                    <div class="form-group">
                        <label>"Timezone (UTC offset, e.g. +05:30, -08:00)"</label>
                        <input type="text" placeholder="UTC" prop:value=dnd_tz
                            on:input=move |ev| set_dnd_tz.set(event_target_value(&ev)) />
                    </div>
                    <button class="btn btn-primary" on:click=save_dnd>"Save"</button>
                </div>
            </section>

            <section class="notifications-section">
                <NotificationCenter />
            </section>

            <section class="my-donations">
                <h3>"My Donations"</h3>
                <Suspense fallback=move || view! { <p>"Loading..."</p> }>
                    {move || donations.get().map(|data| match data {
                        Some(list) if !list.is_empty() => {
                            view! {
                                <table class="data-table">
                                    <thead>
                                        <tr>
                                            <th>"Pledge #"</th>
                                            <th>"Project"</th>
                                            <th>"Amount"</th>
                                            <th>"Method"</th>
                                            <th>"Type"</th>
                                            <th>"Date"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {list.into_iter().map(|d| {
                                            let type_label = if d.is_reversal {
                                                let status = match d.reversal_approved {
                                                    Some(true) => "Approved",
                                                    Some(false) => "Rejected",
                                                    None => "Pending",
                                                };
                                                format!("Refund ({})", status)
                                            } else {
                                                "Donation".to_string()
                                            };
                                            view! {
                                                <tr>
                                                    <td>{d.pledge_number.clone()}</td>
                                                    <td>
                                                        <a href=format!("/projects/{}", d.project_id)>
                                                            {d.project_title.clone()}
                                                        </a>
                                                    </td>
                                                    <td>{format!("${:.2}", d.amount_cents as f64 / 100.0)}</td>
                                                    <td>{d.payment_method.clone()}</td>
                                                    <td>{type_label}</td>
                                                    <td>{d.created_at.clone()}</td>
                                                </tr>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </tbody>
                                </table>
                            }.into_view()
                        }
                        _ => view! { <p class="empty">"No donations yet."</p> }.into_view(),
                    })}
                </Suspense>
            </section>

            <section class="my-favorites">
                <h3>"Favorited Projects"</h3>
                <Suspense fallback=move || view! { <p>"Loading..."</p> }>
                    {move || favorites.get().map(|data| match data {
                        Some(projects) if !projects.is_empty() => {
                            view! {
                                <div class="project-grid">
                                    {projects.into_iter().map(|p| view! {
                                        <ProjectCard project=p />
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_view()
                        }
                        _ => view! { <p class="empty">"No favorites yet."</p> }.into_view(),
                    })}
                </Suspense>
            </section>
        </div>
    }
}
