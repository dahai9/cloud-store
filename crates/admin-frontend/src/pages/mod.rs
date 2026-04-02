use dioxus::prelude::*;
use crate::models::{Route, AdminSessionState};
use crate::api;

mod login;
mod dashboard;

pub use login::LoginPage;
pub use dashboard::*;

#[component]
pub fn App() -> Element {
    let session = use_signal(|| AdminSessionState::new(api::default_api_base()));
    use_context_provider(|| session);

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        Router::<Route> {}
    }
}
