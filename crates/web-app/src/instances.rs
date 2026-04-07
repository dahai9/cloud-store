use crate::auth;
use crate::AppState;
use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use futures_util::{SinkExt, StreamExt};
use provider_adapter::{ComputeProvider, IncusProvider, NodeConnection};
use serde::{Deserialize, Serialize};
use shared_domain::{InstanceStatus, DEFAULT_OS_TEMPLATE};
use sqlx::Row;
use tokio_tungstenite::tungstenite::Message as TungMessage;
use tracing::{error, info, warn};

#[derive(Serialize)]
pub struct InstanceItem {
    pub id: String,
    pub node_id: String,
    pub plan_id: String,
    pub status: String,
    pub os_template: String,
    pub auto_renew: bool,
    pub root_password: Option<String>,
    pub created_at: String,
    pub nat_info: Vec<NatInfo>,
}

#[derive(Serialize)]
pub struct NatInfo {
    pub ip: String,
    pub range: String,
}

#[derive(Deserialize)]
pub struct UpdateAutoRenewRequest {
    pub auto_renew: bool,
}

pub async fn update_auto_renew(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateAutoRenewRequest>,
) -> Result<Json<InstanceItem>, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    // Verify ownership
    let exists =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM instances WHERE id = ? AND user_id = ?")
            .bind(&id)
            .bind(&user.id)
            .fetch_one(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;
    if exists == 0 {
        return Err((StatusCode::FORBIDDEN, "access denied"));
    }

    sqlx::query("UPDATE instances SET auto_renew = ? WHERE id = ?")
        .bind(if payload.auto_renew { 1 } else { 0 })
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, instance_id = %id, "failed to update auto_renew");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to update settings",
            )
        })?;

    get_instance(State(state), headers, Path(id)).await
}

#[derive(Deserialize, Serialize)]
pub struct NatMappingItem {
    pub id: String,
    pub internal_port: i32,
    pub external_port: i32,
    pub protocol: String,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct CreateNatMappingRequest {
    pub internal_port: i32,
    pub external_port: i32,
    pub protocol: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstanceAction {
    Start,
    Stop,
    Restart,
    ResetPassword { new_password: Option<String> },
    Reinstall { os_template: Option<String> },
}

#[derive(Deserialize)]
pub struct ActionRequest {
    pub action: InstanceAction,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum BrowserConsoleMessage {
    Resize { rows: usize, cols: usize },
}

const MIN_TERMINAL_ROWS: usize = 8;
const MAX_TERMINAL_ROWS: usize = 400;
const MIN_TERMINAL_COLS: usize = 20;
const MAX_TERMINAL_COLS: usize = 800;

fn validate_resize(rows: usize, cols: usize) -> Option<(usize, usize)> {
    if !(MIN_TERMINAL_ROWS..=MAX_TERMINAL_ROWS).contains(&rows) {
        return None;
    }

    if !(MIN_TERMINAL_COLS..=MAX_TERMINAL_COLS).contains(&cols) {
        return None;
    }

    Some((rows, cols))
}

pub async fn list_instances(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<InstanceItem>>, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    let rows = sqlx::query_as::<_, (String, String, String, String, String, i64, Option<String>, String)>(
        "SELECT id, node_id, plan_id, status, os_template, COALESCE(auto_renew, 0), root_password, created_at FROM instances WHERE user_id = ? AND status != 'deleted' ORDER BY created_at DESC",
    )
    .bind(&user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, user_id = %user.id, "failed to query instances");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load instances")
    })?;

    let items = rows
        .into_iter()
        .map(
            |(id, node_id, plan_id, status, os_template, auto_renew, root_password, created_at)| {
                let status = match status.to_lowercase().as_str() {
                    "pending" => "pending",
                    "starting" => "starting",
                    "running" => "running",
                    "stopped" => "stopped",
                    "suspended" => "suspended",
                    "deleted" => "deleted",
                    _ => "unknown",
                };
                InstanceItem {
                    id,
                    node_id,
                    plan_id,
                    status: status.to_string(),
                    os_template,
                    auto_renew: auto_renew != 0,
                    root_password,
                    created_at,
                    nat_info: vec![],
                }
            },
        )
        .collect();

    Ok(Json(items))
}

pub async fn get_instance(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<InstanceItem>, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    let row = sqlx::query(
        "SELECT id, node_id, plan_id, status, os_template, COALESCE(auto_renew, 0) as auto_renew, root_password, created_at FROM instances WHERE id = ? AND user_id = ? LIMIT 1"
    )
    .bind(&id)
    .bind(&user.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, instance_id = %id, "failed to query instance");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load instance")
    })?
    .ok_or((StatusCode::NOT_FOUND, "instance not found"))?;

    let node_id: String = row.get("node_id");

    // Fetch all NAT pools for this node
    let pools = sqlx::query(
        "SELECT public_ip, start_port, end_port FROM nat_port_leases WHERE node_id = ?",
    )
    .bind(&node_id)
    .fetch_all(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, node_id = %node_id, "failed to query node nat pools");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to load nat pools",
        )
    })?;

    let nat_info = pools
        .into_iter()
        .map(|p| {
            let start_port: i64 = p.get("start_port");
            let end_port: i64 = p.get("end_port");
            NatInfo {
                ip: p.get("public_ip"),
                range: format!("{}-{}", start_port, end_port),
            }
        })
        .collect();

    Ok(Json(InstanceItem {
        id: row.get("id"),
        node_id,
        plan_id: row.get("plan_id"),
        status: row.get("status"),
        os_template: row.get("os_template"),
        auto_renew: row.get::<i64, _>("auto_renew") != 0,
        root_password: row.get("root_password"),
        created_at: row.get("created_at"),
        nat_info,
    }))
}

pub async fn list_nat_mappings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(instance_id): Path<String>,
) -> Result<Json<Vec<NatMappingItem>>, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    // Verify ownership
    let _ =
        sqlx::query_scalar::<_, String>("SELECT id FROM instances WHERE id = ? AND user_id = ?")
            .bind(&instance_id)
            .bind(&user.id)
            .fetch_optional(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
            .ok_or((StatusCode::FORBIDDEN, "access denied"))?;

    let rows = sqlx::query_as::<_, (String, i64, i64, String, String)>(
        "SELECT id, internal_port, external_port, protocol, created_at FROM nat_mappings WHERE instance_id = ? ORDER BY created_at DESC",
    )
    .bind(&instance_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to load mappings"))?;

    let items = rows
        .into_iter()
        .map(|r| NatMappingItem {
            id: r.0,
            internal_port: r.1 as i32,
            external_port: r.2 as i32,
            protocol: r.3,
            created_at: r.4,
        })
        .collect();

    Ok(Json(items))
}

pub async fn add_nat_mapping(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(instance_id): Path<String>,
    Json(payload): Json<CreateNatMappingRequest>,
) -> Result<Json<NatMappingItem>, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    // Verify ownership and get node details
    let instance = sqlx::query_as::<_, (String, String, String, Option<String>, i64)>(
        "SELECT i.id, i.node_id, n.api_endpoint, n.api_token, p.nat_port_limit
         FROM instances i
         JOIN nodes n ON i.node_id = n.id
         JOIN nat_plans p ON i.plan_id = p.id
         WHERE i.id = ? AND i.user_id = ? LIMIT 1",
    )
    .bind(&instance_id)
    .bind(&user.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        error!(error = %e, "db error");
        (StatusCode::INTERNAL_SERVER_ERROR, "db error")
    })?
    .ok_or((StatusCode::FORBIDDEN, "access denied"))?;

    let node_id = instance.1;
    let nat_limit = instance.4;

    // Check mapping count
    let count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM nat_mappings WHERE instance_id = ?")
            .bind(&instance_id)
            .fetch_one(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    if count >= nat_limit {
        return Err((StatusCode::BAD_REQUEST, "NAT mapping limit reached"));
    }

    // Check if external port is in pool for this node
    let pool_match = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM nat_port_leases WHERE node_id = ? AND ? BETWEEN start_port AND end_port"
    )
    .bind(&node_id)
    .bind(payload.external_port)
    .fetch_one(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    if pool_match == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "External port is not in the allowed pool for this node",
        ));
    }

    // Check if external port is already used
    let used = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM nat_mappings m JOIN instances i ON m.instance_id = i.id WHERE i.node_id = ? AND m.external_port = ? AND m.protocol = ?"
    )
    .bind(&node_id)
    .bind(payload.external_port)
    .bind(&payload.protocol)
    .fetch_one(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    if used > 0 {
        return Err((StatusCode::CONFLICT, "External port is already in use"));
    }

    // Get Public IP for NAT
    let public_ip = sqlx::query_scalar::<_, String>(
        "SELECT public_ip FROM nat_port_leases WHERE node_id = ? AND ? BETWEEN start_port AND end_port LIMIT 1"
    )
    .bind(&node_id)
    .bind(payload.external_port)
    .fetch_one(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to find public ip"))?;

    // Apply via provider
    let node_conn = NodeConnection {
        endpoint: instance.2,
        token: instance.3,
    };
    let provider =
        IncusProvider::new().map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "provider error"))?;

    // Get provider instance id
    let provider_instance_id =
        sqlx::query_scalar::<_, String>("SELECT provider_instance_id FROM instances WHERE id = ?")
            .bind(&instance_id)
            .fetch_one(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    provider
        .add_nat_mapping(
            &node_conn,
            &provider_instance_id,
            &public_ip,
            payload.internal_port,
            payload.external_port,
            &payload.protocol,
        )
        .await
        .map_err(|e| {
            error!(
                error = %e,
                user_id = %user.id,
                instance_id = %instance_id,
                internal_port = payload.internal_port,
                external_port = payload.external_port,
                protocol = %payload.protocol,
                "failed to add nat mapping via provider"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to apply NAT mapping on node",
            )
        })?;

    // Store in DB
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO nat_mappings (id, instance_id, internal_port, external_port, protocol) VALUES (?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(&instance_id)
        .bind(payload.internal_port)
        .bind(payload.external_port)
        .bind(&payload.protocol)
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    let created_at =
        sqlx::query_scalar::<_, String>("SELECT created_at FROM nat_mappings WHERE id = ?")
            .bind(&id)
            .fetch_one(&state.db)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    info!(
        user_id = %user.id,
        instance_id = %instance_id,
        internal_port = payload.internal_port,
        external_port = payload.external_port,
        protocol = %payload.protocol,
        "nat mapping added"
    );

    Ok(Json(NatMappingItem {
        id,
        internal_port: payload.internal_port,
        external_port: payload.external_port,
        protocol: payload.protocol,
        created_at,
    }))
}

pub async fn remove_nat_mapping(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((instance_id, mapping_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    // Verify ownership and get details
    let mapping = sqlx::query_as::<_, (i64, String, String, Option<String>, Option<String>)>(
        "SELECT m.external_port, m.protocol, n.api_endpoint, n.api_token, i.provider_instance_id
         FROM nat_mappings m
         JOIN instances i ON m.instance_id = i.id
         JOIN nodes n ON i.node_id = n.id
         WHERE m.id = ? AND i.id = ? AND i.user_id = ? LIMIT 1",
    )
    .bind(&mapping_id)
    .bind(&instance_id)
    .bind(&user.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
    .ok_or((StatusCode::FORBIDDEN, "access denied"))?;

    let provider_instance_id = mapping
        .4
        .ok_or((StatusCode::BAD_REQUEST, "instance not provisioned"))?;
    let node_conn = NodeConnection {
        endpoint: mapping.2,
        token: mapping.3,
    };

    // Remove via provider
    let provider =
        IncusProvider::new().map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "provider error"))?;

    if let Err(e) = provider
        .remove_nat_mapping(
            &node_conn,
            &provider_instance_id,
            mapping.0 as i32,
            &mapping.1,
        )
        .await
    {
        error!(
            error = %e,
            user_id = %user.id,
            instance_id = %instance_id,
            external_port = mapping.0,
            protocol = %mapping.1,
            "failed to remove NAT mapping on node"
        );
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to remove NAT mapping on node",
        ));
    }

    // Delete from DB
    sqlx::query("DELETE FROM nat_mappings WHERE id = ?")
        .bind(&mapping_id)
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    info!(
        user_id = %user.id,
        instance_id = %instance_id,
        external_port = mapping.0,
        protocol = %mapping.1,
        "nat mapping removed"
    );

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct ActionResponse {
    pub message: String,
    pub new_password: Option<String>,
}

pub fn generate_strong_password(length: usize) -> String {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    let mut rng = thread_rng();
    let special_chars = b"!@#$%^&*()-_=+[]{}<>?";

    loop {
        let pwd: Vec<u8> = (0..length)
            .map(|_| {
                if rng.gen_bool(0.2) {
                    special_chars[rng.gen_range(0..special_chars.len())]
                } else {
                    rng.sample(Alphanumeric)
                }
            })
            .collect();

        // Validate strength
        let has_upper = pwd.iter().any(|&c| (c as char).is_uppercase());
        let has_lower = pwd.iter().any(|&c| (c as char).is_lowercase());
        let has_digit = pwd.iter().any(|&c| (c as char).is_numeric());
        let has_special = pwd.iter().any(|&c| special_chars.contains(&c));

        if has_upper && has_lower && has_digit && has_special {
            return String::from_utf8(pwd).unwrap();
        }
    }
}

pub async fn perform_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<ActionRequest>,
) -> Result<Json<ActionResponse>, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    let row = sqlx::query_as::<_, (String, Option<String>, String, Option<String>)>(
        "SELECT i.id, i.provider_instance_id, n.api_endpoint, n.api_token
            FROM instances i
            JOIN nodes n ON i.node_id = n.id
         WHERE i.id = ? AND i.user_id = ? LIMIT 1",
    )
    .bind(&id)
    .bind(&user.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, instance_id = %id, "failed to query instance for action");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to process action",
        )
    })?
    .ok_or((StatusCode::NOT_FOUND, "instance not found"))?;

    let provider_instance_id = row
        .1
        .ok_or((StatusCode::BAD_REQUEST, "instance is not provisioned"))?;
    let node_conn = NodeConnection {
        endpoint: row.2,
        token: row.3,
    };

    let provider = IncusProvider::new().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to initialize provider",
        )
    })?;

    let mut response_password = None;
    let action_str = match &payload.action {
        InstanceAction::Start => "start",
        InstanceAction::Stop => "stop",
        InstanceAction::Restart => "restart",
        InstanceAction::ResetPassword { .. } => "reset_password",
        InstanceAction::Reinstall { .. } => "reinstall",
    };

    let result = match payload.action {
        InstanceAction::Start => {
            let res = provider
                .start_instance(&node_conn, &provider_instance_id)
                .await;
            if res.is_ok() {
                let _ = update_status(&state, &id, InstanceStatus::Starting).await;
            }
            res.map_err(|e| e.to_string())
        }
        InstanceAction::Stop => {
            let res = provider
                .stop_instance(&node_conn, &provider_instance_id)
                .await;
            if res.is_ok() {
                let _ = update_status(&state, &id, InstanceStatus::Stopped).await;
            }
            res.map_err(|e| e.to_string())
        }
        InstanceAction::Restart => {
            let res = provider
                .restart_instance(&node_conn, &provider_instance_id)
                .await;
            if res.is_ok() {
                let _ = update_status(&state, &id, InstanceStatus::Starting).await;
            }
            res.map_err(|e| e.to_string())
        }
        InstanceAction::ResetPassword { new_password } => {
            let pwd = new_password.unwrap_or_else(|| generate_strong_password(11));
            let res = provider
                .reset_password(&node_conn, &provider_instance_id, &pwd)
                .await;

            if res.is_ok() {
                // Update password in DB
                let _ = sqlx::query("UPDATE instances SET root_password = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(&pwd)
                    .bind(&id)
                    .execute(&state.db)
                    .await;
                response_password = Some(pwd);
                Ok(())
            } else {
                res.map_err(|e| e.to_string())
            }
        }
        InstanceAction::Reinstall { os_template } => {
            let template = os_template.unwrap_or_else(|| DEFAULT_OS_TEMPLATE.to_string());
            let res = provider
                .reinstall_instance(&node_conn, &provider_instance_id, &template)
                .await;
            if res.is_ok() {
                let _ = update_status(&state, &id, InstanceStatus::Pending).await;
            }
            res.map_err(|e| e.to_string())
        }
    };

    match result {
        Ok(_) => {
            info!(
                user_id = %user.id,
                resource_type = "instance",
                resource_id = %id,
                action = %action_str,
                status = "success",
                "action performed"
            );
            Ok(Json(ActionResponse {
                message: "Action accepted".to_string(),
                new_password: response_password,
            }))
        }
        Err(err) => {
            error!(
                user_id = %user.id,
                resource_type = "instance",
                resource_id = %id,
                action = %action_str,
                status = "failed",
                error = %err,
                "action failed"
            );
            Err((StatusCode::INTERNAL_SERVER_ERROR, "action failed"))
        }
    }
}

pub async fn get_metrics(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<provider_adapter::InstanceMetrics>, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    let row = sqlx::query_as::<_, (String, Option<String>, String, Option<String>)>(
        "SELECT i.id, i.provider_instance_id, n.api_endpoint, n.api_token
            FROM instances i
            JOIN nodes n ON i.node_id = n.id
         WHERE i.id = ? AND i.user_id = ? LIMIT 1",
    )
    .bind(&id)
    .bind(&user.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, instance_id = %id, "failed to query instance for metrics");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load metrics")
    })?
    .ok_or((StatusCode::NOT_FOUND, "instance not found"))?;

    let provider_instance_id = row
        .1
        .ok_or((StatusCode::BAD_REQUEST, "instance is not provisioned"))?;
    let node_conn = NodeConnection {
        endpoint: row.2,
        token: row.3,
    };

    let provider = IncusProvider::new().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to initialize provider",
        )
    })?;
    let metrics: provider_adapter::InstanceMetrics = provider
        .get_metrics(&node_conn, &provider_instance_id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to get metrics"))?;

    Ok(Json(metrics))
}

pub async fn get_console(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<provider_adapter::ConsoleToken>, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    let row = sqlx::query_as::<_, (String, Option<String>, String, Option<String>)>(
        "SELECT i.id, i.provider_instance_id, n.api_endpoint, n.api_token
            FROM instances i
            JOIN nodes n ON i.node_id = n.id
         WHERE i.id = ? AND i.user_id = ? LIMIT 1",
    )
    .bind(&id)
    .bind(&user.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, instance_id = %id, "failed to query instance for console");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load console")
    })?
    .ok_or((StatusCode::NOT_FOUND, "instance not found"))?;

    let provider_instance_id = row
        .1
        .ok_or((StatusCode::BAD_REQUEST, "instance is not provisioned"))?;
    let node_conn = NodeConnection {
        endpoint: row.2,
        token: row.3,
    };

    let provider = IncusProvider::new().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to initialize provider",
        )
    })?;
    let token: provider_adapter::ConsoleToken = provider
        .get_exec_token(&node_conn, &provider_instance_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to get console token",
            )
        })?;

    Ok(Json(token))
}

pub async fn update_status(
    state: &AppState,
    id: &str,
    status: InstanceStatus,
) -> Result<(), (StatusCode, &'static str)> {
    let status_str = match status {
        InstanceStatus::Pending => "pending",
        InstanceStatus::Starting => "starting",
        InstanceStatus::Running => "running",
        InstanceStatus::Stopped => "stopped",
        InstanceStatus::Suspended => "suspended",
        InstanceStatus::Deleted => "deleted",
        InstanceStatus::Unknown => "unknown",
    };

    sqlx::query("UPDATE instances SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(status_str)
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, instance_id = %id, status = %status_str, "failed to update instance status");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to update instance status")
        })?;

    Ok(())
}

#[derive(Deserialize)]
pub struct ConsoleWsQuery {
    token: String,
}

pub async fn console_ws(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ConsoleWsQuery>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    // Authenticate via query param since WS upgrades can't carry Authorization headers
    let user = auth::require_user_from_token(&query.token, &state).await?;

    // Look up instance + node
    let row = sqlx::query_as::<_, (String, Option<String>, String, Option<String>)>(
        "SELECT i.id, i.provider_instance_id, n.api_endpoint, n.api_token
            FROM instances i
            JOIN nodes n ON i.node_id = n.id
         WHERE i.id = ? AND i.user_id = ? LIMIT 1",
    )
    .bind(&id)
    .bind(&user.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, instance_id = %id, "failed to query instance for console ws");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load console")
    })?
    .ok_or((StatusCode::NOT_FOUND, "instance not found"))?;

    let provider_instance_id = row
        .1
        .ok_or((StatusCode::BAD_REQUEST, "instance is not provisioned"))?;
    let node_conn = NodeConnection {
        endpoint: row.2,
        token: row.3,
    };

    let provider = IncusProvider::new().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to initialize provider",
        )
    })?;

    // Get the console token (this creates the console session on Incus)
    let console_token = provider
        .get_exec_token(&node_conn, &provider_instance_id)
        .await
        .map_err(|err| {
            error!(error = %err, "failed to get exec token");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to get exec token",
            )
        })?;

    let incus_data_url = console_token.url.clone();
    let incus_control_url = console_token.control_url.clone();

    info!(
        instance_id = %id,
        user_id = %user.id,
        incus_url = %incus_data_url,
        "upgrading to WebSocket proxy for console"
    );

    // Upgrade the HTTP connection to WebSocket and start proxying
    Ok(ws.on_upgrade(move |browser_ws| async move {
        if let Err(err) =
            relay_console(browser_ws, &incus_data_url, &incus_control_url, provider).await
        {
            warn!(error = %err, "console WebSocket proxy ended with error");
        }
    }))
}

async fn relay_console(
    browser_ws: WebSocket,
    incus_data_url: &str,
    incus_control_url: &str,
    provider: IncusProvider,
) -> anyhow::Result<()> {
    const CONTROL_CONNECT_RETRIES: usize = 3;
    const CONTROL_CONNECT_RETRY_DELAY_MS: u64 = 200;

    info!("RELAY_CONSOLE_INVOKED: function entry");

    if incus_control_url.is_empty() {
        anyhow::bail!("RELAY_CONSOLE: missing control websocket URL");
    }

    info!("RELAY_CONSOLE: connecting to control websocket first");
    let control_ws = {
        let mut last_err: Option<anyhow::Error> = None;
        let mut connected = None;

        for attempt in 1..=CONTROL_CONNECT_RETRIES {
            match provider.open_console_ws(incus_control_url).await {
                Ok(ws) => {
                    info!(
                        attempt = attempt,
                        "RELAY_CONSOLE: control websocket connected successfully"
                    );
                    connected = Some(ws);
                    break;
                }
                Err(err) => {
                    warn!(
                        attempt = attempt,
                        error = %err,
                        control_url = %incus_control_url,
                        "RELAY_CONSOLE: control websocket connect attempt failed"
                    );
                    last_err = Some(err);
                    if attempt < CONTROL_CONNECT_RETRIES {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            CONTROL_CONNECT_RETRY_DELAY_MS,
                        ))
                        .await;
                    }
                }
            }
        }

        connected.ok_or_else(|| {
            last_err.unwrap_or_else(|| anyhow::anyhow!("control websocket connection failed"))
        })?
    };

    // Incus console operations are sensitive to websocket attach order.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    info!("RELAY_CONSOLE: connecting to data websocket after control");
    let incus_ws = provider
        .open_console_ws(incus_data_url)
        .await
        .map_err(|err| {
            anyhow::anyhow!("RELAY_CONSOLE: failed to connect data websocket after control: {err}")
        })?;
    info!("RELAY_CONSOLE: data websocket connected successfully");

    let (mut control_sink, mut control_stream) = control_ws.split();
    let (control_send_tx, mut control_send_rx) =
        tokio::sync::mpsc::unbounded_channel::<TungMessage>();

    let control_writer_task = Some(tokio::spawn(async move {
        info!("RELAY_CONSOLE: control writer task started");

        while let Some(msg) = control_send_rx.recv().await {
            if control_sink.send(msg).await.is_err() {
                warn!("RELAY_CONSOLE: control websocket send failed");
                break;
            }
        }

        info!("RELAY_CONSOLE: control writer task ended");
    }));

    let control_reader_task = Some(tokio::spawn({
        let control_send_tx = control_send_tx.clone();
        async move {
            info!("RELAY_CONSOLE: control drain task started");

            while let Some(msg) = control_stream.next().await {
                match msg {
                    Ok(TungMessage::Text(text)) => {
                        info!(response = %text, "RELAY_CONSOLE: received text from Incus control channel");
                    }
                    Ok(TungMessage::Binary(bin)) => {
                        info!(
                            len = bin.len(),
                            "RELAY_CONSOLE: received binary from Incus control channel"
                        );
                    }
                    Ok(TungMessage::Ping(payload)) => {
                        if control_send_tx.send(TungMessage::Pong(payload)).is_err() {
                            warn!("RELAY_CONSOLE: control pong queue closed");
                            break;
                        }
                    }
                    Ok(TungMessage::Close(frame)) => {
                        info!(frame = ?frame, "RELAY_CONSOLE: control channel closed by Incus");
                        break;
                    }
                    Err(err) => {
                        warn!(error = %err, "RELAY_CONSOLE: control websocket stream error");
                        break;
                    }
                    _ => {}
                }
            }

            info!("RELAY_CONSOLE: control drain task ended");
        }
    }));

    info!("relay_console: starting bidirectional relay tasks");

    let (mut browser_sink, mut browser_stream) = browser_ws.split();
    let (incus_sink, mut incus_stream) = incus_ws.split();

    let (data_send_tx, mut data_send_rx) = tokio::sync::mpsc::unbounded_channel::<TungMessage>();
    let mut incus_sink_inner = incus_sink;
    let data_writer_task = tokio::spawn(async move {
        while let Some(msg) = data_send_rx.recv().await {
            if incus_sink_inner.send(msg).await.is_err() {
                break;
            }
        }
        let _ = incus_sink_inner.close().await;
    });

    browser_sink
        .send(AxumMessage::Text(
            "__cloud_store_console_ready__".to_string().into(),
        ))
        .await
        .map_err(|err| anyhow::anyhow!("relay_console: failed to send ready event: {err}"))?;
    info!("relay_console: sent ready event to browser");

    // Task: Browser -> Incus
    let browser_to_incus = tokio::spawn({
        let control_send_tx = control_send_tx.clone();
        let data_send_tx = data_send_tx.clone();
        async move {
            let mut browser_msg_count = 0;
            while let Some(msg) = browser_stream.next().await {
                match msg {
                    Ok(AxumMessage::Binary(data)) => {
                        browser_msg_count += 1;
                        if data_send_tx.send(TungMessage::Binary(data)).is_err() {
                            break;
                        }
                    }
                    Ok(AxumMessage::Text(text)) => {
                        browser_msg_count += 1;
                        let text = text.to_string();

                        match serde_json::from_str::<BrowserConsoleMessage>(&text) {
                            Ok(BrowserConsoleMessage::Resize { rows, cols }) => {
                                let Some((rows, cols)) = validate_resize(rows, cols) else {
                                    continue;
                                };

                                let control_message = TungMessage::Text(
                                    serde_json::json!({
                                        "command": "window-resize",
                                        "args": {
                                            "width": cols.to_string(),
                                            "height": rows.to_string(),
                                        }
                                    })
                                    .to_string()
                                    .into(),
                                );

                                if control_send_tx.send(control_message).is_err() {
                                    break;
                                }
                                continue;
                            }
                            Err(_) => {
                                // For VNC, text messages might actually be intended as data
                                if data_send_tx
                                    .send(TungMessage::Binary(text.into_bytes().into()))
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        }
                    }
                    Ok(AxumMessage::Ping(payload)) => {
                        if data_send_tx.send(TungMessage::Ping(payload)).is_err() {
                            break;
                        }
                    }
                    Ok(AxumMessage::Close(_)) => break,
                    Err(_) => break,
                    _ => {}
                }
            }
            info!(
                "relay_console: browser to incus task ended ({} msgs)",
                browser_msg_count
            );
        }
    });

    // Task: Incus -> Browser
    let incus_to_browser = tokio::spawn({
        let data_send_tx = data_send_tx.clone();
        async move {
            let mut incus_msg_count = 0;
            while let Some(msg) = incus_stream.next().await {
                match msg {
                    Ok(TungMessage::Binary(data)) => {
                        incus_msg_count += 1;
                        if browser_sink.send(AxumMessage::Binary(data)).await.is_err() {
                            break;
                        }
                    }
                    Ok(TungMessage::Text(text)) => {
                        incus_msg_count += 1;
                        // Incus occasionally sends state via text; forward as binary to browser
                        if browser_sink
                            .send(AxumMessage::Binary(text.as_bytes().to_vec().into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Ok(TungMessage::Ping(payload)) => {
                        // Answer pings from Incus directly
                        if data_send_tx.send(TungMessage::Pong(payload)).is_err() {
                            break;
                        }
                    }
                    Ok(TungMessage::Close(_)) => break,
                    Err(_) => break,
                    _ => {}
                }
            }
            let _ = browser_sink.close().await;
            info!(
                "relay_console: incus to browser task ended ({} msgs)",
                incus_msg_count
            );
        }
    });

    // Wait for either task to finish or fail
    tokio::select! {
        _ = browser_to_incus => {
            info!("relay_console: browser_to_incus task finished first");
        },
        _ = incus_to_browser => {
            info!("relay_console: incus_to_browser task finished first");
        },
    }

    data_writer_task.abort();
    if let Some(task) = control_reader_task {
        task.abort();
    }
    if let Some(task) = control_writer_task {
        task.abort();
    }

    info!("relay_console: bidirectional relay session ended");
    Ok(())
}
