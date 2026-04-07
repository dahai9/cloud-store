use crate::api;
use crate::models::{AdminSessionState, NatPortLeaseCreateRequest};
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn NatPortLeasesPage() -> Element {
    let mut session = use_context::<Signal<AdminSessionState>>();
    let mut show_create_form = use_signal(|| false);

    let mut selected_node_id = use_signal(String::new);
    let mut public_ip = use_signal(String::new);
    let mut start_port = use_signal(|| "10000".to_string());
    let mut end_port = use_signal(|| "10100".to_string());

    let refresh_nodes = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();

        spawn(async move {
            session.write().loading = true;
            match api::get_nodes(&api_base, &token).await {
                Ok(nodes) => {
                    session.write().nodes = nodes;
                    session.write().notice = Some(t!("nodes_refresh_success"));
                }
                Err(e) => {
                    session.write().error = Some(t!("nodes_error_refresh", err: e));
                }
            }
            session.write().loading = false;
        });
    };

    let refresh_leases = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();

        spawn(async move {
            session.write().loading = true;
            match api::get_nat_port_leases(&api_base, &token).await {
                Ok(leases) => {
                    session.write().nat_port_leases = leases;
                    session.write().notice = Some(t!("nat_leases_refresh_success"));
                }
                Err(e) => {
                    session.write().error = Some(t!("nodes_error_refresh", err: e));
                }
            }
            session.write().loading = false;
        });
    };

    let open_create_form = move |_| {
        let first_node_id = session()
            .nodes
            .first()
            .map(|node| node.id.clone())
            .unwrap_or_default();

        selected_node_id.set(first_node_id);
        public_ip.set(String::new());
        start_port.set("10000".to_string());
        end_port.set("10100".to_string());
        show_create_form.set(true);
    };

    let save_lease = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();

        let node_id = selected_node_id();
        let public_ip_val = public_ip().trim().to_string();
        let start_port_val = start_port().trim().parse::<i64>().unwrap_or(0);
        let end_port_val = end_port().trim().parse::<i64>().unwrap_or(0);

        if node_id.trim().is_empty() {
            session.write().error = Some(t!("nat_leases_form_node")); // Re-using as label/warning
            return;
        }

        spawn(async move {
            session.write().loading = true;
            let payload = NatPortLeaseCreateRequest {
                node_id,
                public_ip: public_ip_val,
                start_port: start_port_val,
                end_port: end_port_val,
            };

            match api::create_nat_port_lease(&api_base, &token, &payload).await {
                Ok(_) => {
                    show_create_form.set(false);
                    session.write().notice = Some(t!("nat_leases_generate_success"));
                    if let Ok(leases) = api::get_nat_port_leases(&api_base, &token).await {
                        session.write().nat_port_leases = leases;
                    }
                }
                Err(e) => {
                    session.write().error = Some(t!("nodes_error_add", err: e));
                }
            }
            session.write().loading = false;
        });
    };

    let delete_lease = move |lease_id: String| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();

        spawn(async move {
            session.write().loading = true;
            match api::delete_nat_port_lease(&api_base, &token, &lease_id).await {
                Ok(_) => {
                    session.write().notice = Some(t!("instances_action_success"));
                    if let Ok(leases) = api::get_nat_port_leases(&api_base, &token).await {
                        session.write().nat_port_leases = leases;
                    }
                }
                Err(e) => {
                    session.write().error = Some(t!("nodes_error_update", err: e));
                }
            }
            session.write().loading = false;
        });
    };

    rsx! {
        section { class: "card", id: "nat-port-leases",
            h2 { "{t!(\"nat_leases_title\")}" }
            p { class: "status",
                "{t!(\"nat_leases_node_desc\")}"
            }

            div { class: "actions",
                button { class: "btn-secondary", onclick: refresh_nodes, "{t!(\"nav_nodes\")}" }
                button { class: "btn-secondary", onclick: refresh_leases, "{t!(\"refresh\")}" }
                button {
                    class: "btn-primary",
                    onclick: open_create_form,
                    disabled: session().nodes.is_empty(),
                    "{t!(\"nat_leases_add_btn\")}"
                }
            }

            if session().nodes.is_empty() {
                p { class: "status",
                    "{t!(\"nat_leases_no_nodes_warning\")}"
                }
            }

            if show_create_form() {
                div { class: "modal-overlay",
                    div { class: "modal-content",
                        h3 { "{t!(\"nat_leases_title\")}" }
                        div {
                            div { class: "form-group",
                                label { "{t!(\"nat_leases_form_node\")}" }
                                select {
                                    value: "{selected_node_id()}",
                                    oninput: move |evt| selected_node_id.set(evt.value()),
                                    disabled: session().nodes.is_empty(),
                                    option { value: "", "{t!(\"nat_leases_form_node\")}" }
                                    for node in session().nodes.clone() {
                                        option { value: "{node.id}", "{node.name} ({node.region})" }
                                    }
                                }
                            }
                            div { class: "form-group",
                                label { "{t!(\"nat_leases_form_public_ip\")}" }
                                input {
                                    value: "{public_ip}",
                                    oninput: move |evt| public_ip.set(evt.value()),
                                    placeholder: "203.0.113.10",
                                }
                            }
                            div { class: "form-group",
                                label { "{t!(\"nat_leases_form_start_port\")}" }
                                input {
                                    r#type: "number",
                                    value: "{start_port}",
                                    oninput: move |evt| start_port.set(evt.value()),
                                    placeholder: "10000",
                                }
                            }
                            div { class: "form-group",
                                label { "{t!(\"nat_leases_form_end_port\")}" }
                                input {
                                    r#type: "number",
                                    value: "{end_port}",
                                    oninput: move |evt| end_port.set(evt.value()),
                                    placeholder: "10100",
                                }
                            }
                            div { class: "modal-actions",
                                button { class: "btn-primary", onclick: save_lease, "{t!(\"submit\")}" }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| show_create_form.set(false),
                                    "{t!(\"cancel\")}"
                                }
                            }
                        }
                    }
                }
            }

            if session().loading {
                p { class: "status", "{t!(\"processing\")}" }
            }

            if session().nat_port_leases.is_empty() {
                p { class: "status", "{t!(\"nat_leases_no_data\")}" }
            } else {
                ul { class: "list",
                    for lease in session().nat_port_leases.clone() {
                        li { class: "item",
                            div { class: "item-header",
                                strong { "{lease.node_name}" }
                                span { class: "meta",
                                    if lease.reserved {
                                        "{t!(\"nat_leases_status_occupied\")}"
                                    } else {
                                        "{t!(\"nat_leases_status_available\")}"
                                    }

                                }
                            }
                            span { class: "meta",
                                "Node ID: {lease.node_id} | Region: {lease.node_region}"
                            }
                            span { class: "meta",
                                "IP: {lease.public_ip} | Ports: {lease.start_port}-{lease.end_port}"
                            }
                            span { class: "meta", "Created: {lease.created_at}" }
                            if let Some(order_id) = &lease.reserved_for_order_id {
                                span { class: "meta", "Reserved For Order: {order_id}" }
                            }
                            div { class: "actions",
                                if !lease.reserved {
                                    button {
                                        class: "btn-secondary",
                                        onclick: {
                                            let lease_id = lease.id.clone();
                                            move |_| delete_lease(lease_id.clone())
                                        },
                                        "{t!(\"delete\")}"
                                    }
                                } else {
                                    span { class: "meta", "{t!(\"nat_leases_occupied_warning\")}" }
                                }

                            }
                        }
                    }
                }
            }
        }
    }
}

