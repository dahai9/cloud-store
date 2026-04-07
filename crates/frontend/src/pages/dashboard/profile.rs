use crate::models::{DashboardTab, SessionState};
use dioxus::prelude::*;
use dioxus_i18n::t;
use dioxus_motion::prelude::*;

use super::shell::{DashboardShell, LoginRequiredView};

#[component]
pub fn ProfilePage() -> Element {
    let session = use_context::<Signal<SessionState>>();
    let state = session();

    if state.token.is_none() {
        return rsx! {
            LoginRequiredView {}
        };
    }

    let profile = state.profile.clone();
    let user_id = profile
        .as_ref()
        .map(|p| p.user_id.clone())
        .unwrap_or_else(|| "-".to_string());
    let user_email = profile
        .as_ref()
        .map(|p| p.email.clone())
        .unwrap_or_else(|| "-".to_string());
    let user_role = profile
        .as_ref()
        .map(|p| p.role.clone())
        .unwrap_or_else(|| "user".to_string());

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

    rsx! {
        DashboardShell { title: "{t!(\"dash_user_info_title\")}", active_tab: DashboardTab::Profile,
            section {
                class: "grid-two",
                style: "opacity: {opacity.get_value()}; transform: translateY({slide_y.get_value()}px);",
                article { class: "panel",
                    h3 { "{t!(\"dash_account_title\")}" }
                    p { class: "muted", "{t!(\"dash_user_id\")}" }
                    p { class: "fact", "{user_id}" }
                    p { class: "muted", "{t!(\"dash_email\")}" }
                    p { class: "fact", "{user_email}" }
                    p { class: "muted", "{t!(\"dash_role\")}" }
                    p { class: "fact", "{user_role}" }
                }
                article { class: "panel",
                    h3 { "{t!(\"dash_portal_status_title\")}" }
                    p { class: "muted", "{t!(\"dash_ticket_count\")}" }
                    p { class: "fact", "{state.tickets.len()}" }
                    p { class: "muted", "{t!(\"dash_invoice_count\")}" }
                    p { class: "fact", "{state.invoices.len()}" }
                    p { class: "muted", "{t!(\"dash_session\")}" }
                    p { class: "fact", "{t!(\"dash_authenticated\")}" }
                }
            }
        }
    }
}
