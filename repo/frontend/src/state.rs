use gloo_storage::Storage;
use leptos::*;

#[derive(Clone, Copy)]
pub struct AuthState {
    pub token: ReadSignal<Option<String>>,
    pub set_token: WriteSignal<Option<String>>,
    pub user: ReadSignal<Option<common::UserProfile>>,
    pub set_user: WriteSignal<Option<common::UserProfile>>,
}

impl AuthState {
    pub fn login(&self, token: String, user: common::UserProfile) {
        let _ = gloo_storage::LocalStorage::set("auth_token", &token);
        let _ = gloo_storage::LocalStorage::set("current_user", &user);
        self.set_token.set(Some(token));
        self.set_user.set(Some(user));
    }

    pub fn logout(&self) {
        gloo_storage::LocalStorage::delete("auth_token");
        gloo_storage::LocalStorage::delete("current_user");
        self.set_token.set(None);
        self.set_user.set(None);
    }

    pub fn is_logged_in(&self) -> bool {
        self.token.get().is_some()
    }

    pub fn get_token(&self) -> Option<String> {
        self.token.get()
    }
}
