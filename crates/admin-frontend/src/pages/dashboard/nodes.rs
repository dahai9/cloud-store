use dioxus::prelude::*;
use crate::models::AdminSessionState;
use crate::api;

#[component]
pub fn NodesPage() -> Element {
    let mut session = use_context::<Signal<AdminSessionState>>();

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

    rsx! {
        section { class: "card", id: "nodes",
            h2 { "节点库存面板（admin API）" }
            div { class: "actions",
                button {
                    class: "btn-secondary",
                    onclick: refresh_nodes,
                    "刷新节点"
                }
            }

            if session().loading {
                p { class: "status", "刷新中..." }
            }

            if session().nodes.is_empty() {
                p { class: "status", "暂无节点数据。" }
            } else {
                ul { class: "list",
                    for node in session().nodes.clone() {
                        li { class: "item",
                            strong { "{node.name}" }
                            span { class: "meta", "Node ID: {node.id}" }
                            span { class: "meta", "Region: {node.region}" }
                            span { class: "meta",
                                "Capacity: {node.used_capacity}/{node.total_capacity}"
                            }
                        }
                    }
                }
            }
        }
    }
}
