use crate::models::AdminSessionState;
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn OverviewPage() -> Element {
    let session = use_context::<Signal<AdminSessionState>>();

    rsx! {
        section { class: "hero panel-soft",
            h2 { "{t!(\"overview_title\")}" }
            p {
                "{t!(\"overview_desc\")}"
            }
            div { class: "chip-row",
                span { class: "chip", "admin only" }
                span { class: "chip", "guest isolated" }
                span { class: "chip", "same shell" }
            }
        }

        if let Some(profile) = &session().profile {
            section { class: "card",
                h2 { "{t!(\"overview_current_admin\")}" }
                p { class: "status ok",
                    "Email: {profile.email} (ID: {profile.user_id})"
                }
            }
        }

        if let Some(notice) = &session().notice {
            p { class: "status", "{notice}" }
        }

        if let Some(error) = &session().error {
            p { class: "error", "{error}" }
        }
    }
}

