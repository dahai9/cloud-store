use crate::api;
use crate::models::{AdminSessionState, NatPortLeaseCreateRequest};
use dioxus::prelude::*;

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
                    session.write().notice = Some("节点列表已刷新".to_string());
                }
                Err(e) => {
                    session.write().error = Some(format!("刷新节点失败: {e}"));
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
                    session.write().notice = Some("NAT 端口租约已刷新".to_string());
                }
                Err(e) => {
                    session.write().error = Some(format!("刷新租约失败: {e}"));
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
            session.write().error = Some("请先选择节点".to_string());
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
                    session.write().notice = Some("NAT 端口租约创建成功".to_string());
                    if let Ok(leases) = api::get_nat_port_leases(&api_base, &token).await {
                        session.write().nat_port_leases = leases;
                    }
                }
                Err(e) => {
                    session.write().error = Some(format!("创建失败: {e}"));
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
                    session.write().notice = Some("NAT 端口租约已删除".to_string());
                    if let Ok(leases) = api::get_nat_port_leases(&api_base, &token).await {
                        session.write().nat_port_leases = leases;
                    }
                }
                Err(e) => {
                    session.write().error = Some(format!("删除失败: {e}"));
                }
            }
            session.write().loading = false;
        });
    };

    rsx! {
        section { class: "card", id: "nat-port-leases",
            h2 { "NAT 端口租约" }
            p { class: "status",
                "这里管理 worker 用来给实例分配的端口段。每条租约对应一个节点上的一段公网端口范围。"
            }

            div { class: "actions",
                button { class: "btn-secondary", onclick: refresh_nodes, "刷新节点" }
                button { class: "btn-secondary", onclick: refresh_leases, "刷新租约" }
                button {
                    class: "btn-primary",
                    onclick: open_create_form,
                    disabled: session().nodes.is_empty(),
                    "创建租约"
                }
            }

            if session().nodes.is_empty() {
                p { class: "status",
                    "当前还没有可选节点，请先到 Nodes 页面添加节点。"
                }
            }

            if show_create_form() {
                div { class: "modal-overlay",
                    div { class: "modal-content",
                        h3 { "创建 NAT 端口租约" }
                        div {
                            div { class: "form-group",
                                label { "节点" }
                                select {
                                    value: "{selected_node_id()}",
                                    oninput: move |evt| selected_node_id.set(evt.value()),
                                    disabled: session().nodes.is_empty(),
                                    option { value: "", "请选择节点" }
                                    for node in session().nodes.clone() {
                                        option { value: "{node.id}", "{node.name} ({node.region})" }
                                    }
                                }
                            }
                            div { class: "form-group",
                                label { "公网 IP" }
                                input {
                                    value: "{public_ip}",
                                    oninput: move |evt| public_ip.set(evt.value()),
                                    placeholder: "203.0.113.10",
                                }
                            }
                            div { class: "form-group",
                                label { "起始端口" }
                                input {
                                    r#type: "number",
                                    value: "{start_port}",
                                    oninput: move |evt| start_port.set(evt.value()),
                                    placeholder: "10000",
                                }
                            }
                            div { class: "form-group",
                                label { "结束端口" }
                                input {
                                    r#type: "number",
                                    value: "{end_port}",
                                    oninput: move |evt| end_port.set(evt.value()),
                                    placeholder: "10100",
                                }
                            }
                            div { class: "modal-actions",
                                button { class: "btn-primary", onclick: save_lease, "提交" }
                                button {
                                    r#type: "button",
                                    class: "btn-secondary",
                                    onclick: move |_| show_create_form.set(false),
                                    "取消"
                                }
                            }
                        }
                    }
                }
            }

            if session().loading {
                p { class: "status", "处理中..." }
            }

            if session().nat_port_leases.is_empty() {
                p { class: "status", "暂无 NAT 端口租约数据。" }
            } else {
                ul { class: "list",
                    for lease in session().nat_port_leases.clone() {
                        li { class: "item",
                            div { class: "item-header",
                                strong { "{lease.node_name}" }
                                span { class: "meta",
                                    if lease.reserved {
                                        "已占用"
                                    } else {
                                        "可用"
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
                                        "删除"
                                    }
                                } else {
                                    span { class: "meta", "已被 worker 占用，不能删除" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
