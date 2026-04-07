use crate::api;

use crate::models::{AuthTransportRisk, Route, SessionState};

use dioxus::prelude::*;
use dioxus_i18n::t;

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
                        s.instances = bundle.instances;
                        s.balance = bundle.balance;
                        s.balance_transactions = bundle.balance_transactions;
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
                        s.instances = bundle.instances;
                        s.balance = bundle.balance;
                        s.balance_transactions = bundle.balance_transactions;
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
                        "{t!(\"auth_checkout_resume_title\")}"
                    } else {
                        "{t!(\"auth_login_required\")}"
                    }
                }
                div { class: "flex-row", style: "gap: 10px;",
                    button {
                        class: "btn-secondary btn-sm",
                        onclick: move |_| {
                            use unic_langid::langid;
                            let mut i18n = dioxus_i18n::prelude::i18n();
                            if i18n.language() == langid!("en-US") {
                                i18n.set_language(langid!("zh-CN"));
                            } else {
                                i18n.set_language(langid!("en-US"));
                            }
                        },
                        "{t!(\"switch_lang\")}"
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
                        "{t!(\"back_btn\")}"
                    }
                }
            }
            main { class: "public-main login-layout",
                section { class: "login-hero panel-soft",
                    if is_checkout_flow {
                        div { class: "eyebrow", "{t!(\"auth_checkout_resume_eyebrow\")}" }
                        h2 { "{t!(\"auth_checkout_resume_title\")}" }
                        p {
                            "{t!(\"auth_checkout_resume_desc\")}"
                        }
                        div { class: "chip-row",
                            span { class: "chip", "{t!(\"auth_chip_return_plan\")}" }
                            span { class: "chip", "{t!(\"auth_chip_session\")}" }
                            span { class: "chip", "{t!(\"auth_chip_paypal_linked\")}" }
                        }
                        div { class: "login-callout",
                            strong { "{t!(\"auth_tips_title\")}" }
                            p {
                                "{t!(\"auth_checkout_tips_desc\")}"
                            }
                        }
                    } else {
                        div { class: "eyebrow", "{t!(\"auth_secure_access_eyebrow\")}" }
                        h2 { "{t!(\"auth_secure_access_title\")}" }
                        p {
                            "{t!(\"auth_secure_access_desc\")}"
                        }
                        div { class: "chip-row",
                            span { class: "chip", "{t!(\"auth_chip_dashboard\")}" }
                            span { class: "chip", "{t!(\"auth_chip_invoices\")}" }
                            span { class: "chip", "{t!(\"auth_chip_register\")}" }
                        }
                        div { class: "login-callout",
                            strong { "{t!(\"auth_tips_title\")}" }
                            p {
                                "{t!(\"auth_general_tips_desc\")}"
                            }
                        }
                    }
                }

                section { class: "checkout-card login-card",
                    h3 {
                        if is_checkout_flow {
                            "{t!(\"auth_complete_checkout_title\")}"
                        } else {
                            "{t!(\"auth_welcome_back_title\")}"
                        }
                    }
                    p { class: "muted",
                        if is_checkout_flow {
                            "{t!(\"auth_complete_checkout_desc\")}"
                        } else {
                            "{t!(\"auth_welcome_back_desc\")}"
                        }
                    }

                    if let Some(notice) = api::auth_transport_notice(&session().api_base) {
                        p { class: "notice", "{notice}" }
                    }

                    div { class: "form-stack",
                        div { class: "order-meta",
                            label { "{t!(\"auth_email_label\")}" }
                            input {
                                r#type: "email",
                                class: "text-input",
                                value: "{email()}",
                                placeholder: "{t!(\"auth_email_placeholder\")}",
                                autocomplete: "email",
                                oninput: move |evt| email.set(evt.value()),
                            }
                        }

                        div { class: "order-meta",
                            label { "{t!(\"auth_password_label\")}" }
                            input {
                                r#type: "password",
                                class: "text-input",
                                value: "{password()}",
                                placeholder: "{t!(\"auth_password_placeholder\")}",
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
                                "{t!(\"auth_validating_checkout\")}"
                            } else {
                                "{t!(\"auth_validating_sync\")}"
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
                                    "{t!(\"auth_restoring_order\")}"
                                } else {
                                    "{t!(\"auth_loading\")}"
                                }
                            } else if is_checkout_flow {
                                "{t!(\"auth_continue_checkout\")}"
                            } else {
                                "{t!(\"login_btn\")}"
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
                                "{t!(\"auth_register_instead\")}"
                            } else {
                                "{t!(\"auth_register\")}"
                            }
                        }
                    }
                }
            }
        }
    }
}
