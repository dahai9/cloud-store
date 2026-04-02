use dioxus::prelude::*;
use crate::models::{AdminSessionState, NodeItem, NodeCreateRequest, NodeUpdateRequest};
use crate::api;

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
                    s.notice = Some("节点列表已刷新".to_string());
                }
                Err(e) => {
                    session.write().error = Some(format!("刷新失败: {e}"));
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
        let endpoint_val = if endpoint().is_empty() { None } else { Some(endpoint()) };
        let api_token_val = if api_token().is_empty() { None } else { Some(api_token()) };

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
                    session.write().notice = Some("节点添加成功".to_string());
                    // Refresh list
                    if let Ok(nodes) = api::get_nodes(&api_base, &token).await {
                        session.write().nodes = nodes;
                    }
                }
                Err(e) => {
                    session.write().error = Some(format!("添加失败: {e}"));
                }
            }
            session.write().loading = false;
        });
    };

    let on_submit_edit = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        let node_id = editing_node().as_ref().map(|n| n.id.clone()).unwrap_or_default();
        
        let name_val = Some(name());
        let region_val = Some(region());
        let cpu_val = Some(cpu().parse::<i64>().unwrap_or(0));
        let ram_val = Some(ram().parse::<i64>().unwrap_or(0));
        let storage_val = Some(storage().parse::<i64>().unwrap_or(0));
        let endpoint_val = if endpoint().is_empty() { None } else { Some(endpoint()) };
        let api_token_val = if api_token().is_empty() { None } else { Some(api_token()) };

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
                    session.write().notice = Some("节点更新成功".to_string());
                    // Refresh list
                    if let Ok(nodes) = api::get_nodes(&api_base, &token).await {
                        session.write().nodes = nodes;
                    }
                }
                Err(e) => {
                    session.write().error = Some(format!("更新失败: {e}"));
                }
            }
            session.write().loading = false;
        });
    };

    rsx! {
        section { class: "card", id: "nodes",
            h2 { "节点管理" }
            div { class: "actions",
                button {
                    class: "btn-secondary",
                    onclick: refresh_nodes,
                    "刷新列表"
                }
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
                    "添加节点"
                }
            }

            if show_add_form() {
                div { class: "modal-overlay",
                    div { class: "modal-content",
                        h3 { "添加新节点" }
                        div { 
                            div { class: "form-group",
                                label { "节点名称" }
                                input { 
                                    value: "{name}",
                                    oninput: move |evt| name.set(evt.value()),
                                    required: true, 
                                    placeholder: "My Node 01" 
                                }
                            }
                            div { class: "form-group",
                                label { "地区" }
                                input { 
                                    value: "{region}",
                                    oninput: move |evt| region.set(evt.value()),
                                    required: true, 
                                    placeholder: "US-West" 
                                }
                            }
                            div { class: "form-group",
                                label { "CPU 核心" }
                                input { 
                                    value: "{cpu}",
                                    oninput: move |evt| cpu.set(evt.value()),
                                    r#type: "number", 
                                    required: true, 
                                }
                            }
                            div { class: "form-group",
                                label { "内存 (MB)" }
                                input { 
                                    value: "{ram}",
                                    oninput: move |evt| ram.set(evt.value()),
                                    r#type: "number", 
                                    required: true, 
                                }
                            }
                            div { class: "form-group",
                                label { "存储 (GB)" }
                                input { 
                                    value: "{storage}",
                                    oninput: move |evt| storage.set(evt.value()),
                                    r#type: "number", 
                                    required: true, 
                                }
                            }
                            div { class: "form-group",
                                label { "API 端点 (可选)" }
                                input { 
                                    value: "{endpoint}",
                                    oninput: move |evt| endpoint.set(evt.value()),
                                    placeholder: "https://pve.example.com:8006/api2/json" 
                                }
                            }
                            div { class: "form-group",
                                label { "API 令牌 (可选)" }
                                input { 
                                    value: "{api_token}",
                                    oninput: move |evt| api_token.set(evt.value()),
                                    placeholder: "USER@PVE!TOKENID=SECRET" 
                                }
                            }
                            div { class: "modal-actions",
                                button { 
                                    class: "btn-primary", 
                                    onclick: on_submit_add,
                                    "提交" 
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| show_add_form.set(false),
                                    "取消"
                                }
                            }
                        }
                    }
                }
            }

            if let Some(node) = editing_node() {
                div { class: "modal-overlay",
                    div { class: "modal-content",
                        h3 { "编辑节点: {node.name}" }
                        div { 
                            div { class: "form-group",
                                label { "节点名称" }
                                input { 
                                    value: "{name}",
                                    oninput: move |evt| name.set(evt.value()),
                                    required: true, 
                                }
                            }
                            div { class: "form-group",
                                label { "地区" }
                                input { 
                                    value: "{region}",
                                    oninput: move |evt| region.set(evt.value()),
                                    required: true, 
                                }
                            }
                            div { class: "form-group",
                                label { "CPU 核心" }
                                input { 
                                    value: "{cpu}",
                                    oninput: move |evt| cpu.set(evt.value()),
                                    r#type: "number", 
                                    required: true, 
                                }
                            }
                            div { class: "form-group",
                                label { "内存 (MB)" }
                                input { 
                                    value: "{ram}",
                                    oninput: move |evt| ram.set(evt.value()),
                                    r#type: "number", 
                                    required: true, 
                                }
                            }
                            div { class: "form-group",
                                label { "存储 (GB)" }
                                input { 
                                    value: "{storage}",
                                    oninput: move |evt| storage.set(evt.value()),
                                    r#type: "number", 
                                    required: true, 
                                }
                            }
                            div { class: "form-group",
                                label { "API 端点" }
                                input { 
                                    value: "{endpoint}",
                                    oninput: move |evt| endpoint.set(evt.value()),
                                }
                            }
                            div { class: "form-group",
                                label { "API 令牌" }
                                input { 
                                    value: "{api_token}",
                                    oninput: move |evt| api_token.set(evt.value()),
                                }
                            }
                            div { class: "modal-actions",
                                button { 
                                    class: "btn-primary", 
                                    onclick: on_submit_edit,
                                    "保存" 
                                }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| editing_node.set(None),
                                    "取消"
                                }
                            }
                        }
                    }
                }
            }

            if session().loading {
                p { class: "status", "加载中..." }
            }

            if session().nodes.is_empty() {
                p { class: "status", "暂无节点数据。" }
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
                                    "编辑"
                                }
                            }
                            span { class: "meta", "ID: {node.id} | Region: {node.region}" }
                            div { class: "metrics",
                                div { class: "metric",
                                    span { "CPU: {node.cpu_cores_used} / {node.cpu_cores_total} Cores" }
                                    progress { max: "{node.cpu_cores_total}", value: "{node.cpu_cores_used}" }
                                }
                                div { class: "metric",
                                    span { "RAM: {node.memory_mb_used} / {node.memory_mb_total} MB" }
                                    progress { max: "{node.memory_mb_total}", value: "{node.memory_mb_used}" }
                                }
                                div { class: "metric",
                                    span { "Disk: {node.storage_gb_used} / {node.storage_gb_total} GB" }
                                    progress { max: "{node.storage_gb_total}", value: "{node.storage_gb_used}" }
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
