use crate::api;
use crate::models::{AdminSessionState, TicketReplyRequest, TicketStatusUpdateRequest};
use dioxus::prelude::*;

#[component]
pub fn TicketsPage() -> Element {
    let mut session = use_context::<Signal<AdminSessionState>>();
    let state = session();
    let mut selected_ticket_id = use_signal(String::new);
    let mut selected_ticket_status = use_signal(|| "in_progress".to_string());
    let mut ticket_reply = use_signal(String::new);
    let mut ticket_messages = use_signal(Vec::<crate::models::TicketMessageItem>::new);

    let api_base = state.api_base.clone();
    let token = state.token.clone().unwrap_or_default();

    #[cfg(target_arch = "wasm32")]
    let mut active_sse = use_signal(|| None::<web_sys::EventSource>);
    #[cfg(not(target_arch = "wasm32"))]
    let active_sse = use_signal(|| None::<()>);

    let eff_api_base = api_base.clone();
    let eff_token = token.clone();
    use_effect(move || {
        let tid = selected_ticket_id();
        let token = eff_token.clone();
        let api_base = eff_api_base.clone();

        ticket_messages.set(vec![]);

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(es) = active_sse.write().take() {
                es.close();
            }
        }

        if tid.is_empty() {
            #[cfg(not(target_arch = "wasm32"))]
            let _ = (&token, &api_base, &active_sse);
            return;
        }

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::closure::Closure;
            use wasm_bindgen::JsCast;
            use web_sys::{EventSource, MessageEvent};

            let url = format!("{}/api/admin/tickets/{}/messages?token={}", api_base, tid, token);
            if let Ok(es) = EventSource::new(&url) {
                let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                    if let Some(txt) = e.data().as_string() {
                        if let Ok(msg) = serde_json::from_str::<crate::models::TicketMessageItem>(&txt) {
                            ticket_messages.with_mut(|msgs| msgs.push(msg));
                        }
                    }
                }) as Box<dyn FnMut(MessageEvent)>);

                es.add_event_listener_with_callback("message", onmessage.as_ref().unchecked_ref()).unwrap();
                onmessage.forget();

                let mut session_clone = session.clone();
                let mut selected_status_clone = selected_ticket_status.clone();
                let tid_clone = tid.clone();
                let onstatus = Closure::wrap(Box::new(move |e: MessageEvent| {
                    if let Some(status_str) = e.data().as_string() {
                        selected_status_clone.set(status_str.clone());
                        let mut s = session_clone.write();
                        if let Some(ticket) = s.tickets.iter_mut().find(|t| t.id == tid_clone) {
                            ticket.status = status_str;
                        }
                    }
                }) as Box<dyn FnMut(MessageEvent)>);

                es.add_event_listener_with_callback("status", onstatus.as_ref().unchecked_ref()).unwrap();
                onstatus.forget();

                active_sse.set(Some(es));            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (&token, &api_base, &active_sse);
        }
    });

    let refresh_tickets = {
        let api_base = api_base.clone();
        let token = token.clone();
        move |_| {
            let api_base = api_base.clone();
            let token = token.clone();
            spawn(async move {
                session.write().loading = true;
                match api::get_tickets(&api_base, &token).await {
                    Ok(tickets) => session.write().tickets = tickets,
                    Err(e) => session.write().error = Some(format!("刷新失败: {e}")),
                }
                session.write().loading = false;
            });
        }
    };

    let update_ticket = {
        let api_base = api_base.clone();
        let token = token.clone();
        move |_| {
            let api_base = api_base.clone();
            let token = token.clone();
            let ticket_id = selected_ticket_id();
            let status = selected_ticket_status();
            let reply_msg = ticket_reply();

            spawn(async move {
                if ticket_id.is_empty() {
                    return;
                }
                session.write().loading = true;

                let status_payload = TicketStatusUpdateRequest { status };
                if let Err(e) =
                    api::update_ticket_status(&api_base, &token, &ticket_id, &status_payload).await
                {
                    session.write().error = Some(format!("更新状态失败: {e}"));
                    session.write().loading = false;
                    return;
                }

                if !reply_msg.trim().is_empty() {
                    let reply_payload = TicketReplyRequest {
                        message: reply_msg.trim().to_string(),
                    };
                    if let Err(e) =
                        api::reply_ticket(&api_base, &token, &ticket_id, &reply_payload).await
                    {
                        session.write().error = Some(format!("回复失败: {e}"));
                        session.write().loading = false;
                        return;
                    }
                    ticket_reply.set(String::new());
                }

                session.write().notice = Some("工单状态/回复已提交".to_string());
                if let Ok(tickets) = api::get_tickets(&api_base, &token).await {
                    session.write().tickets = tickets;
                }
                // SSE will automatically fetch the new message.
                session.write().loading = false;
            });
        }
    };

    let on_close_ticket = {
        let api_base = api_base.clone();
        let token = token.clone();
        move |_| {
            let api_base = api_base.clone();
            let token = token.clone();
            let ticket_id = selected_ticket_id();
            spawn(async move {
                if ticket_id.is_empty() {
                    return;
                }
                session.write().loading = true;
                match api::close_ticket(&api_base, &token, &ticket_id).await {
                    Ok(_) => {
                        session.write().notice = Some("工单已关闭".to_string());
                        if let Ok(tickets) = api::get_tickets(&api_base, &token).await {
                            session.write().tickets = tickets;
                        }
                        selected_ticket_id.set(String::new());
                    }
                    Err(e) => {
                        session.write().error = Some(format!("关闭失败: {e}"));
                    }
                }
                session.write().loading = false;
            });
        }
    };

    let selected_ticket = session().tickets.iter().find(|t| t.id == selected_ticket_id()).cloned();

    rsx! {
        section { class: "card", id: "tickets",
            if let Some(ticket) = selected_ticket {
                div {
                    div { class: "flex-row-between",
                        h2 { "工单详情: {ticket.subject}" }
                        div { class: "flex-row", style: "gap: 10px;",
                            if ticket.status.to_lowercase() != "closed" {
                                button { class: "btn-secondary", onclick: on_close_ticket, "关闭工单" }
                            }
                            button {
                                class: "btn-secondary",
                                onclick: move |_| selected_ticket_id.set(String::new()),
                                "返回列表"
                            }
                        }
                    }

                    div { class: "meta-strip", style: "margin-bottom: 20px; padding: 10px; background: #f0f4f8; border-radius: 4px; display: flex; align-items: center;",
                        span { style: "margin-right: 20px;", "ID: {ticket.id}" }
                        span { style: "margin-right: 20px;", "分类: {ticket.category}" }
                        span { style: "margin-right: 20px;", "优先级: {ticket.priority}" }
                        span { style: "margin-right: 20px;", "当前状态: {ticket.status}" }
                        span { style: "flex-grow: 1;" }
                        span { style: "margin-right: 10px;", "用户: {ticket.user_id}" }
                        a {
                            class: "btn-secondary btn-sm",
                            href: "/admin/dashboard/guests?search={ticket.user_id}",
                            "管理用户"
                        }
                    }

                    div { class: "message-list", style: "margin: 20px 0; max-height: 400px; overflow-y: auto; padding: 15px; background: #f9f9f9; border: 1px solid #eee; border-radius: 8px;",
                        for msg in ticket_messages() {
                            div {
                                style: if msg.sender_user_id.as_deref() == Some(ticket.user_id.as_str()) { "text-align: left; margin-bottom: 15px;" } else { "text-align: right; margin-bottom: 15px;" },
                                div {
                                    style: format!("display: inline-block; padding: 10px 15px; border-radius: 12px; max-width: 80%; {}",
                                        if msg.sender_user_id.as_deref() == Some(ticket.user_id.as_str()) { "background: white; border: 1px solid #eee;" } else { "background: #e3f2fd; border: 1px solid #bbdefb;" }
                                    ),
                                    p { style: "margin: 0;", "{msg.message}" }
                                    p { style: "margin: 5px 0 0; font-size: 0.75rem; color: #888;",
                                        "{msg.created_at} "
                                        if msg.sender_user_id.as_deref() == Some(ticket.user_id.as_str()) { "(用户)" } else { "(管理员)" }
                                    }
                                }
                            }
                        }
                    }

                    div { class: "admin-controls", style: "padding: 20px; border: 1px solid #eee; border-radius: 8px; background: #fff;",
                        h3 { "管理操作" }
                        div { class: "field",
                            label { "修改状态" }
                            select {
                                style: "width: 100%; padding: 10px; margin-bottom: 15px; border: 1px solid #ddd; border-radius: 4px;",
                                value: "{selected_ticket_status()}",
                                onchange: move |evt| selected_ticket_status.set(evt.value()),
                                option { value: "open", "open (打开)" }
                                option { value: "in_progress", "in_progress (处理中)" }
                                option { value: "resolved", "resolved (已解决)" }
                                option { value: "closed", "closed (已关闭)" }
                            }
                        }
                        div { class: "field",
                            label { "回复内容" }
                            textarea {
                                style: "width: 100%; min-height: 100px; padding: 10px; margin-bottom: 15px; border: 1px solid #ddd; border-radius: 4px;",
                                value: "{ticket_reply()}",
                                oninput: move |evt| ticket_reply.set(evt.value()),
                                placeholder: "输入回复内容..."
                            }
                        }
                        button { class: "btn-primary", onclick: update_ticket, "提交修改与回复" }
                    }
                }
            } else {
                div {
                    div { class: "flex-row-between",
                        h2 { "工单管理" }
                        button { class: "btn-secondary", onclick: refresh_tickets, "刷新工单" }
                    }

                    if session().tickets.is_empty() {
                        p { class: "status", "暂无工单数据。" }
                    } else {
                        table { style: "width: 100%; border-collapse: collapse; margin-top: 20px;",
                            thead {
                                tr {
                                    th { style: "text-align: left; padding: 12px; border-bottom: 2px solid #eee;", "ID" }
                                    th { style: "text-align: left; padding: 12px; border-bottom: 2px solid #eee;", "主题" }
                                    th { style: "text-align: left; padding: 12px; border-bottom: 2px solid #eee;", "分类" }
                                    th { style: "text-align: left; padding: 12px; border-bottom: 2px solid #eee;", "状态" }
                                    th { style: "text-align: left; padding: 12px; border-bottom: 2px solid #eee;", "操作" }
                                }
                            }
                            tbody {
                                for ticket in session().tickets.clone() {
                                    tr {
                                        td { style: "padding: 12px; border-bottom: 1px solid #eee;", "{ticket.id}" }
                                        td { style: "padding: 12px; border-bottom: 1px solid #eee;", "{ticket.subject}" }
                                        td { style: "padding: 12px; border-bottom: 1px solid #eee;", "{ticket.category}" }
                                        td { style: "padding: 12px; border-bottom: 1px solid #eee;",
                                            span {
                                                class: if ticket.status == "open" { "pill pending" } else { "pill paid" },
                                                "{ticket.status}"
                                            }
                                        }
                                        td { style: "padding: 12px; border-bottom: 1px solid #eee;",
                                            button {
                                                class: "btn-secondary btn-sm",
                                                onclick: {
                                                    let t = ticket.clone();
                                                    move |_| {
                                                        selected_ticket_id.set(t.id.clone());
                                                        selected_ticket_status.set(t.status.clone());
                                                    }
                                                },
                                                "查看详情"
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
    }
}
