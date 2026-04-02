use dioxus::prelude::*;
use crate::models::{Route, AdminSessionState};

mod overview;
mod nodes;
mod plans;
mod guests;
mod tickets;

pub use overview::OverviewPage;
pub use nodes::NodesPage;
pub use plans::PlansPage;
pub use guests::GuestsPage;
pub use tickets::TicketsPage;

#[component]
pub fn DashboardLayout() -> Element {
    let mut session = use_context::<Signal<AdminSessionState>>();
    
    // Redirect if not logged in
    if session().token.is_none() {
        return rsx! {
            div { class: "content",
                section { class: "card", style: "max-width: 500px; margin: 50px auto; text-align: center;",
                    h2 { "未登录" }
                    p { "请先登录管理员账号以访问此页面。" }
                    Link { to: Route::LoginPage {}, class: "btn-primary", "去登录" }
                }
            }
        };
    }

    let logout = move |_| {
        let mut s = session.write();
        s.token = None;
        s.profile = None;
        s.nodes.clear();
        s.plans.clear();
        s.guests.clear();
        s.tickets.clear();
        s.notice = Some("已退出管理端会话".to_string());
        navigator().push(Route::LoginPage {});
    };

    rsx! {
        div { class: "layout",
            aside { class: "sidebar",
                div { class: "logo",
                    div { class: "logo-mark", "A" }
                    div { class: "logo-text",
                        h1 { "Cloud Store" }
                        p { "Admin Console" }
                    }
                }

                nav { class: "menu",
                    Link { class: "menu-item", active_class: "active", to: Route::OverviewPage {}, "Overview" }
                    Link { class: "menu-item", active_class: "active", to: Route::NodesPage {}, "Nodes" }
                    Link { class: "menu-item", active_class: "active", to: Route::PlansPage {}, "Products" }
                    Link { class: "menu-item", active_class: "active", to: Route::GuestsPage {}, "Guests" }
                    Link { class: "menu-item", active_class: "active", to: Route::TicketsPage {}, "Tickets" }
                }
            }

            main { class: "content",
                header { class: "topbar",
                    div {
                        h2 { "Admin Console" }
                        p { class: "status",
                            "独立管理端与客户端保持同一视觉系统，但权限和端口隔离。"
                        }
                    }

                    div { class: "top-actions",
                        a {
                            class: "btn-secondary",
                            href: "http://127.0.0.1:8080",
                            "Store"
                        }
                        button { class: "btn-secondary", onclick: logout, "Logout" }
                    }
                }

                Outlet::<Route> {}
            }
        }
    }
}
