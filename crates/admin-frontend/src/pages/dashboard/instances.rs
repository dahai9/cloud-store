use dioxus::prelude::*;
use crate::models::AdminSessionState;
use crate::api;

#[component]
pub fn InstancesPage() -> Element {
    let mut session = use_context::<Signal<AdminSessionState>>();

    let refresh_instances = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        
        spawn(async move {
            session.write().loading = true;
            match api::get_instances(&api_base, &token).await {
                Ok(instances) => {
                    let mut s = session.write();
                    s.instances = instances;
                    s.notice = Some("实例列表已刷新".to_string());
                }
                Err(e) => {
                    session.write().error = Some(format!("刷新失败: {e}"));
                }
            }
            session.write().loading = false;
        });
    };

    rsx! {
        section { class: "card", id: "instances",
            h2 { "全平台实例概览" }
            div { class: "actions",
                button {
                    class: "btn-secondary",
                    onclick: refresh_instances,
                    "刷新列表"
                }
            }

            if session().loading {
                p { class: "status", "刷新中..." }
            }

            if session().instances.is_empty() {
                p { class: "status", "暂无实例数据。" }
            } else {
                div { class: "table-container",
                    table { class: "admin-table",
                        thead {
                            tr {
                                th { "ID" }
                                th { "用户" }
                                th { "节点" }
                                th { "套餐" }
                                th { "状态" }
                                th { "镜像" }
                                th { "创建时间" }
                            }
                        }
                        tbody {
                            for inst in session().instances.clone() {
                                tr {
                                    td { class: "mono", "{inst.id}" }
                                    td { "{inst.user_email}" }
                                    td { "{inst.node_name}" }
                                    td { "{inst.plan_name}" }
                                    td {
                                        span { class: "status-tag {inst.status}", "{inst.status}" }
                                    }
                                    td { "{inst.os_template}" }
                                    td { "{inst.created_at}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
