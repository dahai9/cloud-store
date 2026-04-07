use crate::api;
use crate::models::{
    ActionRequest, AdminInstanceDeleteRequest, AdminSessionState, GuestUpdateRequest,
    InstanceAction, InstanceItem,
};
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn GuestsPage() -> Element {
    let mut session = use_context::<Signal<AdminSessionState>>();
    let mut search_query = use_signal(String::new);
    let mut expanded_guest = use_signal(|| None::<String>);
    let mut guest_instances = use_signal(|| None::<Vec<InstanceItem>>);

    let refresh_guests = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        let search = if search_query().is_empty() {
            None
        } else {
            Some(search_query())
        };
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
                    session.write().notice = Some(t!("plans_update_success"));
                    let search = if search_query().is_empty() {
                        None
                    } else {
                        Some(search_query())
                    };
                    if let Ok(guests) = api::get_guests(&api_base, &token, search).await {
                        session.write().guests = guests;
                    }
                }
                Err(e) => session.write().error = Some(t!("nodes_error_update", err: e)),
            }
            session.write().loading = false;
        });
    };

    let load_instances = move |user_id: String| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        spawn(async move {
            session.write().loading = true;
            match api::get_instances(&api_base, &token, Some(user_id.clone())).await {
                Ok(instances) => {
                    guest_instances.set(Some(instances));
                }
                Err(e) => session.write().error = Some(e),
            }
            session.write().loading = false;
        });
    };

    let stop_instance = move |instance_id: String, user_id: String| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        spawn(async move {
            session.write().loading = true;
            let payload = ActionRequest {
                action: InstanceAction::Stop,
            };
            match api::perform_instance_action(&api_base, &token, &instance_id, &payload).await {
                Ok(_) => {
                    session.write().notice = Some(t!("instances_action_success"));
                    match api::get_instances(&api_base, &token, Some(user_id)).await {
                        Ok(instances) => guest_instances.set(Some(instances)),
                        Err(e) => session.write().error = Some(e),
                    }
                }
                Err(e) => session.write().error = Some(e),
            }
            session.write().loading = false;
        });
    };

    let delete_instance = move |instance_id: String, user_id: String| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        spawn(async move {
            session.write().loading = true;
            let payload = AdminInstanceDeleteRequest {
                refund_amount: Some("0.00".to_string()),
            };
            match api::delete_instance(&api_base, &token, &instance_id, &payload).await {
                Ok(_) => {
                    session.write().notice = Some(t!("instances_action_success"));
                    match api::get_instances(&api_base, &token, Some(user_id)).await {
                        Ok(instances) => guest_instances.set(Some(instances)),
                        Err(e) => session.write().error = Some(e),
                    }
                }
                Err(e) => session.write().error = Some(e),
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
                        li { class: "item", style: "flex-direction: column; align-items: stretch;",
                            div { style: "display: flex; justify-content: space-between; align-items: center;",
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
                                            move |_| {
                                                if expanded_guest() == Some(uid.clone()) {
                                                    expanded_guest.set(None);
                                                    guest_instances.set(None);
                                                } else {
                                                    expanded_guest.set(Some(uid.clone()));
                                                    load_instances(uid.clone());
                                                }
                                            }
                                        },
                                        "Instances"
                                    }
                                    button {
                                        class: "btn-secondary",
                                        onclick: {
                                            let uid = guest.id.clone();
                                            let next = !guest.disabled;
                                            move |_| toggle_disabled(uid.clone(), next)
                                        },
                                        if guest.disabled { "{t!(\"submit\")}" } else { "{t!(\"delete\")}" }
                                    }
                                }
                            }
                            if expanded_guest() == Some(guest.id.clone()) {
                                div { class: "sub-list", style: "margin-top: 1rem; padding: 1rem; background: var(--bg-secondary); border-radius: 4px;",
                                    h3 { "Instances for {guest.email}" }
                                    if let Some(instances) = guest_instances() {
                                        if instances.is_empty() {
                                            p { "No instances found." }
                                        } else {
                                            table { class: "admin-table",
                                                thead {
                                                    tr {
                                                        th { "ID" }
                                                        th { "Node" }
                                                        th { "Status" }
                                                        th { "Created" }
                                                        th { "Actions" }
                                                    }
                                                }
                                                tbody {
                                                    for inst in instances {
                                                        tr {
                                                            td { class: "mono", "{inst.id}" }
                                                            td { "{inst.node_name}" }
                                                            td { span { class: "status-tag {inst.status}", "{inst.status}" } }
                                                            td { "{inst.created_at}" }
                                                            td { class: "actions",
                                                                button {
                                                                    class: "btn-secondary btn-sm",
                                                                    onclick: {
                                                                        let iid = inst.id.clone();
                                                                        let uid = guest.id.clone();
                                                                        move |_| stop_instance(iid.clone(), uid.clone())
                                                                    },
                                                                    "Stop"
                                                                }
                                                                button {
                                                                    class: "btn-secondary btn-sm",
                                                                    onclick: {
                                                                        let iid = inst.id.clone();
                                                                        let uid = guest.id.clone();
                                                                        move |_| delete_instance(iid.clone(), uid.clone())
                                                                    },
                                                                    "{t!(\"delete\")}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "actions", style: "margin-top: 1rem;",
                                            button {
                                                class: "btn-primary",
                                                onclick: move |_| {
                                                    session.write().notice = Some("API is available but manual add form UI is pending in this scope.".to_string());
                                                },
                                                "Add Instance"
                                            }
                                        }
                                    } else {
                                        p { "Loading instances..." }
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
