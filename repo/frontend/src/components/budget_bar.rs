use leptos::*;

#[component]
pub fn BudgetBar(
    label: String,
    current: i64,
    max: i64,
) -> impl IntoView {
    let pct = if max > 0 {
        (current as f64 / max as f64 * 100.0).min(100.0)
    } else {
        0.0
    };

    let color_class = if pct > 90.0 {
        "progress-danger"
    } else if pct > 70.0 {
        "progress-warning"
    } else {
        "progress-normal"
    };

    view! {
        <div class="budget-bar">
            <div class="budget-label">
                <span>{&label}</span>
                <span class="budget-amounts">
                    {format!("${:.2} / ${:.2}", current as f64 / 100.0, max as f64 / 100.0)}
                </span>
            </div>
            <div class="progress-bar">
                <div class={format!("progress-fill {}", color_class)}
                    style=format!("width: {}%", pct)>
                </div>
            </div>
        </div>
    }
}
