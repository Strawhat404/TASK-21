use leptos::*;

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <div class="hero">
            <h1>"Community Giving & Fund Transparency Portal"</h1>
            <p class="subtitle">
                "Support local nonprofits, track how every dollar is spent, and close the loop with transparent project fulfillment."
            </p>
            <div class="hero-actions">
                <a href="/projects" class="btn btn-primary">"Browse Projects"</a>
                <a href="/register" class="btn btn-secondary">"Get Started"</a>
            </div>
        </div>

        <section class="features">
            <div class="feature-card">
                <h3>"Project-Based Drives"</h3>
                <p>"Nonprofits create transparent project campaigns with detailed budgets."</p>
            </div>
            <div class="feature-card">
                <h3>"Budget Transparency"</h3>
                <p>"See exactly how funds are allocated and spent with real-time progress bars."</p>
            </div>
            <div class="feature-card">
                <h3>"Verified Disclosures"</h3>
                <p>"Finance reviewers verify receipts and approve spending disclosures."</p>
            </div>
            <div class="feature-card">
                <h3>"Supporter Engagement"</h3>
                <p>"Follow projects, comment, receive updates, and submit feedback."</p>
            </div>
        </section>
    }
}
