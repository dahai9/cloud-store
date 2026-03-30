#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use gloo_net::http::Request;
#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use web_sys::window;

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    StorefrontPage {},
    #[route("/order")]
    OrderPage {},
    #[route("/login")]
    LoginPage {},
    #[route("/app/profile")]
    ProfilePage {},
    #[route("/app/services")]
    ServicesPage {},
    #[route("/app/tickets")]
    TicketsPage {},
    #[route("/app/balance")]
    BalancePage {},
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, PartialEq)]
enum DashboardTab {
    Profile,
    Services,
    Tickets,
    Balance,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
struct ProductPlan {
    name: &'static str,
    spec: &'static str,
    monthly_price: &'static str,
    badge: &'static str,
}

#[cfg(target_arch = "wasm32")]
const PLANS: [ProductPlan; 3] = [
    ProductPlan {
        name: "NAT Mini",
        spec: "1GB RAM / 50GB SSD / Shared NAT",
        monthly_price: "$5.99",
        badge: "Starter",
    },
    ProductPlan {
        name: "NAT Standard",
        spec: "1GB RAM / 50GB SSD / Better traffic priority",
        monthly_price: "$7.99",
        badge: "Most Popular",
    },
    ProductPlan {
        name: "NAT Pro",
        spec: "1GB RAM / 50GB SSD / Priority support",
        monthly_price: "$9.99",
        badge: "Business",
    },
];

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Serialize)]
struct AuthPayload {
    email: String,
    password: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Deserialize)]
struct AuthTokenResponse {
    token: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Deserialize, Serialize)]
struct AuthProfileResponse {
    user_id: String,
    email: String,
    role: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Deserialize)]
struct InvoiceItem {
    id: String,
    amount: String,
    status: String,
    due_at: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Deserialize)]
struct TicketItem {
    id: String,
    subject: String,
    category: String,
    priority: String,
    status: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
struct SessionState {
    api_base: String,
    token: Option<String>,
    profile: Option<AuthProfileResponse>,
    invoices: Vec<InvoiceItem>,
    tickets: Vec<TicketItem>,
    loading: bool,
    error: Option<String>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, PartialEq)]
enum AuthTransportRisk {
    Secure,
    LoopbackDev,
    InsecureRemote,
}

#[cfg(target_arch = "wasm32")]
const AUTH_TOKEN_KEY: &str = "cloud_store.auth.token";

#[cfg(target_arch = "wasm32")]
const AUTH_PROFILE_KEY: &str = "cloud_store.auth.profile";

#[cfg(target_arch = "wasm32")]
fn default_api_base() -> String {
    option_env!("API_BASE_URL")
        .unwrap_or("http://127.0.0.1:8081")
        .to_string()
}

#[cfg(target_arch = "wasm32")]
fn auth_transport_risk(api_base: &str) -> AuthTransportRisk {
    if api_base.starts_with("https://") {
        AuthTransportRisk::Secure
    } else if api_base.contains("127.0.0.1") || api_base.contains("localhost") {
        AuthTransportRisk::LoopbackDev
    } else {
        AuthTransportRisk::InsecureRemote
    }
}

#[cfg(target_arch = "wasm32")]
fn auth_transport_notice(api_base: &str) -> Option<&'static str> {
    match auth_transport_risk(api_base) {
        AuthTransportRisk::Secure => None,
        AuthTransportRisk::LoopbackDev => {
            Some("当前是本地开发地址，注册/登录请求仍会通过 HTTP 发送。生产环境请改成 HTTPS。")
        }
        AuthTransportRisk::InsecureRemote => {
            Some("当前 API 不是 HTTPS，出于安全原因已禁止提交账号信息。请先切换到 HTTPS。")
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    launch(App);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("frontend is a web-only app; run with dx serve --platform web");
}

#[cfg(target_arch = "wasm32")]
#[component]
fn App() -> Element {
    let session = use_signal(|| {
        let mut initial = SessionState {
            api_base: default_api_base(),
            token: None,
            profile: None,
            invoices: vec![],
            tickets: vec![],
            loading: false,
            error: None,
        };

        if let Some((token, profile)) = load_persisted_session() {
            initial.token = Some(token);
            initial.profile = Some(profile);
        }

        initial
    });

    use_context_provider(|| session);

    use_effect(move || {
        let mut session = session;

        spawn(async move {
            let (api_base, token) = {
                let current = session();
                (current.api_base.clone(), current.token.clone())
            };

            let Some(token) = token else {
                return;
            };

            match load_authenticated_bundle(&api_base, &token).await {
                Ok(bundle) => {
                    let mut current = session.write();
                    current.profile = Some(bundle.profile);
                    current.invoices = bundle.invoices;
                    current.tickets = bundle.tickets;
                    current.loading = false;
                    current.error = None;
                }
                Err(err) => {
                    let mut current = session.write();
                    current.error = Some(err);
                    current.loading = false;
                    current.token = None;
                    current.profile = None;
                    current.invoices.clear();
                    current.tickets.clear();
                    clear_persisted_session();
                }
            }
        });
    });

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        Router::<Route> {}
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
fn StorefrontPage() -> Element {
    let navigator = use_navigator();

    rsx! {
        div { class: "public-shell",
            header { class: "public-header",
                div { class: "brand",
                    div { class: "logo-mark", "C" }
                    div {
                        h1 { "Cloud Store" }
                        p { "13 NAT VPS nodes ready for sale" }
                    }
                }
                div { class: "header-actions",
                    button {
                        class: "btn-secondary",
                        onclick: move |_| {
                            navigator.push(Route::LoginPage {});
                        },
                        "Login"
                    }
                    button {
                        class: "btn-primary",
                        onclick: move |_| {
                            navigator.push(Route::OrderPage {});
                        },
                        "Try Order"
                    }
                }
            }

            main { class: "public-main",
                section { class: "hero",
                    h2 { "NAT VPS Resale Platform" }
                    p {
                        "Guests can browse products and payment methods. Login is required only when final checkout starts or protected pages are opened."
                    }
                    div { class: "chip-row",
                        span { class: "chip", "13 Available Nodes" }
                        span { class: "chip", "PayPal Required" }
                        span { class: "chip", "Service + Ticket Center" }
                    }
                }

                section { class: "product-grid",
                    for plan in PLANS {
                        article { class: "product-card",
                            div { class: "tag", "{plan.badge}" }
                            h3 { "{plan.name}" }
                            p { "{plan.spec}" }
                            div { class: "price", "{plan.monthly_price} / month" }
                            button {
                                class: "btn-secondary",
                                onclick: move |_| {
                                    navigator.push(Route::OrderPage {});
                                },
                                "Select"
                            }
                        }
                    }
                }

                section { class: "pay-preview",
                    h3 { "Available Payment Methods" }
                    ul {
                        li { "PayPal (required)" }
                        li { "Alipay" }
                        li { "Bank Transfer" }
                    }
                    p { class: "muted", "You can view products and payment methods without login." }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
fn OrderPage() -> Element {
    let navigator = use_navigator();
    let session = use_context::<Signal<SessionState>>();
    let mut selected_plan = use_signal(|| "NAT Standard".to_string());

    let on_checkout = move |_| {
        if session().token.is_some() {
            navigator.push(Route::BalancePage {});
        } else {
            navigator.push(Route::LoginPage {});
        }
    };

    rsx! {
        div { class: "public-shell",
            header { class: "public-header",
                h1 { "Create Order" }
                button {
                    class: "btn-secondary",
                    onclick: move |_| {
                        navigator.push(Route::StorefrontPage {});
                    },
                    "Back"
                }
            }

            main { class: "public-main",
                section { class: "checkout-card",
                    h3 { "Order Summary" }
                    div { class: "order-meta",
                        label { "Product" }
                        select {
                            value: "{selected_plan()}",
                            onchange: move |evt| selected_plan.set(evt.value()),
                            option { value: "NAT Mini", "NAT Mini" }
                            option { value: "NAT Standard", "NAT Standard" }
                            option { value: "NAT Pro", "NAT Pro" }
                        }
                    }
                    p { "Spec: 1GB RAM / 50GB SSD" }
                    p { "Monthly Price: starts from $5.99" }

                    h4 { "Payment Method" }
                    div { class: "pay-methods",
                        label {
                            input { r#type: "radio", name: "pay", checked: true }
                            " PayPal"
                        }
                        label {
                            input { r#type: "radio", name: "pay" }
                            " Alipay"
                        }
                        label {
                            input { r#type: "radio", name: "pay" }
                            " Bank Transfer"
                        }
                    }

                    button { class: "btn-primary full", onclick: on_checkout, "Proceed To Checkout" }

                    if session().token.is_none() {
                        p { class: "notice",
                            "You can configure the order now. Login is required only at checkout."
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
fn LoginPage() -> Element {
    let navigator = use_navigator();
    let mut session = use_context::<Signal<SessionState>>();
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);

    let on_login = move |_| {
        let email_value = email();
        let password_value = password();
        let nav = navigator.clone();
        let mut state = session;

        spawn(async move {
            {
                let mut current = state.write();
                current.loading = true;
                current.error = None;
            }

            let api_base = state().api_base.clone();
            match authenticate_and_load(&api_base, "login", &email_value, &password_value).await {
                Ok(bundle) => {
                    {
                        let mut s = state.write();
                        s.token = Some(bundle.token);
                        s.profile = Some(bundle.profile);
                        s.invoices = bundle.invoices;
                        s.tickets = bundle.tickets;
                        s.loading = false;
                        s.error = None;
                    }
                    persist_authenticated_session(&state());
                    nav.push(Route::ProfilePage {});
                }
                Err(err) => {
                    let mut s = state.write();
                    s.loading = false;
                    s.error = Some(err);
                }
            }
        });
    };

    let on_register = move |_| {
        let email_value = email();
        let password_value = password();
        let nav = navigator.clone();
        let mut state = session;

        spawn(async move {
            {
                let mut current = state.write();
                current.loading = true;
                current.error = None;
            }

            let api_base = state().api_base.clone();
            match authenticate_and_load(&api_base, "register", &email_value, &password_value).await
            {
                Ok(bundle) => {
                    {
                        let mut s = state.write();
                        s.token = Some(bundle.token);
                        s.profile = Some(bundle.profile);
                        s.invoices = bundle.invoices;
                        s.tickets = bundle.tickets;
                        s.loading = false;
                        s.error = None;
                    }
                    persist_authenticated_session(&state());
                    nav.push(Route::ProfilePage {});
                }
                Err(err) => {
                    let mut s = state.write();
                    s.loading = false;
                    s.error = Some(err);
                }
            }
        });
    };

    rsx! {
        div { class: "public-shell",
            header { class: "public-header",
                h1 { "Login Required" }
                button {
                    r#type: "button",
                    class: "btn-secondary",
                    onclick: move |_| {
                        navigator.push(Route::StorefrontPage {});
                    },
                    "Back"
                }
            }
            main { class: "public-main",
                section { class: "checkout-card",
                    h3 { "Sign in to continue checkout" }
                    p { "You can login with an existing account, or register a new user directly." }

                    if let Some(notice) = auth_transport_notice(&session().api_base) {
                        p { class: "notice", "{notice}" }
                    }

                    div { class: "order-meta",
                        label { "Email" }
                        input {
                            r#type: "email",
                            value: "{email()}",
                            placeholder: "you@example.com",
                            autocomplete: "email",
                            oninput: move |evt| email.set(evt.value()),
                        }
                    }

                    div { class: "order-meta",
                        label { "Password" }
                        input {
                            r#type: "password",
                            value: "{password()}",
                            placeholder: "password",
                            autocomplete: "current-password",
                            oninput: move |evt| password.set(evt.value()),
                        }
                    }

                    if let Some(err) = &session().error {
                        p { class: "notice", "{err}" }
                    }

                    if session().loading {
                        p { class: "muted", "正在验证账号并同步用户数据..." }
                    }

                    button {
                        r#type: "button",
                        class: "btn-primary full",
                        disabled: session().loading
                            || matches!(
                                auth_transport_risk(&session().api_base),
                                AuthTransportRisk::InsecureRemote
                            ),
                        onclick: on_login,
                        if session().loading {
                            "Loading..."
                        } else {
                            "Login"
                        }
                    }
                    button {
                        r#type: "button",
                        class: "btn-secondary full mt-8",
                        disabled: session().loading
                            || matches!(
                                auth_transport_risk(&session().api_base),
                                AuthTransportRisk::InsecureRemote
                            ),
                        onclick: on_register,
                        "Register"
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
fn ProfilePage() -> Element {
    let session = use_context::<Signal<SessionState>>();
    let state = session();

    if state.token.is_none() {
        return rsx! {
            LoginRequiredView {}
        };
    }

    let profile = state.profile.clone();
    let user_id = profile
        .as_ref()
        .map(|p| p.user_id.clone())
        .unwrap_or_else(|| "-".to_string());
    let user_email = profile
        .as_ref()
        .map(|p| p.email.clone())
        .unwrap_or_else(|| "-".to_string());
    let user_role = profile
        .as_ref()
        .map(|p| p.role.clone())
        .unwrap_or_else(|| "user".to_string());

    rsx! {
        DashboardShell { title: "User Information", active_tab: DashboardTab::Profile,
            section { class: "grid-two",
                article { class: "panel",
                    h3 { "Account" }
                    p { class: "muted", "User ID" }
                    p { class: "fact", "{user_id}" }
                    p { class: "muted", "Email" }
                    p { class: "fact", "{user_email}" }
                    p { class: "muted", "Role" }
                    p { class: "fact", "{user_role}" }
                }
                article { class: "panel",
                    h3 { "Portal Status" }
                    p { class: "muted", "Ticket Count" }
                    p { class: "fact", "{state.tickets.len()}" }
                    p { class: "muted", "Invoice Count" }
                    p { class: "fact", "{state.invoices.len()}" }
                    p { class: "muted", "Session" }
                    p { class: "fact", "Authenticated" }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
fn ServicesPage() -> Element {
    let session = use_context::<Signal<SessionState>>();

    if session().token.is_none() {
        return rsx! {
            LoginRequiredView {}
        };
    }

    rsx! {
        DashboardShell { title: "My Services", active_tab: DashboardTab::Services,
            section { class: "panel",
                h3 { "Active Instances" }
                div { class: "service-list",
                    article { class: "service-item",
                        div {
                            h4 { "US-NY NAT Standard" }
                            p { class: "muted", "Expires: 2026-04-30" }
                        }
                        span { class: "pill paid", "Running" }
                    }
                    article { class: "service-item",
                        div {
                            h4 { "HK NAT Mini" }
                            p { class: "muted", "Expires: 2026-05-12" }
                        }
                        span { class: "pill pending", "Pending Payment" }
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
fn TicketsPage() -> Element {
    let session = use_context::<Signal<SessionState>>();
    let state = session();

    if state.token.is_none() {
        return rsx! {
            LoginRequiredView {}
        };
    }

    rsx! {
        DashboardShell { title: "Ticket Center", active_tab: DashboardTab::Tickets,
            section { class: "panel",
                h3 { "Recent Tickets" }
                table {
                    thead {
                        tr {
                            th { "ID" }
                            th { "Title" }
                            th { "Category" }
                            th { "Priority" }
                            th { "Status" }
                        }
                    }
                    tbody {
                        if state.tickets.is_empty() {
                            tr {
                                td { colspan: "5", "No tickets found" }
                            }
                        } else {
                            for item in &state.tickets {
                                tr {
                                    td { "{item.id}" }
                                    td { "{item.subject}" }
                                    td { "{item.category}" }
                                    td { "{item.priority}" }
                                    td {
                                        span { class: if item.status.eq_ignore_ascii_case("open") { "pill pending" } else { "pill paid" },
                                            "{item.status}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
fn BalancePage() -> Element {
    let session = use_context::<Signal<SessionState>>();
    let state = session();

    if state.token.is_none() {
        return rsx! {
            LoginRequiredView {}
        };
    }

    let balance_value = state
        .invoices
        .iter()
        .filter_map(|item| item.amount.parse::<f64>().ok())
        .sum::<f64>();
    let amount_text = format!("$ {balance_value:.2}");

    rsx! {
        DashboardShell { title: "Balance & Finance", active_tab: DashboardTab::Balance,
            section { class: "balance-card",
                p { class: "muted", "Invoice Total" }
                div { class: "amount", "{amount_text}" }
            }

            section { class: "table-card",
                div { class: "tab-strip",
                    button { class: "tab active", "Invoice Records" }
                }

                table {
                    thead {
                        tr {
                            th { "ID" }
                            th { "Amount" }
                            th { "Status" }
                            th { "Due At" }
                        }
                    }
                    tbody {
                        if state.invoices.is_empty() {
                            tr {
                                td { colspan: "4", "No invoices found" }
                            }
                        } else {
                            for item in &state.invoices {
                                tr {
                                    td { "{item.id}" }
                                    td { "$ {item.amount}" }
                                    td {
                                        span { class: if item.status.eq_ignore_ascii_case("paid") { "pill paid" } else { "pill pending" },
                                            "{item.status}"
                                        }
                                    }
                                    td { "{item.due_at}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
fn DashboardShell(title: &'static str, active_tab: DashboardTab, children: Element) -> Element {
    let navigator = use_navigator();
    let mut session = use_context::<Signal<SessionState>>();

    rsx! {
        div { class: "layout",
            aside { class: "sidebar",
                div { class: "logo",
                    div { class: "logo-mark", "C" }
                    div { class: "logo-text",
                        h1 { "Cloud Store" }
                        p { "Customer Center" }
                    }
                }
                nav { class: "menu",
                    Link {
                        class: if active_tab == DashboardTab::Profile { "menu-item active" } else { "menu-item" },
                        to: Route::ProfilePage {},
                        "User Info"
                    }
                    Link {
                        class: if active_tab == DashboardTab::Services { "menu-item active" } else { "menu-item" },
                        to: Route::ServicesPage {},
                        "My Services"
                    }
                    Link {
                        class: if active_tab == DashboardTab::Tickets { "menu-item active" } else { "menu-item" },
                        to: Route::TicketsPage {},
                        "Tickets"
                    }
                    Link {
                        class: if active_tab == DashboardTab::Balance { "menu-item active" } else { "menu-item" },
                        to: Route::BalancePage {},
                        "Balance"
                    }
                }
            }

            main { class: "content",
                header { class: "topbar",
                    h2 { "{title}" }
                    div { class: "top-actions",
                        button {
                            class: "btn-secondary",
                            onclick: move |_| {
                                navigator.push(Route::StorefrontPage {});
                            },
                            "Store"
                        }
                        button {
                            class: "btn-secondary",
                            onclick: move |_| {
                                let mut s = session.write();
                                s.token = None;
                                s.profile = None;
                                s.invoices.clear();
                                s.tickets.clear();
                                s.error = None;
                                s.loading = false;
                                clear_persisted_session();
                                navigator.push(Route::StorefrontPage {});
                            },
                            "Logout"
                        }
                    }
                }
                {children}
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
fn LoginRequiredView() -> Element {
    let navigator = use_navigator();

    rsx! {
        div { class: "public-shell",
            main { class: "public-main",
                section { class: "checkout-card",
                    h3 { "Login Required" }
                    p { "This page is only available after login." }
                    button {
                        class: "btn-primary",
                        onclick: move |_| {
                            navigator.push(Route::LoginPage {});
                        },
                        "Go To Login"
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn persist_authenticated_session(session: &SessionState) {
    let Some(storage) = browser_storage() else {
        return;
    };

    if let Some(token) = &session.token {
        let _ = storage.set_item(AUTH_TOKEN_KEY, token);
    }

    if let Some(profile) = &session.profile {
        if let Ok(serialized) = serde_json::to_string(profile) {
            let _ = storage.set_item(AUTH_PROFILE_KEY, &serialized);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn load_persisted_session() -> Option<(String, AuthProfileResponse)> {
    let storage = browser_storage()?;
    let token = storage.get_item(AUTH_TOKEN_KEY).ok().flatten()?;
    let profile = storage.get_item(AUTH_PROFILE_KEY).ok().flatten()?;
    let profile = serde_json::from_str::<AuthProfileResponse>(&profile).ok()?;

    Some((token, profile))
}

#[cfg(target_arch = "wasm32")]
fn clear_persisted_session() {
    let Some(storage) = browser_storage() else {
        return;
    };

    let _ = storage.remove_item(AUTH_TOKEN_KEY);
    let _ = storage.remove_item(AUTH_PROFILE_KEY);
}

#[cfg(target_arch = "wasm32")]
fn browser_storage() -> Option<web_sys::Storage> {
    window()?.local_storage().ok().flatten()
}

#[cfg(target_arch = "wasm32")]
struct BootstrapBundle {
    token: String,
    profile: AuthProfileResponse,
    invoices: Vec<InvoiceItem>,
    tickets: Vec<TicketItem>,
}

#[cfg(target_arch = "wasm32")]
async fn authenticate_and_load(
    api_base: &str,
    endpoint: &str,
    email: &str,
    password: &str,
) -> Result<BootstrapBundle, String> {
    if email.trim().is_empty() || password.trim().is_empty() {
        return Err("email and password are required".to_string());
    }

    if matches!(
        auth_transport_risk(api_base),
        AuthTransportRisk::InsecureRemote
    ) {
        return Err(
            "当前 API 不是 HTTPS，出于安全原因已禁止提交账号信息。请先切换到 HTTPS。".to_string(),
        );
    }

    let auth = AuthPayload {
        email: email.trim().to_string(),
        password: password.to_string(),
    };

    let url = format!("{api_base}/api/auth/{endpoint}");
    let resp = Request::post(&url)
        .header("Content-Type", "application/json")
        .json(&auth)
        .map_err(|e| format!("failed to build auth request: {e}"))?
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    if !resp.ok() {
        let status = resp.status();
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "auth failed".to_string());
        return Err(format!("auth failed ({status}): {body}"));
    }

    let auth_result = resp
        .json::<AuthTokenResponse>()
        .await
        .map_err(|e| format!("failed to parse auth response: {e}"))?;

    let token = auth_result.token;
    let profile = fetch_profile(api_base, &token).await?;
    let invoices = fetch_invoices(api_base, &token).await?;
    let tickets = fetch_tickets(api_base, &token).await?;

    Ok(BootstrapBundle {
        token,
        profile,
        invoices,
        tickets,
    })
}

#[cfg(target_arch = "wasm32")]
async fn load_authenticated_bundle(api_base: &str, token: &str) -> Result<BootstrapBundle, String> {
    let profile = fetch_profile(api_base, token).await?;
    let invoices = fetch_invoices(api_base, token).await?;
    let tickets = fetch_tickets(api_base, token).await?;

    Ok(BootstrapBundle {
        token: token.to_string(),
        profile,
        invoices,
        tickets,
    })
}

#[cfg(target_arch = "wasm32")]
async fn fetch_profile(api_base: &str, token: &str) -> Result<AuthProfileResponse, String> {
    let url = format!("{api_base}/api/auth/me");
    let resp = Request::get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("failed to load profile: {e}"))?;

    if !resp.ok() {
        return Err(format!(
            "profile request failed with status {}",
            resp.status()
        ));
    }

    resp.json::<AuthProfileResponse>()
        .await
        .map_err(|e| format!("failed to parse profile response: {e}"))
}

#[cfg(target_arch = "wasm32")]
async fn fetch_invoices(api_base: &str, token: &str) -> Result<Vec<InvoiceItem>, String> {
    let url = format!("{api_base}/api/invoices");
    let resp = Request::get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("failed to load invoices: {e}"))?;

    if !resp.ok() {
        return Err(format!(
            "invoice request failed with status {}",
            resp.status()
        ));
    }

    resp.json::<Vec<InvoiceItem>>()
        .await
        .map_err(|e| format!("failed to parse invoices response: {e}"))
}

#[cfg(target_arch = "wasm32")]
async fn fetch_tickets(api_base: &str, token: &str) -> Result<Vec<TicketItem>, String> {
    let url = format!("{api_base}/api/tickets");
    let resp = Request::get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("failed to load tickets: {e}"))?;

    if !resp.ok() {
        return Err(format!(
            "ticket request failed with status {}",
            resp.status()
        ));
    }

    resp.json::<Vec<TicketItem>>()
        .await
        .map_err(|e| format!("failed to parse tickets response: {e}"))
}
