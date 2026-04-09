use leptos::*;

use crate::analytics;
use crate::api;
use crate::components::project_card::ProjectCard;

#[component]
pub fn ProjectListPage() -> impl IntoView {
    // Track page impression
    create_effect(move |_| {
        analytics::emit(common::EventKind::Impression, "page", "project_list");
    });
    let (search, set_search) = create_signal(String::new());
    let (cause_filter, set_cause) = create_signal(String::new());
    let (status_filter, set_status) = create_signal(String::new());
    let (zip_filter, set_zip) = create_signal(String::new());
    let (page, set_page) = create_signal(1i64);

    let projects = create_resource(
        move || (search.get(), cause_filter.get(), status_filter.get(), zip_filter.get(), page.get()),
        |(search, cause, status, zip, page)| async move {
            let cause = if cause.is_empty() { None } else { Some(cause.as_str().to_string()) };
            let status = if status.is_empty() { None } else { Some(status.as_str().to_string()) };
            let zip = if zip.is_empty() { None } else { Some(zip.as_str().to_string()) };
            let search = if search.is_empty() { None } else { Some(search.as_str().to_string()) };
            match api::list_projects(
                cause.as_deref(),
                status.as_deref(),
                zip.as_deref(),
                search.as_deref(),
                page,
            )
            .await
            {
                Ok(resp) => Some(resp),
                Err(e) => {
                    leptos::logging::warn!("Failed to load projects: {}", e);
                    None
                }
            }
        },
    );

    view! {
        <div class="project-list-page">
            <h2>"Browse Projects"</h2>

            <div class="filters">
                <input type="text" placeholder="Search projects..."
                    class="search-input"
                    on:input=move |ev| {
                        set_search.set(event_target_value(&ev));
                        set_page.set(1);
                    }
                    prop:value=search />

                <select class="filter-select" on:change=move |ev| { set_cause.set(event_target_value(&ev)); set_page.set(1); }>
                    <option value="">"All Causes"</option>
                    <option value="education">"Education"</option>
                    <option value="health">"Health"</option>
                    <option value="environment">"Environment"</option>
                    <option value="housing">"Housing"</option>
                    <option value="food">"Food Security"</option>
                    <option value="youth">"Youth Programs"</option>
                    <option value="arts">"Arts & Culture"</option>
                    <option value="other">"Other"</option>
                </select>

                <select class="filter-select" on:change=move |ev| { set_status.set(event_target_value(&ev)); set_page.set(1); }>
                    <option value="">"All Statuses"</option>
                    <option value="active">"Active"</option>
                    <option value="funded">"Funded"</option>
                    <option value="closed">"Closed"</option>
                </select>

                <input type="text" placeholder="ZIP Code"
                    class="zip-input"
                    on:input=move |ev| { set_zip.set(event_target_value(&ev)); set_page.set(1); }
                    prop:value=zip_filter />
            </div>

            <Suspense fallback=move || view! { <p>"Loading projects..."</p> }>
                {move || projects.get().map(|data| {
                    match data {
                        Some(resp) if resp.items.is_empty() => {
                            view! {
                                <div class="empty-project-list">
                                    <div class="empty-state-card">
                                        <h3>"No Campaigns Found"</h3>
                                        <p>"There are no community giving campaigns matching your criteria. Try broadening your search or adjusting the cause, status, or ZIP code filters above."</p>
                                        <p class="empty-hint">"Local nonprofits are always launching new project-based donation drives \u{2014} check back soon!"</p>
                                    </div>
                                </div>
                            }.into_view()
                        }
                        Some(resp) => {
                            let total_pages = (resp.total + resp.per_page - 1) / resp.per_page;
                            view! {
                                <div class="project-grid">
                                    {resp.items.into_iter().map(|p| view! {
                                        <ProjectCard project=p />
                                    }).collect::<Vec<_>>()}
                                </div>
                                {(total_pages > 1).then(|| view! {
                                    <div class="pagination">
                                        <button class="btn btn-sm"
                                            disabled=move || page.get() <= 1
                                            on:click=move |_| set_page.update(|p| *p -= 1)>
                                            "Previous"
                                        </button>
                                        <span class="page-info">
                                            {move || format!("Page {} of {}", page.get(), total_pages)}
                                        </span>
                                        <button class="btn btn-sm"
                                            disabled=move || page.get() >= total_pages
                                            on:click=move |_| set_page.update(|p| *p += 1)>
                                            "Next"
                                        </button>
                                    </div>
                                })}
                            }.into_view()
                        }
                        None => view! {
                            <div class="empty-project-list">
                                <div class="empty-state-card error-state">
                                    <h3>"Unable to Load Projects"</h3>
                                    <p>"Could not reach the server. Please check your connection and try again."</p>
                                </div>
                            </div>
                        }.into_view(),
                    }
                })}
            </Suspense>
        </div>
    }
}
