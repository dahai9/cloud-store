mod admin;
mod auth;
mod billing;
mod instances;
mod payment;
mod routes;
mod tickets;

use anyhow::Context;
use axum::extract::State;
use axum::{routing::delete, routing::get, routing::patch, routing::post, Json, Router};
use serde::Serialize;
use sqlx::SqlitePool;
use std::io::ErrorKind;
use std::path::PathBuf;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: SqlitePool,
    pub(crate) session_secret: String,
    pub(crate) paypal_client_id: String,
    pub(crate) paypal_client_secret: String,
    pub(crate) paypal_webhook_id: String,
    pub(crate) paypal_base_url: String,
    pub(crate) paypal_return_base_url: String,
    pub(crate) frontend_base_url: String,
    pub(crate) http_client: reqwest::Client,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "api-server",
    })
}

#[derive(Serialize)]
struct DbHealthResponse {
    status: &'static str,
    database: &'static str,
}

async fn db_health(
    State(state): State<AppState>,
) -> Result<Json<DbHealthResponse>, (axum::http::StatusCode, &'static str)> {
    sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .map_err(|_| {
            (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "database unavailable",
            )
        })?;

    Ok(Json(DbHealthResponse {
        status: "ok",
        database: "connected",
    }))
}

fn require_env(name: &str) -> anyhow::Result<String> {
    std::env::var(name).with_context(|| format!("missing required env var: {name}"))
}

fn normalize_database_url(raw_url: &str) -> anyhow::Result<String> {
    let Some(without_prefix) = raw_url.strip_prefix("sqlite://") else {
        return Ok(raw_url.to_owned());
    };

    if without_prefix.starts_with('/') {
        return Ok(raw_url.to_owned());
    }

    let (path_part, query_part) = without_prefix
        .split_once('?')
        .map_or((without_prefix, None), |(path, query)| (path, Some(query)));

    let base_dir = std::env::current_dir().context("failed to read current working directory")?;
    let absolute_path: PathBuf = base_dir.join(path_part);

    if let Some(parent) = absolute_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("failed to create database directory: {}", parent.display())
        })?;
    }

    let absolute = absolute_path.display().to_string();
    let normalized = match query_part {
        Some(query) => format!("sqlite://{absolute}?{query}"),
        None => format!("sqlite://{absolute}"),
    };

    Ok(normalized)
}

fn read_bind_host() -> String {
    std::env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string())
}

fn read_admin_bind_host(default_host: &str) -> String {
    std::env::var("ADMIN_APP_HOST").unwrap_or_else(|_| default_host.to_string())
}

fn read_bind_port() -> u16 {
    std::env::var("APP_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8081)
}

fn read_admin_bind_port() -> u16 {
    std::env::var("ADMIN_APP_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8082)
}

fn read_env_or_default(name: &str, default_value: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default_value.to_string())
}

fn read_required_env_trimmed(name: &str) -> anyhow::Result<String> {
    Ok(require_env(name)?.trim().to_string())
}

fn read_optional_env_trimmed(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

async fn bind_listener(host: &str, preferred_port: u16) -> anyhow::Result<tokio::net::TcpListener> {
    let preferred = format!("{host}:{preferred_port}");

    match tokio::net::TcpListener::bind(&preferred).await {
        Ok(listener) => {
            info!(bind_addr = %preferred, "bound web-app listener");
            Ok(listener)
        }
        Err(err) if err.kind() == ErrorKind::AddrInUse => {
            let fallback_port = preferred_port.saturating_add(1);
            let fallback = format!("{host}:{fallback_port}");
            warn!(bind_addr = %preferred, fallback_addr = %fallback, "preferred port is in use, trying fallback");
            let listener = tokio::net::TcpListener::bind(&fallback)
                .await
                .with_context(|| format!("failed to bind both {preferred} and {fallback}"))?;
            info!(bind_addr = %fallback, "bound web-app listener on fallback port");
            Ok(listener)
        }
        Err(err) => Err(err).with_context(|| format!("failed to bind listener at {preferred}")),
    }
}

fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ])
}

fn build_guest_router(app_state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/db-health", get(db_health))
        .route("/api/plans", get(billing::list_public_plans))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/me", get(auth::me))
        .route(
            "/api/payment/paypal/create",
            post(payment::paypal::create_order),
        )
        .route(
            "/api/payment/paypal/retry/{invoice_id}",
            post(payment::paypal::retry_invoice_payment),
        )
        .route(
            "/api/payment/paypal/return",
            get(payment::paypal::paypal_return),
        )
        .route(
            "/api/payment/paypal/cancel",
            get(payment::paypal::paypal_cancel),
        )
        .route(
            "/api/payment/paypal/webhook",
            post(payment::paypal::webhook),
        )
        .route("/api/tickets", get(tickets::list_tickets))
        .route("/api/tickets", post(tickets::create_ticket))
        .route(
            "/api/tickets/{ticket_id}/messages",
            get(tickets::list_ticket_messages),
        )
        .route(
            "/api/tickets/{ticket_id}/reply",
            post(tickets::reply_ticket),
        )
        .route(
            "/api/tickets/{ticket_id}/close",
            post(tickets::close_ticket),
        )
        .route("/api/user/balance", get(billing::get_balance))
        .route(
            "/api/user/balance/transactions",
            get(billing::list_balance_transactions),
        )
        .route("/api/user/balance/recharge", post(billing::recharge_balance))
        .route("/api/invoices", get(billing::list_invoices))
        .route("/api/instances", get(instances::list_instances))
        .route("/api/instances/{id}", get(instances::get_instance))
        .route(
            "/api/instances/{id}/auto-renew",
            patch(instances::update_auto_renew),
        )
        .route(
            "/api/instances/{id}/nat-mappings",
            get(instances::list_nat_mappings),
        )
        .route(
            "/api/instances/{id}/nat-mappings",
            post(instances::add_nat_mapping),
        )
        .route(
            "/api/instances/{id}/nat-mappings/{mapping_id}",
            delete(instances::remove_nat_mapping),
        )
        .route(
            "/api/instances/{id}/action",
            post(instances::perform_action),
        )
        .route("/api/instances/{id}/metrics", get(instances::get_metrics))
        .route("/api/instances/{id}/console", get(instances::get_console))
        .route("/api/instances/{id}/console/ws", get(instances::console_ws))
        .layer(cors_layer())
        .with_state(app_state)
}

fn build_admin_router(app_state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/db-health", get(db_health))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/me", get(auth::me))
        .route("/api/admin/nodes", get(admin::list_nodes))
        .route("/api/admin/nodes", post(admin::add_node))
        .route("/api/admin/nodes/{id}", patch(admin::update_node))
        .route(
            "/api/admin/nat-port-leases",
            get(admin::list_nat_port_leases),
        )
        .route(
            "/api/admin/nat-port-leases",
            post(admin::add_nat_port_lease),
        )
        .route(
            "/api/admin/nat-port-leases/{lease_id}",
            delete(admin::delete_nat_port_lease),
        )
        .route("/api/admin/instances", get(admin::list_instances))
        .route("/api/admin/instances", post(admin::admin_add_instance))
        .route(
            "/api/admin/instances/{instance_id}",
            delete(admin::admin_delete_instance),
        )
        .route("/api/admin/plans", get(admin::list_plans))
        .route("/api/admin/plans", post(admin::add_plan))
        .route("/api/admin/plans/{plan_id}", patch(admin::update_plan))
        .route("/api/admin/guests", get(admin::list_guests))
        .route("/api/admin/guests/{user_id}", patch(admin::update_guest))
        .route("/api/admin/tickets", get(tickets::list_tickets))
        .route("/api/admin/tickets", post(admin::admin_create_ticket))
        .route(
            "/api/admin/tickets/{ticket_id}/status",
            patch(tickets::admin_update_ticket_status),
        )
        .route(
            "/api/admin/tickets/{ticket_id}/messages",
            get(tickets::admin_list_ticket_messages),
        )
        .route(
            "/api/admin/tickets/{ticket_id}/reply",
            post(tickets::admin_reply_ticket),
        )
        .route(
            "/api/admin/tickets/{ticket_id}/close",
            post(tickets::admin_close_ticket),
        )
        .route("/api/admin/invoices", get(billing::list_invoices))
        .layer(cors_layer())
        .with_state(app_state)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .compact()
        .init();

    let _ = dotenvy::dotenv();

    let database_url = normalize_database_url(&require_env("DATABASE_URL")?)?;
    let session_secret = require_env("SESSION_SECRET")?;
    let paypal_client_id = read_required_env_trimmed("PAYPAL_CLIENT_ID")?;
    let paypal_client_secret = read_required_env_trimmed("PAYPAL_CLIENT_SECRET")?;
    let paypal_webhook_id = read_required_env_trimmed("PAYPAL_WEBHOOK_ID")?;
    let paypal_base_url =
        read_env_or_default("PAYPAL_BASE_URL", "https://api-m.sandbox.paypal.com");
    let paypal_return_base_url =
        read_env_or_default("PAYPAL_RETURN_BASE_URL", "http://127.0.0.1:8081");
    let frontend_base_url = read_env_or_default("FRONTEND_BASE_URL", "http://127.0.0.1:8080");
    let guest_bind_host = read_bind_host();
    let guest_bind_port = read_bind_port();
    let admin_bind_host = read_admin_bind_host(&guest_bind_host);
    let admin_bind_port = read_admin_bind_port();
    let bootstrap_admin_email = read_optional_env_trimmed("ADMIN_BOOTSTRAP_EMAIL");
    let bootstrap_admin_password = read_optional_env_trimmed("ADMIN_BOOTSTRAP_PASSWORD");

    info!(database_url = %database_url, "starting web-app server");

    let db = SqlitePool::connect(&database_url)
        .await
        .context("failed to connect sqlite database")?;

    sqlx::migrate!("../../migrations")
        .run(&db)
        .await
        .context("failed to run database migrations")?;

    let app_state = AppState {
        db,
        session_secret,
        paypal_client_id,
        paypal_client_secret,
        paypal_webhook_id,
        paypal_base_url,
        paypal_return_base_url,
        frontend_base_url,
        http_client: reqwest::Client::new(),
    };

    auth::ensure_single_admin(&app_state, bootstrap_admin_email, bootstrap_admin_password)
        .await
        .context("failed while validating/bootstrapping single admin account")?;

    let guest_router = build_guest_router(app_state.clone());
    let admin_router = build_admin_router(app_state);

    let guest_listener = bind_listener(&guest_bind_host, guest_bind_port).await?;
    let admin_listener = bind_listener(&admin_bind_host, admin_bind_port).await?;

    info!(
        guest_host = %guest_bind_host,
        guest_port = guest_bind_port,
        admin_host = %admin_bind_host,
        admin_port = admin_bind_port,
        "starting guest and admin api listeners"
    );

    let guest_server = axum::serve(guest_listener, guest_router);
    let admin_server = axum::serve(admin_listener, admin_router);

    tokio::try_join!(guest_server, admin_server)?;

    let _ = routes::portal_links;

    Ok(())
}
