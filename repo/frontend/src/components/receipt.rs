use leptos::*;

#[component]
pub fn ReceiptDisplay(donation: common::DonationRecord) -> impl IntoView {
    let on_print = |_| {
        if let Some(window) = web_sys::window() {
            let _ = window.print();
        }
    };

    view! {
        <div class="receipt" id="donation-receipt">
            <div class="receipt-header">
                <h2>"Donation Confirmed!"</h2>
                <p class="receipt-subtitle">"Thank you for your generous contribution"</p>
            </div>
            <div class="receipt-body">
                <div class="receipt-row">
                    <span class="receipt-label">"Pledge Number:"</span>
                    <span class="receipt-value pledge-number">{&donation.pledge_number}</span>
                </div>
                <div class="receipt-row">
                    <span class="receipt-label">"Project:"</span>
                    <span class="receipt-value">{&donation.project_title}</span>
                </div>
                <div class="receipt-row">
                    <span class="receipt-label">"Amount:"</span>
                    <span class="receipt-value amount">
                        {format!("${:.2}", donation.amount_cents as f64 / 100.0)}
                    </span>
                </div>
                {donation.budget_line_name.as_ref().map(|name| view! {
                    <div class="receipt-row">
                        <span class="receipt-label">"Designated To:"</span>
                        <span class="receipt-value">{name}</span>
                    </div>
                })}
                <div class="receipt-row">
                    <span class="receipt-label">"Date:"</span>
                    <span class="receipt-value">{&donation.created_at}</span>
                </div>
            </div>
            <div class="receipt-footer">
                <button class="btn btn-secondary" on:click=on_print>"Print Receipt"</button>
                <a href=format!("/projects/{}", donation.project_id)
                    class="btn btn-primary">"View Project"</a>
            </div>
        </div>
    }
}
