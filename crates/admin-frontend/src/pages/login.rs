use crate::api;
use crate::models::{AdminSessionState, AuthPayload, Route};
use dioxus::prelude::*;

#[component]
pub fn LoginPage() -> Element {
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
                                session.write().error =
                                    Some("当前账号不是管理员，无法进入管理端".to_string());
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
                            let guests = api::get_guests(&api_base_val, &auth.token)
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
                            s.notice = Some("已登录管理员账号".to_string());
                            s.loading = false;

                            navigator().push(Route::OverviewPage {});
                        }
                        Err(e) => {
                            session.write().error = Some(format!("获取个人信息失败: {e}"));
                            session.write().loading = false;
                        }
                    }
                }
                Err(e) => {
                    session.write().error = Some(format!("登录失败: {e}"));
                    session.write().loading = false;
                }
            }
        });
    };

    rsx! {
        div { class: "content",
            section { class: "card", style: "max-width: 500px; margin: 50px auto;",
                h2 { "管理员登录" }

                div { class: "field",
                    label { "Admin API Base" }
                    input {
                        value: "{api_base()}",
                        oninput: move |evt| api_base.set(evt.value()),
                        placeholder: "http://127.0.0.1:8082",
                    }
                }

                div { class: "field",
                    label { "Email" }
                    input {
                        value: "{email()}",
                        oninput: move |evt| email.set(evt.value()),
                        placeholder: "admin@cloud-store.local",
                    }
                }

                div { class: "field",
                    label { "Password" }
                    input {
                        r#type: "password",
                        value: "{password()}",
                        oninput: move |evt| password.set(evt.value()),
                        placeholder: "********",
                    }
                }

                div { class: "actions",
                    button { class: "btn-primary", onclick: do_login, "登录并验证管理员权限" }
                }

                if session().loading {
                    p { class: "status", "处理中..." }
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
