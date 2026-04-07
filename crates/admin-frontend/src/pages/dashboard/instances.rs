use crate::api;
use crate::models::AdminSessionState;
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn InstancesPage() -> Element {
    let mut session = use_context::<Signal<AdminSessionState>>();

    let refresh_instances = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();

        spawn(async move {
            session.write().loading = true;
            match api::get_instances(&api_base, &token, None).await {
                Ok(instances) => {
                    let mut s = session.write();
                    s.instances = instances;
                    s.notice = Some(t!("nodes_refresh_success"));
                }
                Err(e) => {
                    session.write().error = Some(t!("nodes_error_refresh", err: e));
                }
            }
            session.write().loading = false;
        });
    };

    rsx! {
        section { class: "card", id: "instances",
            h2 { "{t!(\"instances_title\")}" }
            div { class: "actions",
                button {
                    class: "btn-primary",
                    onclick: move |_| {
                        session.write().notice = Some("API call to /api/admin/instances is available for manual creation but UI form is pending.".to_string());
                    },
                    "{t!(\"nodes_add_btn\")}"
                }
                button {
                    class: "btn-secondary",
                    onclick: refresh_instances,
                    "{t!(\"refresh\")}"
                }
            }

            if session().loading {
                p { class: "status", "{t!(\"loading\")}" }
            }

            if session().instances.is_empty() {
                p { class: "status", "{t!(\"instances_no_data\")}" }
            } else {
                div { class: "table-container",
                    table { class: "admin-table",
                        thead {
                            tr {
                                th { "{t!(\"instances_table_id\")}" }
                                th { "{t!(\"instances_table_user\")}" }
                                th { "{t!(\"instances_table_node\")}" }
                                th { "{t!(\"instances_table_plan\")}" }
                                th { "{t!(\"instances_table_status\")}" }
                                th { "{t!(\"instances_table_image\")}" }
                                th { "{t!(\"instances_table_created\")}" }
                                th { "{t!(\"actions_label\")}" }
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
                                    td {
                                        button {
                                            class: "btn-secondary btn-sm",
                                            onclick: {
                                                let id = inst.id.clone();
                                                let api_base = session().api_base.clone();
                                                let token = session().token.clone().unwrap_or_default();
                                                move |_| {
                                                    let id = id.clone();
                                                    let api_base = api_base.clone();
                                                    let token = token.clone();
                                                    spawn(async move {
                                                        // Simple prompt for refund
                                                        let refund = "0.00".to_string(); // In real app we would use a modal
                                                        let payload = crate::models::AdminInstanceDeleteRequest {
                                                            refund_amount: Some(refund),
                                                        };
                                                        session.write().loading = true;
                                                        match api::delete_instance(&api_base, &token, &id, &payload).await {
                                                            Ok(_) => {
                                                                session.write().notice = Some(t!("instances_action_success"));
                                                                if let Ok(instances) = api::get_instances(&api_base, &token, None).await {
                                                                    session.write().instances = instances;
                                                                }
                                                            }
                                                            Err(e) => session.write().error = Some(e),
                                                        }
                                                        session.write().loading = false;
                                                    });
                                                }
                                            },
                                            "{t!(\"delete\")}"
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
}
