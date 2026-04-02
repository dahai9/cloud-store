use dioxus::prelude::*;
use crate::models::{AdminSessionState, TicketStatusUpdateRequest, TicketReplyRequest};
use crate::api;

#[component]
pub fn TicketsPage() -> Element {
    let mut session = use_context::<Signal<AdminSessionState>>();
    let mut selected_ticket_id = use_signal(String::new);
    let mut selected_ticket_status = use_signal(|| "in_progress".to_string());
    let mut ticket_reply = use_signal(String::new);

    let refresh_tickets = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        spawn(async move {
            session.write().loading = true;
            match api::get_tickets(&api_base, &token).await {
                Ok(tickets) => session.write().tickets = tickets,
                Err(e) => session.write().error = Some(format!("刷新失败: {e}")),
            }
            session.write().loading = false;
        });
    };

    let update_ticket = move |_| {
        let api_base = session().api_base.clone();
        let token = session().token.clone().unwrap_or_default();
        let ticket_id = selected_ticket_id();
        let status = selected_ticket_status();
        let reply_msg = ticket_reply();

        spawn(async move {
            if ticket_id.is_empty() { return; }
            session.write().loading = true;
            
            let status_payload = TicketStatusUpdateRequest { status };
            if let Err(e) = api::update_ticket_status(&api_base, &token, &ticket_id, &status_payload).await {
                session.write().error = Some(format!("更新状态失败: {e}"));
                session.write().loading = false;
                return;
            }

            if !reply_msg.trim().is_empty() {
                let reply_payload = TicketReplyRequest { message: reply_msg.trim().to_string() };
                if let Err(e) = api::reply_ticket(&api_base, &token, &ticket_id, &reply_payload).await {
                    session.write().error = Some(format!("回复失败: {e}"));
                    session.write().loading = false;
                    return;
                }
            }

            session.write().notice = Some("工单状态/回复已提交".to_string());
            if let Ok(tickets) = api::get_tickets(&api_base, &token).await {
                session.write().tickets = tickets;
            }
            session.write().loading = false;
        });
    };

    rsx! {
        section { class: "card", id: "tickets",
            h2 { "工单管理" }
            div { class: "actions",
                button { class: "btn-secondary", onclick: refresh_tickets, "刷新工单" }
                button { class: "btn-primary", onclick: update_ticket, "提交状态/回复" }
            }

            div { class: "field",
                label { "Ticket ID" }
                input {
                    value: "{selected_ticket_id()}",
                    oninput: move |evt| selected_ticket_id.set(evt.value()),
                    placeholder: "输入 Ticket ID",
                }
            }
            div { class: "field",
                label { "Status(open/in_progress/resolved/closed)" }
                input {
                    value: "{selected_ticket_status()}",
                    oninput: move |evt| selected_ticket_status.set(evt.value()),
                    placeholder: "in_progress",
                }
            }
            div { class: "field",
                label { "管理员回复（可空）" }
                input {
                    value: "{ticket_reply()}",
                    oninput: move |evt| ticket_reply.set(evt.value()),
                    placeholder: "填写后会一并回复",
                }
            }

            if session().loading { p { class: "status", "处理中..." } }

            if session().tickets.is_empty() {
                p { class: "status", "暂无工单数据。" }
            } else {
                ul { class: "list",
                    for ticket in session().tickets.clone() {
                        li { class: "item",
                            strong { "{ticket.subject}" }
                            span { class: "meta", "Ticket ID: {ticket.id}" }
                            span { class: "meta",
                                "Category: {ticket.category} | Priority: {ticket.priority}"
                            }
                            span { class: "meta", "Status: {ticket.status}" }
                            div { class: "actions",
                                button {
                                    class: "btn-secondary",
                                    onclick: {
                                        let tid = ticket.id.clone();
                                        let status = ticket.status.clone();
                                        move |_| {
                                            selected_ticket_id.set(tid.clone());
                                            selected_ticket_status.set(status.clone());
                                        }
                                    },
                                    "设为当前编辑"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
