use crate::api;
use crate::models::{AdminSessionState, Route};
use dioxus::prelude::*;
use dioxus_i18n::prelude::*;
use dioxus_i18n::t;
use unic_langid::langid;

mod dashboard;
mod login;

pub use dashboard::*;
pub use login::LoginPage;

#[component]
pub fn App() -> Element {
    let mut i18n = use_init_i18n(|| {
        I18nConfig::new(langid!("en-US"))
            .with_locale(Locale::new_static(langid!("en-US"), include_str!("../../../admin-frontend/i18n/en-US.ftl")))
            .with_locale(Locale::new_static(langid!("zh-CN"), include_str!("../../../admin-frontend/i18n/zh-CN.ftl")))
    });

    let session = use_signal(|| AdminSessionState::new(api::default_api_base()));
    use_context_provider(|| session);

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        div {
            style: "position: fixed; top: 10px; right: 10px; z-index: 1000;",
            button {
                class: "btn-secondary btn-sm",
                onclick: move |_| {
                    if i18n.language() == langid!("en-US") {
                        i18n.set_language(langid!("zh-CN"));
                    } else {
                        i18n.set_language(langid!("en-US"));
                    }
                },
                "{t!(\"switch_lang\")}"
            }
        }
        Router::<Route> {}
    }
}
