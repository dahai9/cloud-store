use crate::models::{AdminSessionState, Route};
use dioxus::prelude::*;
use dioxus_i18n::prelude::i18n;
use dioxus_i18n::t;
use unic_langid::langid;

mod guests;
mod instances;
mod nat_port_leases;
mod nodes;
mod overview;
mod plans;
mod tickets;

pub use guests::GuestsPage;
pub use instances::InstancesPage;
pub use nat_port_leases::NatPortLeasesPage;
pub use nodes::NodesPage;
pub use overview::OverviewPage;
pub use plans::PlansPage;
pub use tickets::TicketsPage;

#[component]
pub fn DashboardLayout() -> Element {
    let mut session = use_context::<Signal<AdminSessionState>>();
    let mut _i18n = i18n();

    // Redirect if not logged in
    if session().token.is_none() {
        return rsx! {
            div { class: "content",
                section {
                    class: "card",
                    style: "max-width: 500px; margin: 50px auto; text-align: center;",
                    h2 { "{t!(\"dash_layout_not_logged_in\")}" }
                    p { "{t!(\"dash_layout_please_login\")}" }
                    Link { to: Route::LoginPage {}, class: "btn-primary", "{t!(\"dash_layout_go_to_login\")}" }
                }
            }
        };
    }

    let logout = move |_| {
        let mut s = session.write();
        s.token = None;
        s.profile = None;
        s.nodes.clear();
        s.nat_port_leases.clear();
        s.plans.clear();
        s.guests.clear();
        s.tickets.clear();
        s.notice = Some(t!("dash_layout_logout_notice"));
        navigator().push(Route::LoginPage {});
    };

    rsx! {
        div { class: "layout",
            aside { class: "sidebar",
                div { class: "logo",
                    div { class: "logo-mark", "A" }
                    div { class: "logo-text",
                        h1 { "{t!(\"app_title\")}" }
                        p { "{t!(\"admin_console\")}" }
                    }
                }

                nav { class: "menu",
                    Link {
                        class: "menu-item",
                        active_class: "active",
                        to: Route::OverviewPage {},
                        "{t!(\"nav_overview\")}"
                    }
                    Link {
                        class: "menu-item",
                        active_class: "active",
                        to: Route::NodesPage {},
                        "{t!(\"nav_nodes\")}"
                    }
                    Link {
                        class: "menu-item",
                        active_class: "active",
                        to: Route::NatPortLeasesPage {},
                        "{t!(\"nav_nat_leases\")}"
                    }
                    Link {
                        class: "menu-item",
                        active_class: "active",
                        to: Route::InstancesPage {},
                        "{t!(\"nav_instances\")}"
                    }
                    Link {
                        class: "menu-item",
                        active_class: "active",
                        to: Route::PlansPage {},
                        "{t!(\"nav_plans\")}"
                    }
                    Link {
                        class: "menu-item",
                        active_class: "active",
                        to: Route::GuestsPage {},
                        "{t!(\"nav_guests\")}"
                    }
                    Link {
                        class: "menu-item",
                        active_class: "active",
                        to: Route::TicketsPage {},
                        "{t!(\"nav_tickets\")}"
                    }
                }
            }

            main { class: "content",
                header { class: "topbar",
                    div {
                        h2 { "{t!(\"admin_console\")}" }
                        p { class: "status",
                            "{t!(\"dash_layout_admin_desc_p1\")}"
                        }
                    }

                    div { class: "top-actions",
                        button {
                            class: "btn-secondary btn-sm",
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
                        a {
                            class: "btn-secondary",
                            href: "http://127.0.0.1:8080",
                            "{t!(\"store_btn\")}"
                        }
                        button { class: "btn-secondary", onclick: logout, "{t!(\"nav_logout\")}" }
                    }

                }

                Outlet::<Route> {}
            }
        }
    }
}

