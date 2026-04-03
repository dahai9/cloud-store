use crate::api;

use crate::models::{Route, SessionState};

use dioxus::prelude::*;

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
                        h1 { "Cloud Store" }
                        p { "{session().public_plans.len()} NAT VPS nodes ready for sale" }
                    }
                }
                div { class: "header-actions",
                    if is_logged_in {
                        button {
                            class: "btn-secondary",
                            onclick: move |_| {
                                navigator.push(Route::ServicesPage {});
                            },
                            "Customer Center"
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
                            "Login"
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
                        "Try Order"
                    }
                }
            }

            main { class: "public-main",
                section { class: "hero",
                    h2 { "NAT VPS Resale Platform" }
                    p {
                        "Guests can browse products and payment methods. Login is required only when final checkout starts or protected pages are opened."
                    }
                    div { class: "chip-row",
                        span { class: "chip", "{session().public_plans.len()} Available Nodes" }
                        span { class: "chip", "PayPal Required" }
                        span { class: "chip", "Service + Ticket Center" }
                    }
                }

                section { class: "product-grid",
                    for (i , plan) in session().public_plans.iter().enumerate() {
                        article { class: "product-card", key: "{plan.id}",
                            div { class: "tag",
                                if i == 0 {
                                    "Starter"
                                } else if i == 1 {
                                    "Most Popular"
                                } else {
                                    "Business"
                                }
                            }
                            h3 { "{plan.name}" }
                            p {
                                "{plan.cpu_cores}C / {plan.memory_mb}MB RAM / {plan.storage_gb}GB SSD / {plan.bandwidth_mbps}Mbps / {format_traffic_gb(plan.traffic_gb)}"
                            }
                            div { class: "price", "${plan.monthly_price} / month" }
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
                                "Select"
                            }
                        }
                    }
                }

                section { class: "pay-preview",
                    h3 { "Available Payment Methods" }
                    ul {
                        li { "PayPal (required)" }
                        li { "Alipay" }
                        li { "Bank Transfer" }
                    }
                    p { class: "muted", "You can view products and payment methods without login." }
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
                                error.set(Some("无法打开 PayPal 沙箱支付页面".to_string()));
                            }
                        } else {
                            error.set(Some("浏览器窗口不可用".to_string()));
                        }
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let _ = response;
                        error.set(Some(
                            "当前平台暂不支持直接打开支付链接，请在 Web 端操作".to_string(),
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
                h1 { "Create Order" }
                button {
                    class: "btn-secondary",
                    onclick: move |_| {
                        navigator.push(Route::StorefrontPage {});
                    },
                    "Back"
                }
            }

            main { class: "public-main",
                section { class: "checkout-card",
                    h3 { "Order Summary" }
                    p { class: "muted",
                        "选择套餐后会创建订单并跳转到 PayPal 沙箱支付页。"
                    }
                    div { class: "order-meta",
                        label { "Product" }
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
                            "Spec: {plan.cpu_cores}C / {plan.memory_mb}MB RAM / {plan.storage_gb}GB SSD / {plan.bandwidth_mbps}Mbps / {format_traffic_gb(plan.traffic_gb)}"
                        }
                        p { "Monthly Price: ${plan.monthly_price}" }
                    } else {
                        p { "Loading plan details..." }
                    }

                    h4 { "Payment Method" }
                    div { class: "pay-methods",
                        label {
                            input {
                                r#type: "radio",
                                name: "pay",
                                checked: true,
                                disabled: true,
                            }
                            " PayPal"
                        }
                        label {
                            input {
                                r#type: "radio",
                                name: "pay",
                                disabled: true,
                            }
                            " Alipay"
                        }
                        label {
                            input {
                                r#type: "radio",
                                name: "pay",
                                disabled: true,
                            }
                            " Bank Transfer"
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
                            "Opening PayPal Sandbox..."
                        } else {
                            "Proceed To Checkout"
                        }
                    }

                    if session().token.is_none() {
                        p { class: "notice",
                            "You can configure the order now. Login is required only at checkout."
                        }
                    }
                }
            }
        }
    }
}

fn format_traffic_gb(traffic_gb: i64) -> String {
    if traffic_gb == -1 {
        "无限流量".to_string()
    } else {
        format!("{traffic_gb}GB 流量")
    }
}
