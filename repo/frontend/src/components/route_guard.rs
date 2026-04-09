use leptos::*;
use leptos_router::*;

use crate::state::AuthState;

/// Redirects to /login if user is not authenticated.
#[component]
pub fn AuthGuard(children: ChildrenFn) -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();

    view! {
        {move || {
            if auth.is_logged_in() {
                children().into_view()
            } else {
                let navigate = use_navigate();
                navigate("/login", Default::default());
                view! { <p>"Redirecting to login..."</p> }.into_view()
            }
        }}
    }
}

/// Redirects to /login if not authenticated, or shows an error if role doesn't match.
#[component]
pub fn RoleGuard(
    #[prop(into)] allowed: Vec<common::Role>,
    children: ChildrenFn,
) -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();

    view! {
        {move || {
            if !auth.is_logged_in() {
                let navigate = use_navigate();
                navigate("/login", Default::default());
                return view! { <p>"Redirecting to login..."</p> }.into_view();
            }
            let user = auth.user.get();
            let has_role = user.as_ref().map_or(false, |u| allowed.contains(&u.role));
            if has_role {
                children().into_view()
            } else {
                view! {
                    <div class="error-msg">
                        "You do not have permission to access this page."
                    </div>
                }.into_view()
            }
        }}
    }
}
