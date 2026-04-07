use crate::api;
use crate::models::{DashboardTab, SessionState};
use dioxus::prelude::*;
use dioxus_i18n::t;
use dioxus_motion::prelude::*;

use super::shell::{DashboardShell, LoginRequiredView};

#[component]
pub fn TicketsPage() -> Element {
    let mut session = use_context::<Signal<SessionState>>();
    let state = session();
    let mut show_create_form = use_signal(|| false);
    let mut selected_ticket_id = use_signal(|| None::<String>);
    let mut ticket_messages = use_signal(Vec::<crate::models::TicketMessageItem>::new);

    if state.token.is_none() {
        return rsx! {
            LoginRequiredView {}
        };
    }

    let api_base = state.api_base.clone();
    let token = state.token.clone().unwrap();

    let _active_sse = use_signal::<Option<()>>(|| None);

    let eff_api_base = api_base.clone();
    let eff_token = token.clone();

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
        let tid = selected_ticket_id();
        let token = eff_token.clone();
        let api_base = eff_api_base.clone();

        ticket_messages.set(vec![]);

        #[cfg(target_arch = "wasm32")]
        {
            // Close existing SSE connection if any (wasm only)
        }

        if let Some(tid) = tid {
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::closure::Closure;
                use wasm_bindgen::JsCast;
                use web_sys::{EventSource, MessageEvent};

                let url = format!("{}/api/tickets/{}/messages?token={}", api_base, tid, token);
                if let Ok(es) = EventSource::new(&url) {
                    let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                        if let Some(txt) = e.data().as_string() {
                            if let Ok(msg) =
                                serde_json::from_str::<crate::models::TicketMessageItem>(&txt)
                            {
                                ticket_messages.with_mut(|msgs| msgs.push(msg));
                            }
                        }
                    })
                        as Box<dyn FnMut(MessageEvent)>);

                    es.add_event_listener_with_callback(
                        "message",
                        onmessage.as_ref().unchecked_ref(),
                    )
                    .unwrap();
                    onmessage.forget();

                    let mut session_clone = session.clone();
                    let tid_clone = tid.clone();
                    let onstatus = Closure::wrap(Box::new(move |e: MessageEvent| {
                        if let Some(status_str) = e.data().as_string() {
                            let mut s = session_clone.write();
                            if let Some(ticket) = s.tickets.iter_mut().find(|t| t.id == tid_clone) {
                                ticket.status = status_str;
                            }
                        }
                    })
                        as Box<dyn FnMut(MessageEvent)>);

                    es.add_event_listener_with_callback(
                        "status",
                        onstatus.as_ref().unchecked_ref(),
                    )
                    .unwrap();
                    onstatus.forget();

                    // EventSource is wasm-only
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (&tid, &token, &api_base);
            }
        }
    });

    let on_create_ticket = {
        let api_base = api_base.clone();
        let token = token.clone();
        move |_| {
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsCast;
                use web_sys::HtmlInputElement;
                use web_sys::HtmlSelectElement;
                use web_sys::HtmlTextAreaElement;
                let window = web_sys::window().unwrap();
                let document = window.document().unwrap();
                let subject = document
                    .get_element_by_id("ticket_subject")
                    .unwrap()
                    .dyn_into::<HtmlInputElement>()
                    .unwrap()
                    .value();
                let category = document
                    .get_element_by_id("ticket_category")
                    .unwrap()
                    .dyn_into::<HtmlSelectElement>()
                    .unwrap()
                    .value();
                let priority = document
                    .get_element_by_id("ticket_priority")
                    .unwrap()
                    .dyn_into::<HtmlSelectElement>()
                    .unwrap()
                    .value();
                let message = document
                    .get_element_by_id("ticket_message")
                    .unwrap()
                    .dyn_into::<HtmlTextAreaElement>()
                    .unwrap()
                    .value();

                if !subject.is_empty() && !message.is_empty() {
                    let api_base = api_base.clone();
                    let token = token.clone();
                    let payload = crate::models::CreateTicketRequest {
                        subject,
                        category,
                        priority,
                        message,
                    };
                    spawn(async move {
                        session.write().loading = true;
                        match api::create_ticket(&api_base, &token, &payload).await {
                            Ok(_) => {
                                show_create_form.set(false);
                                if let Ok(bundle) =
                                    api::load_authenticated_bundle(&api_base, &token).await
                                {
                                    let mut s = session.write();
                                    s.tickets = bundle.tickets;
                                    s.loading = false;
                                } else {
                                    session.write().loading = false;
                                }
                            }
                            Err(e) => {
                                let mut s = session.write();
                                s.error = Some(e);
                                s.loading = false;
                            }
                        }
                    });
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (&api_base, &token);
            }
        }
    };

    let on_reply_ticket = {
        let api_base = api_base.clone();
        let token = token.clone();
        move |_| {
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsCast;
                use web_sys::HtmlTextAreaElement;
                let window = web_sys::window().unwrap();
                let document = window.document().unwrap();
                let message = document
                    .get_element_by_id("reply_message")
                    .unwrap()
                    .dyn_into::<HtmlTextAreaElement>()
                    .unwrap()
                    .value();
                let ticket_id = selected_ticket_id().unwrap();

                if !message.trim().is_empty() {
                    let api_base = api_base.clone();
                    let token = token.clone();
                    spawn(async move {
                        match api::reply_ticket(&api_base, &token, &ticket_id, &message).await {
                            Ok(_) => {
                                // SSE will automatically fetch the new message.
                                // Just clear the input.
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let window = web_sys::window().unwrap();
                                    let document = window.document().unwrap();
                                    if let Some(el) = document.get_element_by_id("reply_message") {
                                        if let Ok(ta) =
                                            el.dyn_into::<web_sys::HtmlTextAreaElement>()
                                        {
                                            ta.set_value("");
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                session.write().error = Some(e);
                            }
                        }
                    });
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (&api_base, &token);
            }
        }
    };

    let on_close_ticket = {
        let api_base = api_base.clone();
        let token = token.clone();
        move |_| {
            let api_base = api_base.clone();
            let token = token.clone();
            let ticket_id = selected_ticket_id().unwrap();
            spawn(async move {
                match api::close_ticket(&api_base, &token, &ticket_id).await {
                    Ok(_) => {
                        if let Ok(bundle) = api::load_authenticated_bundle(&api_base, &token).await
                        {
                            let mut s = session.write();
                            s.tickets = bundle.tickets;
                        }
                    }
                    Err(e) => {
                        let mut s = session.write();
                        s.error = Some(e);
                    }
                }
            });
        }
    };

    let selected_ticket =
        selected_ticket_id().and_then(|id| state.tickets.iter().find(|t| t.id == id));

    rsx! {
            DashboardShell { title: "{t!(\"dash_ticket_center\")}", active_tab: DashboardTab::Tickets,
                div {
                    class: "tickets-container",
                    style: "opacity: {opacity.get_value()}; transform: translateY({slide_y.get_value()}px);",
                    if let Some(ticket) = selected_ticket {
                        section { class: "panel",
                        div { class: "flex-row-between",
                            h3 { "{ticket.subject}" }
                            div { class: "flex-row", style: "gap: 10px;",
                                if ticket.status.to_lowercase() != "closed" {
                                    button {
                                        class: "btn-secondary",
                                        onclick: on_close_ticket,
                                        "{t!(\"dash_close_ticket\")}"
                                    }
                                }
                                button {
                                    class: "btn-secondary",
                                    onclick: move |_| *selected_ticket_id.write() = None,
                                    "{t!(\"dash_back_to_list\")}"
                                }
                            }
                        }
                        div { class: "meta-strip",
                            span { class: "meta-item", {t!("dash_category", cat: ticket.category.clone())} }
                            span { class: "meta-item", {t!("dash_priority", prio: ticket.priority.clone())} }
                            span { class: "meta-item", {t!("dash_status_label", status: ticket.status.clone())} }
                        }

                        div { class: "message-list", style: "margin: 20px 0; max-height: 400px; overflow-y: auto; padding: 10px; background: #f9f9f9; border-radius: 8px;",
                            for msg in ticket_messages() {
                                div {
                                    class: if msg.sender_user_id.as_deref() == Some(ticket.user_id.as_str()) { "message own" } else { "message other" },
                                    style: if msg.sender_user_id.as_deref() == Some(ticket.user_id.as_str()) { "text-align: right; margin-bottom: 15px;" } else { "text-align: left; margin-bottom: 15px;" },
                                    div {
                                        style: format!("display: inline-block; padding: 10px 15px; border-radius: 12px; max-width: 80%; {}",
                                            if msg.sender_user_id.as_deref() == Some(ticket.user_id.as_str()) { "background: #e3f2fd; border: 1px solid #bbdefb;" } else { "background: white; border: 1px solid #eee;" }
                                        ),
                                        p { style: "margin: 0;", "{msg.message}" }
                                        p { class: "muted x-small", style: "margin-top: 5px;",
                                            "{msg.created_at} "
                                            if msg.sender_user_id.as_deref() == Some(ticket.user_id.as_str()) { "{t!(\"dash_msg_me\")}" } else { "{t!(\"dash_msg_staff\")}" }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "reply-form",
                            textarea {
                                id: "reply_message",
                                rows: "3",
                                placeholder: "{t!(\"dash_type_reply\")}"
                            }
                            button {
                                class: "btn-primary",
                                onclick: on_reply_ticket,
                                "{t!(\"dash_send_reply\")}"
                            }
                        }
                    }
                } else {
                    section { class: "panel",
                        div { class: "flex-row-between",
                            h3 { "{t!(\"dash_recent_tickets\")}" }
                            button {
                                class: "btn-primary",
                                onclick: move |_| *show_create_form.write() = !show_create_form(),
                                if show_create_form() { "{t!(\"dash_cancel\")}" } else { "{t!(\"dash_create_ticket\")}" }
                            }
                        }

                        if show_create_form() {
                            div { class: "add-mapping-form", style: "margin-top: 20px; border-top: 1px solid #eee; padding-top: 20px;",
                                h4 { "{t!(\"dash_new_support_ticket\")}" }
                                div { class: "form-group",
                                    label { "{t!(\"dash_subject\")}" }
                                    input {
                                        r#type: "text",
                                        id: "ticket_subject",
                                        placeholder: "{t!(\"dash_subject_placeholder\")}"
                                    }
                                }
                                div { class: "form-row",
                                    div { class: "form-group",
                                        label { "{t!(\"dash_form_category\")}" }
                                        select { id: "ticket_category",
                                            option { value: "Technical", "Technical" }
                                            option { value: "Billing", "Billing" }
                                            option { value: "AfterSales", "After-Sales" }
                                            option { value: "Network", "Network" }
                                            option { value: "Abuse", "Abuse" }
                                            option { value: "Other", "Other" }
                                        }
                                    }
                                    div { class: "form-group",
                                        label { "{t!(\"dash_form_priority\")}" }
                                        select { id: "ticket_priority",
                                            option { value: "Low", "Low" }
                                            option { value: "Medium", "Medium" }
                                            option { value: "High", "High" }
                                            option { value: "Urgent", "Urgent" }
                                        }
                                    }
                                }
                                div { class: "form-group",
                                    label { "{t!(\"dash_message\")}" }
                                    textarea {
                                        id: "ticket_message",
                                        rows: "5",
                                        placeholder: "{t!(\"dash_message_placeholder\")}"
                                    }
                                }
                                button {
                                    class: "btn-primary",
                                    onclick: on_create_ticket,
                                    "{t!(\"dash_submit_ticket\")}"
                                }
                            }
                        }

                        table {
                            thead {
                                tr {
                                    th { "{t!(\"dash_ticket_id\")}" }
                                    th { "{t!(\"dash_ticket_title\")}" }
                                    th { "{t!(\"dash_form_category\")}" }
                                    th { "{t!(\"dash_form_priority\")}" }
                                    th { "{t!(\"dash_status\")}" }
                                    th { "{t!(\"dash_action\")}" }
                                }
                            }
                            tbody {
                                if state.tickets.is_empty() {
                                    tr {
                                        td { colspan: "6", "{t!(\"dash_no_tickets\")}" }
                                    }
                                } else {
                                    for item in &state.tickets {
                                        tr {
                                            td { "{item.id}" }
                                            td { "{item.subject}" }
                                            td { "{item.category}" }
                                            td { "{item.priority}" }
                                            td {
                                                span { class: if item.status.eq_ignore_ascii_case("open") { "pill pending" } else { "pill paid" },
                                                    "{item.status}"
                                                }
                                            }
                                            td {
                                                button {
                                                    class: "btn-secondary btn-sm",
                                                    onclick: {
                                                        let id = item.id.clone();
                                                        move |_| {
                                                            *selected_ticket_id.write() = Some(id.clone());
                                                        }
                                                    },
                                                    "{t!(\"dash_ticket_details\")}"
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
}
