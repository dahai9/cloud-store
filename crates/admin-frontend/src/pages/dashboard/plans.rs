use dioxus::prelude::*;
use crate::models::{AdminSessionState, AdminPlanUpdateRequest};
use crate::api;

#[component]
pub fn PlansPage() -> Element {
    let mut session = use_context::<Signal<AdminSessionState>>();
    let mut selected_plan_id = use_signal(String::new);
    let mut selected_plan_inventory = use_signal(String::new);

    let refresh_plans = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        spawn(async move {
            session.write().loading = true;
            match api::get_plans(&api_base, &token).await {
                Ok(plans) => session.write().plans = plans,
                Err(e) => session.write().error = Some(format!("刷新失败: {e}")),
            }
            session.write().loading = false;
        });
    };

    let update_inventory = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        let plan_id = selected_plan_id();
        let inv_text = selected_plan_inventory();

        spawn(async move {
            if plan_id.is_empty() { return; }
            let max_inventory = if inv_text.trim().is_empty() {
                None
            } else {
                match inv_text.trim().parse::<i64>() {
                    Ok(v) => Some(v),
                    Err(_) => {
                        session.write().error = Some("库存上限必须是数字".to_string());
                        return;
                    }
                }
            };

            session.write().loading = true;
            let payload = AdminPlanUpdateRequest { active: None, max_inventory };
            match api::update_plan(&api_base, &token, &plan_id, &payload).await {
                Ok(_) => {
                    session.write().notice = Some("Plan 库存设置已更新".to_string());
                    // Refresh
                    if let Ok(plans) = api::get_plans(&api_base, &token).await {
                        session.write().plans = plans;
                    }
                }
                Err(e) => session.write().error = Some(format!("更新失败: {e}")),
            }
            session.write().loading = false;
        });
    };

    let toggle_active = move |plan_id: String, next_active: bool| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        spawn(async move {
            session.write().loading = true;
            let payload = AdminPlanUpdateRequest { active: Some(next_active), max_inventory: None };
            match api::update_plan(&api_base, &token, &plan_id, &payload).await {
                Ok(_) => {
                    session.write().notice = Some("Plan 上下架更新成功".to_string());
                    if let Ok(plans) = api::get_plans(&api_base, &token).await {
                        session.write().plans = plans;
                    }
                }
                Err(e) => session.write().error = Some(format!("更新失败: {e}")),
            }
            session.write().loading = false;
        });
    };

    rsx! {
        section { class: "card", id: "plans",
            h2 { "产品上架/库存" }
            div { class: "actions",
                button { class: "btn-secondary", onclick: refresh_plans, "刷新 Plan" }
                button { class: "btn-primary", onclick: update_inventory, "更新选中 Plan 库存" }
            }

            div { class: "field",
                label { "Plan ID" }
                input {
                    value: "{selected_plan_id()}",
                    oninput: move |evt| selected_plan_id.set(evt.value()),
                    placeholder: "输入 Plan ID",
                }
            }
            div { class: "field",
                label { "Max Inventory（留空表示不限）" }
                input {
                    value: "{selected_plan_inventory()}",
                    oninput: move |evt| selected_plan_inventory.set(evt.value()),
                    placeholder: "例如: 200",
                }
            }

            if session().loading { p { class: "status", "处理中..." } }

            if session().plans.is_empty() {
                p { class: "status", "暂无 Plan 数据。" }
            } else {
                ul { class: "list",
                    for plan in session().plans.clone() {
                        li { class: "item",
                            strong { "{plan.name} ({plan.code})" }
                            span { class: "meta", "Plan ID: {plan.id}" }
                            span { class: "meta", "Price: ${plan.monthly_price}" }
                            span { class: "meta", "Active: {plan.active}" }
                            span { class: "meta",
                                "Sold/Max: {plan.sold_inventory}/{plan.max_inventory.unwrap_or(-1)} (-1 表示不限)"
                            }
                            div { class: "actions",
                                button {
                                    class: "btn-secondary",
                                    onclick: {
                                        let pid = plan.id.clone();
                                        let next = !plan.active;
                                        move |_| toggle_active(pid.clone(), next)
                                    },
                                    if plan.active { "下架" } else { "上架" }
                                }
                                button {
                                    class: "btn-secondary",
                                    onclick: {
                                        let pid = plan.id.clone();
                                        let max_inv = plan.max_inventory.map(|v| v.to_string()).unwrap_or_default();
                                        move |_| {
                                            selected_plan_id.set(pid.clone());
                                            selected_plan_inventory.set(max_inv.clone());
                                        }
                                    },
                                    "设为当前编辑"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
