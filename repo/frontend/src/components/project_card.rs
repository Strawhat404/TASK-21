use leptos::*;

#[component]
pub fn ProjectCard(project: common::ProjectSummary) -> impl IntoView {
    let pct = if project.goal_cents > 0 {
        (project.raised_cents as f64 / project.goal_cents as f64 * 100.0).min(100.0)
    } else {
        0.0
    };

    view! {
        <div class="project-card">
            <div class="card-header">
                <h3>
                    <a href=format!("/projects/{}", project.id)>{&project.title}</a>
                </h3>
                <span class="badge">{project.status.as_str()}</span>
            </div>
            <div class="card-tags">
                <span class="cause-tag">{&project.cause}</span>
                <span class="zip-tag">{&project.zip_code}</span>
            </div>
            <div class="card-progress">
                <div class="progress-bar">
                    <div class="progress-fill" style=format!("width: {}%", pct)></div>
                </div>
                <div class="progress-text">
                    {format!("${:.2} of ${:.2} ({:.0}%)",
                        project.raised_cents as f64 / 100.0,
                        project.goal_cents as f64 / 100.0,
                        pct)}
                </div>
            </div>
            <div class="card-footer">
                <span class="manager">"By " {&project.manager_name}</span>
                <a href=format!("/donate/{}", project.id) class="btn btn-primary btn-sm">
                    "Donate"
                </a>
            </div>
        </div>
    }
}
