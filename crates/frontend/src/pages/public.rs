
use crate::api;

use crate::models::{Route, SessionState, PLANS};

use dioxus::prelude::*;


#[cfg(target_arch = "wasm32")]
use web_sys::window;


#[component]
pub fn StorefrontPage() -> Element {
    let navigator = use_navigator();
    let session = use_context::<Signal<SessionState>>();
    let is_logged_in = session().token.is_some();

    rsx! {
        div { class: "public-shell",
            header { class: "public-header",
                div { class: "brand",
                    div { class: "logo-mark", "C" }
                    div {
                        h1 { "Cloud Store" }
                        p { "13 NAT VPS nodes ready for sale" }
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
                            navigator
                                .push(Route::OrderPage {
                                    plan: "nat-standard".to_string(),
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
                        span { class: "chip", "13 Available Nodes" }
                        span { class: "chip", "PayPal Required" }
                        span { class: "chip", "Service + Ticket Center" }
                    }
                }

                section { class: "product-grid",
                    for plan in PLANS {
                        article { class: "product-card",
                            div { class: "tag", "{plan.badge}" }
                            h3 { "{plan.name}" }
                            p { "{plan.spec}" }
                            div { class: "price", "{plan.monthly_price} / month" }
                            button {
                                class: "btn-secondary",
                                onclick: move |_| {
                                    navigator
                                        .push(Route::OrderPage {
                                            plan: plan.code.to_string(),
                                        });
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
    let session = use_context::<Signal<SessionState>>();
    
    let default_plan = if plan.is_empty() {
        "nat-standard".to_string()
    } else {
        plan.clone()
    };
    
    let mut selected_plan = use_signal(|| default_plan);
    let checkout_loading = use_signal(|| false);
    let checkout_error = use_signal(|| None::<String>);

    let selected_plan_details = PLANS
        .iter()
        .copied()
        .find(|plan| plan.code == selected_plan())
        .unwrap_or(PLANS[1]);

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
                        error.set(Some("当前平台暂不支持直接打开支付链接，请在 Web 端操作".to_string()));
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
                            option { value: "nat-mini", "NAT Mini" }
                            option { value: "nat-standard", "NAT Standard" }
                            option { value: "nat-pro", "NAT Pro" }
                        }
                    }
                    p { "Spec: {selected_plan_details.spec}" }
                    p { "Monthly Price: {selected_plan_details.monthly_price}" }

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
