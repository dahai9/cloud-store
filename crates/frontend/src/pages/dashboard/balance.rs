use crate::api;
use crate::models::{DashboardTab, SessionState};
use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use dioxus_i18n::t;
use dioxus_motion::prelude::*;
use gloo_timers::future::TimeoutFuture;

use super::shell::{DashboardShell, LoginRequiredView};

#[component]
pub fn BalancePage() -> Element {
    let session = use_context::<Signal<SessionState>>();
    let state = session();
    #[allow(unused_mut)]
    let mut selected_invoice_id = use_signal(|| None::<String>);
    #[allow(unused_mut)]
    let mut modal_closing = use_signal(|| false);
    #[allow(unused_mut)]
    let mut modal_origin = use_signal(|| "50% 55%".to_string());
    #[allow(unused_mut)]
    let mut payment_loading = use_signal(|| false);
    #[allow(unused_mut)]
    let mut payment_error = use_signal(|| None::<String>);
    #[allow(unused_mut)]
    let mut recharge_amount = use_signal(|| "10.00".to_string());

    let mut opacity = use_motion(0.0f32);
    let mut slide_y = use_motion(20.0f32);

    use_effect(move || {
        opacity.animate_to(
            1.0,
            AnimationConfig::new(AnimationMode::Spring(Spring::default())),
        );
        slide_y.animate_to(
            0.0,
            AnimationConfig::new(AnimationMode::Spring(Spring::default())),
        );
    });

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

    let balance_text = format!("$ {}", state.balance);
    let now = Utc::now();

    let on_recharge = move |_| {
        let api_base = session.peek().api_base.clone();
        let token = session.peek().token.clone().unwrap_or_default();
        let amount = recharge_amount();
        let mut loading = payment_loading;
        let mut error = payment_error;

        spawn(async move {
            *loading.write() = true;
            *error.write() = None;
            match api::recharge_balance(&api_base, &token, &amount).await {
                Ok(response) => {
                    *loading.write() = false;
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(win) = web_sys::window() {
                            if win.location().set_href(&response.approval_url).is_err() {
                                *error.write() = Some("无法打开 PayPal 支付页面".to_string());
                            }
                        }
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let _ = response;
                        *error.write() = Some("请在浏览器中完成支付".to_string());
                    }
                }
                Err(e) => {
                    *loading.write() = false;
                    *error.write() = Some(e);
                }
            }
        });
    };

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
            *loading.write() = true;
            *error.write() = None;

            match api::retry_paypal_invoice(&api_base, &token, &invoice.id).await {
                Ok(response) => {
                    *loading.write() = false;
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(win) = web_sys::window() {
                            if win.location().set_href(&response.approval_url).is_err() {
                                *error.write() = Some("无法打开 PayPal 沙箱支付页面".to_string());
                            }
                        } else {
                            *error.write() = Some("浏览器窗口不可用".to_string());
                        }
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let _ = response;
                        *error.write() =
                            Some("当前平台暂不支持直接打开支付链接，请在 Web 端操作".to_string());
                    }
                }
                Err(err) => {
                    *loading.write() = false;
                    *error.write() = Some(err);
                }
            }
        });
    };

    let on_refund = move |order_id: String| {
        let api_base = session.peek().api_base.clone();
        let token = session.peek().token.clone().unwrap_or_default();
        let mut session = session;

        spawn(async move {
            match api::refund_failed_order(&api_base, &token, &order_id).await {
                Ok(_) => {
                    // Refresh balance and transactions
                    if let Ok(bundle) = api::load_authenticated_bundle(&api_base, &token).await {
                        let mut s = session.write();
                        s.balance = bundle.balance;
                        s.balance_transactions = bundle.balance_transactions;
                    }
                }
                Err(e) => {
                    session.write().error = Some(e);
                }
            }
        });
    };

    rsx! {
            DashboardShell { title: "{t!(\"dash_balance_finance\")}", active_tab: DashboardTab::Balance,
                div {
                    class: "balance-page-content",
                    style: "opacity: {opacity.get_value()}; transform: translateY({slide_y.get_value()}px);",
                    section { class: "balance-card",
                    div { class: "balance-info-main",
                        p { class: "muted", "{t!(\"dash_available_balance\")}" }
                        div { class: "amount", "{balance_text}" }
                    }
                    div { class: "recharge-controls",
                        div { class: "amount-input-wrapper",
                            span { class: "currency-symbol", "$" }
                            input {
                                class: "recharge-input",
                                r#type: "number",
                                step: "1.00",
                                min: "1.00",
                                value: "{recharge_amount}",
                                oninput: move |ev| *recharge_amount.write() = ev.value(),
                            }
                        }
                        div { class: "preset-amounts",
                            for amt in ["10.00", "50.00", "100.00"] {
                                button {
                                    class: if recharge_amount() == amt { "preset-btn active" } else { "preset-btn" },
                                    onclick: move |_| *recharge_amount.write() = amt.to_string(),
                                    "${amt}"
                                }
                            }
                        }
                        button {
                            class: "btn-primary recharge-btn",
                            disabled: *payment_loading.read(),
                            onclick: on_recharge,
                            if *payment_loading.read() {
                                "{t!(\"dash_preparing_checkout\")}"
                            } else {
                                "{t!(\"dash_recharge_btn\")}"
                            }
                        }
                    }
                }

                section { class: "balance-layout",
                    article { class: "table-card",
                        div { class: "tab-strip",
                            button { class: "tab active", "{t!(\"dash_transaction_history\")}" }
                        }

                        table {
                            thead {
                                tr {
                                    th { "{t!(\"dash_date_col\")}" }
                                    th { "{t!(\"dash_type_col\")}" }
                                    th { "{t!(\"dash_amount_col\")}" }
                                    th { "{t!(\"dash_desc_col\")}" }
                                    th { "{t!(\"dash_status\")}" }
                                    th { "" }
                                }
                            }
                            tbody {
                                if state.balance_transactions.is_empty() {
                                    tr {
                                        td { colspan: "6", "{t!(\"dash_no_transactions\")}" }
                                    }
                                } else {
                                    for tx in &state.balance_transactions {
                                        tr {
                                            td { "{tx.created_at}" }
                                            td { "{tx.r#type}" }
                                            td {
                                                span {
                                                    class: if tx.amount.starts_with('-') { "text-danger" } else { "text-success" },
                                                    "{tx.amount}"
                                                }
                                            }
                                            td { "{tx.description}" }
                                            td {
                                                if let Some(status) = &tx.order_status {
                                                    span { class: format!("pill {}", match status.as_str() {
                                                        "active" => "status-running",
                                                        "failed" => "status-deleted",
                                                        "refunded" => "status-stopped",
                                                        _ => "status-starting",
                                                    }), "{status}" }
                                                } else {
                                                    "-"
                                                }
                                            }
                                            td {
                                                if tx.r#type == "purchase" && tx.order_status.as_deref() == Some("failed") {
                                                    if let Some(order_id) = &tx.order_id {
                                                        button {
                                                            class: "btn-secondary btn-sm",
                                                            onclick: {
                                                                let oid = order_id.clone();
                                                                move |_| on_refund(oid.clone())
                                                            },
                                                            "{t!(\"dash_refund_btn\")}"
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

                    article { class: "table-card",
                        div { class: "tab-strip",
                            button { class: "tab active", "{t!(\"dash_invoice_records\")}" }
                        }

                        table {
                            thead {
                                tr {
                                    th { "ID" }
                                    th { "{t!(\"dash_amount_col\")}" }
                                    th { "{t!(\"dash_status\")}" }
                                    th { "{t!(\"dash_due_at_col\")}" }
                                    th { "" }
                                }
                            }
                            tbody {
                                if state.invoices.is_empty() {
                                    tr {
                                        td { colspan: "5", "{t!(\"dash_no_invoices\")}" }
                                    }
                                } else {
                                    for (item , invoice_id) in invoice_rows.iter().cloned() {
                                        tr {
                                            class: if selected_invoice.as_ref().map(|selected| selected.id == item.id).unwrap_or(false) { "invoice-row active" } else { "invoice-row" },
                                            onclick: {
                                                let invoice_id = invoice_id.clone();
                                                move |event| {
                                                    *modal_closing.write() = false;
                                                    let coords = event.data().client_coordinates();
                                                    *modal_origin.write() = modal_origin_from_client(coords.x, coords.y);
                                                    *selected_invoice_id.write() = Some(invoice_id.clone());
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
                                                            *modal_closing.write() = false;
                                                            let coords = event.data().client_coordinates();
                                                            *modal_origin.write() = modal_origin_from_client(coords.x, coords.y);
                                                            *selected_invoice_id.write() = Some(invoice_id.clone());
                                                        }
                                                    },
                                                    "{t!(\"dash_ticket_details\")}"
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
                                    p { class: "muted modal-kicker", "{t!(\"dash_invoice_details\")}" }
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
                                    "{t!(\"dash_close_ticket\")}" // reusing close translation
                                }
                            }

                            div { class: "detail-grid",
                                article { class: "detail-item",
                                    p { class: "muted", "{t!(\"dash_status\")}" }
                                    span { class: invoice_pill_class(&modal_status), "{modal_status}" }
                                }
                                article { class: "detail-item",
                                    p { class: "muted", "{t!(\"dash_amount_col\")}" }
                                    p { class: "fact", "$ {invoice.amount}" }
                                }
                                article { class: "detail-item",
                                    p { class: "muted", "{t!(\"dash_order_id\")}" }
                                    p { class: "fact", "{selected_order_id}" }
                                }
                                article { class: "detail-item",
                                    p { class: "muted", "{t!(\"dash_external_payment_ref\")}" }
                                    p { class: "fact", "{selected_external_payment_ref}" }
                                }
                                article { class: "detail-item",
                                    p { class: "muted", "{t!(\"dash_created_at_label\")}" }
                                    p { class: "fact", "{invoice.created_at}" }
                                }
                                article { class: "detail-item",
                                    p { class: "muted", "{t!(\"dash_due_at_col\")}" }
                                    p { class: "fact", "{invoice.due_at}" }
                                }
                                article { class: "detail-item",
                                    p { class: "muted", "{t!(\"dash_paid_at\")}" }
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
                                            "{t!(\"dash_preparing_checkout\")}"
                                        } else {
                                            "{t!(\"dash_pay_again\")}"
                                        }
                                    }
                                } else if modal_status == "expired" {
                                    p { class: "notice",
                                        "{t!(\"dash_invoice_expired\")}"
                                    }
                                } else if modal_status == "paid" {
                                    p { class: "notice", "{t!(\"dash_invoice_paid\")}" }
                                }

                                p { class: "muted modal-note",
                                    "{t!(\"dash_invoice_note\")}"
                                }
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
