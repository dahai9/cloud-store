#[cfg(target_arch = "wasm32")]
use crate::api;
#[cfg(target_arch = "wasm32")]
use crate::models::{DashboardTab, Route, SessionState};
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
#[component]
pub fn ServicesPage() -> Element {
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

#[cfg(target_arch = "wasm32")]
#[component]
pub fn BalancePage() -> Element {
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

#[cfg(target_arch = "wasm32")]
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
