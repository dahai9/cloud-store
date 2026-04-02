
use crate::api;

use crate::models::Route;

use dioxus::prelude::*;


mod auth;

mod dashboard;

mod public;


pub use auth::LoginPage;

pub use dashboard::{
    BalancePage, InstanceDetailPage, ProfilePage, ServicesPage, TicketsPage,
};

pub use public::{OrderPage, StorefrontPage};


#[component]
pub fn App() -> Element {
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
        Router::<Route> {}
    }
}
