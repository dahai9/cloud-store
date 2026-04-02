use dioxus::prelude::*;
use crate::models::{AdminSessionState, GuestUpdateRequest};
use crate::api;

#[component]
pub fn GuestsPage() -> Element {
    let mut session = use_context::<Signal<AdminSessionState>>();

    let refresh_guests = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        spawn(async move {
            session.write().loading = true;
            match api::get_guests(&api_base, &token).await {
                Ok(guests) => session.write().guests = guests,
                Err(e) => session.write().error = Some(format!("刷新失败: {e}")),
            }
            session.write().loading = false;
        });
    };

    let toggle_disabled = move |user_id: String, next_disabled: bool| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        spawn(async move {
            session.write().loading = true;
            let payload = GuestUpdateRequest { disabled: next_disabled };
            match api::update_guest(&api_base, &token, &user_id, &payload).await {
                Ok(_) => {
                    session.write().notice = Some("Guest 配置更新成功".to_string());
                    if let Ok(guests) = api::get_guests(&api_base, &token).await {
                        session.write().guests = guests;
                    }
                }
                Err(e) => session.write().error = Some(format!("更新失败: {e}")),
            }
            session.write().loading = false;
        });
    };

    rsx! {
        section { class: "card", id: "guests",
            h2 { "Guest 配置（启用/禁用）" }
            div { class: "actions",
                button { class: "btn-secondary", onclick: refresh_guests, "刷新 Guest" }
            }

            if session().loading { p { class: "status", "处理中..." } }

            if session().guests.is_empty() {
                p { class: "status", "暂无 Guest 数据。" }
            } else {
                ul { class: "list",
                    for guest in session().guests.clone() {
                        li { class: "item",
                            strong { "{guest.email}" }
                            span { class: "meta", "Guest ID: {guest.id}" }
                            span { class: "meta", "Disabled: {guest.disabled}" }
                            span { class: "meta", "Created: {guest.created_at}" }
                            div { class: "actions",
                                button {
                                    class: "btn-secondary",
                                    onclick: {
                                        let uid = guest.id.clone();
                                        let next = !guest.disabled;
                                        move |_| toggle_disabled(uid.clone(), next)
                                    },
                                    if guest.disabled { "启用" } else { "禁用" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
