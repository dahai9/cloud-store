use chrono::{DateTime, Utc};

use crate::api;

use crate::models::{DashboardTab, Route, SessionState};

use dioxus::prelude::*;

use gloo_timers::future::TimeoutFuture;

#[component]
pub fn ProfilePage() -> Element {
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

#[component]
pub fn ServicesPage() -> Element {
    let session = use_context::<Signal<SessionState>>();
    let state = session();

    if state.token.is_none() {
        return rsx! {
            LoginRequiredView {}
        };
    }

    rsx! {
        DashboardShell { title: "My Services", active_tab: DashboardTab::Services,
            section { class: "panel",
                h3 { "Active Instances" }
                div { class: "service-list",
                    if state.instances.is_empty() {
                        p { class: "muted", "You don't have any active instances yet." }
                    } else {
                        for item in &state.instances {
                            Link {
                                to: Route::InstanceDetailPage { id: item.id.clone() },
                                article { class: "service-item",
                                    div {
                                        h4 { "{item.plan_id.to_uppercase()} - {item.id}" }
                                        p { class: "muted", "Created: {item.created_at}" }
                                    }
                                    span { class: format!("pill {}", instance_status_class(&item.status)),
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

fn instance_status_class(status: &str) -> &'static str {
    match status.to_lowercase().as_str() {
        "running" => "paid",
        "stopped" => "expired",
        "pending" | "starting" => "pending",
        _ => "pending",
    }
}

#[component]
pub fn InstanceDetailPage(id: String) -> Element {
    let session = use_context::<Signal<SessionState>>();
    let state = session();
    let navigator = use_navigator();

    if state.token.is_none() {
        return rsx! {
            LoginRequiredView {}
        };
    }

    let token = state.token.clone().unwrap();
    let api_base = state.api_base.clone();

    let mut instance = use_signal(|| None::<crate::models::InstanceItem>);
    let mut metrics = use_signal(|| None::<crate::models::InstanceMetrics>);
    let mut error = use_signal(|| None::<String>);
    let mut action_loading = use_signal(|| false);

    // Initial load
    use_effect({
        let id = id.clone();
        let api_base = api_base.clone();
        let token = token.clone();
        move || {
            let id = id.clone();
            let api_base = api_base.clone();
            let token = token.clone();
            spawn(async move {
                match api::fetch_instance_details(&api_base, &token, &id).await {
                    Ok(data) => instance.set(Some(data)),
                    Err(err) => error.set(Some(err)),
                }
            });
        }
    });

    // Periodic metrics update
    use_effect({
        let id = id.clone();
        let api_base = api_base.clone();
        let token = token.clone();
        move || {
            let id = id.clone();
            let api_base = api_base.clone();
            let token = token.clone();
            spawn(async move {
                loop {
                    match api::fetch_instance_metrics(&api_base, &token, &id).await {
                        Ok(data) => metrics.set(Some(data)),
                        Err(_) => {}
                    }
                    TimeoutFuture::new(5000).await;
                }
            });
        }
    });

    let on_action = use_callback({
        let id = id.clone();
        let api_base = api_base.clone();
        let token = token.clone();
        move |action: crate::models::InstanceAction| {
            let id = id.clone();
            let api_base = api_base.clone();
            let token = token.clone();
            spawn(async move {
                action_loading.set(true);
                match api::perform_instance_action(&api_base, &token, &id, action).await {
                    Ok(_) => {
                        // Refresh details after a short delay
                        TimeoutFuture::new(1000).await;
                        if let Ok(data) = api::fetch_instance_details(&api_base, &token, &id).await
                        {
                            instance.set(Some(data));
                        }
                    }
                    Err(err) => error.set(Some(err)),
                }
                action_loading.set(false);
            });
        }
    });

    let on_console = {
        let id = id.clone();
        let api_base = api_base.clone();
        let token = token.clone();
        move |_| {
            let id = id.clone();
            let api_base = api_base.clone();
            let token = token.clone();
            spawn(async move {
                match api::fetch_instance_console(&api_base, &token, &id).await {
                    Ok(console) => {
                        #[cfg(target_arch = "wasm32")]
                        {
                            if let Some(win) = web_sys::window() {
                                let url = format!("{}?token={}", console.url, console.token);
                                let _ = win.open_with_url_and_target(&url, "_blank");
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let _ = console;
                            error.set(Some("Console only available in web browser".to_string()));
                        }
                    }
                    Err(err) => error.set(Some(err)),
                }
            });
        }
    };

    let Some(inst) = instance() else {
        return rsx! {
            DashboardShell { title: "Instance Details", active_tab: DashboardTab::Services,
                div { class: "panel",
                    if let Some(err) = error() {
                        p { class: "notice error-notice", "{err}" }
                    } else {
                        p { "Loading instance details..." }
                    }
                }
            }
        };
    };

    rsx! {
        DashboardShell { title: "Manage Instance", active_tab: DashboardTab::Services,
            div { class: "instance-detail-header",
                button {
                    class: "btn-secondary",
                    onclick: move |_| {
                        navigator.push(Route::ServicesPage {});
                    },
                    "← Back to List"
                }
                h3 { "{inst.plan_id.to_uppercase()} ({inst.id})" }
            }

            if let Some(err) = error() {
                p { class: "notice error-notice", "{err}" }
            }

            section { class: "grid-two",
                article { class: "panel",
                    h4 { "Status & Info" }
                    div { class: "detail-list",
                        div { class: "detail-item",
                            span { class: "muted", "Status" }
                            span { class: format!("pill {}", instance_status_class(&inst.status)),
                                "{inst.status}"
                            }
                        }
                        div { class: "detail-item",
                            span { class: "muted", "Node" }
                            span { class: "fact", "{inst.node_id}" }
                        }
                        div { class: "detail-item",
                            span { class: "muted", "OS Template" }
                            span { class: "fact", "{inst.os_template}" }
                        }
                        div { class: "detail-item",
                            span { class: "muted", "Created At" }
                            span { class: "fact", "{inst.created_at}" }
                        }
                    }
                }

                article { class: "panel",
                    h4 { "Real-time Metrics" }
                    if let Some(m) = metrics() {
                        div { class: "metrics-grid",
                            div { class: "metric-card",
                                p { class: "muted", "CPU" }
                                p { class: "fact", "{m.cpu_usage_percent:.1}%" }
                            }
                            div { class: "metric-card",
                                p { class: "muted", "RAM" }
                                p { class: "fact", "{m.memory_used_mb:.0} MB" }
                            }
                            div { class: "metric-card",
                                p { class: "muted", "Net TX" }
                                p { class: "fact", "{m.network_tx_bytes / 1024} KB" }
                            }
                            div { class: "metric-card",
                                p { class: "muted", "Net RX" }
                                p { class: "fact", "{m.network_rx_bytes / 1024} KB" }
                            }
                        }
                    } else {
                        p { class: "muted", "Loading metrics..." }
                    }
                }
            }

            section { class: "panel",
                h4 { "Actions" }
                div { class: "action-bar",
                    button {
                        class: "btn-primary",
                        disabled: action_loading(),
                        onclick: move |_| on_action(crate::models::InstanceAction::Start),
                        "Start"
                    }
                    button {
                        class: "btn-secondary",
                        disabled: action_loading(),
                        onclick: move |_| on_action(crate::models::InstanceAction::Stop),
                        "Stop"
                    }
                    button {
                        class: "btn-secondary",
                        disabled: action_loading(),
                        onclick: move |_| on_action(crate::models::InstanceAction::Restart),
                        "Restart"
                    }
                    button {
                        class: "btn-secondary",
                        disabled: action_loading(),
                        onclick: move |_| on_action(crate::models::InstanceAction::Reinstall { os_template: None }),
                        "Reinstall"
                    }
                    button {
                        class: "btn-secondary",
                        disabled: action_loading(),
                        onclick: move |_| on_action(crate::models::InstanceAction::ResetPassword { new_password: None }),
                        "Reset Password"
                    }
                    button {
                        class: "btn-primary",
                        onclick: on_console,
                        "Open Console (VNC)"
                    }
                }
            }
        }
    }
}

#[component]
pub fn TicketsPage() -> Element {
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

#[component]
pub fn BalancePage() -> Element {
    let session = use_context::<Signal<SessionState>>();
    let state = session();
    let mut selected_invoice_id = use_signal(|| None::<String>);
    let mut modal_closing = use_signal(|| false);
    let mut modal_origin = use_signal(|| "50% 55%".to_string());
    let payment_loading = use_signal(|| false);
    let payment_error = use_signal(|| None::<String>);

    if state.token.is_none() {
        return rsx! {
            LoginRequiredView {}
        };
    }

    let invoices = state.invoices.clone();
    let selected_invoice = selected_invoice_id()
        .as_ref()
        .and_then(|invoice_id| invoices.iter().find(|item| item.id == *invoice_id))
        .cloned();

    let invoice_rows: Vec<(crate::models::InvoiceItem, String)> = invoices
        .iter()
        .cloned()
        .map(|item| {
            let invoice_id = item.id.clone();
            (item, invoice_id)
        })
        .collect();

    let balance_value = invoices
        .iter()
        .filter_map(|item| item.amount.parse::<f64>().ok())
        .sum::<f64>();
    let amount_text = format!("$ {balance_value:.2}");
    let now = Utc::now();

    let selected_order_id = selected_invoice
        .as_ref()
        .and_then(|invoice| invoice.order_id.clone())
        .unwrap_or_else(|| "-".to_string());
    let selected_external_payment_ref = selected_invoice
        .as_ref()
        .and_then(|invoice| invoice.external_payment_ref.clone())
        .unwrap_or_else(|| "-".to_string());
    let selected_paid_at = selected_invoice
        .as_ref()
        .and_then(|invoice| invoice.paid_at.clone())
        .unwrap_or_else(|| "-".to_string());
    let modal_status = selected_invoice
        .as_ref()
        .map(|invoice| invoice_status_label(invoice, now))
        .unwrap_or_else(|| "open".to_string());
    let modal_can_repay = selected_invoice
        .as_ref()
        .map(|invoice| invoice_is_payable(invoice, now))
        .unwrap_or(false);
    let repay_invoice = selected_invoice.clone();
    let backdrop_class = if modal_closing() {
        "modal-backdrop closing"
    } else {
        "modal-backdrop"
    };
    let modal_class = if modal_closing() {
        "invoice-modal closing"
    } else {
        "invoice-modal"
    };
    let modal_style = format!("--modal-origin: {};", modal_origin());

    let on_repay = move |_| {
        let Some(invoice) = repay_invoice.clone() else {
            return;
        };

        let Some(token) = state.token.clone() else {
            return;
        };

        let api_base = state.api_base.clone();
        let mut loading = payment_loading;
        let mut error = payment_error;

        spawn(async move {
            loading.set(true);
            error.set(None);

            match api::retry_paypal_invoice(&api_base, &token, &invoice.id).await {
                Ok(response) => {
                    loading.set(false);
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(win) = web_sys::window() {
                            if win.location().set_href(&response.approval_url).is_err() {
                                error.set(Some("无法打开 PayPal 沙箱支付页面".to_string()));
                            }
                        } else {
                            error.set(Some("浏览器窗口不可用".to_string()));
                        }
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let _ = response;
                        error.set(Some(
                            "当前平台暂不支持直接打开支付链接，请在 Web 端操作".to_string(),
                        ));
                    }
                }
                Err(err) => {
                    loading.set(false);
                    error.set(Some(err));
                }
            }
        });
    };

    rsx! {
        DashboardShell { title: "Balance & Finance", active_tab: DashboardTab::Balance,
            section { class: "balance-card",
                p { class: "muted", "Invoice Total" }
                div { class: "amount", "{amount_text}" }
            }

            section { class: "balance-layout",
                article { class: "table-card",
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
                                th { "" }
                            }
                        }
                        tbody {
                            if state.invoices.is_empty() {
                                tr {
                                    td { colspan: "5", "No invoices found" }
                                }
                            } else {
                                for (item , invoice_id) in invoice_rows.iter().cloned() {
                                    tr {
                                        class: if selected_invoice.as_ref().map(|selected| selected.id == item.id).unwrap_or(false) { "invoice-row active" } else { "invoice-row" },
                                        onclick: {
                                            let invoice_id = invoice_id.clone();
                                            move |event| {
                                                modal_closing.set(false);
                                                let coords = event.data().client_coordinates();
                                                modal_origin.set(modal_origin_from_client(coords.x as f64, coords.y as f64));
                                                selected_invoice_id.set(Some(invoice_id.clone()));
                                            }
                                        },
                                        td { "{item.id}" }
                                        td { "$ {item.amount}" }
                                        td {
                                            span { class: invoice_pill_class(&invoice_status_label(&item, now)),
                                                "{invoice_status_label(&item, now)}"
                                            }
                                        }
                                        td { "{item.due_at}" }
                                        td {
                                            button {
                                                class: "btn-secondary invoice-action",
                                                onclick: {
                                                    let invoice_id = invoice_id.clone();
                                                    move |event| {
                                                        modal_closing.set(false);
                                                        let coords = event.data().client_coordinates();
                                                        modal_origin.set(modal_origin_from_client(coords.x as f64, coords.y as f64));
                                                        selected_invoice_id.set(Some(invoice_id.clone()));
                                                    }
                                                },
                                                "Details"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(err) = &*payment_error.read() {
                p { class: "notice error-notice", "{err}" }
            }

            if let Some(invoice) = selected_invoice {
                div {
                    class: "{backdrop_class}",
                    onclick: move |_| {
                        if modal_closing() {
                            return;
                        }
                        modal_closing.set(true);
                        spawn(async move {
                            TimeoutFuture::new(220).await;
                            selected_invoice_id.set(None);
                            modal_closing.set(false);
                        });
                    },
                    div {
                        class: "{modal_class}",
                        style: "{modal_style}",
                        onclick: move |event| event.stop_propagation(),
                        div { class: "modal-header",
                            div {
                                p { class: "muted modal-kicker", "Invoice Details" }
                                h3 { "{invoice.id}" }
                            }
                            button {
                                class: "btn-secondary modal-close",
                                onclick: move |_| {
                                    if modal_closing() {
                                        return;
                                    }
                                    modal_closing.set(true);
                                    spawn(async move {
                                        TimeoutFuture::new(220).await;
                                        selected_invoice_id.set(None);
                                        modal_closing.set(false);
                                    });
                                },
                                "Close"
                            }
                        }

                        div { class: "detail-grid",
                            article { class: "detail-item",
                                p { class: "muted", "Status" }
                                span { class: invoice_pill_class(&modal_status), "{modal_status}" }
                            }
                            article { class: "detail-item",
                                p { class: "muted", "Amount" }
                                p { class: "fact", "$ {invoice.amount}" }
                            }
                            article { class: "detail-item",
                                p { class: "muted", "Order ID" }
                                p { class: "fact", "{selected_order_id}" }
                            }
                            article { class: "detail-item",
                                p { class: "muted", "External Payment Ref" }
                                p { class: "fact", "{selected_external_payment_ref}" }
                            }
                            article { class: "detail-item",
                                p { class: "muted", "Created At" }
                                p { class: "fact", "{invoice.created_at}" }
                            }
                            article { class: "detail-item",
                                p { class: "muted", "Due At" }
                                p { class: "fact", "{invoice.due_at}" }
                            }
                            article { class: "detail-item",
                                p { class: "muted", "Paid At" }
                                p { class: "fact", "{selected_paid_at}" }
                            }
                        }

                        div { class: "modal-actions",
                            if modal_can_repay {
                                button {
                                    class: "btn-primary full",
                                    disabled: *payment_loading.read(),
                                    onclick: on_repay,
                                    if *payment_loading.read() {
                                        "Preparing checkout..."
                                    } else {
                                        "Pay Again"
                                    }
                                }
                            } else if modal_status == "expired" {
                                p { class: "notice",
                                    "This invoice is expired and can no longer be paid."
                                }
                            } else if modal_status == "paid" {
                                p { class: "notice", "This invoice has already been paid." }
                            }

                            p { class: "muted modal-note",
                                "This invoice can be repaid until 24 hours after creation, subject to inventory availability."
                            }
                        }
                    }
                }
            }
        }
    }
}

fn modal_origin_from_client(client_x: f64, client_y: f64) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(win) = web_sys::window() else {
            return "50% 55%".to_string();
        };

        let viewport_width = win
            .inner_width()
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(1280.0)
            .max(1.0);
        let viewport_height = win
            .inner_height()
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(720.0)
            .max(1.0);

        let x = ((client_x / viewport_width) * 100.0).clamp(8.0, 92.0);
        let y = ((client_y / viewport_height) * 100.0).clamp(12.0, 88.0);

        format!("{x:.2}% {y:.2}%")
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (client_x, client_y);
        "50% 55%".to_string()
    }
}

fn invoice_status_label(invoice: &crate::models::InvoiceItem, now: DateTime<Utc>) -> String {
    if invoice.status.eq_ignore_ascii_case("open") && invoice_is_overdue(invoice, now) {
        "expired".to_string()
    } else {
        invoice.status.clone()
    }
}

fn invoice_is_payable(invoice: &crate::models::InvoiceItem, now: DateTime<Utc>) -> bool {
    invoice.status.eq_ignore_ascii_case("open") && !invoice_is_overdue(invoice, now)
}

fn invoice_is_overdue(invoice: &crate::models::InvoiceItem, now: DateTime<Utc>) -> bool {
    chrono::DateTime::parse_from_rfc3339(&invoice.due_at)
        .map(|due_at| due_at.with_timezone(&Utc) <= now)
        .unwrap_or(false)
}

fn invoice_pill_class(status: &str) -> &'static str {
    if status.eq_ignore_ascii_case("paid") {
        "pill paid"
    } else if status.eq_ignore_ascii_case("expired") {
        "pill expired"
    } else {
        "pill pending"
    }
}

#[component]
pub fn DashboardShell(title: &'static str, active_tab: DashboardTab, children: Element) -> Element {
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
                                api::clear_persisted_session();
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

#[component]
pub fn LoginRequiredView() -> Element {
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
                            navigator
                                .push(Route::LoginPage {
                                    source: Some("protected".to_string()),
                                    plan: None,
                                });
                        },
                        "Go To Login"
                    }
                }
            }
        }
    }
}
