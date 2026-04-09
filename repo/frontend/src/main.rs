mod analytics;
mod api;
mod components;
mod pages;
mod state;

use gloo_storage::Storage;
use leptos::*;
use leptos_router::*;

use pages::*;

fn main() {
    mount_to_body(|| view! { <App /> });
}

#[component]
fn App() -> impl IntoView {
    // Provide global auth state
    let (auth_token, set_auth_token) = create_signal(
        gloo_storage::LocalStorage::get::<String>("auth_token").ok(),
    );
    let (current_user, set_current_user) = create_signal(
        gloo_storage::LocalStorage::get::<common::UserProfile>("current_user").ok(),
    );

    provide_context(state::AuthState {
        token: auth_token,
        set_token: set_auth_token,
        user: current_user,
        set_user: set_current_user,
    });

    // Emit session_start event on app mount
    create_effect(move |_| {
        analytics::emit(common::EventKind::SessionStart, "app", "mount");
    });

    view! {
        <Router>
            <components::nav::NavBar />
            <main class="container">
                <Routes>
                    <Route path="/" view=home::HomePage />
                    <Route path="/login" view=login::LoginPage />
                    <Route path="/register" view=login::RegisterPage />
                    <Route path="/projects" view=project_list::ProjectListPage />
                    <Route path="/projects/:id" view=project_detail::ProjectDetailPage />
                    <Route path="/donate/:id" view=|| view! {
                        <components::route_guard::AuthGuard>
                            <donate::DonatePage />
                        </components::route_guard::AuthGuard>
                    } />
                    <Route path="/profile" view=|| view! {
                        <components::route_guard::AuthGuard>
                            <profile::ProfilePage />
                        </components::route_guard::AuthGuard>
                    } />
                    <Route path="/dashboard" view=|| view! {
                        <components::route_guard::RoleGuard allowed=vec![
                            common::Role::ProjectManager,
                            common::Role::FinanceReviewer,
                            common::Role::Administrator,
                        ]>
                            <dashboard::DashboardPage />
                        </components::route_guard::RoleGuard>
                    } />
                    <Route path="/admin" view=|| view! {
                        <components::route_guard::RoleGuard allowed=vec![common::Role::Administrator]>
                            <admin::AdminPage />
                        </components::route_guard::RoleGuard>
                    } />
                    <Route path="/finance" view=|| view! {
                        <components::route_guard::RoleGuard allowed=vec![
                            common::Role::FinanceReviewer,
                            common::Role::Administrator,
                        ]>
                            <finance::FinancePage />
                        </components::route_guard::RoleGuard>
                    } />
                    <Route path="/staff" view=|| view! {
                        <components::route_guard::RoleGuard allowed=vec![
                            common::Role::ProjectManager,
                            common::Role::FinanceReviewer,
                            common::Role::Administrator,
                        ]>
                            <staff::StaffPage />
                        </components::route_guard::RoleGuard>
                    } />
                    <Route path="/fulfillment/:id" view=|| view! {
                        <components::route_guard::RoleGuard allowed=vec![
                            common::Role::ProjectManager,
                            common::Role::Administrator,
                        ]>
                            <fulfillment::FulfillmentPage />
                        </components::route_guard::RoleGuard>
                    } />
                    <Route path="/fulfillment/:id/proof" view=|| view! {
                        <components::route_guard::RoleGuard allowed=vec![
                            common::Role::ProjectManager,
                            common::Role::Administrator,
                        ]>
                            <fulfillment::ServiceProofPage />
                        </components::route_guard::RoleGuard>
                    } />
                </Routes>
            </main>
        </Router>
    }
}
