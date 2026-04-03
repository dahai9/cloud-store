use anyhow::{anyhow, Context};
use provider_adapter::{ComputeProvider, IncusProvider, NodeConnection, ProvisionRequest};
use rust_decimal::Decimal;
use shared_domain::{NatPlan, DEFAULT_OS_TEMPLATE};
use sqlx::{Row, SqlitePool};
use std::path::PathBuf;
use std::str::FromStr;
use tracing::{error, info, warn};
use uuid::Uuid;

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

    let provider = IncusProvider::new().context("failed to initialize incus provider")?;

    loop {
        if let Err(e) = expire_overdue_invoices(&db).await {
            error!(error = %e, "failed to expire overdue invoices");
        }

        if let Err(e) = run_provisioning_tick(&db, &provider).await {
            error!(error = %e, "failed to run provisioning tick");
        }

        run_renewal_tick().await;

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn run_provisioning_tick(
    db: &SqlitePool,
    provider: &dyn ComputeProvider,
) -> anyhow::Result<()> {
    let orders = sqlx::query(
        "SELECT id, user_id, plan_id FROM orders WHERE status IN ('paid', 'provisioning') LIMIT 10",
    )
    .fetch_all(db)
    .await?;

    for order in orders {
        let order_id: String = order.get("id");
        let user_id: String = order.get("user_id");
        let plan_id: String = order.get("plan_id");

        if let Err(e) = process_order(db, provider, &order_id, &user_id, &plan_id).await {
            error!(order_id = %order_id, error = %e, "failed to provision order");
            sqlx::query("UPDATE orders SET status = 'failed' WHERE id = ?")
                .bind(&order_id)
                .execute(db)
                .await?;
        }
    }
    Ok(())
}

async fn process_order(
    db: &SqlitePool,
    provider: &dyn ComputeProvider,
    order_id_str: &str,
    user_id_str: &str,
    plan_id_str: &str,
) -> anyhow::Result<()> {
    info!(order_id = %order_id_str, "processing provisioning for order");

    let order_id = Uuid::parse_str(order_id_str)?;
    let user_id = Uuid::parse_str(user_id_str)?;

    // 1. Mark as Provisioning
    sqlx::query(
        "UPDATE orders SET status = 'provisioning', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(order_id_str)
    .execute(db)
    .await?;

    // 2. Fetch Plan details
    let plan_row = sqlx::query(
        "SELECT id, code, name, memory_mb, storage_gb, cpu_cores, cpu_allowance_pct, bandwidth_mbps, traffic_gb, CAST(monthly_price AS TEXT) AS monthly_price FROM nat_plans WHERE id = ?"
    )
    .bind(plan_id_str)
    .fetch_one(db)
    .await?;

    let plan = NatPlan {
        id: Uuid::parse_str(plan_row.get("id"))?,
        code: plan_row.get("code"),
        name: plan_row.get("name"),
        memory_mb: plan_row.get::<i64, _>("memory_mb") as i32,
        storage_gb: plan_row.get::<i64, _>("storage_gb") as i32,
        cpu_cores: plan_row.get::<i64, _>("cpu_cores") as i32,
        cpu_allowance_pct: plan_row.get::<i64, _>("cpu_allowance_pct") as i32,
        bandwidth_mbps: plan_row.get::<i64, _>("bandwidth_mbps") as i32,
        traffic_gb: plan_row.get::<i64, _>("traffic_gb") as i32,
        monthly_price: Decimal::from_str(&plan_row.get::<String, _>("monthly_price"))
            .unwrap_or(Decimal::ZERO),
        active: true,
    };

    // 3. Find Node with capacity (Least Used RAM)
    let node_row = sqlx::query(
           "SELECT id, cpu_cores_total, memory_mb_total, storage_gb_total, api_endpoint, api_token FROM nodes
            WHERE active = 1
            AND (memory_mb_total - memory_mb_used) >= ?
            AND (storage_gb_total - storage_gb_used) >= ?
         ORDER BY (memory_mb_total - memory_mb_used) DESC LIMIT 1"
    )
    .bind(plan.memory_mb as i64)
    .bind(plan.storage_gb as i64)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| anyhow!("no suitable node found with enough capacity"))?;

    let node_id_str: String = node_row.get("id");
    let node_id = Uuid::parse_str(&node_id_str)?;
    let node_conn = NodeConnection {
        endpoint: node_row
            .get::<Option<String>, _>("api_endpoint")
            .unwrap_or_default(),
        token: node_row.get::<Option<String>, _>("api_token"),
    };

    // 4. Find/Reserve NAT Port Lease
    let lease_row = sqlx::query(
        "SELECT id, public_ip, start_port, end_port FROM nat_port_leases
        WHERE node_id = ? AND (reserved_for_order_id = ? OR reserved = 0)
         LIMIT 1",
    )
    .bind(&node_id_str)
    .bind(order_id_str)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| anyhow!("no available NAT port leases on the selected node"))?;

    let lease_id: String = lease_row.get("id");

    // 5. Call Provider to create LXC container
    let os_template = DEFAULT_OS_TEMPLATE.to_string();
    let req = ProvisionRequest {
        order_id,
        user_id,
        node_id,
        plan: plan.clone(),
        os_template: os_template.clone(),
    };

    let result = provider.provision_instance(&node_conn, req).await?;

    // 6. Update DB in a transaction
    let mut tx = db.begin().await?;

    // Increment Node usage
    sqlx::query(
        "UPDATE nodes SET
            memory_mb_used = memory_mb_used + ?,
            storage_gb_used = storage_gb_used + ?
         WHERE id = ?",
    )
    .bind(plan.memory_mb as i64)
    .bind(plan.storage_gb as i64)
    .bind(&node_id_str)
    .execute(&mut *tx)
    .await?;

    // Update Port Lease
    sqlx::query(
        "UPDATE nat_port_leases SET
            reserved = 1,
            reserved_for_order_id = ?
         WHERE id = ?",
    )
    .bind(order_id_str)
    .bind(&lease_id)
    .execute(&mut *tx)
    .await?;

    // Create Instance record
    let instance_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO instances (id, user_id, node_id, order_id, plan_id, provider_instance_id, status, os_template)
         VALUES (?, ?, ?, ?, ?, ?, 'running', ?)"
    )
    .bind(&instance_id)
    .bind(user_id_str)
    .bind(&node_id_str)
    .bind(order_id_str)
    .bind(plan_id_str)
    .bind(&result.instance_id)
    .bind(&os_template)
    .execute(&mut *tx)
    .await?;

    // Complete Order
    sqlx::query("UPDATE orders SET status = 'active', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(order_id_str)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    info!(
        order_id = %order_id_str,
        instance_id = %instance_id,
        node_id = %node_id_str,
        "provisioning completed successfully"
    );

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
