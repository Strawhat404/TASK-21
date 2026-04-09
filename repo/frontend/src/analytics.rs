use crate::api;
use leptos::*;

/// Generate or retrieve a session ID from sessionStorage.
pub fn get_session_id() -> String {
    use gloo_storage::{SessionStorage, Storage};
    match SessionStorage::get::<String>("analytics_session_id") {
        Ok(id) => id,
        Err(_) => {
            let id = uuid::Uuid::new_v4().to_string();
            let _ = SessionStorage::set("analytics_session_id", &id);
            id
        }
    }
}

/// Fire-and-forget event tracking — errors are silently ignored.
pub fn emit(kind: common::EventKind, target_type: &str, target_id: &str) {
    let req = common::TrackEventRequest {
        event_kind: kind,
        target_type: target_type.to_string(),
        target_id: target_id.to_string(),
        session_id: get_session_id(),
        dwell_ms: None,
        metadata: None,
    };
    spawn_local(async move {
        let _ = api::track_event(&req).await;
    });
}

/// Emit with dwell time.
pub fn emit_dwell(kind: common::EventKind, target_type: &str, target_id: &str, dwell_ms: i64) {
    let req = common::TrackEventRequest {
        event_kind: kind,
        target_type: target_type.to_string(),
        target_id: target_id.to_string(),
        session_id: get_session_id(),
        dwell_ms: Some(dwell_ms),
        metadata: None,
    };
    spawn_local(async move {
        let _ = api::track_event(&req).await;
    });
}
