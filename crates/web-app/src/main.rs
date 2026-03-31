mod auth;
mod billing;
mod payment;
mod routes;
mod tickets;

use anyhow::Context;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::{routing::get, routing::post, Json, Router};
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

#[derive(Serialize)]
struct NodeItem {
    id: String,
    name: String,
    region: String,
    total_capacity: i64,
    used_capacity: i64,
}

async fn list_nodes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<NodeItem>>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state)?;

    let rows = sqlx::query_as::<_, (String, String, String, i64, i64)>(
        "SELECT id, name, region, total_capacity, used_capacity FROM nodes ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| {
        tracing::error!(error = %err, "failed to list nodes");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load nodes")
    })?;

    let items = rows
        .into_iter()
        .map(
            |(id, name, region, total_capacity, used_capacity)| NodeItem {
                id,
                name,
                region,
                total_capacity,
                used_capacity,
            },
        )
        .collect();

    Ok(Json(items))
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

fn read_bind_port() -> u16 {
    std::env::var("APP_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8081)
}

fn read_env_or_default(name: &str, default_value: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default_value.to_string())
}

fn read_required_env_trimmed(name: &str) -> anyhow::Result<String> {
    Ok(require_env(name)?.trim().to_string())
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
    let bind_host = read_bind_host();
    let bind_port = read_bind_port();

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

    let router = Router::new()
        .route("/api/health", get(health))
        .route("/api/db-health", get(db_health))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/me", get(auth::me))
        .route(
            "/api/payment/paypal/create",
            post(payment::paypal::create_order),
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
        .route("/api/invoices", get(billing::list_invoices))
        .route("/api/admin/nodes", get(list_nodes))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(app_state);

    let listener = bind_listener(&bind_host, bind_port).await?;
    axum::serve(listener, router).await?;

    let _ = routes::portal_links;

    Ok(())
}
