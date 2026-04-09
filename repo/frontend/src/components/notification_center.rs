use leptos::*;

use crate::api;
use crate::state::AuthState;

#[component]
pub fn NotificationCenter() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let (refresh, set_refresh) = create_signal(0u32);

    let notifications = create_resource(
        move || (auth.get_token(), refresh.get()),
        |(token, _)| async move {
            match token {
                Some(t) => api::list_notifications(&t).await.ok(),
                None => None,
            }
        },
    );

    let mark_all = move |_| {
        if let Some(token) = auth.get_token() {
            spawn_local(async move {
                let _ = api::mark_all_read(&token).await;
                set_refresh.update(|c| *c += 1);
            });
        }
    };

    view! {
        <div class="notification-center">
            <div class="notif-header">
                <h3>"Message Center"</h3>
                <button class="btn btn-sm" on:click=mark_all>"Mark All Read"</button>
            </div>
            <Suspense fallback=move || view! { <p>"Loading..."</p> }>
                {move || notifications.get().map(|data| match data {
                    Some(list) if !list.is_empty() => {
                        let unread_count = list.iter().filter(|n| !n.is_read).count();
                        view! {
                            <div class="notif-summary">
                                {format!("{} unread of {} total", unread_count, list.len())}
                            </div>
                            <div class="notif-list">
                                {list.into_iter().map(|n| {
                                    let nid = n.id.clone();
                                    let is_read = n.is_read;
                                    let on_click = move |_| {
                                        if !is_read {
                                            let nid = nid.clone();
                                            if let Some(token) = auth.get_token() {
                                                spawn_local(async move {
                                                    let _ = api::mark_notification_read(&token, &nid).await;
                                                    set_refresh.update(|c| *c += 1);
                                                });
                                            }
                                        }
                                    };
                                    view! {
                                        <div class={if n.is_read { "notif-item read" } else { "notif-item unread" }}
                                            on:click=on_click>
                                            <div class="notif-title">{&n.title}</div>
                                            <div class="notif-body">{&n.body}</div>
                                            <div class="notif-time">{&n.created_at}</div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_view()
                    }
                    _ => view! {
                        <p class="empty">"No notifications."</p>
                    }.into_view(),
                })}
            </Suspense>
        </div>
    }
}
