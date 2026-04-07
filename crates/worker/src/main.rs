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

        if let Err(e) = sync_instance_statuses(&db, &provider).await {
            error!(error = %e, "failed to sync instance statuses");
        }

        if let Err(e) = run_renewal_tick(&db).await {
            error!(error = %e, "failed to run renewal tick");
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
    }
}

async fn sync_instance_statuses(db: &SqlitePool, provider: &IncusProvider) -> anyhow::Result<()> {
    let rows = sqlx::query(
        "SELECT i.id, i.provider_instance_id, n.api_endpoint, n.api_token
         FROM instances i
         JOIN nodes n ON i.node_id = n.id
         WHERE i.status != 'deleted' AND i.provider_instance_id IS NOT NULL",
    )
    .fetch_all(db)
    .await?;

    for row in rows {
        let id: String = row.get("id");
        let provider_instance_id: String = row.get("provider_instance_id");
        let api_endpoint: String = row
            .get::<Option<String>, _>("api_endpoint")
            .unwrap_or_default();
        let api_token: Option<String> = row.get("api_token");

        if api_endpoint.is_empty() {
            continue;
        }

        let node_conn = NodeConnection {
            endpoint: api_endpoint,
            token: api_token,
        };

        match provider
            .get_metrics(&node_conn, &provider_instance_id)
            .await
        {
            Ok(metrics) => {
                let status = match metrics.status.to_lowercase().as_str() {
                    "pending" => "pending",
                    "starting" => "starting",
                    "running" => "running",
                    "stopped" => "stopped",
                    "suspended" => "suspended",
                    "deleted" => "deleted",
                    _ => "unknown",
                };

                sqlx::query("UPDATE instances SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status != ?")
                    .bind(status)
                    .bind(&id)
                    .bind(status)
                    .execute(db)
                    .await?;
            }
            Err(e) => {
                warn!(instance_id = %id, error = %e, "failed to fetch live status for sync");
            }
        }
    }

    Ok(())
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
        "SELECT id, code, name, memory_mb, storage_gb, cpu_cores, cpu_allowance_pct, bandwidth_mbps, traffic_gb, CAST(monthly_price AS TEXT) AS monthly_price, nat_port_limit FROM nat_plans WHERE id = ?"
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
        nat_port_limit: plan_row.get::<i64, _>("nat_port_limit") as i32,
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

    // 4. Ensure node has NAT capacity (at least one range defined)
    let has_nat =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM nat_port_leases WHERE node_id = ?")
            .bind(&node_id_str)
            .fetch_one(db)
            .await?;

    if has_nat == 0 {
        anyhow::bail!("selected node has no NAT port pools configured");
    }

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

    // Set initial root password
    let root_password = Uuid::new_v4()
        .to_string()
        .split('-')
        .next()
        .unwrap()
        .to_string()
        + "!"
        + Uuid::new_v4().to_string().split('-').next_back().unwrap();
    if let Err(e) = provider
        .reset_password(&node_conn, &result.instance_id, &root_password)
        .await
    {
        error!(error = %e, instance_id = %result.instance_id, "failed to set initial root password");
    }

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

    // Create Instance record
    let instance_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO instances (id, user_id, node_id, order_id, plan_id, provider_instance_id, status, os_template, root_password)
         VALUES (?, ?, ?, ?, ?, ?, 'running', ?, ?)"
    )
    .bind(&instance_id)
    .bind(user_id_str)
    .bind(&node_id_str)
    .bind(order_id_str)
    .bind(plan_id_str)
    .bind(&result.instance_id)
    .bind(&os_template)
    .bind(&root_password)
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

async fn run_renewal_tick(db: &SqlitePool) -> anyhow::Result<()> {
    // 1. Find active subscriptions that have auto_renew enabled and are expiring within 24 hours
    let subscriptions = sqlx::query(
        "SELECT s.id, s.user_id, s.order_id, s.current_period_end, i.id as instance_id, p.monthly_price, p.name as plan_name
         FROM subscriptions s
         JOIN instances i ON s.order_id = i.order_id
         JOIN nat_plans p ON i.plan_id = p.id
         WHERE s.status = 'active' AND i.auto_renew = 1 AND i.status != 'deleted'
         AND datetime(s.current_period_end) <= datetime('now', '+1 day')
         LIMIT 20"
    )
    .fetch_all(db)
    .await?;

    for sub in subscriptions {
        let sub_id: String = sub.get("id");
        let user_id: String = sub.get("user_id");
        let instance_id: String = sub.get("instance_id");
        let monthly_price: Decimal = sub.get::<String, _>("monthly_price").parse().unwrap_or(Decimal::ZERO);
        let plan_name: String = sub.get("plan_name");
        let current_end: String = sub.get("current_period_end");

        // 2. Check user balance
        let balance: Decimal = sqlx::query_scalar::<_, String>("SELECT CAST(balance AS TEXT) FROM users WHERE id = ?")
            .bind(&user_id)
            .fetch_one(db)
            .await?
            .parse()
            .unwrap_or(Decimal::ZERO);

        if balance >= monthly_price {
            info!(instance_id = %instance_id, user_id = %user_id, "processing auto-renewal");

            let mut tx = db.begin().await?;

            // Deduct balance
            sqlx::query("UPDATE users SET balance = balance - ? WHERE id = ?")
                .bind(monthly_price.to_string())
                .bind(&user_id)
                .execute(&mut *tx)
                .await?;

            // Create transaction record
            let tx_id = Uuid::new_v4().to_string();
            let description = format!("Auto-renewal for {} ({})", plan_name, instance_id);
            sqlx::query(
                "INSERT INTO balance_transactions (id, user_id, amount, type, description) VALUES (?, ?, ?, 'auto_renew', ?)"
            )
            .bind(&tx_id)
            .bind(&user_id)
            .bind((-monthly_price).to_string())
            .bind(&description)
            .execute(&mut *tx)
            .await?;

            // Extend subscription
            sqlx::query(
                "UPDATE subscriptions SET current_period_start = current_period_end, 
                 current_period_end = datetime(current_period_end, '+1 month'),
                 updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?"
            )
            .bind(&sub_id)
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;
            info!(instance_id = %instance_id, "auto-renewal successful, extended from {}", current_end);
        } else {
            warn!(instance_id = %instance_id, user_id = %user_id, "auto-renewal failed: insufficient balance");
            // Optional: disable auto-renew or notify user
            sqlx::query("UPDATE instances SET auto_renew = 0 WHERE id = ?")
                .bind(&instance_id)
                .execute(db)
                .await?;
        }
    }

    Ok(())
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
