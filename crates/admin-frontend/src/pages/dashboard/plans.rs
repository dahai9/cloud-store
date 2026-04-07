use crate::api;
use crate::models::{
    AdminPlanCreateRequest, AdminPlanItem, AdminPlanUpdateRequest, AdminSessionState,
};
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn PlansPage() -> Element {
    let mut session = use_context::<Signal<AdminSessionState>>();

    let mut is_editing = use_signal(|| false);
    let mut is_creating = use_signal(|| false);

    let mut form_id = use_signal(String::new);
    let mut form_code = use_signal(String::new);
    let mut form_name = use_signal(String::new);
    let mut form_monthly_price = use_signal(String::new);
    let mut form_memory_mb = use_signal(|| 1024i64);
    let mut form_storage_gb = use_signal(|| 50i64);
    let mut form_cpu_cores = use_signal(|| 1i64);
    let mut form_cpu_allowance_pct = use_signal(|| 100i64);
    let mut form_bandwidth_mbps = use_signal(|| 100i64);
    let mut form_traffic_gb = use_signal(|| 1000i64);
    let mut form_active = use_signal(|| true);
    let mut form_max_inventory = use_signal(String::new);

    let refresh_plans = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        spawn(async move {
            session.write().loading = true;
            match api::get_plans(&api_base, &token).await {
                Ok(plans) => {
                    let mut s = session.write();
                    s.plans = plans;
                    s.notice = Some(t!("plans_refresh_success"));
                }
                Err(e) => session.write().error = Some(t!("nodes_error_refresh", err: e)),
            }
            session.write().loading = false;
        });
    };

    let start_create = move |_| {
        is_creating.set(true);
        is_editing.set(false);
        form_id.set(String::new());
        form_code.set(String::new());
        form_name.set(String::new());
        form_monthly_price.set(String::new());
        form_memory_mb.set(1024);
        form_storage_gb.set(50);
        form_cpu_cores.set(1);
        form_cpu_allowance_pct.set(100);
        form_bandwidth_mbps.set(100);
        form_traffic_gb.set(1000);
        form_max_inventory.set(String::new());
    };

    let mut start_edit = move |plan: AdminPlanItem| {
        is_editing.set(true);
        is_creating.set(false);
        form_id.set(plan.id.clone());
        form_code.set(plan.code.clone());
        form_name.set(plan.name.clone());
        form_monthly_price.set(plan.monthly_price.clone());
        form_memory_mb.set(plan.memory_mb);
        form_storage_gb.set(plan.storage_gb);
        form_cpu_cores.set(plan.cpu_cores);
        form_cpu_allowance_pct.set(plan.cpu_allowance_pct);
        form_bandwidth_mbps.set(plan.bandwidth_mbps);
        form_traffic_gb.set(plan.traffic_gb);
        form_active.set(plan.active);
        form_max_inventory.set(
            plan.max_inventory
                .map(|v| v.to_string())
                .unwrap_or_default(),
        );
    };

    let cancel_edit = move |_| {
        is_editing.set(false);
        is_creating.set(false);
    };

    let save_plan = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();

        let code = form_code();
        let name = form_name();
        let monthly_price = form_monthly_price();
        let memory_mb = form_memory_mb();
        let storage_gb = form_storage_gb();
        let cpu_cores = form_cpu_cores();
        let cpu_allowance_pct = form_cpu_allowance_pct();
        let bandwidth_mbps = form_bandwidth_mbps();
        let traffic_gb = form_traffic_gb();
        let active = form_active();
        let max_inv_str = form_max_inventory();

        let max_inventory = if max_inv_str.trim().is_empty() {
            None
        } else {
            match max_inv_str.trim().parse::<i64>() {
                Ok(v) => Some(v),
                Err(_) => {
                    // Use a generic numeric error or hardcode if not common
                    session.write().error = Some("库存上限必须是数字".to_string());
                    return;
                }
            }
        };

        let creating = is_creating();
        let editing = is_editing();
        let pid = form_id();

        spawn(async move {
            session.write().loading = true;

            if creating {
                let payload = AdminPlanCreateRequest {
                    code,
                    name,
                    monthly_price,
                    memory_mb,
                    storage_gb,
                    cpu_cores,
                    cpu_allowance_pct,
                    bandwidth_mbps,
                    traffic_gb,
                };
                match api::create_plan(&api_base, &token, &payload).await {
                    Ok(_) => {
                        session.write().notice = Some(t!("plans_add_success"));
                        is_creating.set(false);
                        match api::get_plans(&api_base, &token).await {
                            Ok(plans) => session.write().plans = plans,
                            Err(e) => session.write().error = Some(t!("nodes_error_refresh", err: e)),
                        }
                    }
                    Err(e) => session.write().error = Some(t!("nodes_error_add", err: e)),
                }
            } else if editing {
                let payload = AdminPlanUpdateRequest {
                    code: Some(code),
                    name: Some(name),
                    monthly_price: Some(monthly_price),
                    memory_mb: Some(memory_mb),
                    storage_gb: Some(storage_gb),
                    cpu_cores: Some(cpu_cores),
                    cpu_allowance_pct: Some(cpu_allowance_pct),
                    bandwidth_mbps: Some(bandwidth_mbps),
                    traffic_gb: Some(traffic_gb),
                    active: Some(active),
                    max_inventory,
                };
                match api::update_plan(&api_base, &token, &pid, &payload).await {
                    Ok(_) => {
                        session.write().notice = Some(t!("plans_update_success"));
                        is_editing.set(false);
                        match api::get_plans(&api_base, &token).await {
                            Ok(plans) => session.write().plans = plans,
                            Err(e) => session.write().error = Some(t!("nodes_error_refresh", err: e)),
                        }
                    }
                    Err(e) => session.write().error = Some(t!("nodes_error_update", err: e)),
                }
            }

            session.write().loading = false;
        });
    };

    let toggle_active = move |plan_id: String, next_active: bool| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        spawn(async move {
            session.write().loading = true;
            let payload = AdminPlanUpdateRequest {
                code: None,
                name: None,
                monthly_price: None,
                memory_mb: None,
                storage_gb: None,
                cpu_cores: None,
                cpu_allowance_pct: None,
                bandwidth_mbps: None,
                traffic_gb: None,
                active: Some(next_active),
                max_inventory: None,
            };
            match api::update_plan(&api_base, &token, &plan_id, &payload).await {
                Ok(_) => {
                    session.write().notice = Some(t!("plans_update_success"));
                    match api::get_plans(&api_base, &token).await {
                        Ok(plans) => session.write().plans = plans,
                        Err(e) => session.write().error = Some(t!("nodes_error_refresh", err: e)),
                    }
                }
                Err(e) => session.write().error = Some(t!("nodes_error_update", err: e)),
            }
            session.write().loading = false;
        });
    };

    rsx! {
        section { class: "card", id: "plans",
            h2 { "{t!(\"plans_title\")}" }
            div { class: "actions",
                button { class: "btn-secondary", onclick: refresh_plans, "{t!(\"refresh\")}" }
                if !is_creating() && !is_editing() {
                    button { class: "btn-primary", onclick: start_create, "{t!(\"plans_add_btn\")}" }
                }
            }

            if is_creating() || is_editing() {
                div { class: "form",
                    h3 {
                        if is_creating() {
                            "{t!(\"plans_add_modal_title\")}"
                        } else {
                            {t!("plans_edit_modal_title", name: form_name())}
                        }
                    }

                    div { class: "field",
                        label { "{t!(\"plans_form_id\")}" }
                        input {
                            value: "{form_code()}",
                            oninput: move |evt| form_code.set(evt.value()),
                        }
                    }
                    div { class: "field",
                        label { "{t!(\"plans_form_name\")}" }
                        input {
                            value: "{form_name()}",
                            oninput: move |evt| form_name.set(evt.value()),
                        }
                    }
                    div { class: "field",
                        label { "{t!(\"plans_form_price\")}" }
                        input {
                            value: "{form_monthly_price()}",
                            oninput: move |evt| form_monthly_price.set(evt.value()),
                        }
                    }
                    div { class: "field",
                        label { "{t!(\"plans_form_cpu\")}" }
                        input {
                            r#type: "number",
                            min: "1",
                            step: "1",
                            value: "{form_cpu_cores()}",
                            oninput: move |evt| {
                                if let Ok(v) = evt.value().parse() {
                                    form_cpu_cores.set(v)
                                }
                            },
                        }
                    }
                    div { class: "field",
                        label { "{t!(\"plans_form_cpu_allowance\")}" }
                        input {
                            r#type: "number",
                            min: "1",
                            step: "1",
                            value: "{form_cpu_allowance_pct()}",
                            oninput: move |evt| {
                                if let Ok(v) = evt.value().parse() {
                                    form_cpu_allowance_pct.set(v)
                                }
                            },
                        }
                    }
                    div { class: "field",
                        label { "{t!(\"plans_form_ram\")}" }
                        input {
                            r#type: "number",
                            value: "{form_memory_mb()}",
                            oninput: move |evt| {
                                if let Ok(v) = evt.value().parse() {
                                    form_memory_mb.set(v)
                                }
                            },
                        }
                    }
                    div { class: "field",
                        label { "{t!(\"plans_form_storage\")}" }
                        input {
                            r#type: "number",
                            value: "{form_storage_gb()}",
                            oninput: move |evt| {
                                if let Ok(v) = evt.value().parse() {
                                    form_storage_gb.set(v)
                                }
                            },
                        }
                    }
                    div { class: "field",
                        label { "{t!(\"plans_form_bw\")}" }
                        input {
                            r#type: "number",
                            value: "{form_bandwidth_mbps()}",
                            oninput: move |evt| {
                                if let Ok(v) = evt.value().parse() {
                                    form_bandwidth_mbps.set(v)
                                }
                            },
                        }
                    }
                    div { class: "field",
                        label { "{t!(\"plans_form_traffic\")}" }
                        input {
                            r#type: "number",
                            min: "-1",
                            value: "{form_traffic_gb()}",
                            oninput: move |evt| {
                                if let Ok(v) = evt.value().parse() {
                                    form_traffic_gb.set(v)
                                }
                            },
                        }
                    }
                    if is_editing() {
                        div { class: "field",
                            label { "{t!(\"plans_form_max_inv_hint\")}" }
                            input {
                                value: "{form_max_inventory()}",
                                oninput: move |evt| form_max_inventory.set(evt.value()),
                            }
                        }
                        div { class: "field",
                            label {
                                input {
                                    r#type: "checkbox",
                                    checked: form_active(),
                                    onchange: move |evt| form_active.set(evt.value().parse().unwrap_or(false)),
                                }
                                " {t!(\"plans_form_active\")}"
                            }
                        }
                    }

                    div { class: "actions",
                        button { class: "btn-primary", onclick: save_plan, "{t!(\"save\")}" }
                        button { class: "btn-secondary", onclick: cancel_edit, "{t!(\"cancel\")}" }
                    }
                }
            } else {
                if session().loading {
                    p { class: "status", "{t!(\"processing\")}" }
                }
                if session().plans.is_empty() {
                    p { class: "status", "{t!(\"plans_no_data\")}" }
                } else {
                    ul { class: "list",
                        for plan in session().plans.clone() {
                            li { class: "item", key: "{plan.id}",
                                strong { "{plan.name} ({plan.code})" }
                                span { class: "meta", "Plan ID: {plan.id}" }
                                span { class: "meta", "Price: ${plan.monthly_price}/mo" }
                                span { class: "meta",
                                    "{plan.cpu_cores}C / {plan.cpu_allowance_pct}% CPU / {plan.memory_mb}MB / {plan.storage_gb}GB / {plan.bandwidth_mbps}Mbps / "
                                    if plan.traffic_gb == -1 {
                                        "{t!(\"plans_unlimited_traffic\")}"
                                    } else {
                                        {t!("plans_traffic_value", value: plan.traffic_gb)}
                                    }
                                }
                                span { class: "meta", "Active: {plan.active}" }
                                span { class: "meta",
                                    "{t!(\"plans_sold_max_label\")}: {plan.sold_inventory} / "
                                    if let Some(max) = plan.max_inventory {
                                        "{max}"
                                    } else {
                                        "{t!(\"plans_unlimited\")}"
                                    }
                                }
                                div { class: "actions",
                                    button {
                                        class: "btn-secondary",
                                        onclick: {
                                            let pid = plan.id.clone();
                                            let next = !plan.active;
                                            move |_| toggle_active(pid.clone(), next)
                                        },
                                        if plan.active {
                                            "{t!(\"delete\")}" // Usually "下架" in CN
                                        } else {
                                            "{t!(\"submit\")}" // Usually "上架" in CN
                                        }

                                    }
                                    button {
                                        class: "btn-secondary",
                                        onclick: {
                                            let p = plan.clone();
                                            move |_| start_edit(p.clone())
                                        },
                                        "{t!(\"edit\")}"
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


