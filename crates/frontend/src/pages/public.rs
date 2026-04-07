use crate::api;

use crate::models::{Route, SessionState};

use dioxus::prelude::*;
use dioxus_i18n::t;

#[cfg(target_arch = "wasm32")]
use web_sys::window;

#[component]
pub fn StorefrontPage() -> Element {
    let navigator = use_navigator();
    let mut session = use_context::<Signal<SessionState>>();
    let is_logged_in = session().token.is_some();

    use_effect(move || {
        if session().public_plans.is_empty() {
            let api_base = session().api_base.clone();
            spawn(async move {
                if let Ok(plans) = api::get_public_plans(&api_base).await {
                    session.write().public_plans = plans;
                }
            });
        }
    });

    rsx! {
        div { class: "public-shell",
            header { class: "public-header",
                div { class: "brand",
                    div { class: "logo-mark", "C" }
                    div {
                        h1 { "{t!(\"app_title\")}" }
                        p { {t!("nodes_ready_for_sale", count: session().public_plans.len())} }
                    }
                }
                div { class: "header-actions",
                    if is_logged_in {
                        button {
                            class: "btn-secondary",
                            onclick: move |_| {
                                navigator.push(Route::ServicesPage {});
                            },
                            "{t!(\"customer_center_btn\")}"
                        }
                    } else {
                        button {
                            class: "btn-secondary",
                            onclick: move |_| {
                                navigator
                                    .push(Route::LoginPage {
                                        source: None,
                                        plan: None,
                                    });
                            },
                            "{t!(\"login_btn\")}"
                        }
                    }
                    button {
                        class: "btn-primary",
                        onclick: move |_| {
                            let first_plan = session()
                                .public_plans
                                .first()
                                .map(|p| p.code.clone())
                                .unwrap_or_else(|| "nat-standard".to_string());
                            navigator
                                .push(Route::OrderPage {
                                    plan: first_plan,
                                });
                        },
                        "{t!(\"try_order_btn\")}"
                    }
                }
            }

            main { class: "public-main",
                section { class: "hero",
                    h2 { "{t!(\"hero_title\")}" }
                    p {
                        "{t!(\"hero_desc\")}"
                    }
                    div { class: "chip-row",
                        span { class: "chip", {t!("hero_chip_nodes", count: session().public_plans.len())} }
                        span { class: "chip", "{t!(\"hero_chip_paypal\")}" }
                        span { class: "chip", "{t!(\"hero_chip_service\")}" }
                    }
                }

                section { class: "product-grid",
                    for (i , plan) in session().public_plans.iter().enumerate() {
                        article { class: "product-card", key: "{plan.id}",
                            div { class: "tag",
                                if i == 0 {
                                    "{t!(\"plan_starter\")}"
                                } else if i == 1 {
                                    "{t!(\"plan_popular\")}"
                                } else {
                                    "{t!(\"plan_business\")}"
                                }
                            }
                            h3 { "{plan.name}" }
                            p {
                                {t!("plan_spec", cores: plan.cpu_cores, cpu_pct: plan.cpu_allowance_pct, mem: plan.memory_mb, disk: plan.storage_gb, bw: plan.bandwidth_mbps, traffic: format_traffic_gb(plan.traffic_gb))}
                            }
                            div { class: "price", {t!("price_per_month", price: plan.monthly_price.clone())} }
                            button {
                                class: "btn-secondary",
                                onclick: {
                                    let code = plan.code.clone();
                                    move |_| {
                                        navigator
                                            .push(Route::OrderPage {
                                                plan: code.clone(),
                                            });
                                    }
                                },
                                "{t!(\"select_btn\")}"
                            }
                        }
                    }
                }

                section { class: "pay-preview",
                    h3 { "{t!(\"pay_preview_title\")}" }
                    ul {
                        li { "{t!(\"pay_paypal\")}" }
                        li { "{t!(\"pay_alipay\")}" }
                        li { "{t!(\"pay_bank\")}" }
                    }
                    p { class: "muted", "{t!(\"pay_preview_desc\")}" }
                }
            }
        }
    }
}

#[component]
pub fn OrderPage(plan: String) -> Element {
    let navigator = use_navigator();
    let mut session = use_context::<Signal<SessionState>>();

    use_effect(move || {
        if session().public_plans.is_empty() {
            let api_base = session().api_base.clone();
            spawn(async move {
                if let Ok(plans) = api::get_public_plans(&api_base).await {
                    session.write().public_plans = plans;
                }
            });
        }
    });

    let default_plan = if plan.is_empty() {
        "nat-standard".to_string()
    } else {
        plan.clone()
    };

    let mut selected_plan = use_signal(|| default_plan);
    let checkout_loading = use_signal(|| false);
    let checkout_error = use_signal(|| None::<String>);

    let selected_plan_details = session()
        .public_plans
        .iter()
        .find(|p| p.code == selected_plan())
        .cloned();

    let on_checkout = move |_| {
        if session().token.is_none() {
            navigator.push(Route::LoginPage {
                source: Some("order".to_string()),
                plan: Some(selected_plan()),
            });
            return;
        }

        let token = session().token.clone().unwrap_or_default();
        let api_base = session().api_base.clone();
        let plan_code = selected_plan().clone();
        let mut loading = checkout_loading;
        let mut error = checkout_error;

        spawn(async move {
            loading.set(true);
            error.set(None);

            match api::create_paypal_checkout(&api_base, &token, &plan_code).await {
                Ok(response) => {
                    loading.set(false);
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(win) = window() {
                            if win.location().set_href(&response.approval_url).is_err() {
                                error.set(Some(t!("payment_error_open_sandbox").to_string()));
                            }
                        } else {
                            error.set(Some(t!("payment_error_no_window").to_string()));
                        }
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let _ = response;
                        error.set(Some(
                            t!("payment_error_not_supported").to_string(),
                        ));
                    }
                }
                Err(err) => {
                    loading.set(false);
                    error.set(Some(err));
                }
            }
        });
    };

    rsx! {
        div { class: "public-shell",
            header { class: "public-header",
                h1 { "{t!(\"create_order_title\")}" }
                button {
                    class: "btn-secondary",
                    onclick: move |_| {
                        navigator.push(Route::StorefrontPage {});
                    },
                    "{t!(\"back_btn\")}"
                }
            }

            main { class: "public-main",
                section { class: "checkout-card",
                    h3 { "{t!(\"order_summary_title\")}" }
                    p { class: "muted",
                        "{t!(\"order_summary_desc\")}"
                    }
                    div { class: "order-meta",
                        label { "{t!(\"product_label\")}" }
                        select {
                            value: "{selected_plan()}",
                            onchange: move |evt| selected_plan.set(evt.value()),
                            for p in session().public_plans.clone() {
                                option { value: "{p.code}", "{p.name}" }
                            }
                        }
                    }

                    if let Some(plan) = selected_plan_details {
                        p {
                            {t!("spec_label", cores: plan.cpu_cores, cpu_pct: plan.cpu_allowance_pct, mem: plan.memory_mb, disk: plan.storage_gb, bw: plan.bandwidth_mbps, traffic: format_traffic_gb(plan.traffic_gb))}
                        }
                        p { {t!("monthly_price_label", price: plan.monthly_price.clone())} }
                    } else {
                        p { "{t!(\"loading_plan_details\")}" }
                    }

                    h4 { "{t!(\"payment_method_title\")}" }
                    div { class: "pay-methods",
                        label {
                            input {
                                r#type: "radio",
                                name: "pay",
                                checked: true,
                                disabled: true,
                            }
                            " {t!(\"pay_paypal\")}"
                        }
                        label {
                            input {
                                r#type: "radio",
                                name: "pay",
                                disabled: true,
                            }
                            " {t!(\"pay_alipay\")}"
                        }
                        label {
                            input {
                                r#type: "radio",
                                name: "pay",
                                disabled: true,
                            }
                            " {t!(\"pay_bank\")}"
                        }
                    }

                    if let Some(err) = &*checkout_error.read() {
                        p { class: "notice", "{err}" }
                    }

                    button {
                        class: "btn-primary full",
                        disabled: *checkout_loading.read(),
                        onclick: on_checkout,
                        if *checkout_loading.read() {
                            "{t!(\"opening_paypal\")}"
                        } else {
                            "{t!(\"proceed_checkout\")}"
                        }
                    }

                    if session().token.is_none() {
                        p { class: "notice",
                            "{t!(\"login_required_notice\")}"
                        }
                    }
                }
            }
        }
    }
}

fn format_traffic_gb(traffic_gb: i64) -> String {
    if traffic_gb == -1 {
        // Warning: This helper is called outside of component scope. 
        // Best approach is tracking it inside component, but if it has to be here, 
        // we might run into issues with `t!` macro which requires context. 
        // Actually, since Dioxus 0.5+ context isn't implicitly passed to arbitrary functions.
        // I will change it to return raw bytes and have it translated inside the macro.
        "Unlimited".to_string()
    } else {
        format!("{traffic_gb}")
    }
}
