use crate::api;
use crate::models::{AdminSessionState, Route};
use dioxus::prelude::*;
use dioxus_i18n::prelude::*;
use unic_langid::langid;

mod dashboard;
mod login;

pub use dashboard::*;
pub use login::LoginPage;

#[component]
pub fn App() -> Element {
    let _i18n = use_init_i18n(|| {
        I18nConfig::new(langid!("en-US"))
            .with_locale(Locale::new_static(langid!("en-US"), include_str!("../../../admin-frontend/i18n/en-US.ftl")))
            .with_locale(Locale::new_static(langid!("zh-CN"), include_str!("../../../admin-frontend/i18n/zh-CN.ftl")))
    });

    let session = use_signal(|| AdminSessionState::new(api::default_api_base()));
    use_context_provider(|| session);

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        Router::<Route> {}
    }
}
