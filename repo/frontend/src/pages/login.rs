use leptos::*;
use leptos_router::*;

use crate::api;
use crate::state::AuthState;

#[component]
pub fn LoginPage() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let (email, set_email) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (error, set_error) = create_signal(Option::<String>::None);
    let navigate = use_navigate();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let email = email.get();
        let password = password.get();
        let navigate = navigate.clone();
        spawn_local(async move {
            match api::login(&email, &password).await {
                Ok(resp) => {
                    auth.login(resp.token, resp.user);
                    navigate("/projects", Default::default());
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    view! {
        <div class="auth-page">
            <h2>"Sign In"</h2>
            {move || error.get().map(|e| view! { <div class="error-msg">{e}</div> })}
            <form on:submit=on_submit>
                <div class="form-group">
                    <label>"Email"</label>
                    <input type="email" required
                        on:input=move |ev| set_email.set(event_target_value(&ev))
                        prop:value=email />
                </div>
                <div class="form-group">
                    <label>"Password"</label>
                    <input type="password" required
                        on:input=move |ev| set_password.set(event_target_value(&ev))
                        prop:value=password />
                </div>
                <button type="submit" class="btn btn-primary">"Sign In"</button>
            </form>
            <p class="auth-switch">"Don't have an account? " <a href="/register">"Register"</a></p>
        </div>
    }
}

#[component]
pub fn RegisterPage() -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let (email, set_email) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (name, set_name) = create_signal(String::new());
    let (role, set_role) = create_signal("supporter".to_string());
    let (error, set_error) = create_signal(Option::<String>::None);
    let navigate = use_navigate();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let email = email.get();
        let password = password.get();
        let name = name.get();
        let selected_role = common::Role::from_str(&role.get()).unwrap_or(common::Role::Supporter);
        let navigate = navigate.clone();
        spawn_local(async move {
            match api::register(&email, &password, &name, selected_role).await {
                Ok(resp) => {
                    auth.login(resp.token, resp.user);
                    navigate("/projects", Default::default());
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    view! {
        <div class="auth-page">
            <h2>"Create Account"</h2>
            {move || error.get().map(|e| view! { <div class="error-msg">{e}</div> })}
            <form on:submit=on_submit>
                <div class="form-group">
                    <label>"Display Name"</label>
                    <input type="text" required
                        on:input=move |ev| set_name.set(event_target_value(&ev))
                        prop:value=name />
                </div>
                <div class="form-group">
                    <label>"Email"</label>
                    <input type="email" required
                        on:input=move |ev| set_email.set(event_target_value(&ev))
                        prop:value=email />
                </div>
                <div class="form-group">
                    <label>"Password (min 8 characters)"</label>
                    <input type="password" required minlength="8"
                        on:input=move |ev| set_password.set(event_target_value(&ev))
                        prop:value=password />
                </div>
                <div class="form-group">
                    <label>"Role"</label>
                    <select on:change=move |ev| set_role.set(event_target_value(&ev))>
                        <option value="supporter">"Supporter"</option>
                        <option value="project_manager">"Project Manager"</option>
                        <option value="finance_reviewer">"Finance Reviewer"</option>
                    </select>
                    <p class="role-note">"Administrator role can only be assigned by an existing admin after registration."</p>
                </div>
                <button type="submit" class="btn btn-primary">"Register"</button>
            </form>
            <p class="auth-switch">"Already have an account? " <a href="/login">"Sign In"</a></p>
        </div>
    }
}
