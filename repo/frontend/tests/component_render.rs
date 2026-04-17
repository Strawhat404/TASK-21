//! Component-level tests that import, mount, and assert on actual Leptos
//! frontend components. Uses `mount_to` from Leptos CSR to render each
//! component into a real browser DOM element, then queries the rendered
//! HTML to verify structure, content, and CSS classes.
//!
//! These tests require a browser environment (wasm_bindgen_test with
//! `run_in_browser`).

use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

use web::components::budget_bar::BudgetBar;
use web::components::project_card::ProjectCard;
use web::components::receipt::ReceiptDisplay;
use web::pages::home::HomePage;

use leptos::*;

/// Mount a component into a fresh container div, returning the container
/// element for DOM queries.
fn mount_test<F, V>(id: &str, f: F) -> web_sys::Element
where
    F: FnOnce() -> V + 'static,
    V: IntoView,
{
    let document = web_sys::window().unwrap().document().unwrap();
    let container = document.create_element("div").unwrap();
    container.set_id(id);
    document.body().unwrap().append_child(&container).unwrap();
    let html_el: web_sys::HtmlElement = container.clone().unchecked_into();
    mount_to(html_el, f);
    container
}

/// Cleanup helper — removes an element from the DOM.
fn cleanup(el: &web_sys::Element) {
    el.remove();
}

// ══════════════════════════════════════════════════════════════════════
// BudgetBar component
// ══════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn budget_bar_renders_label_and_amounts() {
    let el = mount_test("bb-1", || {
        view! { <BudgetBar label="Materials".to_string() current=25000 max=100000 /> }
    });
    let html = el.inner_html();
    assert!(html.contains("Materials"), "Should render label");
    assert!(html.contains("$250.00"), "Should render current amount");
    assert!(html.contains("$1000.00"), "Should render max amount");
    cleanup(&el);
}

#[wasm_bindgen_test]
fn budget_bar_50_pct_has_normal_class() {
    let el = mount_test("bb-2", || {
        view! { <BudgetBar label="Test".to_string() current=5000 max=10000 /> }
    });
    let html = el.inner_html();
    assert!(html.contains("progress-normal"), "50% should use progress-normal");
    assert!(html.contains("width: 50%"), "50% should have width: 50%");
    cleanup(&el);
}

#[wasm_bindgen_test]
fn budget_bar_75_pct_has_warning_class() {
    let el = mount_test("bb-3", || {
        view! { <BudgetBar label="Test".to_string() current=7500 max=10000 /> }
    });
    let html = el.inner_html();
    assert!(html.contains("progress-warning"), "75% should use progress-warning");
    cleanup(&el);
}

#[wasm_bindgen_test]
fn budget_bar_95_pct_has_danger_class() {
    let el = mount_test("bb-4", || {
        view! { <BudgetBar label="Test".to_string() current=9500 max=10000 /> }
    });
    let html = el.inner_html();
    assert!(html.contains("progress-danger"), "95% should use progress-danger");
    cleanup(&el);
}

#[wasm_bindgen_test]
fn budget_bar_zero_max_renders_zero_pct() {
    let el = mount_test("bb-5", || {
        view! { <BudgetBar label="Empty".to_string() current=100 max=0 /> }
    });
    let html = el.inner_html();
    assert!(html.contains("width: 0%"), "Zero max should render width: 0%");
    assert!(html.contains("progress-normal"), "0% should use progress-normal");
    cleanup(&el);
}

#[wasm_bindgen_test]
fn budget_bar_over_100_clamps() {
    let el = mount_test("bb-6", || {
        view! { <BudgetBar label="Over".to_string() current=15000 max=10000 /> }
    });
    let html = el.inner_html();
    assert!(html.contains("width: 100%"), "Over 100% should clamp to width: 100%");
    assert!(html.contains("progress-danger"), ">100% should be danger");
    cleanup(&el);
}

// ══════════════════════════════════════════════════════════════════════
// ProjectCard component
// ══════════════════════════════════════════════════════════════════════

fn sample_project() -> common::ProjectSummary {
    common::ProjectSummary {
        id: "p-test-1".into(),
        title: "Clean Water Initiative".into(),
        cause: "health".into(),
        zip_code: "90210".into(),
        status: common::ProjectStatus::Active,
        goal_cents: 500_000,
        raised_cents: 125_000,
        spent_cents: 50_000,
        manager_name: "Alice Manager".into(),
        created_at: "2025-01-01T00:00:00Z".into(),
    }
}

#[wasm_bindgen_test]
fn project_card_renders_title_as_link() {
    let el = mount_test("pc-1", || {
        view! { <ProjectCard project=sample_project() /> }
    });
    let html = el.inner_html();
    assert!(html.contains("Clean Water Initiative"), "Should render title");
    assert!(
        html.contains("/projects/p-test-1"),
        "Title should link to project detail"
    );
    cleanup(&el);
}

#[wasm_bindgen_test]
fn project_card_renders_cause_and_zip_tags() {
    let el = mount_test("pc-2", || {
        view! { <ProjectCard project=sample_project() /> }
    });
    let html = el.inner_html();
    assert!(html.contains("health"), "Should render cause tag");
    assert!(html.contains("90210"), "Should render zip code tag");
    cleanup(&el);
}

#[wasm_bindgen_test]
fn project_card_renders_status_badge() {
    let el = mount_test("pc-3", || {
        view! { <ProjectCard project=sample_project() /> }
    });
    let html = el.inner_html();
    assert!(html.contains("active"), "Should render status badge");
    assert!(html.contains("badge"), "Status should be in a badge element");
    cleanup(&el);
}

#[wasm_bindgen_test]
fn project_card_renders_progress_text() {
    let el = mount_test("pc-4", || {
        view! { <ProjectCard project=sample_project() /> }
    });
    let html = el.inner_html();
    // $1250.00 raised of $5000.00 goal = 25%
    assert!(html.contains("$1250.00"), "Should show raised amount");
    assert!(html.contains("$5000.00"), "Should show goal amount");
    assert!(html.contains("25%"), "Should show percentage");
    cleanup(&el);
}

#[wasm_bindgen_test]
fn project_card_renders_manager_name() {
    let el = mount_test("pc-5", || {
        view! { <ProjectCard project=sample_project() /> }
    });
    let html = el.inner_html();
    assert!(html.contains("Alice Manager"), "Should show manager name");
    assert!(html.contains("By"), "Should have 'By' prefix for manager");
    cleanup(&el);
}

#[wasm_bindgen_test]
fn project_card_renders_donate_button_with_correct_link() {
    let el = mount_test("pc-6", || {
        view! { <ProjectCard project=sample_project() /> }
    });
    let html = el.inner_html();
    assert!(
        html.contains("/donate/p-test-1"),
        "Donate button should link to /donate/p-test-1"
    );
    assert!(html.contains("Donate"), "Should show Donate text");
    cleanup(&el);
}

#[wasm_bindgen_test]
fn project_card_funded_status_renders() {
    let mut proj = sample_project();
    proj.status = common::ProjectStatus::Funded;
    proj.raised_cents = proj.goal_cents; // fully funded
    let el = mount_test("pc-7", || {
        view! { <ProjectCard project=proj /> }
    });
    let html = el.inner_html();
    assert!(html.contains("funded"), "Should render funded badge");
    assert!(html.contains("100%"), "Fully funded should show 100%");
    cleanup(&el);
}

// ══════════════════════════════════════════════════════════════════════
// ReceiptDisplay component
// ══════════════════════════════════════════════════════════════════════

fn sample_donation() -> common::DonationRecord {
    common::DonationRecord {
        id: "d-1".into(),
        pledge_number: "PLG-ABC123".into(),
        project_id: "p-1".into(),
        project_title: "Build a Well".into(),
        donor_id: "u-1".into(),
        amount_cents: 5_000,
        payment_method: "cash".into(),
        is_reversal: false,
        reversal_of: None,
        reversal_approved: None,
        budget_line_id: None,
        budget_line_name: None,
        created_at: "2025-06-15T10:00:00Z".into(),
    }
}

#[wasm_bindgen_test]
fn receipt_display_renders_pledge_number() {
    let el = mount_test("rd-1", || {
        view! { <ReceiptDisplay donation=sample_donation() /> }
    });
    let html = el.inner_html();
    assert!(html.contains("PLG-ABC123"), "Should show pledge number");
    assert!(
        html.contains("pledge-number"),
        "Pledge number should have CSS class"
    );
    cleanup(&el);
}

#[wasm_bindgen_test]
fn receipt_display_renders_amount_formatted() {
    let el = mount_test("rd-2", || {
        view! { <ReceiptDisplay donation=sample_donation() /> }
    });
    let html = el.inner_html();
    assert!(html.contains("$50.00"), "Should format 5000 cents as $50.00");
    cleanup(&el);
}

#[wasm_bindgen_test]
fn receipt_display_renders_project_title() {
    let el = mount_test("rd-3", || {
        view! { <ReceiptDisplay donation=sample_donation() /> }
    });
    let html = el.inner_html();
    assert!(html.contains("Build a Well"), "Should show project title");
    cleanup(&el);
}

#[wasm_bindgen_test]
fn receipt_display_renders_date() {
    let el = mount_test("rd-4", || {
        view! { <ReceiptDisplay donation=sample_donation() /> }
    });
    let html = el.inner_html();
    assert!(
        html.contains("2025-06-15"),
        "Should show the donation date"
    );
    cleanup(&el);
}

#[wasm_bindgen_test]
fn receipt_display_renders_view_project_link() {
    let el = mount_test("rd-5", || {
        view! { <ReceiptDisplay donation=sample_donation() /> }
    });
    let html = el.inner_html();
    assert!(
        html.contains("/projects/p-1"),
        "View Project link should point to /projects/p-1"
    );
    assert!(html.contains("View Project"), "Should have View Project text");
    cleanup(&el);
}

#[wasm_bindgen_test]
fn receipt_display_renders_print_button() {
    let el = mount_test("rd-6", || {
        view! { <ReceiptDisplay donation=sample_donation() /> }
    });
    let html = el.inner_html();
    assert!(html.contains("Print Receipt"), "Should have Print Receipt button");
    cleanup(&el);
}

#[wasm_bindgen_test]
fn receipt_display_shows_designated_line_when_present() {
    let mut don = sample_donation();
    don.budget_line_name = Some("Materials Fund".into());
    let el = mount_test("rd-7", || {
        view! { <ReceiptDisplay donation=don /> }
    });
    let html = el.inner_html();
    assert!(
        html.contains("Designated To"),
        "Should show Designated To label"
    );
    assert!(
        html.contains("Materials Fund"),
        "Should show the budget line name"
    );
    cleanup(&el);
}

#[wasm_bindgen_test]
fn receipt_display_hides_designated_when_absent() {
    // sample_donation() has budget_line_name = None
    let el = mount_test("rd-8", || {
        view! { <ReceiptDisplay donation=sample_donation() /> }
    });
    let html = el.inner_html();
    assert!(
        !html.contains("Designated To"),
        "Should NOT show Designated To when no budget line"
    );
    cleanup(&el);
}

#[wasm_bindgen_test]
fn receipt_display_has_receipt_id() {
    let el = mount_test("rd-9", || {
        view! { <ReceiptDisplay donation=sample_donation() /> }
    });
    let html = el.inner_html();
    assert!(
        html.contains("donation-receipt"),
        "Receipt container should have id='donation-receipt'"
    );
    cleanup(&el);
}

#[wasm_bindgen_test]
fn receipt_display_confirmation_heading() {
    let el = mount_test("rd-10", || {
        view! { <ReceiptDisplay donation=sample_donation() /> }
    });
    let html = el.inner_html();
    assert!(
        html.contains("Donation Confirmed"),
        "Receipt should show confirmation heading"
    );
    cleanup(&el);
}

// ══════════════════════════════════════════════════════════════════════
// HomePage component
// ══════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn home_page_renders_hero_section() {
    let el = mount_test("hp-1", || {
        view! { <HomePage /> }
    });
    let html = el.inner_html();
    assert!(
        html.contains("Community Giving"),
        "Hero heading should contain 'Community Giving'"
    );
    assert!(
        html.contains("Fund Transparency"),
        "Hero heading should contain 'Fund Transparency'"
    );
    cleanup(&el);
}

#[wasm_bindgen_test]
fn home_page_renders_browse_projects_link() {
    let el = mount_test("hp-2", || {
        view! { <HomePage /> }
    });
    let html = el.inner_html();
    assert!(
        html.contains("/projects"),
        "Should link to /projects"
    );
    assert!(
        html.contains("Browse Projects"),
        "Should have 'Browse Projects' CTA"
    );
    cleanup(&el);
}

#[wasm_bindgen_test]
fn home_page_renders_register_link() {
    let el = mount_test("hp-3", || {
        view! { <HomePage /> }
    });
    let html = el.inner_html();
    assert!(
        html.contains("/register"),
        "Should link to /register"
    );
    assert!(
        html.contains("Get Started"),
        "Should have 'Get Started' CTA"
    );
    cleanup(&el);
}

#[wasm_bindgen_test]
fn home_page_renders_four_feature_cards() {
    let el = mount_test("hp-4", || {
        view! { <HomePage /> }
    });
    let html = el.inner_html();
    assert!(html.contains("Project-Based Drives"), "Feature 1");
    assert!(html.contains("Budget Transparency"), "Feature 2");
    assert!(html.contains("Verified Disclosures"), "Feature 3");
    assert!(html.contains("Supporter Engagement"), "Feature 4");
    cleanup(&el);
}

#[wasm_bindgen_test]
fn home_page_has_features_section_class() {
    let el = mount_test("hp-5", || {
        view! { <HomePage /> }
    });
    let html = el.inner_html();
    assert!(html.contains("features"), "Should have features section");
    assert!(html.contains("feature-card"), "Should have feature-card elements");
    cleanup(&el);
}

#[wasm_bindgen_test]
fn home_page_hero_has_subtitle() {
    let el = mount_test("hp-6", || {
        view! { <HomePage /> }
    });
    let html = el.inner_html();
    assert!(
        html.contains("Support local nonprofits"),
        "Subtitle should describe the platform purpose"
    );
    cleanup(&el);
}
