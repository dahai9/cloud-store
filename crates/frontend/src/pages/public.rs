use crate::api;

use crate::models::{Route, SessionState};

use dioxus::prelude::*;
use dioxus_i18n::prelude::*;
use dioxus_i18n::t;
use dioxus_motion::prelude::*;

#[cfg(target_arch = "wasm32")]
use web_sys::window;

#[component]
pub fn StorefrontPage() -> Element {
    let navigator = use_navigator();
    let mut session = use_context::<Signal<SessionState>>();
    let is_logged_in = session().token.is_some();

    let mut hero_opacity = use_motion(0.0f32);
    let mut hero_y = use_motion(20.0f32);

    use_effect(move || {
        hero_opacity.animate_to(
            1.0,
            AnimationConfig::new(AnimationMode::Spring(Spring::default())),
        );
        hero_y.animate_to(
            0.0,
            AnimationConfig::new(AnimationMode::Spring(Spring::default())),
        );
    });

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
                    button {
                        class: "btn-secondary",
                        onclick: move |_| {
                            use unic_langid::langid;
                            let mut i18n = i18n();
                            if i18n.language() == langid!("en-US") {
                                i18n.set_language(langid!("zh-CN"));
                            } else {
                                i18n.set_language(langid!("en-US"));
                            }
                        },
                        "{t!(\"switch_lang\")}"
                    }
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
                section { class: "hero", style: "opacity: {hero_opacity.get_value()}; transform: translateY({hero_y.get_value()}px);",
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
                        AnimatedProductCard {
                            key: "{plan.id}",
                            i: i,
                            plan: plan.clone()
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
pub fn AnimatedProductCard(i: usize, plan: crate::models::PublicPlanItem) -> Element {
    let navigator = use_navigator();
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
        article {
            class: "product-card",
            style: "opacity: {opacity.get_value()}; transform: translateY({slide_y.get_value()}px);",
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
                class: "btn-secondary full",
                onclick: {
                    let code = plan.code.clone();
                    move |_| {
                        navigator.push(Route::OrderPage { plan: code.clone() });
                    }
                },
                "{t!(\"select_btn\")}"
            }
        }
    }
}

#[component]
pub fn OrderPage(plan: String) -> Element {
    let navigator = use_navigator();
    let mut session = use_context::<Signal<SessionState>>();

    let mut opacity = use_motion(0.0f32);
    let mut slide_y = use_motion(20.0f32);

    use_effect(move || {
        opacity.animate_to(
            1.0,
            AnimationConfig::new(AnimationMode::Spring(Spring::default())),
        );
        slide_y.animate_to(
            0.0,
            AnimationConfig::new(AnimationMode::Spring(Spring::default())),
        );
    });

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

    let mut selected_plan = use_signal(|| {
        if plan.is_empty() {
            "nat-standard".to_string()
        } else {
            plan.clone()
        }
    });

    use_effect(move || {
        if !plan.is_empty() {
            selected_plan.set(plan.clone());
        }
    });

    #[allow(unused_mut)]
    let mut checkout_loading = use_signal(|| false);
    #[allow(unused_mut)]
    let mut checkout_error = use_signal(|| None::<String>);
    #[allow(unused_mut)]
    let mut payment_method = use_signal(|| "paypal".to_string());
    #[allow(unused_mut)]
    let mut payment_success = use_signal(|| false);

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
        let method = payment_method();
        let mut loading = checkout_loading;
        let mut error = checkout_error;
        let mut success = payment_success;

        spawn(async move {
            *loading.write() = true;
            *error.write() = None;

            let method_opt = if method == "balance" {
                Some("balance".to_string())
            } else {
                Some("paypal".to_string())
            };

            match api::create_paypal_checkout(&api_base, &token, &plan_code, method_opt).await {
                Ok(response) => {
                    if method == "balance" {
                        *loading.write() = false;
                        *success.write() = true;
                        gloo_timers::future::TimeoutFuture::new(1500).await;
                        navigator.push(Route::ServicesPage {});
                        return;
                    }

                    loading.set(false);
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(win) = window() {
                            if win.location().set_href(&response.approval_url).is_err() {
                                *error.write() = Some(t!("payment_error_open_sandbox").to_string());
                            }
                        } else {
                            *error.write() = Some(t!("payment_error_no_window").to_string());
                        }
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let _ = response;
                        *error.write() = Some(t!("payment_error_not_supported").to_string());
                    }
                }
                Err(err) => {
                    *loading.write() = false;
                    *error.write() = Some(err);
                }
            }
        });
    };

    rsx! {
        div { class: "public-shell",
            header { class: "public-header",
                h1 { "{t!(\"create_order_title\")}" }
                div { class: "flex-row", style: "gap: 10px;",
                    button {
                        class: "btn-secondary",
                        onclick: move |_| {
                            use unic_langid::langid;
                            let mut i18n = i18n();
                            if i18n.language() == langid!("en-US") {
                                i18n.set_language(langid!("zh-CN"));
                            } else {
                                i18n.set_language(langid!("en-US"));
                            }
                        },
                        "{t!(\"switch_lang\")}"
                    }
                    button {
                        class: "btn-secondary",
                        onclick: move |_| {
                            navigator.push(Route::StorefrontPage {});
                        },
                        "{t!(\"back_btn\")}"
                    }
                }
            }

            main { class: "public-main",
                section { class: "checkout-card", style: "opacity: {opacity.get_value()}; transform: translateY({slide_y.get_value()}px);",
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

                    if let Some(ref plan) = selected_plan_details {
                        p {
                            {t!("spec_label", cores: plan.cpu_cores, cpu_pct: plan.cpu_allowance_pct, mem: plan.memory_mb, disk: plan.storage_gb, bw: plan.bandwidth_mbps, traffic: format_traffic_gb(plan.traffic_gb))}
                        }
                        p { {t!("monthly_price_label", price: plan.monthly_price.clone())} }
                    } else {
                        p { "{t!(\"loading_plan_details\")}" }
                    }

                    h4 { "{t!(\"payment_method_title\")}" }
                    div { class: "pay-methods-grid",
                        label {
                            class: if payment_method() == "paypal" { "pay-method-item active" } else { "pay-method-item" },
                            input {
                                r#type: "radio",
                                name: "pay",
                                checked: payment_method() == "paypal",
                                onchange: move |_| *payment_method.write() = "paypal".to_string(),
                            }
                            span { class: "method-name", "{t!(\"dash_pay_with_paypal\")}" }
                        }
                        if session().token.is_some() {
                            {
                                let balance_f: f64 = session().balance.parse().unwrap_or(0.0);
                                let price_f: f64 = selected_plan_details.as_ref().map(|p| p.monthly_price.parse().unwrap_or(0.0)).unwrap_or(0.0);
                                let has_enough = balance_f >= price_f;
                                rsx! {
                                    label {
                                        class: if payment_method() == "balance" { "pay-method-item active" } else { "pay-method-item" },
                                        style: if !has_enough { "opacity: 0.6; cursor: not-allowed;" } else { "" },
                                        input {
                                            r#type: "radio",
                                            name: "pay",
                                            checked: payment_method() == "balance",
                                            disabled: !has_enough,
                                            onchange: move |_| *payment_method.write() = "balance".to_string(),
                                        }
                                        div { class: "method-info-box",
                                            span { class: "method-name", "{t!(\"dash_pay_with_balance\")}" }
                                            span { class: "method-meta", "{t!(\"dash_current_balance\")}: ${session().balance}" }
                                            if !has_enough {
                                                span { class: "text-danger method-hint", "{t!(\"dash_insufficient_balance\")}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(err) = &*checkout_error.read() {
                        p { class: "notice", "{err}" }
                    }

                    if payment_success() {
                        div { class: "success-banner",
                            "{t!(\"dash_payment_success_redirect\")}"
                        }
                    } else {
                        button {
                            class: "btn-primary full",
                            disabled: *checkout_loading.read(),
                            onclick: on_checkout,
                            if *checkout_loading.read() {
                                "{t!(\"dash_preparing_checkout\")}"
                            } else if payment_method() == "balance" {
                                "{t!(\"dash_pay_now\")}"
                            } else {
                                "{t!(\"dash_proceed_to_checkout\")}"
                            }
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
