use crate::api;
use crate::models::Route;
use dioxus::prelude::*;
use dioxus_i18n::prelude::*;
use dioxus_i18n::t;
use unic_langid::langid;

mod auth;
mod dashboard;
mod public;

pub use auth::LoginPage;
pub use dashboard::{
    BalancePage, ConsolePage, InstanceDetailPage, ProfilePage, ServicesPage, TicketsPage,
};
pub use public::{OrderPage, StorefrontPage};

#[component]
pub fn App() -> Element {
    let mut i18n = use_init_i18n(|| {
        I18nConfig::new(langid!("en-US"))
            .with_locale(Locale::new_static(langid!("en-US"), include_str!("../../../frontend/i18n/en-US.ftl")))
            .with_locale(Locale::new_static(langid!("zh-CN"), include_str!("../../../frontend/i18n/zh-CN.ftl")))
    });

    let session = use_signal(api::load_initial_session);
    use_context_provider(|| session);

    use_effect(move || {
        let mut session = session;

        spawn(async move {
            let (api_base, token) = {
                let current = session();
                (current.api_base.clone(), current.token.clone())
            };

            let Some(token) = token else {
                return;
            };

            match api::load_authenticated_bundle(&api_base, &token).await {
                Ok(bundle) => {
                    let mut current = session.write();
                    current.profile = Some(bundle.profile);
                    current.invoices = bundle.invoices;
                    current.tickets = bundle.tickets;
                    current.instances = bundle.instances;
                    current.balance = bundle.balance;
                    current.balance_transactions = bundle.balance_transactions;
                    current.loading = false;
                    current.error = None;

                }
                Err(err) => {
                    let mut current = session.write();
                    current.error = Some(err);
                    current.loading = false;
                    current.token = None;
                    current.profile = None;
                    current.invoices.clear();
                    current.tickets.clear();
                    current.instances.clear();
                    api::clear_persisted_session();
                }
            }
        });
    });

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
