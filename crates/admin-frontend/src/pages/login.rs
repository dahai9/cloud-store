use crate::api;
use crate::models::{AdminSessionState, AuthPayload, Route};
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn LoginPage() -> Element {
    let mut i18n = dioxus_i18n::prelude::i18n();
    let mut session = use_context::<Signal<AdminSessionState>>();
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut api_base = use_signal(|| session().api_base.clone());

    let do_login = move |_| {
        let api_base_val = api_base();
        let email_val = email();
        let password_val = password();

        spawn(async move {
            session.write().loading = true;
            session.write().error = None;
            session.write().api_base = api_base_val.clone();

            let payload = AuthPayload {
                email: email_val.trim().to_string(),
                password: password_val,
            };

            match api::login(&api_base_val, &payload).await {
                Ok(auth) => {
                    match api::get_profile(&api_base_val, &auth.token).await {
                        Ok(profile) => {
                            if profile.role != "admin" {
                                session.write().error = Some(t!("login_err_not_admin"));
                                session.write().loading = false;
                                return;
                            }

                            // Load initial data
                            let nodes = api::get_nodes(&api_base_val, &auth.token)
                                .await
                                .unwrap_or_default();
                            let nat_port_leases =
                                api::get_nat_port_leases(&api_base_val, &auth.token)
                                    .await
                                    .unwrap_or_default();
                            let plans = api::get_plans(&api_base_val, &auth.token)
                                .await
                                .unwrap_or_default();
                            let guests = api::get_guests(&api_base_val, &auth.token, None)
                                .await
                                .unwrap_or_default();
                            let tickets = api::get_tickets(&api_base_val, &auth.token)
                                .await
                                .unwrap_or_default();

                            let mut s = session.write();
                            s.token = Some(auth.token);
                            s.profile = Some(profile);
                            s.nodes = nodes;
                            s.nat_port_leases = nat_port_leases;
                            s.plans = plans;
                            s.guests = guests;
                            s.tickets = tickets;
                            s.notice = Some(t!("login_success_notice"));
                            s.loading = false;

                            navigator().push(Route::OverviewPage {});
                        }
                        Err(e) => {
                            session.write().error = Some(t!("login_err_prefix_profile", err: e));
                            session.write().loading = false;
                        }
                    }
                }
                Err(e) => {
                    session.write().error = Some(t!("login_err_prefix_login", err: e));
                    session.write().loading = false;
                }
            }
        });
    };

    rsx! {
        div { class: "content",
            div {
                style: "display: flex; justify-content: flex-end; margin-bottom: 20px;",
                button {
                    class: "btn-secondary",
                    onclick: move |_| {
                        use unic_langid::langid;
                        if i18n.language() == langid!("en-US") {
                            i18n.set_language(langid!("zh-CN"));
                        } else {
                            i18n.set_language(langid!("en-US"));
                        }
                    },
                    "{t!(\"switch_lang\")}"
                }
            }
            section { class: "card", style: "max-width: 500px; margin: 0 auto;",
                h2 { "{t!(\"login_admin_title\")}" }

                div { class: "field",
                    label { "{t!(\"login_api_base_label\")}" }
                    input {
                        value: "{api_base()}",
                        oninput: move |evt| api_base.set(evt.value()),
                        placeholder: "http://127.0.0.1:8082",
                    }
                }

                div { class: "field",
                    label { "{t!(\"login_email_label\")}" }
                    input {
                        value: "{email()}",
                        oninput: move |evt| email.set(evt.value()),
                        placeholder: "admin@cloud-store.local",
                    }
                }

                div { class: "field",
                    label { "{t!(\"login_password_label\")}" }
                    input {
                        r#type: "password",
                        value: "{password()}",
                        oninput: move |evt| password.set(evt.value()),
                        placeholder: "********",
                    }
                }

                div { class: "actions",
                    button { class: "btn-primary", onclick: do_login, "{t!(\"login_submit_btn\")}" }
                }

                if session().loading {
                    p { class: "status", "{t!(\"processing\")}" }
                }

                if let Some(message) = &session().notice {
                    p { class: "status", "{message}" }
                }

                if let Some(message) = &session().error {
                    p { class: "error", "{message}" }
                }
            }
        }
    }
}

