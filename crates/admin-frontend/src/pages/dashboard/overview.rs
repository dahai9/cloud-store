use dioxus::prelude::*;
use crate::models::AdminSessionState;

#[component]
pub fn OverviewPage() -> Element {
    let session = use_context::<Signal<AdminSessionState>>();

    rsx! {
        section { class: "hero panel-soft",
            h2 { "管理面板" }
            p {
                "这里管理节点库存、产品上下架、Guest 配置和工单状态。界面壳子与客户中心保持一致，只是内容和权限不同。"
            }
            div { class: "chip-row",
                span { class: "chip", "admin only" }
                span { class: "chip", "guest isolated" }
                span { class: "chip", "same shell" }
            }
        }

        if let Some(profile) = &session().profile {
            section { class: "card",
                h2 { "当前管理员" }
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
