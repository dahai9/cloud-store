use anyhow::Context;
use provider_adapter::{ComputeProvider, StubProvider};
use sqlx::SqlitePool;
use std::path::PathBuf;
use tracing::info;
use tracing::warn;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .compact()
        .init();

    let _ = dotenvy::dotenv();
    let database_url = normalize_database_url(&require_env("DATABASE_URL")?)?;
    let db = SqlitePool::connect(&database_url)
        .await
        .context("failed to connect sqlite database")?;

    info!("worker booted");

    expire_overdue_invoices(&db).await?;

    let provider = StubProvider;
    run_provisioning_tick(&provider).await?;
    run_renewal_tick().await;

    Ok(())
}

async fn run_provisioning_tick(provider: &dyn ComputeProvider) -> anyhow::Result<()> {
    let _ = provider;
    info!("provisioning tick placeholder");
    Ok(())
}

async fn run_renewal_tick() {
    info!("renewal tick placeholder");
}

async fn expire_overdue_invoices(db: &SqlitePool) -> anyhow::Result<()> {
    let result = sqlx::query(
        "UPDATE invoices SET status = 'expired' WHERE status = 'open' AND datetime(due_at) <= datetime('now')",
    )
    .execute(db)
    .await
    .context("failed to expire overdue invoices")?;

    if result.rows_affected() > 0 {
        warn!(
            expired_count = result.rows_affected(),
            "expired overdue invoices"
        );
    } else {
        info!("no overdue invoices to expire");
    }

    Ok(())
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
