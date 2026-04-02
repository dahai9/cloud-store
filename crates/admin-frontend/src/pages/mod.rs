use crate::api;
use crate::models::{AdminSessionState, Route};
use dioxus::prelude::*;

mod dashboard;
mod login;

pub use dashboard::*;
pub use login::LoginPage;

#[component]
pub fn App() -> Element {
    let session = use_signal(|| AdminSessionState::new(api::default_api_base()));
    use_context_provider(|| session);

    rsx! {
        document::Stylesheet { href: asset!("/assets/main.css") }
        Router::<Route> {}
    }
}
