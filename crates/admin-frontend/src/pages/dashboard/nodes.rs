use crate::api;
use crate::models::{AdminSessionState, NodeCreateRequest, NodeItem, NodeUpdateRequest};
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn NodesPage() -> Element {
    let mut session = use_context::<Signal<AdminSessionState>>();
    let mut show_add_form = use_signal(|| false);
    let mut editing_node = use_signal::<Option<NodeItem>>(|| None);

    let mut name = use_signal(String::new);
    let mut region = use_signal(String::new);
    let mut cpu = use_signal(|| "16".to_string());
    let mut ram = use_signal(|| "32768".to_string());
    let mut storage = use_signal(|| "1000".to_string());
    let mut endpoint = use_signal(String::new);
    let mut api_token = use_signal(String::new);

    let refresh_nodes = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();

        spawn(async move {
            session.write().loading = true;
            match api::get_nodes(&api_base, &token).await {
                Ok(nodes) => {
                    let mut s = session.write();
                    s.nodes = nodes;
                    s.notice = Some(t!("nodes_refresh_success"));
                }
                Err(e) => {
                    session.write().error = Some(t!("nodes_error_refresh", err: e));
                }
            }
            session.write().loading = false;
        });
    };

    let on_submit_add = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();

        let name_val = name();
        let region_val = region();
        let cpu_val = cpu().parse::<i64>().unwrap_or(0);
        let ram_val = ram().parse::<i64>().unwrap_or(0);
        let storage_val = storage().parse::<i64>().unwrap_or(0);
        let endpoint_val = if endpoint().is_empty() {
            None
        } else {
            Some(endpoint())
        };
        let api_token_val = if api_token().is_empty() {
            None
        } else {
            Some(api_token())
        };

        spawn(async move {
            session.write().loading = true;
            let req = NodeCreateRequest {
                name: name_val,
                region: region_val,
                cpu_cores_total: cpu_val,
                memory_mb_total: ram_val,
                storage_gb_total: storage_val,
                api_endpoint: endpoint_val,
                api_token: api_token_val,
            };
            match api::create_node(&api_base, &token, &req).await {
                Ok(_) => {
                    show_add_form.set(false);
                    session.write().notice = Some(t!("nodes_add_success"));
                    // Refresh list
                    if let Ok(nodes) = api::get_nodes(&api_base, &token).await {
                        session.write().nodes = nodes;
                    }
                }
                Err(e) => {
                    session.write().error = Some(t!("nodes_error_add", err: e));
                }
            }
            session.write().loading = false;
        });
    };

    let on_submit_edit = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        let node_id = editing_node()
            .as_ref()
            .map(|n| n.id.clone())
            .unwrap_or_default();

        let name_val = Some(name());
        let region_val = Some(region());
        let cpu_val = Some(cpu().parse::<i64>().unwrap_or(0));
        let ram_val = Some(ram().parse::<i64>().unwrap_or(0));
        let storage_val = Some(storage().parse::<i64>().unwrap_or(0));
        let endpoint_val = if endpoint().is_empty() {
            None
        } else {
            Some(endpoint())
        };
        let api_token_val = if api_token().is_empty() {
            None
        } else {
            Some(api_token())
        };

        spawn(async move {
            session.write().loading = true;
            let req = NodeUpdateRequest {
                name: name_val,
                region: region_val,
                cpu_cores_total: cpu_val,
                memory_mb_total: ram_val,
                storage_gb_total: storage_val,
                api_endpoint: endpoint_val,
                api_token: api_token_val,
            };
            match api::update_node(&api_base, &token, &node_id, &req).await {
                Ok(_) => {
                    editing_node.set(None);
                    session.write().notice = Some(t!("nodes_update_success"));
                    // Refresh list
                    if let Ok(nodes) = api::get_nodes(&api_base, &token).await {
                        session.write().nodes = nodes;
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
        section { class: "card", id: "nodes",
            h2 { "{t!(\"nodes_title\")}" }
            div { class: "actions",
                button { class: "btn-secondary", onclick: refresh_nodes, "{t!(\"refresh\")}" }
                button {
                    class: "btn-primary",
                    onclick: move |_| {
                        show_add_form.set(true);
                        editing_node.set(None);
                        name.set(String::new());
                        region.set(String::new());
                        cpu.set("16".to_string());
                        ram.set("32768".to_string());
                        storage.set("1000".to_string());
                        endpoint.set(String::new());
                        api_token.set(String::new());
                    },
                    "{t!(\"nodes_add_btn\")}"
                }
            }

            if show_add_form() {
                div { class: "modal-overlay",
                    div { class: "modal-content",
                        h3 { "{t!(\"nodes_add_modal_title\")}" }
                        div {
                            div { class: "form-group",
                                label { "{t!(\"nodes_form_name\")}" }
                                input {
                                    value: "{name}",
                                    oninput: move |evt| name.set(evt.value()),
                                    required: true,
                                    placeholder: "My Node 01",
                                }
                            }
                            div { class: "form-group",
                                label { "{t!(\"nodes_form_region\")}" }
                                input {
                                    value: "{region}",
                                    oninput: move |evt| region.set(evt.value()),
                                    required: true,
                                    placeholder: "US-West",
                                }
                            }
                            div { class: "form-group",
                                label { "{t!(\"nodes_form_cpu\")}" }
                                input {
                                    value: "{cpu}",
                                    oninput: move |evt| cpu.set(evt.value()),
                                    r#type: "number",
                                    required: true,
                                }
                            }
                            div { class: "form-group",
                                label { "{t!(\"nodes_form_ram\")}" }
                                input {
                                    value: "{ram}",
                                    oninput: move |evt| ram.set(evt.value()),
                                    r#type: "number",
                                    required: true,
                                }
                            }
                            div { class: "form-group",
                                label { "{t!(\"nodes_form_storage\")}" }
                                input {
                                    value: "{storage}",
                                    oninput: move |evt| storage.set(evt.value()),
                                    r#type: "number",
                                    required: true,
                                }
                            }
                            div { class: "form-group",
                                label { "{t!(\"nodes_form_api_endpoint\")}" }
                                input {
                                    value: "{endpoint}",
                                    oninput: move |evt| endpoint.set(evt.value()),
                                    placeholder: "https://node.example.com:18443",
                                }
                            }
                            div { class: "form-group",
                                label { "{t!(\"nodes_form_incus_token\")}" }
                                input {
                                    value: "{api_token}",
                                    oninput: move |evt| api_token.set(evt.value()),
                                    placeholder: "{t!(\"nodes_incus_token_placeholder\")}",
                                }
                            }
                            div { class: "modal-actions",
                                button {
                                    class: "btn-primary",
                                    onclick: on_submit_add,
                                    "{t!(\"submit\")}"
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| show_add_form.set(false),
                                    "{t!(\"cancel\")}"
                                }
                            }
                        }
                    }
                }
            }

            if let Some(node) = editing_node() {
                div { class: "modal-overlay",
                    div { class: "modal-content",
                        h3 { {t!("nodes_edit_modal_title", name: node.name.clone())} }
                        div {
                            div { class: "form-group",
                                label { "{t!(\"nodes_form_name\")}" }
                                input {
                                    value: "{name}",
                                    oninput: move |evt| name.set(evt.value()),
                                    required: true,
                                }
                            }
                            div { class: "form-group",
                                label { "{t!(\"nodes_form_region\")}" }
                                input {
                                    value: "{region}",
                                    oninput: move |evt| region.set(evt.value()),
                                    required: true,
                                }
                            }
                            div { class: "form-group",
                                label { "{t!(\"nodes_form_cpu\")}" }
                                input {
                                    value: "{cpu}",
                                    oninput: move |evt| cpu.set(evt.value()),
                                    r#type: "number",
                                    required: true,
                                }
                            }
                            div { class: "form-group",
                                label { "{t!(\"nodes_form_ram\")}" }
                                input {
                                    value: "{ram}",
                                    oninput: move |evt| ram.set(evt.value()),
                                    r#type: "number",
                                    required: true,
                                }
                            }
                            div { class: "form-group",
                                label { "{t!(\"nodes_form_storage\")}" }
                                input {
                                    value: "{storage}",
                                    oninput: move |evt| storage.set(evt.value()),
                                    r#type: "number",
                                    required: true,
                                }
                            }
                            div { class: "form-group",
                                label { "{t!(\"nodes_form_api_endpoint_edit\")}" }
                                input {
                                    value: "{endpoint}",
                                    oninput: move |evt| endpoint.set(evt.value()),
                                }
                            }
                            div { class: "form-group",
                                label { "{t!(\"nodes_form_incus_token_edit\")}" }
                                input {
                                    value: "{api_token}",
                                    oninput: move |evt| api_token.set(evt.value()),
                                }
                            }
                            div { class: "modal-actions",
                                button {
                                    class: "btn-primary",
                                    onclick: on_submit_edit,
                                    "{t!(\"save\")}"
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| editing_node.set(None),
                                    "{t!(\"cancel\")}"
                                }
                            }
                        }
                    }
                }
            }

            if session().loading {
                p { class: "status", "{t!(\"loading\")}" }
            }

            if session().nodes.is_empty() {
                p { class: "status", "{t!(\"nodes_no_data\")}" }
            } else {
                ul { class: "list",
                    for node in session().nodes.clone() {
                        li { class: "item",
                            div { class: "item-header",
                                strong { "{node.name}" }
                                button {
                                    class: "btn-small",
                                    onclick: {
                                        let node = node.clone();
                                        move |_| {
                                            let n = node.clone();
                                            name.set(n.name.clone());
                                            region.set(n.region.clone());
                                            cpu.set(n.cpu_cores_total.to_string());
                                            ram.set(n.memory_mb_total.to_string());
                                            storage.set(n.storage_gb_total.to_string());
                                            endpoint.set(n.api_endpoint.clone().unwrap_or_default());
                                            api_token.set(n.api_token.clone().unwrap_or_default());
                                            editing_node.set(Some(n));
                                            show_add_form.set(false);
                                        }
                                    },
                                    "{t!(\"edit\")}"
                                }
                            }
                            span { class: "meta", "ID: {node.id} | Region: {node.region}" }
                            div { class: "metrics",
                                div { class: "metric",
                                    span { "CPU" }
                                    div { class: "progress-track",
                                        div {
                                            class: format!("progress-bar {}", if node.cpu_cores_used as f64 / node.cpu_cores_total as f64 > 0.8 { "danger" } else { "ok" }),
                                            style: "width: {node.cpu_cores_used as f64 / node.cpu_cores_total as f64 * 100.0}%",
                                        }
                                    }
                                    span { class: "metric-value", "{node.cpu_cores_used} / {node.cpu_cores_total} Cores" }
                                }
                                div { class: "metric",
                                    span { "RAM" }
                                    div { class: "progress-track",
                                        div {
                                            class: format!("progress-bar {}", if node.memory_mb_used as f64 / node.memory_mb_total as f64 > 0.8 { "danger" } else { "ok" }),
                                            style: "width: {node.memory_mb_used as f64 / node.memory_mb_total as f64 * 100.0}%",
                                        }
                                    }
                                    span { class: "metric-value", "{node.memory_mb_used} / {node.memory_mb_total} MB" }
                                }
                                div { class: "metric",
                                    span { "Disk" }
                                    div { class: "progress-track",
                                        div {
                                            class: format!("progress-bar {}", if node.storage_gb_used as f64 / node.storage_gb_total as f64 > 0.8 { "danger" } else { "ok" }),
                                            style: "width: {node.storage_gb_used as f64 / node.storage_gb_total as f64 * 100.0}%",
                                        }
                                    }
                                    span { class: "metric-value", "{node.storage_gb_used} / {node.storage_gb_total} GB" }
                                }
                            }
                            if let Some(ep) = &node.api_endpoint {
                                span { class: "meta-api", "API: {ep}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
