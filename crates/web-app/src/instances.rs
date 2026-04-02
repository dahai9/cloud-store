use crate::auth;
use crate::AppState;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use provider_adapter::{ComputeProvider, StubProvider};
use serde::{Deserialize, Serialize};
use shared_domain::InstanceStatus;
use tracing::error;

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
        .map(|(id, node_id, plan_id, status, os_template, created_at)| InstanceItem {
            id,
            node_id,
            plan_id,
            status,
            os_template,
            created_at,
        })
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

    let instance = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT id, provider_instance_id FROM instances WHERE id = ? AND user_id = ? LIMIT 1",
    )
    .bind(&id)
    .bind(&user.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, instance_id = %id, "failed to query instance for action");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to process action")
    })?
    .ok_or((StatusCode::NOT_FOUND, "instance not found"))?;

    let provider_instance_id = instance.1.ok_or((StatusCode::BAD_REQUEST, "instance is not provisioned"))?;

    let provider = StubProvider;

    match payload.action {
        InstanceAction::Start => {
            provider.start_instance(&provider_instance_id).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to start instance"))?;
            update_status(&state, &id, InstanceStatus::Starting).await?;
        }
        InstanceAction::Stop => {
            provider.stop_instance(&provider_instance_id).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to stop instance"))?;
            update_status(&state, &id, InstanceStatus::Stopped).await?;
        }
        InstanceAction::Restart => {
            provider.restart_instance(&provider_instance_id).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to restart instance"))?;
            update_status(&state, &id, InstanceStatus::Starting).await?;
        }
        InstanceAction::ResetPassword { new_password } => {
            let pwd = new_password.unwrap_or_else(|| "RandomPassword123!".to_string());
            provider.reset_password(&provider_instance_id, &pwd).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to reset password"))?;
        }
        InstanceAction::Reinstall { os_template } => {
            let template = os_template.unwrap_or_else(|| "debian-12".to_string());
            provider.reinstall_instance(&provider_instance_id, &template).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to reinstall instance"))?;
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

    let instance = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT id, provider_instance_id FROM instances WHERE id = ? AND user_id = ? LIMIT 1",
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

    let provider_instance_id = instance.1.ok_or((StatusCode::BAD_REQUEST, "instance is not provisioned"))?;

    let provider = StubProvider;
    let metrics = provider.get_metrics(&provider_instance_id).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to get metrics"))?;

    Ok(Json(metrics))
}

pub async fn get_console(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<provider_adapter::ConsoleToken>, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    let instance = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT id, provider_instance_id FROM instances WHERE id = ? AND user_id = ? LIMIT 1",
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

    let provider_instance_id = instance.1.ok_or((StatusCode::BAD_REQUEST, "instance is not provisioned"))?;

    let provider = StubProvider;
    let token = provider.get_console_token(&provider_instance_id).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to get console token"))?;

    Ok(Json(token))
}

async fn update_status(state: &AppState, id: &str, status: InstanceStatus) -> Result<(), (StatusCode, &'static str)> {
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
