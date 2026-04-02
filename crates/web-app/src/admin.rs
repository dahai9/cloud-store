use crate::auth;
use crate::AppState;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Serialize)]
pub struct AdminPlanItem {
    pub id: String,
    pub code: String,
    pub name: String,
    pub monthly_price: String,
    pub memory_mb: i64,
    pub storage_gb: i64,
    pub cpu_cores: i64,
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
    pub bandwidth_mbps: Option<i64>,
    pub traffic_gb: Option<i64>,
    pub active: Option<bool>,
    pub max_inventory: Option<i64>,
}

#[derive(Serialize)]
pub struct GuestItem {
    pub id: String,
    pub email: String,
    pub disabled: bool,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct GuestUpdateRequest {
    pub disabled: bool,
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

    let rows = sqlx::query_as::<_, (String, String, String, String, i64, i64, i64, i64, i64, i64, Option<i64>, i64)>(
        "SELECT id, code, name, CAST(monthly_price AS TEXT), memory_mb, storage_gb, cpu_cores, bandwidth_mbps, traffic_gb, active, max_inventory, sold_inventory FROM nat_plans ORDER BY created_at DESC",
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

    let id = uuid::Uuid::new_v4().to_string();
    let price: f64 = payload
        .monthly_price
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid price"))?;

    sqlx::query(
        "INSERT INTO nat_plans (id, code, name, memory_mb, storage_gb, cpu_cores, bandwidth_mbps, traffic_gb, monthly_price, active) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&payload.code)
    .bind(&payload.name)
    .bind(payload.memory_mb)
    .bind(payload.storage_gb)
    .bind(payload.cpu_cores)
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

    let current = sqlx::query_as::<_, (String, String, String, String, i64, i64, i64, i64, i64, i64, Option<i64>, i64)>(
        "SELECT id, code, name, CAST(monthly_price AS TEXT), memory_mb, storage_gb, cpu_cores, bandwidth_mbps, traffic_gb, active, max_inventory, sold_inventory FROM nat_plans WHERE id = ? LIMIT 1",
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
    let next_bandwidth_mbps = payload.bandwidth_mbps.unwrap_or(current.7);
    let next_traffic_gb = payload.traffic_gb.unwrap_or(current.8);
    let next_active = payload.active.unwrap_or(current.9 != 0);
    let next_max_inventory = payload.max_inventory.or(current.10);

    if let Some(limit) = next_max_inventory {
        if limit < current.11 {
            return Err((
                StatusCode::BAD_REQUEST,
                "max_inventory cannot be lower than sold_inventory",
            ));
        }
    }

    sqlx::query("UPDATE nat_plans SET code = ?, name = ?, monthly_price = ?, memory_mb = ?, storage_gb = ?, cpu_cores = ?, bandwidth_mbps = ?, traffic_gb = ?, active = ?, max_inventory = ? WHERE id = ?")
        .bind(&next_code)
        .bind(&next_name)
        .bind(price_val)
        .bind(next_memory_mb)
        .bind(next_storage_gb)
        .bind(next_cpu_cores)
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
        bandwidth_mbps: next_bandwidth_mbps,
        traffic_gb: next_traffic_gb,
        active: next_active,
        max_inventory: next_max_inventory,
        sold_inventory: current.11,
    }))
}

pub async fn list_guests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<GuestItem>>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    let rows = sqlx::query_as::<_, (String, String, i64, String)>(
        "SELECT id, email, COALESCE(disabled, 0), created_at FROM users WHERE role = 'user' ORDER BY created_at DESC LIMIT 200",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, "failed to query guests");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load guests")
    })?;

    let items = rows
        .into_iter()
        .map(|(id, email, disabled, created_at)| GuestItem {
            id,
            email,
            disabled: disabled != 0,
            created_at,
        })
        .collect();

    Ok(Json(items))
}

pub async fn update_guest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(payload): Json<GuestUpdateRequest>,
) -> Result<Json<GuestItem>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    let target = sqlx::query_as::<_, (String, String, String, i64, String)>(
        "SELECT id, email, role, COALESCE(disabled, 0), created_at FROM users WHERE id = ? LIMIT 1",
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
        disabled: payload.disabled,
        created_at: target.4,
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
