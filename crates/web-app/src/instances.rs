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
use tokio_tungstenite::tungstenite::Message as TungMessage;
use tracing::{debug, error, info, warn};

#[derive(Serialize)]
pub struct InstanceItem {
    pub id: String,
    pub node_id: String,
    pub plan_id: String,
    pub status: String,
    pub os_template: String,
    pub created_at: String,
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

pub async fn list_instances(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<InstanceItem>>, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    let rows = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT id, node_id, plan_id, status, os_template, created_at FROM instances WHERE user_id = ? ORDER BY created_at DESC",
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
            |(id, node_id, plan_id, status, os_template, created_at)| InstanceItem {
                id,
                node_id,
                plan_id,
                status,
                os_template,
                created_at,
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

    let row = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT id, node_id, plan_id, status, os_template, created_at FROM instances WHERE id = ? AND user_id = ? LIMIT 1",
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

    Ok(Json(InstanceItem {
        id: row.0,
        node_id: row.1,
        plan_id: row.2,
        status: row.3,
        os_template: row.4,
        created_at: row.5,
    }))
}

pub async fn perform_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<ActionRequest>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
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

    match payload.action {
        InstanceAction::Start => {
            provider
                .start_instance(&node_conn, &provider_instance_id)
                .await
                .map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "failed to start instance",
                    )
                })?;
            update_status(&state, &id, InstanceStatus::Starting).await?;
        }
        InstanceAction::Stop => {
            provider
                .stop_instance(&node_conn, &provider_instance_id)
                .await
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to stop instance"))?;
            update_status(&state, &id, InstanceStatus::Stopped).await?;
        }
        InstanceAction::Restart => {
            provider
                .restart_instance(&node_conn, &provider_instance_id)
                .await
                .map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "failed to restart instance",
                    )
                })?;
            update_status(&state, &id, InstanceStatus::Starting).await?;
        }
        InstanceAction::ResetPassword { new_password } => {
            let pwd = new_password.unwrap_or_else(|| "RandomPassword123!".to_string());
            provider
                .reset_password(&node_conn, &provider_instance_id, &pwd)
                .await
                .map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "failed to reset password",
                    )
                })?;
        }
        InstanceAction::Reinstall { os_template } => {
            let template = os_template.unwrap_or_else(|| DEFAULT_OS_TEMPLATE.to_string());
            provider
                .reinstall_instance(&node_conn, &provider_instance_id, &template)
                .await
                .map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "failed to reinstall instance",
                    )
                })?;
            update_status(&state, &id, InstanceStatus::Pending).await?;
        }
    }

    Ok(StatusCode::ACCEPTED)
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
        .get_console_token(&node_conn, &provider_instance_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to get console token",
            )
        })?;

    Ok(Json(token))
}

async fn update_status(
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
        .get_console_token(&node_conn, &provider_instance_id)
        .await
        .map_err(|err| {
            error!(error = %err, "failed to get console token");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to get console token",
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
    let mut control_ws = {
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

    let control_task = Some(tokio::spawn(async move {
        info!("RELAY_CONSOLE: control drain task started");

        while let Some(msg) = control_ws.next().await {
            match msg {
                Ok(TungMessage::Ping(payload)) => {
                    if let Err(err) = control_ws.send(TungMessage::Pong(payload)).await {
                        warn!(error = %err, "RELAY_CONSOLE: control websocket pong failed");
                        break;
                    }
                }
                Ok(TungMessage::Close(_)) => break,
                Err(err) => {
                    warn!(error = %err, "RELAY_CONSOLE: control websocket stream error");
                    break;
                }
                _ => {}
            }
        }
        info!("RELAY_CONSOLE: control drain task ended");
    }));

    info!("relay_console: proceeding to split websockets");

    info!("relay_console: about to split browser websocket");
    let (mut browser_sink, mut browser_stream) = browser_ws.split();
    info!("relay_console: browser websocket split successfully");

    info!("relay_console: about to split incus websocket");
    let (mut incus_sink, mut incus_stream) = incus_ws.split();
    info!("relay_console: incus websocket split successfully, sinks and streams created");

    browser_sink
        .send(AxumMessage::Text(
            "__cloud_store_console_ready__".to_string().into(),
        ))
        .await
        .map_err(|err| anyhow::anyhow!("relay_console: failed to send ready event: {err}"))?;
    info!("relay_console: sent ready event to browser");

    info!("relay_console: starting combined relay loop");

    let mut browser_msg_count = 0;
    let mut incus_msg_count = 0;
    loop {
        tokio::select! {
            msg = browser_stream.next() => {
                match msg {
                    Some(Ok(AxumMessage::Binary(data))) => {
                        browser_msg_count += 1;
                        debug!("relay_console: browser→incus received message #{}", browser_msg_count);
                        debug!("relay_console: browser→incus #{}: binary {} bytes", browser_msg_count, data.len());
                        if incus_sink.send(TungMessage::Binary(data.into())).await.is_err() {
                            info!("relay_console: browser→incus: incus sink closed (msg #{})", browser_msg_count);
                            break;
                        }
                    }
                    Some(Ok(AxumMessage::Text(text))) => {
                        browser_msg_count += 1;
                        debug!("relay_console: browser→incus received message #{}", browser_msg_count);
                        debug!("relay_console: browser→incus #{}: text", browser_msg_count);
                        if incus_sink.send(TungMessage::Binary(text.as_bytes().to_vec().into())).await.is_err() {
                            info!("relay_console: browser→incus: incus sink closed");
                            break;
                        }
                    }
                    Some(Ok(AxumMessage::Close(_))) => {
                        browser_msg_count += 1;
                        info!("relay_console: browser→incus: browser close frame received (after {} msgs)", browser_msg_count);
                        break;
                    }
                    Some(Ok(AxumMessage::Ping(_))) => {
                        browser_msg_count += 1;
                        debug!("relay_console: browser→incus #{}: ping", browser_msg_count);
                    }
                    Some(Ok(AxumMessage::Pong(_))) => {
                        browser_msg_count += 1;
                        debug!("relay_console: browser→incus #{}: pong", browser_msg_count);
                    }
                    Some(Err(err)) => {
                        browser_msg_count += 1;
                        info!("relay_console: browser→incus: browser error: {} (after {} msgs)", err, browser_msg_count);
                        break;
                    }
                    None => {
                        info!("relay_console: browser→incus stream ended");
                        break;
                    }
                }
            }
            msg = incus_stream.next() => {
                match msg {
                    Some(Ok(TungMessage::Binary(data))) => {
                        incus_msg_count += 1;
                        debug!("relay_console: incus→browser received message #{}", incus_msg_count);
                        debug!("relay_console: incus→browser #{}: binary {} bytes", incus_msg_count, data.len());
                        if browser_sink.send(AxumMessage::Binary(data.into())).await.is_err() {
                            info!("relay_console: incus→browser: browser sink closed (msg #{})", incus_msg_count);
                            break;
                        }
                    }
                    Some(Ok(TungMessage::Text(text))) => {
                        incus_msg_count += 1;
                        debug!("relay_console: incus→browser received message #{}", incus_msg_count);
                        debug!("relay_console: incus→browser #{}: text", incus_msg_count);
                        if browser_sink.send(AxumMessage::Binary(text.as_bytes().to_vec().into())).await.is_err() {
                            info!("relay_console: incus→browser: browser sink closed");
                            break;
                        }
                    }
                    Some(Ok(TungMessage::Ping(payload))) => {
                        incus_msg_count += 1;
                        debug!("relay_console: incus→browser received message #{}", incus_msg_count);
                        debug!("relay_console: incus→browser #{}: ping {} bytes", incus_msg_count, payload.len());
                        if incus_sink.send(TungMessage::Pong(payload)).await.is_err() {
                            info!("relay_console: incus→browser: incus pong send failed");
                            break;
                        }
                    }
                    Some(Ok(TungMessage::Pong(_))) => {
                        incus_msg_count += 1;
                        debug!("relay_console: incus→browser #{}: pong", incus_msg_count);
                    }
                    Some(Ok(TungMessage::Frame(_))) => {
                        debug!("relay_console: incus→browser #{}: frame", incus_msg_count);
                    }
                    Some(Ok(TungMessage::Close(_))) => {
                        incus_msg_count += 1;
                        info!("relay_console: incus→browser: incus close frame received (after {} msgs)", incus_msg_count);
                        break;
                    }
                    Some(Err(err)) => {
                        incus_msg_count += 1;
                        info!("relay_console: incus→browser: incus error: {} (after {} msgs)", err, incus_msg_count);
                        break;
                    }
                    None => {
                        info!("relay_console: incus→browser stream ended");
                        break;
                    }
                }
            }
        }
    }

    info!("relay_console: relay session ended");

    if let Some(task) = control_task {
        task.abort();
    }

    Ok(())
}
