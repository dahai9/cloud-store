mod payment;
mod routes;
mod tickets;
mod billing;

use axum::{routing::get, Json, Router};
use dioxus::prelude::*;
use serde::Serialize;
use tracing::info;

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/portal")]
    Portal {},
    #[route("/admin")]
    Admin {},
}

#[component]
fn Home() -> Element {
    rsx! {
        div { class: "container",
            h1 { "Cloud Store" }
            p { "NAT VPS sales platform (Dioxus fullstack skeleton)." }
            ul {
                li {
                    Link { to: Route::Portal {}, "User Portal" }
                }
                li {
                    Link { to: Route::Admin {}, "Admin Console" }
                }
            }
        }
    }
}

#[component]
fn Portal() -> Element {
    rsx! {
        div { class: "container",
            h2 { "User Portal" }
            p { "Orders, invoices, subscriptions and support tickets." }
        }
    }
}

#[component]
fn Admin() -> Element {
    rsx! {
        div { class: "container",
            h2 { "Admin Console" }
            p { "Node inventory, NAT pools, service lifecycle and ticket operations." }
        }
    }
}

fn app() -> Element {
    rsx! {
        document::Stylesheet { href: "/assets/app.css" }
        Router::<Route> {}
    }
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "web-app",
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .compact()
        .init();

    let _ = dotenvy::dotenv();

    info!("starting web-app server");

    let router = Router::new()
        .route("/api/health", get(health))
        .route("/api/payment/paypal/create", get(payment::paypal::create_order))
        .route("/api/payment/paypal/webhook", get(payment::paypal::webhook_stub))
        .route("/api/tickets", get(tickets::list_tickets))
        .route("/api/invoices", get(billing::list_invoices));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, router).await?;

    let _ = app;
    let _ = routes::portal_links;

    Ok(())
}
