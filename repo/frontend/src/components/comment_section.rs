use leptos::*;

use crate::api;
use crate::state::AuthState;

#[component]
pub fn CommentSection(project_id: String) -> impl IntoView {
    let auth = use_context::<AuthState>().unwrap();
    let (refresh, set_refresh) = create_signal(0u32);
    let pid = project_id.clone();

    let comments = create_resource(
        move || (pid.clone(), refresh.get()),
        |(pid, _)| async move { api::list_comments(&pid).await.ok() },
    );

    let (pending_delete, set_pending_delete) = create_signal(Option::<String>::None);
    let (delete_password, set_delete_password) = create_signal(String::new());
    let (new_comment, set_new_comment) = create_signal(String::new());
    let (ticket_subject, set_ticket_subject) = create_signal(String::new());
    let (ticket_body, set_ticket_body) = create_signal(String::new());
    let (feedback_msg, set_feedback_msg) = create_signal(Option::<String>::None);

    let pid_for_comment = std::rc::Rc::new(project_id.clone());
    let on_post_comment = {
        let pid = pid_for_comment.clone();
        move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            if let Some(token) = auth.get_token() {
                let pid = (*pid).clone();
                let body = new_comment.get();
                spawn_local(async move {
                    match api::post_comment(&token, &pid, &body).await {
                        Ok(_) => {
                            set_new_comment.set(String::new());
                            set_refresh.update(|c| *c += 1);
                        }
                        Err(e) => set_feedback_msg.set(Some(e)),
                    }
                });
            }
        }
    };

    let pid_for_ticket = std::rc::Rc::new(project_id.clone());
    let on_submit_ticket = {
        let pid = pid_for_ticket.clone();
        move |ev: leptos::ev::SubmitEvent| {
            ev.prevent_default();
            if let Some(token) = auth.get_token() {
                let pid = (*pid).clone();
                let subject = ticket_subject.get();
                let body = ticket_body.get();
                spawn_local(async move {
                    match api::submit_ticket(&token, &pid, &subject, &body).await {
                        Ok(r) => {
                            set_feedback_msg.set(Some(r.message));
                            set_ticket_subject.set(String::new());
                            set_ticket_body.set(String::new());
                        }
                        Err(e) => set_feedback_msg.set(Some(e)),
                    }
                });
            }
        }
    };

    view! {
        <div class="comment-section">
            <h3>"Comments"</h3>
            {move || feedback_msg.get().map(|m| view! { <div class="info-msg">{m}</div> })}

            <Suspense fallback=move || view! { <p>"Loading comments..."</p> }>
                {move || comments.get().map(|data| match data {
                    Some(list) if !list.is_empty() => {
                        view! {
                            <div class="comment-list">
                                {list.into_iter().map(|c| {
                                    let cid = c.id.clone();
                                    let cid2 = c.id.clone();
                                    let is_admin = auth.user.get().map_or(false, |u|
                                        matches!(u.role, common::Role::Administrator));
                                    // Two-step: first click sets pending, second confirms with password
                                    let on_delete_click = move |_| {
                                        let current_pending = pending_delete.get();
                                        if current_pending.as_deref() == Some(&cid) {
                                            // Second click: confirm deletion with password
                                            let cid = cid.clone();
                                            let pw = delete_password.get();
                                            if pw.is_empty() {
                                                set_feedback_msg.set(Some("Enter your password to confirm deletion".into()));
                                                return;
                                            }
                                            if let Some(token) = auth.get_token() {
                                                spawn_local(async move {
                                                    match api::delete_comment(&token, &cid, &pw).await {
                                                        Ok(_) => {
                                                            set_refresh.update(|c| *c += 1);
                                                            set_pending_delete.set(None);
                                                            set_delete_password.set(String::new());
                                                        }
                                                        Err(e) => set_feedback_msg.set(Some(e)),
                                                    }
                                                });
                                            }
                                        } else {
                                            // First click: arm confirmation
                                            set_pending_delete.set(Some(cid.clone()));
                                        }
                                    };
                                    view! {
                                        <div class="comment">
                                            <div class="comment-header">
                                                <strong>{&c.author_name}</strong>
                                                <span class="comment-date">{&c.created_at}</span>
                                                {is_admin.then(move || {
                                                    let cid_a = cid2.clone();
                                                    let cid_b = cid2;
                                                    view! {
                                                        <span>
                                                            {move || {
                                                                let armed = pending_delete.get().as_deref() == Some(cid_a.as_str());
                                                                armed.then(|| view! {
                                                                    <input type="password" class="inline-pw"
                                                                        placeholder="Password"
                                                                        on:input=move |ev| set_delete_password.set(event_target_value(&ev))
                                                                        prop:value=delete_password />
                                                                })
                                                            }}
                                                            <button class="btn-delete-sm" on:click=on_delete_click>
                                                                {move || if pending_delete.get().as_deref() == Some(cid_b.as_str()) { "Confirm Remove" } else { "Remove" }}
                                                            </button>
                                                        </span>
                                                    }
                                                })}
                                            </div>
                                            <p>{&c.body}</p>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_view()
                    }
                    _ => view! { <p class="empty">"No comments yet. Be the first!"</p> }.into_view(),
                })}
            </Suspense>

            {move || {
                let on_post_comment = on_post_comment.clone();
                auth.is_logged_in().then(move || view! {
                    <form class="comment-form" on:submit=on_post_comment>
                        <textarea placeholder="Write a comment..."
                            on:input=move |ev| set_new_comment.set(event_target_value(&ev))
                            prop:value=new_comment />
                        <button type="submit" class="btn btn-primary btn-sm">"Post Comment"</button>
                    </form>
                })
            }}

            <h3>"Submit Feedback"</h3>
            {move || {
                let on_submit_ticket = on_submit_ticket.clone();
                auth.is_logged_in().then(move || view! {
                    <form class="ticket-form" on:submit=on_submit_ticket>
                    <div class="form-group">
                        <input type="text" placeholder="Subject"
                            on:input=move |ev| set_ticket_subject.set(event_target_value(&ev))
                            prop:value=ticket_subject />
                    </div>
                    <div class="form-group">
                        <textarea placeholder="Describe your feedback..."
                            on:input=move |ev| set_ticket_body.set(event_target_value(&ev))
                            prop:value=ticket_body />
                    </div>
                    <button type="submit" class="btn btn-secondary btn-sm">"Submit Feedback"</button>
                </form>
                })
            }}
        </div>
    }
}
