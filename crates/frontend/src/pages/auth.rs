use crate::api;

use crate::models::{AuthTransportRisk, Route, SessionState};

use dioxus::prelude::*;

#[component]
pub fn LoginPage(source: Option<String>, plan: Option<String>) -> Element {
    let navigator = use_navigator();
    let session = use_context::<Signal<SessionState>>();
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let is_checkout_flow = matches!(source.as_deref(), Some("order"));
    let return_plan = plan.clone().unwrap_or_else(|| "nat-standard".to_string());
    let login_return_plan = return_plan.clone();
    let register_return_plan = return_plan.clone();
    let back_return_plan = return_plan.clone();

    let on_login = move |_| {
        let email_value = email();
        let password_value = password();
        let nav = navigator;
        let mut state = session;
        let is_checkout_flow = is_checkout_flow;
        let return_plan = login_return_plan.clone();

        spawn(async move {
            {
                let mut current = state.write();
                current.loading = true;
                current.error = None;
            }

            let api_base = state().api_base.clone();
            match api::authenticate_and_load(&api_base, "login", &email_value, &password_value)
                .await
            {
                Ok(bundle) => {
                    {
                        let mut s = state.write();
                        s.token = Some(bundle.token);
                        s.profile = Some(bundle.profile);
                        s.invoices = bundle.invoices;
                        s.tickets = bundle.tickets;
                        s.loading = false;
                        s.error = None;
                    }
                    api::persist_authenticated_session(&state());
                    if is_checkout_flow {
                        nav.push(Route::OrderPage { plan: return_plan });
                    } else {
                        nav.push(Route::ProfilePage {});
                    }
                }
                Err(err) => {
                    let mut s = state.write();
                    s.loading = false;
                    s.error = Some(err);
                }
            }
        });
    };

    let on_register = move |_| {
        let email_value = email();
        let password_value = password();
        let nav = navigator;
        let mut state = session;
        let is_checkout_flow = is_checkout_flow;
        let return_plan = register_return_plan.clone();

        spawn(async move {
            {
                let mut current = state.write();
                current.loading = true;
                current.error = None;
            }

            let api_base = state().api_base.clone();
            match api::authenticate_and_load(&api_base, "register", &email_value, &password_value)
                .await
            {
                Ok(bundle) => {
                    {
                        let mut s = state.write();
                        s.token = Some(bundle.token);
                        s.profile = Some(bundle.profile);
                        s.invoices = bundle.invoices;
                        s.tickets = bundle.tickets;
                        s.loading = false;
                        s.error = None;
                    }
                    api::persist_authenticated_session(&state());
                    if is_checkout_flow {
                        nav.push(Route::OrderPage { plan: return_plan });
                    } else {
                        nav.push(Route::ProfilePage {});
                    }
                }
                Err(err) => {
                    let mut s = state.write();
                    s.loading = false;
                    s.error = Some(err);
                }
            }
        });
    };

    rsx! {
        div { class: "public-shell",
            header { class: "public-header",
                h1 {
                    if is_checkout_flow {
                        "Continue Checkout"
                    } else {
                        "Login Required"
                    }
                }
                button {
                    r#type: "button",
                    class: "btn-secondary",
                    onclick: move |_| {
                        if is_checkout_flow {
                            navigator
                                .push(Route::OrderPage {
                                    plan: back_return_plan.clone(),
                                });
                        } else {
                            navigator.push(Route::StorefrontPage {});
                        }
                    },
                    "Back"
                }
            }
            main { class: "public-main login-layout",
                section { class: "login-hero panel-soft",
                    if is_checkout_flow {
                        div { class: "eyebrow", "Checkout resume" }
                        h2 { "Sign in to continue checkout" }
                        p {
                            "Log in to continue the order you already selected. The chosen plan will be restored after authentication."
                        }
                        div { class: "chip-row",
                            span { class: "chip", "Return to selected plan" }
                            span { class: "chip", "Session state is persisted" }
                            span { class: "chip", "PayPal approval stays linked" }
                        }
                        div { class: "login-callout",
                            strong { "Tips" }
                            p {
                                "If you came from checkout, do not switch browser tabs before signing in so the plan choice stays intact."
                            }
                        }
                    } else {
                        div { class: "eyebrow", "Secure access" }
                        h2 { "Sign in to manage your account" }
                        p {
                            "Use your existing account to open the dashboard, review invoices, and manage services."
                        }
                        div { class: "chip-row",
                            span { class: "chip", "Dashboard access" }
                            span { class: "chip", "Invoices and services" }
                            span { class: "chip", "Register if you do not have an account" }
                        }
                        div { class: "login-callout",
                            strong { "Tips" }
                            p {
                                "You can log in first and then browse products or open the customer center from the home page."
                            }
                        }
                    }
                }

                section { class: "checkout-card login-card",
                    h3 {
                        if is_checkout_flow {
                            "Complete your checkout"
                        } else {
                            "Welcome back"
                        }
                    }
                    p { class: "muted",
                        if is_checkout_flow {
                            "Sign in to continue the selected plan and finish payment."
                        } else {
                            "You can login with an existing account, or register a new user directly."
                        }
                    }

                    if let Some(notice) = api::auth_transport_notice(&session().api_base) {
                        p { class: "notice", "{notice}" }
                    }

                    div { class: "form-stack",
                        div { class: "order-meta",
                            label { "Email" }
                            input {
                                r#type: "email",
                                class: "text-input",
                                value: "{email()}",
                                placeholder: "you@example.com",
                                autocomplete: "email",
                                oninput: move |evt| email.set(evt.value()),
                            }
                        }

                        div { class: "order-meta",
                            label { "Password" }
                            input {
                                r#type: "password",
                                class: "text-input",
                                value: "{password()}",
                                placeholder: "password",
                                autocomplete: "current-password",
                                oninput: move |evt| password.set(evt.value()),
                            }
                        }
                    }

                    if let Some(err) = &session().error {
                        p { class: "notice error-notice", "{err}" }
                    }

                    if session().loading {
                        p { class: "muted",
                            if is_checkout_flow {
                                "正在验证账号并恢复订单..."
                            } else {
                                "正在验证账号并同步用户数据..."
                            }
                        }
                    }

                    div { class: "login-actions",
                        button {
                            r#type: "button",
                            class: "btn-primary full",
                            disabled: session().loading
                                || matches!(
                                    api::auth_transport_risk(&session().api_base),
                                    AuthTransportRisk::InsecureRemote
                                ),
                            onclick: on_login,
                            if session().loading {
                                if is_checkout_flow {
                                    "Restoring order..."
                                } else {
                                    "Loading..."
                                }
                            } else if is_checkout_flow {
                                "Continue Checkout"
                            } else {
                                "Login"
                            }
                        }
                        button {
                            r#type: "button",
                            class: "btn-secondary full",
                            disabled: session().loading
                                || matches!(
                                    api::auth_transport_risk(&session().api_base),
                                    AuthTransportRisk::InsecureRemote
                                ),
                            onclick: on_register,
                            if is_checkout_flow {
                                "Register instead"
                            } else {
                                "Register"
                            }
                        }
                    }
                }
            }
        }
    }
}
