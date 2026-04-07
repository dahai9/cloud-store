use crate::api;
use crate::models::{DashboardTab, Route, SessionState};
use dioxus::prelude::*;
use dioxus_i18n::t;
use dioxus_motion::prelude::*;
use gloo_timers::future::TimeoutFuture;

use super::shell::{DashboardShell, LoginRequiredView};

#[component]
pub fn ServicesPage() -> Element {
    let mut session = use_context::<Signal<SessionState>>();
    let state = session();

    if state.token.is_none() {
        return rsx! {
            LoginRequiredView {}
        };
    }

    let mut has_started = use_signal(|| false);

    use_effect(move || {
        if has_started() {
            return;
        }
        has_started.set(true);

        let api_base = session.peek().api_base.clone();
        let token = session.peek().token.clone().unwrap_or_default();
        spawn(async move {
            loop {
                if let Ok(instances) = api::fetch_instances(&api_base, &token).await {
                    let changed = {
                        let current = session.peek();
                        current.instances != instances
                    };

                    if changed {
                        session.write().instances = instances;
                    }
                }
                TimeoutFuture::new(15000).await;
            }
        });
    });

    rsx! {
        DashboardShell { title: "{t!(\"dash_my_services_title\")}", active_tab: DashboardTab::Services,
            section { class: "panel",
                h3 { "{t!(\"dash_active_instances\")}" }
                div { class: "service-list",
                    if state.instances.is_empty() {
                        p { class: "muted", "{t!(\"dash_no_instances\")}" }
                    } else {
                        for (i, item) in state.instances.iter().enumerate() {
                            AnimatedServiceItem {
                                key: "{item.id}",
                                i: i,
                                item: item.clone()
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn instance_status_class(status: &str) -> &'static str {
    match status.to_lowercase().as_str() {
        "running" => "status-running",
        "stopped" | "frozen" => "status-stopped",
        "pending" | "starting" | "provisioning" => "status-starting",
        "deleted" => "status-deleted",
        "unknown" => "status-unknown",
        _ => "status-unknown",
    }
}

#[component]
pub fn AnimatedServiceItem(i: usize, item: crate::models::InstanceItem) -> Element {
    let mut opacity = use_motion(0.0f32);
    let mut slide_y = use_motion(20.0f32);

    use_effect(move || {
        spawn(async move {
            gloo_timers::future::TimeoutFuture::new((i * 100) as u32).await;
            opacity.animate_to(
                1.0,
                AnimationConfig::new(AnimationMode::Spring(Spring::default())),
            );
            slide_y.animate_to(
                0.0,
                AnimationConfig::new(AnimationMode::Spring(Spring::default())),
            );
        });
    });

    rsx! {
        Link {
            to: Route::InstanceDetailPage {
                id: item.id.clone(),
            },
            article { class: "service-item",
                style: "opacity: {opacity.get_value()}; transform: translateY({slide_y.get_value()}px);",
                div {
                    h4 { "{item.plan_id.to_uppercase()} - {item.id}" }
                    p { class: "muted", {t!("dash_created_at", time: item.created_at.clone())} }
                }
                span { class: format!("pill {}", instance_status_class(&item.status)),
                    {t!("dash_status_label", status: item.status.clone())}
                }
            }
        }
    }
}

#[component]
pub fn InstanceDetailPage(id: String) -> Element {
    let session = use_context::<Signal<SessionState>>();
    let navigator = use_navigator();

    if session.peek().token.is_none() {
        return rsx! {
            LoginRequiredView {}
        };
    }

    let token = session.peek().token.clone().unwrap();
    let api_base = session.peek().api_base.clone();

    let mut instance = use_signal(|| None::<crate::models::InstanceItem>);
    let mut metrics = use_signal(|| None::<crate::models::InstanceMetrics>);
    let mut metrics_history = use_signal(Vec::<crate::models::InstanceMetrics>::new);
    let mut nat_mappings = use_signal(Vec::<crate::models::NatMappingItem>::new);
    let mut error = use_signal(|| None::<String>);
    let mut action_loading = use_signal(|| false);
    let mut show_password = use_signal(|| false);

    let mut detail_opacity = use_motion(0.0f32);
    let mut detail_y = use_motion(10.0f32);

    use_effect(move || {
        detail_opacity.animate_to(
            1.0,
            AnimationConfig::new(AnimationMode::Spring(Spring::default())),
        );
        detail_y.animate_to(
            0.0,
            AnimationConfig::new(AnimationMode::Spring(Spring::default())),
        );
    });

    // Initial load
    use_effect({
        let id = id.clone();
        let api_base = api_base.clone();
        let token = token.clone();
        move || {
            let id = id.clone();
            let api_base = api_base.clone();
            let token = token.clone();
            spawn(async move {
                match api::fetch_instance_details(&api_base, &token, &id).await {
                    Ok(data) => instance.set(Some(data)),
                    Err(err) => error.set(Some(err)),
                }
                if let Ok(data) = api::fetch_nat_mappings(&api_base, &token, &id).await {
                    nat_mappings.set(data);
                }
            });
        }
    });

    let mut metrics_started = use_signal(|| false);

    // Periodic metrics update
    use_effect({
        let id = id.clone();
        let api_base = api_base.clone();
        let token = token.clone();
        move || {
            if metrics_started() {
                return;
            }
            metrics_started.set(true);

            let id = id.clone();
            let api_base = api_base.clone();
            let token = token.clone();
            spawn(async move {
                loop {
                    if let Ok(data) = api::fetch_instance_metrics(&api_base, &token, &id).await {
                        // Update status from metrics if it's different
                        let needs_update = if let Some(inst) = instance.peek().as_ref() {
                            inst.status != data.status
                        } else {
                            false
                        };

                        if needs_update {
                            let mut inst = {
                                let guard = instance.peek();
                                guard.clone().unwrap()
                            };
                            inst.status = data.status.clone();
                            instance.set(Some(inst));
                        }

                        // Push to history
                        let mut history = metrics_history.peek().clone();
                        history.push(data.clone());
                        if history.len() > 30 {
                            history.remove(0);
                        }
                        metrics_history.set(history);
                        metrics.set(Some(data));
                    }
                    TimeoutFuture::new(5000).await;
                }
            });
        }
    });

    let on_action = use_callback({
        let id = id.clone();
        let api_base = api_base.clone();
        let token = token.clone();
        move |action: crate::models::InstanceAction| {
            let id = id.clone();
            let api_base = api_base.clone();
            let token = token.clone();
            spawn(async move {
                action_loading.set(true);
                match api::perform_instance_action(&api_base, &token, &id, action).await {
                    Ok(resp) => {
                        if let Some(pwd) = resp.new_password {
                            // If it's a password reset, update the state immediately and show it
                            let inst_opt = instance.peek().clone();
                            if let Some(mut inst) = inst_opt {
                                inst.root_password = Some(pwd);
                                instance.set(Some(inst));
                                show_password.set(true);
                            }
                        }

                        // Refresh details after a short delay
                        TimeoutFuture::new(1000).await;
                        if let Ok(data) = api::fetch_instance_details(&api_base, &token, &id).await
                        {
                            instance.set(Some(data));
                        }
                    }
                    Err(err) => error.set(Some(err)),
                }
                action_loading.set(false);
            });
        }
    });

    let on_console = {
        let id = id.clone();
        move |_| {
            navigator.push(Route::ConsolePage { id: id.clone() });
        }
    };

    let _on_add_nat_mapping = use_callback({
        let id = id.clone();
        let api_base = api_base.clone();
        let token = token.clone();
        move |payload: crate::models::CreateNatMappingRequest| {
            let id = id.clone();
            let api_base = api_base.clone();
            let token = token.clone();
            spawn(async move {
                match api::create_nat_mapping(&api_base, &token, &id, &payload).await {
                    Ok(new_mapping) => {
                        let mut current = nat_mappings.peek().clone();
                        current.insert(0, new_mapping);
                        nat_mappings.set(current);
                    }
                    Err(err) => error.set(Some(err)),
                }
            });
        }
    });

    let on_remove_nat_mapping = use_callback({
        let id = id.clone();
        let api_base = api_base.clone();
        let token = token.clone();
        move |mapping_id: String| {
            let id = id.clone();
            let api_base = api_base.clone();
            let token = token.clone();
            spawn(async move {
                match api::remove_nat_mapping(&api_base, &token, &id, &mapping_id).await {
                    Ok(_) => {
                        let mut current = nat_mappings.peek().clone();
                        current.retain(|m| m.id != mapping_id);
                        nat_mappings.set(current);
                    }
                    Err(err) => error.set(Some(err)),
                }
            });
        }
    });

    let Some(inst) = instance() else {
        return rsx! {
            DashboardShell { title: "{t!(\"dash_instance_details\")}", active_tab: DashboardTab::Services,
                div { class: "panel",
                    if let Some(err) = error() {
                        p { class: "notice error-notice", "{err}" }
                    } else {
                        p { "{t!(\"dash_loading_instance\")}" }
                    }
                }
            }
        };
    };

    rsx! {
        DashboardShell { title: "{t!(\"dash_manage_instance\")}", active_tab: DashboardTab::Services,
            div { class: "instance-detail-header",
                button {
                    class: "btn-secondary",
                    onclick: move |_| {
                        navigator.push(Route::ServicesPage {});
                    },
                    "{t!(\"dash_back_to_list\")}"
                }
                h3 { "{inst.plan_id.to_uppercase()} ({inst.id})" }
            }

            if let Some(err) = error() {
                p { class: "notice error-notice", "{err}" }
            }

            section { class: "grid-two",
                style: "opacity: {detail_opacity.get_value()}; transform: translateY({detail_y.get_value()}px);",
                article { class: "panel",
                    h4 { "{t!(\"dash_status_info\")}" }
                    div { class: "detail-list",
                        div { class: "detail-item",
                            span { class: "muted", "{t!(\"dash_status\")}" }
                            span { class: format!("pill {}", instance_status_class(&inst.status)),
                                {t!("dash_status_label", status: inst.status.clone())}
                            }
                        }
                        div { class: "detail-item",
                            span { class: "muted", "{t!(\"dash_node\")}" }
                            span { class: "fact", "{inst.node_id}" }
                        }
                        div { class: "detail-item",
                            span { class: "muted", "{t!(\"dash_os_template\")}" }
                            span { class: "fact", "{inst.os_template}" }
                        }
                        div { class: "detail-item",
                            span { class: "muted", "{t!(\"dash_root_password\")}" }
                            div { class: "password-field",
                                if show_password() {
                                    span { class: "fact mono", "{inst.root_password.as_deref().unwrap_or(\"********\")}" }
                                } else {
                                    span { class: "fact mono", "********" }
                                }
                                button {
                                    class: "btn-secondary btn-sm",
                                    onclick: move |_| show_password.set(!show_password()),
                                    if show_password() { "{t!(\"dash_hide_btn\")}" } else { "{t!(\"dash_show_btn\")}" }
                                }
                            }
                        }
                        div { class: "detail-item",
                            span { class: "muted", "{t!(\"dash_created_at_label\")}" }
                            span { class: "fact", "{inst.created_at}" }
                        }
                        div { class: "detail-item",
                            span { class: "muted", "{t!(\"dash_auto_renew\")}" }
                            button {
                                class: if inst.auto_renew { "btn-success btn-sm" } else { "btn-secondary btn-sm" },
                                onclick: {
                                    let id = inst.id.clone();
                                    let api_base = api_base.clone();
                                    let token = token.clone();
                                    let next = !inst.auto_renew;
                                    move |_| {
                                        let id = id.clone();
                                        let api_base = api_base.clone();
                                        let token = token.clone();
                                        spawn(async move {
                                            match api::update_auto_renew(&api_base, &token, &id, next).await {
                                                Ok(updated) => instance.set(Some(updated)),
                                                Err(e) => error.set(Some(e)),
                                            }
                                        });
                                    }
                                },
                                if inst.auto_renew { "{t!(\"dash_enabled\")}" } else { "{t!(\"dash_disabled\")}" }
                            }
                        }
                    }
                }

                article { class: "panel",
                    h4 { "{t!(\"dash_realtime_metrics\")}" }
                    if let Some(m) = metrics() {
                        div { class: "metrics-grid",
                            MetricCardWithChart {
                                index: 0,
                                title: "CPU".to_string(),
                                value: format!("{:.1}%", m.cpu_usage_percent),
                                history: metrics_history().iter().map(|mh| mh.cpu_usage_percent).collect(),
                                color: "#1f57cc",
                                max_val: 100.0,
                            }
                            MetricCardWithChart {
                                index: 1,
                                title: "RAM".to_string(),
                                value: format!("{:.0} MB", m.memory_used_mb),
                                history: metrics_history().iter().map(|mh| mh.memory_used_mb).collect(),
                                color: "#1dbf73",
                                // Assuming max RAM is not strictly known, we'll let it scale or use a reasonable max
                                max_val: metrics_history().iter().map(|mh| mh.memory_used_mb).fold(0.0, f64::max).max(512.0),
                            }
                            MetricCardWithChart {
                                index: 2,
                                title: "TX".to_string(),
                                value: format!("{:.1} KB/s", (m.network_tx_bytes as f64) / 1024.0),
                                history: metrics_history().iter().map(|mh| mh.network_tx_bytes as f64).collect(),
                                color: "#ff8b00",
                                max_val: metrics_history().iter().map(|mh| mh.network_tx_bytes as f64).fold(0.0, f64::max).max(1024.0),
                            }
                            MetricCardWithChart {
                                index: 3,
                                title: "RX".to_string(),
                                value: format!("{:.1} KB/s", (m.network_rx_bytes as f64) / 1024.0),
                                history: metrics_history().iter().map(|mh| mh.network_rx_bytes as f64).collect(),
                                color: "#9333ea",
                                max_val: metrics_history().iter().map(|mh| mh.network_rx_bytes as f64).fold(0.0, f64::max).max(1024.0),
                            }
                        }
                    } else {
                        p { class: "muted", "{t!(\"dash_loading_metrics\")}" }
                    }
                }
            }

            section { class: "panel",
                h4 { "{t!(\"dash_nat_port_mappings\")}" }
                div { class: "muted small",
                    if inst.nat_info.is_empty() {
                        p { "{t!(\"dash_public_ip_pending\")}" }
                    } else {
                        for pool in &inst.nat_info {
                            p { {t!("dash_public_ip_info", ip: pool.ip.clone(), range: pool.range.clone())} }
                        }
                    }
                }
                div { class: "nat-mappings-container",
                    table {
                        thead {
                            tr {
                                th { "{t!(\"dash_internal_port\")}" }
                                th { "{t!(\"dash_external_port\")}" }
                                th { "{t!(\"dash_protocol\")}" }
                                th { "{t!(\"dash_action\")}" }
                            }
                        }
                        tbody {
                            if nat_mappings().is_empty() {
                                tr {
                                    td { colspan: "4", "{t!(\"dash_no_port_mappings\")}" }
                                }
                            } else {
                                for mapping in nat_mappings() {
                                    tr {
                                        td { "{mapping.internal_port}" }
                                        td { "{mapping.external_port}" }
                                        td { "{mapping.protocol.to_uppercase()}" }
                                        td {
                                            button {
                                                class: "btn-secondary btn-sm",
                                                onclick: {
                                                    let mid = mapping.id.clone();
                                                    move |_| on_remove_nat_mapping(mid.clone())
                                                },
                                                "{t!(\"dash_delete_btn\")}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div { class: "add-mapping-form",
                        h5 { "{t!(\"dash_add_new_mapping\")}" }
                        div { class: "form-row",
                            input {
                                r#type: "number",
                                placeholder: "{t!(\"dash_internal_port_placeholder\")}",
                                id: "internal_port",
                                oninput: move |_e| {
                                    // We'll use a signal for the form state or just use JS to get values on click
                                }
                            }
                            input {
                                r#type: "number",
                                placeholder: "{t!(\"dash_external_port_placeholder\")}",
                                id: "external_port"
                            }
                            select {
                                id: "protocol",
                                option { value: "tcp", "TCP" }
                                option { value: "udp", "UDP" }
                            }
                            button {
                                class: "btn-primary",
                                onclick: move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        use wasm_bindgen::JsCast;
                                        use web_sys::HtmlInputElement;
                                        use web_sys::HtmlSelectElement;
                                        let window = web_sys::window().unwrap();
                                        let document = window.document().unwrap();
                                        let i_port = document.get_element_by_id("internal_port").unwrap().dyn_into::<HtmlInputElement>().unwrap().value().parse::<i32>().unwrap_or(0);
                                        let e_port = document.get_element_by_id("external_port").unwrap().dyn_into::<HtmlInputElement>().unwrap().value().parse::<i32>().unwrap_or(0);
                                        let proto = document.get_element_by_id("protocol").unwrap().dyn_into::<HtmlSelectElement>().unwrap().value();
                                        if i_port > 0 && e_port > 0 {
                                            _on_add_nat_mapping(crate::models::CreateNatMappingRequest {
                                                internal_port: i_port,
                                                external_port: e_port,
                                                protocol: proto,
                                            });
                                        }
                                    }
                                },
                                "{t!(\"dash_add_btn\")}"
                            }
                        }
                    }
                }
            }

            section { class: "panel danger-zone",
                h4 { "{t!(\"dash_instance_actions\")}" }
                div { class: "action-bar",
                    button {
                        class: "btn-primary",
                        disabled: action_loading(),
                        onclick: move |_| on_action(crate::models::InstanceAction::Start),
                        if action_loading() {
                            span { class: "spinner" }
                            "{t!(\"dash_processing\")}"
                        } else {
                            "{t!(\"dash_start_btn\")}"
                        }
                    }
                    button {
                        class: "btn-secondary",
                        disabled: action_loading(),
                        onclick: move |_| on_action(crate::models::InstanceAction::Stop),
                        if action_loading() {
                            span { class: "spinner" }
                            "{t!(\"dash_processing\")}"
                        } else {
                            "{t!(\"dash_stop_btn\")}"
                        }
                    }
                    button {
                        class: "btn-secondary",
                        disabled: action_loading(),
                        onclick: move |_| on_action(crate::models::InstanceAction::Restart),
                        if action_loading() {
                            span { class: "spinner" }
                            "{t!(\"dash_processing\")}"
                        } else {
                            "{t!(\"dash_restart_btn\")}"
                        }
                    }
                    button {
                        class: "btn-secondary",
                        disabled: action_loading(),
                        onclick: move |_| {
                             if instance.peek().is_some() {
                                // Simple reload for now, real app would show a confirmation modal
                                on_action(crate::models::InstanceAction::Reinstall {
                                    os_template: None,
                                });
                             }
                        },
                        if action_loading() {
                            span { class: "spinner" }
                            "{t!(\"dash_processing\")}"
                        } else {
                            "{t!(\"dash_reinstall_btn\")}"
                        }
                    }
                    button {
                        class: "btn-secondary",
                        disabled: action_loading(),
                        onclick: move |_| {
                            on_action(crate::models::InstanceAction::ResetPassword {
                                new_password: None,
                            });
                        },
                        if action_loading() {
                            span { class: "spinner" }
                            "{t!(\"dash_processing\")}"
                        } else {
                            "{t!(\"dash_reset_password_btn\")}"
                        }
                    }
                    button { class: "btn-primary", onclick: on_console, "{t!(\"dash_open_console_btn\")}" }
                }
            }
        }
    }
}

#[component]
pub fn ConsolePage(id: String) -> Element {
    let session = use_context::<Signal<SessionState>>();
    let navigator = use_navigator();

    if session.peek().token.is_none() {
        return rsx! {
            LoginRequiredView {}
        };
    }

    let token = session.peek().token.clone().unwrap();
    let api_base = session.peek().api_base.clone();

    // Build the proxy WebSocket URL:
    //   http://host:port  ->  ws://host:port/api/instances/{id}/console/ws?token=JWT
    //   https://host:port ->  wss://host:port/api/instances/{id}/console/ws?token=JWT
    let ws_url = use_memo({
        let id = id.clone();
        let api_base = api_base.clone();
        let token = token.clone();
        move || crate::api::build_console_ws_url(&api_base, &id, &token)
    });

    rsx! {
        DashboardShell { title: "{t!(\"dash_instance_console\")}", active_tab: DashboardTab::Services,
            div { class: "instance-detail-header",
                button {
                    class: "btn-secondary",
                    onclick: move |_| {
                        navigator
                            .push(Route::InstanceDetailPage {
                                id: id.clone(),
                            });
                    },
                    "{t!(\"dash_back_to_instance\")}"
                }
                h3 { "{t!(\"dash_interactive_console\")}" }
            }

            div { class: "panel",
                div { class: "terminal-wrapper",
                    crate::terminal::TerminalView { url: ws_url() }
                }
            }
        }
    }
}

#[component]
fn MetricCardWithChart(
    title: String,
    value: String,
    history: Vec<f64>,
    color: &'static str,
    max_val: f64,
    index: usize,
) -> Element {
    let mut opacity = use_motion(0.0f32);
    let mut slide_y = use_motion(20.0f32);

    use_effect(move || {
        spawn(async move {
            gloo_timers::future::TimeoutFuture::new((index * 100) as u32).await;
            opacity.animate_to(
                1.0,
                AnimationConfig::new(AnimationMode::Spring(Spring::default())),
            );
            slide_y.animate_to(
                0.0,
                AnimationConfig::new(AnimationMode::Spring(Spring::default())),
            );
        });
    });
    let width = 200;
    let height = 44;

    // Create SVG points
    let (line_points, fill_points) = if history.len() < 2 {
        ("".to_string(), "".to_string())
    } else {
        let x_step = width as f64 / (history.len() - 1) as f64;
        let coords: Vec<(f64, f64)> = history
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let x = i as f64 * x_step;
                let normalized_v = if max_val > 0.0 {
                    (v / max_val).min(1.0)
                } else {
                    0.0
                };
                let y = height as f64 - (normalized_v * height as f64);
                (x, y)
            })
            .collect();

        let line_p = coords
            .iter()
            .map(|(x, y)| format!("{:.1},{:.1}", x, y))
            .collect::<Vec<_>>()
            .join(" ");

        // To create a filled area, we need to close the shape by adding bottom-right and bottom-left points
        let mut fill_p = line_p.clone();
        if let Some((last_x, _)) = coords.last() {
            fill_p.push_str(&format!(
                " {:.1},{:.1} {:.1},{:.1}",
                last_x, height, 0, height
            ));
        }

        (line_p, fill_p)
    };

    rsx! {
        div { class: "metric-card-chart",
            style: "opacity: {opacity.get_value()}; transform: translateY({slide_y.get_value()}px);",
            div { class: "metric-info",
                p { class: "muted", "{title}" }
                p { class: "fact", "{value}" }
            }
            div { class: "metric-chart-container",
                svg {
                    width: "100%",
                    height: "100%",
                    view_box: "0 0 {width} {height}",
                    preserve_aspect_ratio: "none",

                    // Area fill (semi-transparent)
                    polygon {
                        fill: "{color}",
                        fill_opacity: "0.1",
                        points: "{fill_points}",
                    }

                    // Top trend line
                    polyline {
                        fill: "none",
                        stroke: "{color}",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        points: "{line_points}",
                    }
                }
            }
        }
    }
}
