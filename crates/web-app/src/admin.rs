use crate::auth;
use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::error;

const UNLIMITED_TRAFFIC_GB: i64 = -1;

fn validate_traffic_gb(traffic_gb: i64) -> Result<(), (StatusCode, &'static str)> {
    if traffic_gb < UNLIMITED_TRAFFIC_GB {
        return Err((StatusCode::BAD_REQUEST, "traffic_gb must be -1 or greater"));
    }

    Ok(())
}

fn validate_cpu_cores(cpu_cores: i64) -> Result<(), (StatusCode, &'static str)> {
    if cpu_cores < 1 {
        return Err((StatusCode::BAD_REQUEST, "cpu_cores must be at least 1"));
    }

    Ok(())
}

fn validate_cpu_allowance_pct(cpu_allowance_pct: i64) -> Result<(), (StatusCode, &'static str)> {
    if cpu_allowance_pct < 1 {
        return Err((
            StatusCode::BAD_REQUEST,
            "cpu_allowance_pct must be at least 1",
        ));
    }

    Ok(())
}

fn validate_nat_port_lease_range(
    public_ip: &str,
    start_port: i64,
    end_port: i64,
) -> Result<(), (StatusCode, &'static str)> {
    if public_ip.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "public_ip is required"));
    }

    if start_port < 1 || end_port < 1 || start_port > end_port || end_port > 65_535 {
        return Err((StatusCode::BAD_REQUEST, "invalid port range"));
    }

    Ok(())
}

#[derive(Serialize)]
pub struct AdminPlanItem {
    pub id: String,
    pub code: String,
    pub name: String,
    pub monthly_price: String,
    pub memory_mb: i64,
    pub storage_gb: i64,
    pub cpu_cores: i64,
    pub cpu_allowance_pct: i64,
    pub bandwidth_mbps: i64,
    pub traffic_gb: i64,
    pub active: bool,
    pub max_inventory: Option<i64>,
    pub sold_inventory: i64,
}

#[derive(Deserialize)]
pub struct AdminPlanCreateRequest {
    pub code: String,
    pub name: String,
    pub monthly_price: String,
    pub memory_mb: i64,
    pub storage_gb: i64,
    pub cpu_cores: i64,
    pub cpu_allowance_pct: i64,
    pub bandwidth_mbps: i64,
    pub traffic_gb: i64,
}

#[derive(Deserialize)]
pub struct AdminPlanUpdateRequest {
    pub code: Option<String>,
    pub name: Option<String>,
    pub monthly_price: Option<String>,
    pub memory_mb: Option<i64>,
    pub storage_gb: Option<i64>,
    pub cpu_cores: Option<i64>,
    pub cpu_allowance_pct: Option<i64>,
    pub bandwidth_mbps: Option<i64>,
    pub traffic_gb: Option<i64>,
    pub active: Option<bool>,
    pub max_inventory: Option<i64>,
}

#[derive(Serialize)]
pub struct GuestItem {
    pub id: String,
    pub email: String,
    pub balance: String,
    pub disabled: bool,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct GuestUpdateRequest {
    pub disabled: bool,
}

#[derive(Deserialize)]
pub struct AdminInstanceCreateRequest {
    pub user_id: String,
    pub plan_id: String,
    pub node_id: Option<String>,
}

#[derive(Deserialize)]
pub struct AdminInstanceDeleteRequest {
    pub refund_amount: Option<String>,
}

#[derive(Deserialize)]
pub struct AdminTicketCreateRequest {
    pub user_id: String,
    pub category: String,
    pub priority: String,
    pub subject: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NodeItem {
    pub id: String,
    pub name: String,
    pub region: String,
    pub cpu_cores_total: i64,
    pub memory_mb_total: i64,
    pub storage_gb_total: i64,
    pub cpu_cores_used: i64,
    pub memory_mb_used: i64,
    pub storage_gb_used: i64,
    pub api_endpoint: Option<String>,
    pub api_token: Option<String>,
}

#[derive(Deserialize)]
pub struct NodeCreateRequest {
    pub name: String,
    pub region: String,
    pub cpu_cores_total: i64,
    pub memory_mb_total: i64,
    pub storage_gb_total: i64,
    pub api_endpoint: Option<String>,
    pub api_token: Option<String>,
}

#[derive(Deserialize)]
pub struct NodeUpdateRequest {
    pub name: Option<String>,
    pub region: Option<String>,
    pub cpu_cores_total: Option<i64>,
    pub memory_mb_total: Option<i64>,
    pub storage_gb_total: Option<i64>,
    pub api_endpoint: Option<String>,
    pub api_token: Option<String>,
}

#[derive(Serialize)]
pub struct NatPortLeaseItem {
    pub id: String,
    pub node_id: String,
    pub node_name: String,
    pub node_region: String,
    pub public_ip: String,
    pub start_port: i64,
    pub end_port: i64,
    pub reserved: bool,
    pub reserved_for_order_id: Option<String>,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct NatPortLeaseCreateRequest {
    pub node_id: String,
    pub public_ip: String,
    pub start_port: i64,
    pub end_port: i64,
}

#[derive(Serialize)]
pub struct InstanceItem {
    pub id: String,
    pub user_email: String,
    pub node_name: String,
    pub plan_name: String,
    pub status: String,
    pub os_template: String,
    pub created_at: String,
}

pub async fn list_plans(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AdminPlanItem>>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    let rows = sqlx::query_as::<_, (String, String, String, String, i64, i64, i64, i64, i64, i64, i64, Option<i64>, i64)>(
        "SELECT id, code, name, CAST(monthly_price AS TEXT), memory_mb, storage_gb, cpu_cores, cpu_allowance_pct, bandwidth_mbps, traffic_gb, active, max_inventory, sold_inventory FROM nat_plans ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, "failed to query admin plans");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to load admin plans",
        )
    })?;

    let items = rows
        .into_iter()
        .map(
            |(
                id,
                code,
                name,
                monthly_price,
                memory_mb,
                storage_gb,
                cpu_cores,
                cpu_allowance_pct,
                bandwidth_mbps,
                traffic_gb,
                active,
                max_inventory,
                sold_inventory,
            )| {
                AdminPlanItem {
                    id,
                    code,
                    name,
                    monthly_price,
                    memory_mb,
                    storage_gb,
                    cpu_cores,
                    cpu_allowance_pct,
                    bandwidth_mbps,
                    traffic_gb,
                    active: active != 0,
                    max_inventory,
                    sold_inventory,
                }
            },
        )
        .collect();

    Ok(Json(items))
}

pub async fn add_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AdminPlanCreateRequest>,
) -> Result<Json<AdminPlanItem>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    validate_traffic_gb(payload.traffic_gb)?;
    validate_cpu_cores(payload.cpu_cores)?;
    validate_cpu_allowance_pct(payload.cpu_allowance_pct)?;

    let id = uuid::Uuid::new_v4().to_string();
    let price: f64 = payload
        .monthly_price
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid price"))?;

    sqlx::query(
        "INSERT INTO nat_plans (id, code, name, memory_mb, storage_gb, cpu_cores, cpu_allowance_pct, bandwidth_mbps, traffic_gb, monthly_price, active) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&payload.code)
    .bind(&payload.name)
    .bind(payload.memory_mb)
    .bind(payload.storage_gb)
    .bind(payload.cpu_cores)
    .bind(payload.cpu_allowance_pct)
    .bind(payload.bandwidth_mbps)
    .bind(payload.traffic_gb)
    .bind(price)
    .bind(1) // default active
    .execute(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, "failed to create plan");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to create plan")
    })?;

    Ok(Json(AdminPlanItem {
        id,
        code: payload.code,
        name: payload.name,
        monthly_price: payload.monthly_price,
        memory_mb: payload.memory_mb,
        storage_gb: payload.storage_gb,
        cpu_cores: payload.cpu_cores,
        cpu_allowance_pct: payload.cpu_allowance_pct,
        bandwidth_mbps: payload.bandwidth_mbps,
        traffic_gb: payload.traffic_gb,
        active: true,
        max_inventory: None,
        sold_inventory: 0,
    }))
}

pub async fn update_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(plan_id): Path<String>,
    Json(payload): Json<AdminPlanUpdateRequest>,
) -> Result<Json<AdminPlanItem>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    let current = sqlx::query_as::<_, (String, String, String, String, i64, i64, i64, i64, i64, i64, i64, Option<i64>, i64)>(
        "SELECT id, code, name, CAST(monthly_price AS TEXT), memory_mb, storage_gb, cpu_cores, cpu_allowance_pct, bandwidth_mbps, traffic_gb, active, max_inventory, sold_inventory FROM nat_plans WHERE id = ? LIMIT 1",
    )
    .bind(&plan_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, plan_id = %plan_id, "failed to query target plan");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load target plan")
    })?
    .ok_or((StatusCode::NOT_FOUND, "plan not found"))?;

    let next_code = payload.code.unwrap_or(current.1);
    let next_name = payload.name.unwrap_or(current.2);
    let next_price = payload.monthly_price.unwrap_or(current.3);
    let price_val: f64 = next_price
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid price"))?;
    let next_memory_mb = payload.memory_mb.unwrap_or(current.4);
    let next_storage_gb = payload.storage_gb.unwrap_or(current.5);
    let next_cpu_cores = payload.cpu_cores.unwrap_or(current.6);
    validate_cpu_cores(next_cpu_cores)?;
    let next_cpu_allowance_pct = payload.cpu_allowance_pct.unwrap_or(current.7);
    validate_cpu_allowance_pct(next_cpu_allowance_pct)?;
    let next_bandwidth_mbps = payload.bandwidth_mbps.unwrap_or(current.8);
    let next_traffic_gb = payload.traffic_gb.unwrap_or(current.9);
    let next_active = payload.active.unwrap_or(current.10 != 0);
    let next_max_inventory = payload.max_inventory.or(current.11);

    validate_traffic_gb(next_traffic_gb)?;

    if let Some(limit) = next_max_inventory {
        if limit < current.12 {
            return Err((
                StatusCode::BAD_REQUEST,
                "max_inventory cannot be lower than sold_inventory",
            ));
        }
    }

    sqlx::query("UPDATE nat_plans SET code = ?, name = ?, monthly_price = ?, memory_mb = ?, storage_gb = ?, cpu_cores = ?, cpu_allowance_pct = ?, bandwidth_mbps = ?, traffic_gb = ?, active = ?, max_inventory = ? WHERE id = ?")
        .bind(&next_code)
        .bind(&next_name)
        .bind(price_val)
        .bind(next_memory_mb)
        .bind(next_storage_gb)
        .bind(next_cpu_cores)
        .bind(next_cpu_allowance_pct)
        .bind(next_bandwidth_mbps)
        .bind(next_traffic_gb)
        .bind(if next_active { 1 } else { 0 })
        .bind(next_max_inventory)
        .bind(&plan_id)
        .execute(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, plan_id = %plan_id, "failed to update plan settings");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to update plan")
        })?;

    Ok(Json(AdminPlanItem {
        id: current.0,
        code: next_code,
        name: next_name,
        monthly_price: next_price,
        memory_mb: next_memory_mb,
        storage_gb: next_storage_gb,
        cpu_cores: next_cpu_cores,
        cpu_allowance_pct: next_cpu_allowance_pct,
        bandwidth_mbps: next_bandwidth_mbps,
        traffic_gb: next_traffic_gb,
        active: next_active,
        max_inventory: next_max_inventory,
        sold_inventory: current.12,
    }))
}

#[derive(Deserialize)]
pub struct GuestSearchQuery {
    pub search: Option<String>,
}

pub async fn list_guests(
    State(state): State<AppState>,
    Query(query): Query<GuestSearchQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<GuestItem>>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    let search_pattern = query.search.unwrap_or_default().trim().to_string();

    let rows = if search_pattern.is_empty() {
        sqlx::query_as::<_, (String, String, String, i64, String)>(
            "SELECT id, email, CAST(balance AS TEXT), COALESCE(disabled, 0), created_at FROM users WHERE role = 'user' ORDER BY created_at DESC LIMIT 200",
        )
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, (String, String, String, i64, String)>(
            "SELECT id, email, CAST(balance AS TEXT), COALESCE(disabled, 0), created_at FROM users WHERE role = 'user' AND (email LIKE ? OR id = ?) ORDER BY created_at DESC LIMIT 200",
        )
        .bind(format!("%{search_pattern}%"))
        .bind(&search_pattern)
        .fetch_all(&state.db)
        .await
    }.map_err(|err| {
        error!(error = %err, "failed to query guests");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load guests")
    })?;

    let items = rows
        .into_iter()
        .map(|(id, email, balance, disabled, created_at)| GuestItem {
            id,
            email,
            balance,
            disabled: disabled != 0,
            created_at,
        })
        .collect();

    Ok(Json(items))
}

async fn has_open_after_sales_ticket(db: &sqlx::SqlitePool, user_id: &str) -> bool {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM support_tickets WHERE user_id = ? AND category = 'AfterSales' AND status != 'closed'"
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .unwrap_or(0) > 0
}

pub async fn admin_add_instance(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AdminInstanceCreateRequest>,
) -> Result<Json<InstanceItem>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    if !has_open_after_sales_ticket(&state.db, &payload.user_id).await {
        return Err((StatusCode::FORBIDDEN, "User must have an open AfterSales ticket to add an instance manually."));
    }

    // Basic validation: user exists?
    let user_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE id = ?")
        .bind(&payload.user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;
    if user_exists == 0 {
        return Err((StatusCode::NOT_FOUND, "user not found"));
    }

    // Plan exists?
    let plan_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM nat_plans WHERE id = ?")
        .bind(&payload.plan_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;
    if plan_exists == 0 {
        return Err((StatusCode::NOT_FOUND, "plan not found"));
    }

    // We create a "paid" order so the worker picks it up
    let order_id = uuid::Uuid::new_v4().to_string();
    let idempotency_key = format!("admin-manual-{}", order_id);

    // Get plan price for the order
    let price: f64 = sqlx::query_scalar::<_, f64>("SELECT monthly_price FROM nat_plans WHERE id = ?")
        .bind(&payload.plan_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    sqlx::query(
        "INSERT INTO orders (id, user_id, plan_id, status, total_amount, idempotency_key) VALUES (?, ?, ?, 'paid', ?, ?)"
    )
    .bind(&order_id)
    .bind(&payload.user_id)
    .bind(&payload.plan_id)
    .bind(price)
    .bind(&idempotency_key)
    .execute(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, "failed to create admin order");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to initiate provisioning")
    })?;

    Ok(Json(InstanceItem {
        id: "Pending".to_string(),
        user_email: payload.user_id, // This is just a placeholder until it's provisioned
        node_name: "TBD".to_string(),
        plan_name: "TBD".to_string(),
        status: "provisioning".to_string(),
        os_template: "TBD".to_string(),
        created_at: "Now".to_string(),
    }))
}

pub async fn admin_delete_instance(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(instance_id): Path<String>,
    Json(payload): Json<AdminInstanceDeleteRequest>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    // Get instance info
    let instance = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id, user_id, status FROM instances WHERE id = ? LIMIT 1"
    )
    .bind(&instance_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
    .ok_or((StatusCode::NOT_FOUND, "instance not found"))?;

    if instance.2 == "deleted" {
        return Err((StatusCode::BAD_REQUEST, "instance already deleted"));
    }

    if !has_open_after_sales_ticket(&state.db, &instance.1).await {
        return Err((StatusCode::FORBIDDEN, "User must have an open AfterSales ticket to delete an instance manually."));
    }

    let refund_amount: f64 = payload.refund_amount.unwrap_or_else(|| "0".to_string())
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid refund amount"))?;

    let mut tx = state.db.begin().await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    // Mark as deleted
    sqlx::query("UPDATE instances SET status = 'deleted', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&instance_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to update instance"))?;

    if refund_amount > 0.0 {
        // Add balance to user
        sqlx::query("UPDATE users SET balance = balance + ? WHERE id = ?")
            .bind(refund_amount)
            .bind(&instance.1)
            .execute(&mut *tx)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to update user balance"))?;

        // Create transaction record
        let tx_id = uuid::Uuid::new_v4().to_string();
        let description = format!("Refund for instance {}", instance_id);
        sqlx::query(
            "INSERT INTO balance_transactions (id, user_id, amount, type, description) VALUES (?, ?, ?, 'refund', ?)"
        )
        .bind(&tx_id)
        .bind(&instance.1)
        .bind(refund_amount)
        .bind(&description)
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to create balance transaction"))?;
    }

    tx.commit().await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to commit transaction"))?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn admin_create_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AdminTicketCreateRequest>,
) -> Result<Json<crate::tickets::TicketItem>, (StatusCode, &'static str)> {
    let admin = auth::require_admin(&headers, &state).await?;

    let ticket_id = uuid::Uuid::new_v4().to_string();
    let mut tx = state.db.begin().await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    sqlx::query(
        "INSERT INTO support_tickets (id, user_id, category, priority, subject, status) VALUES (?, ?, ?, ?, ?, 'open')"
    )
    .bind(&ticket_id)
    .bind(&payload.user_id)
    .bind(&payload.category)
    .bind(&payload.priority)
    .bind(&payload.subject)
    .execute(&mut *tx)
    .await
    .map_err(|err| {
        error!(error = %err, "failed to create ticket");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to create ticket")
    })?;

    let message_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO support_messages (id, ticket_id, sender_user_id, message) VALUES (?, ?, ?, ?)"
    )
    .bind(&message_id)
    .bind(&ticket_id)
    .bind(&admin.id)
    .bind(&payload.message)
    .execute(&mut *tx)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to create initial message"))?;

    tx.commit().await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to commit"))?;

    Ok(Json(crate::tickets::TicketItem {
        id: ticket_id,
        user_id: payload.user_id,
        subject: payload.subject,
        category: payload.category,
        priority: payload.priority,
        status: "open".to_string(),
    }))
}

pub async fn update_guest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(payload): Json<GuestUpdateRequest>,
) -> Result<Json<GuestItem>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    let target = sqlx::query_as::<_, (String, String, String, String, i64, String)>(
        "SELECT id, email, role, CAST(balance AS TEXT), COALESCE(disabled, 0), created_at FROM users WHERE id = ? LIMIT 1",
    )
    .bind(&user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, user_id = %user_id, "failed to query target guest");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load guest")
    })?
    .ok_or((StatusCode::NOT_FOUND, "guest not found"))?;

    if target.2 != "user" {
        return Err((
            StatusCode::BAD_REQUEST,
            "only guest(user) can be configured",
        ));
    }

    sqlx::query("UPDATE users SET disabled = ? WHERE id = ?")
        .bind(if payload.disabled { 1 } else { 0 })
        .bind(&user_id)
        .execute(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, user_id = %user_id, "failed to update guest config");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to update guest")
        })?;

    Ok(Json(GuestItem {
        id: target.0,
        email: target.1,
        balance: target.3,
        disabled: payload.disabled,
        created_at: target.5,
    }))
}

pub async fn list_nodes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<NodeItem>>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    let rows = sqlx::query_as::<_, (String, String, String, i64, i64, i64, i64, i64, i64, Option<String>, Option<String>)>(
        "SELECT id, name, region, cpu_cores_total, memory_mb_total, storage_gb_total, cpu_cores_used, memory_mb_used, storage_gb_used, api_endpoint, api_token FROM nodes ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, "failed to list nodes");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load nodes")
    })?;

    let items = rows
        .into_iter()
        .map(
            |(
                id,
                name,
                region,
                cpu_cores_total,
                memory_mb_total,
                storage_gb_total,
                cpu_cores_used,
                memory_mb_used,
                storage_gb_used,
                api_endpoint,
                api_token,
            )| NodeItem {
                id,
                name,
                region,
                cpu_cores_total,
                memory_mb_total,
                storage_gb_total,
                cpu_cores_used,
                memory_mb_used,
                storage_gb_used,
                api_endpoint,
                api_token,
            },
        )
        .collect();

    Ok(Json(items))
}

pub async fn add_node(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<NodeCreateRequest>,
) -> Result<Json<NodeItem>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO nodes (id, name, region, cpu_cores_total, memory_mb_total, storage_gb_total, api_endpoint, api_token) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&payload.name)
    .bind(&payload.region)
    .bind(payload.cpu_cores_total)
    .bind(payload.memory_mb_total)
    .bind(payload.storage_gb_total)
    .bind(&payload.api_endpoint)
    .bind(&payload.api_token)
    .execute(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, "failed to create node");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to create node")
    })?;

    Ok(Json(NodeItem {
        id,
        name: payload.name,
        region: payload.region,
        cpu_cores_total: payload.cpu_cores_total,
        memory_mb_total: payload.memory_mb_total,
        storage_gb_total: payload.storage_gb_total,
        cpu_cores_used: 0,
        memory_mb_used: 0,
        storage_gb_used: 0,
        api_endpoint: payload.api_endpoint,
        api_token: payload.api_token,
    }))
}

pub async fn update_node(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(node_id): Path<String>,
    Json(payload): Json<NodeUpdateRequest>,
) -> Result<Json<NodeItem>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    let current = sqlx::query_as::<_, (String, String, String, i64, i64, i64, i64, i64, i64, Option<String>, Option<String>)>(
        "SELECT id, name, region, cpu_cores_total, memory_mb_total, storage_gb_total, cpu_cores_used, memory_mb_used, storage_gb_used, api_endpoint, api_token FROM nodes WHERE id = ? LIMIT 1",
    )
    .bind(&node_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, node_id = %node_id, "failed to query target node");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load node")
    })?
    .ok_or((StatusCode::NOT_FOUND, "node not found"))?;

    let next_name = payload.name.unwrap_or(current.1);
    let next_region = payload.region.unwrap_or(current.2);
    let next_cpu_cores_total = payload.cpu_cores_total.unwrap_or(current.3);
    let next_memory_mb_total = payload.memory_mb_total.unwrap_or(current.4);
    let next_storage_gb_total = payload.storage_gb_total.unwrap_or(current.5);
    let next_api_endpoint = payload.api_endpoint.or(current.9);
    let next_api_token = payload.api_token.or(current.10);

    sqlx::query(
        "UPDATE nodes SET name = ?, region = ?, cpu_cores_total = ?, memory_mb_total = ?, storage_gb_total = ?, api_endpoint = ?, api_token = ? WHERE id = ?"
    )
    .bind(&next_name)
    .bind(&next_region)
    .bind(next_cpu_cores_total)
    .bind(next_memory_mb_total)
    .bind(next_storage_gb_total)
    .bind(&next_api_endpoint)
    .bind(&next_api_token)
    .bind(&node_id)
    .execute(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, node_id = %node_id, "failed to update node");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to update node")
    })?;

    Ok(Json(NodeItem {
        id: current.0,
        name: next_name,
        region: next_region,
        cpu_cores_total: next_cpu_cores_total,
        memory_mb_total: next_memory_mb_total,
        storage_gb_total: next_storage_gb_total,
        cpu_cores_used: current.6,
        memory_mb_used: current.7,
        storage_gb_used: current.8,
        api_endpoint: next_api_endpoint,
        api_token: next_api_token,
    }))
}

pub async fn list_nat_port_leases(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<NatPortLeaseItem>>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            String,
            i64,
            i64,
            i64,
            Option<String>,
            String,
        ),
    >(
        "SELECT l.id, l.node_id, n.name, n.region, l.public_ip, l.start_port, l.end_port, l.reserved, l.reserved_for_order_id, l.created_at\n         FROM nat_port_leases l\n         INNER JOIN nodes n ON n.id = l.node_id\n         ORDER BY l.created_at DESC\n         LIMIT 200",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, "failed to list nat port leases");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to load nat port leases",
        )
    })?;

    let items = rows
        .into_iter()
        .map(
            |(
                id,
                node_id,
                node_name,
                node_region,
                public_ip,
                start_port,
                end_port,
                reserved,
                reserved_for_order_id,
                created_at,
            )| NatPortLeaseItem {
                id,
                node_id,
                node_name,
                node_region,
                public_ip,
                start_port,
                end_port,
                reserved: reserved != 0,
                reserved_for_order_id,
                created_at,
            },
        )
        .collect();

    Ok(Json(items))
}

pub async fn add_nat_port_lease(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<NatPortLeaseCreateRequest>,
) -> Result<Json<NatPortLeaseItem>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    let NatPortLeaseCreateRequest {
        node_id,
        public_ip,
        start_port,
        end_port,
    } = payload;

    let public_ip = public_ip.trim().to_string();
    validate_nat_port_lease_range(&public_ip, start_port, end_port)?;

    let node = sqlx::query_as::<_, (String, String)>(
        "SELECT name, region FROM nodes WHERE id = ? LIMIT 1",
    )
    .bind(&node_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, node_id = %node_id, "failed to query node for nat port lease");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to load node for nat port lease",
        )
    })?
    .ok_or((StatusCode::NOT_FOUND, "node not found"))?;

    let existing = sqlx::query_scalar::<_, String>(
        "SELECT id FROM nat_port_leases WHERE node_id = ? AND public_ip = ? AND start_port = ? AND end_port = ? LIMIT 1",
    )
    .bind(&node_id)
    .bind(&public_ip)
    .bind(start_port)
    .bind(end_port)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, node_id = %node_id, public_ip = %public_ip, start_port = start_port, end_port = end_port, "failed to check duplicate nat port lease");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to validate nat port lease",
        )
    })?;

    if existing.is_some() {
        return Err((StatusCode::CONFLICT, "nat port lease already exists"));
    }

    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO nat_port_leases (id, node_id, public_ip, start_port, end_port, reserved) VALUES (?, ?, ?, ?, ?, 0)",
    )
    .bind(&id)
    .bind(&node_id)
    .bind(&public_ip)
    .bind(start_port)
    .bind(end_port)
    .execute(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, node_id = %node_id, public_ip = %public_ip, start_port = start_port, end_port = end_port, "failed to create nat port lease");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to create nat port lease",
        )
    })?;

    let created_at = sqlx::query_scalar::<_, String>(
        "SELECT created_at FROM nat_port_leases WHERE id = ? LIMIT 1",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, lease_id = %id, "failed to load created nat port lease");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to load nat port lease",
        )
    })?;

    Ok(Json(NatPortLeaseItem {
        id,
        node_id,
        node_name: node.0,
        node_region: node.1,
        public_ip,
        start_port,
        end_port,
        reserved: false,
        reserved_for_order_id: None,
        created_at,
    }))
}

pub async fn delete_nat_port_lease(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(lease_id): Path<String>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    let lease =
        sqlx::query_as::<_, (i64,)>("SELECT reserved FROM nat_port_leases WHERE id = ? LIMIT 1")
            .bind(&lease_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| {
                error!(error = %err, lease_id = %lease_id, "failed to query nat port lease");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to load nat port lease",
                )
            })?
            .ok_or((StatusCode::NOT_FOUND, "nat port lease not found"))?;

    if lease.0 != 0 {
        return Err((
            StatusCode::CONFLICT,
            "reserved nat port lease cannot be deleted",
        ));
    }

    sqlx::query("DELETE FROM nat_port_leases WHERE id = ?")
        .bind(&lease_id)
        .execute(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, lease_id = %lease_id, "failed to delete nat port lease");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to delete nat port lease",
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_instances(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<InstanceItem>>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, String)>(
        "SELECT 
            i.id, 
            u.email as user_email, 
            n.name as node_name, 
            p.name as plan_name, 
            i.status, 
            i.os_template, 
            i.created_at 
        FROM instances i
        JOIN users u ON i.user_id = u.id
        JOIN nodes n ON i.node_id = n.id
        JOIN nat_plans p ON i.plan_id = p.id
        ORDER BY i.created_at DESC LIMIT 500",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, "failed to list instances");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to load instances",
        )
    })?;

    let items = rows
        .into_iter()
        .map(
            |(id, user_email, node_name, plan_name, status, os_template, created_at)| {
                InstanceItem {
                    id,
                    user_email,
                    node_name,
                    plan_name,
                    status,
                    os_template,
                    created_at,
                }
            },
        )
        .collect();

    Ok(Json(items))
}
