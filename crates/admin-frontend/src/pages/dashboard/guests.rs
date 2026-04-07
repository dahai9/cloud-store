use crate::api;
use crate::models::{AdminSessionState, GuestUpdateRequest};
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn GuestsPage() -> Element {
    let mut session = use_context::<Signal<AdminSessionState>>();
    let mut search_query = use_signal(String::new);

    let refresh_guests = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        let search = if search_query().is_empty() { None } else { Some(search_query()) };
        spawn(async move {
            session.write().loading = true;
            match api::get_guests(&api_base, &token, search).await {
                Ok(guests) => session.write().guests = guests,
                Err(e) => session.write().error = Some(t!("nodes_error_refresh", err: e)),
            }
            session.write().loading = false;
        });
    };

    let toggle_disabled = move |user_id: String, next_disabled: bool| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        spawn(async move {
            session.write().loading = true;
            let payload = GuestUpdateRequest {
                disabled: next_disabled,
            };
            match api::update_guest(&api_base, &token, &user_id, &payload).await {
                Ok(_) => {
                    session.write().notice = Some(t!("plans_update_success")); // Generic update success
                    let search = if search_query().is_empty() { None } else { Some(search_query()) };
                    if let Ok(guests) = api::get_guests(&api_base, &token, search).await {
                        session.write().guests = guests;
                    }
                }
                Err(e) => session.write().error = Some(t!("nodes_error_update", err: e)),
            }
            session.write().loading = false;
        });
    };

    rsx! {
        section { class: "card", id: "guests",
            h2 { "{t!(\"guests_title\")}" }
            div { class: "actions",
                input {
                    r#type: "text",
                    placeholder: "{t!(\"guests_search_placeholder\")}",
                    value: "{search_query}",
                    oninput: move |e| search_query.set(e.value()),
                    onkeypress: move |e| {
                        if e.key() == Key::Enter {
                            refresh_guests(());
                        }
                    }
                }
                button { class: "btn-secondary", onclick: move |_| refresh_guests(()), "{t!(\"refresh\")}" }
            }

            if session().loading { p { class: "status", "{t!(\"processing\")}" } }

            if session().guests.is_empty() {
                p { class: "status", "{t!(\"guests_no_data\")}" }
            } else {
                ul { class: "list",
                    for guest in session().guests.clone() {
                        li { class: "item",
                            div {
                                strong { "{guest.email}" }
                                span { class: "meta", " | ID: {guest.id}" }
                            }
                            div {
                                span { class: "meta", "Balance: ${guest.balance}" }
                                span { class: "meta", " | Disabled: {guest.disabled}" }
                                span { class: "meta", " | Created: {guest.created_at}" }
                            }
                            div { class: "actions",
                                button {
                                    class: "btn-secondary",
                                    onclick: {
                                        let uid = guest.id.clone();
                                        let next = !guest.disabled;
                                        move |_| toggle_disabled(uid.clone(), next)
                                    },
                                    if guest.disabled { "{t!(\"submit\")}" } else { "{t!(\"delete\")}" } // Generic enable/disable labels from plans logic
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}


