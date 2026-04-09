use leptos::*;
use leptos_router::*;

use crate::api;
use crate::state::AuthState;

#[component]
pub fn NavBar() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();

    let on_logout = move |_| {
        auth.logout();
        let navigate = use_navigate();
        navigate("/", Default::default());
    };

    // Poll unread notification count every 30 seconds
    let unread_count = create_resource(
        move || auth.get_token(),
        |token| async move {
            match token {
                Some(t) => api::list_notifications(&t)
                    .await
                    .ok()
                    .map(|list| list.iter().filter(|n| !n.is_read).count())
                    .unwrap_or(0),
                None => 0,
            }
        },
    );

    // Auto-refresh notification count periodically
    create_effect(move |_| {
        if auth.is_logged_in() {
            let handle = gloo_timers::callback::Interval::new(30_000, move || {
                unread_count.refetch();
            });
            std::mem::forget(handle);
        }
    });

    view! {
        <nav class="navbar">
            <div class="nav-brand">
                <a href="/">"Fund Transparency"</a>
            </div>
            <div class="nav-links">
                <a href="/projects">"Projects"</a>
                {move || {
                    if auth.is_logged_in() {
                        let user = auth.user.get();
                        let is_staff = user.as_ref().map_or(false, |u|
                            matches!(u.role, common::Role::ProjectManager | common::Role::FinanceReviewer | common::Role::Administrator)
                        );
                        let is_finance = user.as_ref().map_or(false, |u|
                            matches!(u.role, common::Role::FinanceReviewer | common::Role::Administrator)
                        );
                        let is_admin = user.as_ref().map_or(false, |u|
                            matches!(u.role, common::Role::Administrator)
                        );

                        view! {
                            <a href="/profile" class="nav-profile-link">
                                "My Profile"
                                {move || {
                                    let count = unread_count.get().unwrap_or(0);
                                    (count > 0).then(|| view! {
                                        <span class="notif-badge">{count.to_string()}</span>
                                    })
                                }}
                            </a>
                            {is_staff.then(|| view! { <a href="/staff">"Staff Ops"</a> })}
                        {is_staff.then(|| view! { <a href="/dashboard">"Dashboard"</a> })}
                            {is_finance.then(|| view! { <a href="/finance">"Finance"</a> })}
                            {is_admin.then(|| view! { <a href="/admin">"Admin"</a> })}
                            <button class="btn-nav-logout" on:click=on_logout>"Sign Out"</button>
                        }.into_view()
                    } else {
                        view! {
                            <a href="/login">"Sign In"</a>
                            <a href="/register" class="btn btn-primary btn-sm">"Register"</a>
                        }.into_view()
                    }
                }}
            </div>
        </nav>
    }
}
