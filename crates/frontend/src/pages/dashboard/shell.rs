use crate::api;
use crate::models::{DashboardTab, Route, SessionState};
use dioxus::prelude::*;
use dioxus_i18n::prelude::i18n;
use dioxus_i18n::t;

#[component]
pub fn DashboardShell(title: String, active_tab: DashboardTab, children: Element) -> Element {
    let navigator = use_navigator();
    let mut session = use_context::<Signal<SessionState>>();
    let mut _i18n = i18n();

    rsx! {
        div { class: "layout",
            aside { class: "sidebar",
                div { class: "logo",
                    div { class: "logo-mark", "C" }
                    div { class: "logo-text",
                        h1 { "{t!(\"app_title\")}" }
                        p { "{t!(\"dash_customer_center\")}" }
                    }
                }
                nav { class: "menu",
                    Link {
                        class: if active_tab == DashboardTab::Profile { "menu-item active" } else { "menu-item" },
                        to: Route::ProfilePage {},
                        "{t!(\"nav_profile\")}"
                    }
                    Link {
                        class: if active_tab == DashboardTab::Services { "menu-item active" } else { "menu-item" },
                        to: Route::ServicesPage {},
                        "{t!(\"nav_services\")}"
                    }
                    Link {
                        class: if active_tab == DashboardTab::Tickets { "menu-item active" } else { "menu-item" },
                        to: Route::TicketsPage {},
                        "{t!(\"nav_tickets\")}"
                    }
                    Link {
                        class: if active_tab == DashboardTab::Balance { "menu-item active" } else { "menu-item" },
                        to: Route::BalancePage {},
                        "{t!(\"nav_billing\")}"
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
                                use unic_langid::langid;
                                if _i18n.language() == langid!("en-US") {
                                    _i18n.set_language(langid!("zh-CN"));
                                } else {
                                    _i18n.set_language(langid!("en-US"));
                                }
                            },
                            "{t!(\"switch_lang\")}"
                        }
                        button {
                            class: "btn-secondary",
                            onclick: move |_| {
                                navigator.push(Route::StorefrontPage {});
                            },
                            "{t!(\"dash_store_btn\")}"
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
                            "{t!(\"dash_logout_btn\")}"
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
                    h3 { "{t!(\"dash_login_required_view\")}" }
                    p { "{t!(\"dash_page_only_after_login\")}" }
                    button {
                        class: "btn-primary",
                        onclick: move |_| {
                            navigator
                                .push(Route::LoginPage {
                                    source: Some("protected".to_string()),
                                    plan: None,
                                });
                        },
                        "{t!(\"dash_go_to_login_btn\")}"
                    }
                }
            }
        }
    }
}
