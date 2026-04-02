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
    pub active: bool,
    pub max_inventory: Option<i64>,
    pub sold_inventory: i64,
}

#[derive(Deserialize)]
pub struct AdminPlanUpdateRequest {
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

pub async fn list_plans(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AdminPlanItem>>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    let rows = sqlx::query_as::<_, (String, String, String, String, i64, Option<i64>, i64)>(
        "SELECT id, code, name, CAST(monthly_price AS TEXT), active, max_inventory, sold_inventory FROM nat_plans ORDER BY created_at DESC",
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
            |(id, code, name, monthly_price, active, max_inventory, sold_inventory)| {
                AdminPlanItem {
                    id,
                    code,
                    name,
                    monthly_price,
                    active: active != 0,
                    max_inventory,
                    sold_inventory,
                }
            },
        )
        .collect();

    Ok(Json(items))
}

pub async fn update_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(plan_id): Path<String>,
    Json(payload): Json<AdminPlanUpdateRequest>,
) -> Result<Json<AdminPlanItem>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    let current = sqlx::query_as::<_, (String, String, String, String, i64, Option<i64>, i64)>(
        "SELECT id, code, name, CAST(monthly_price AS TEXT), active, max_inventory, sold_inventory FROM nat_plans WHERE id = ? LIMIT 1",
    )
    .bind(&plan_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, plan_id = %plan_id, "failed to query target plan");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load target plan")
    })?
    .ok_or((StatusCode::NOT_FOUND, "plan not found"))?;

    let next_active = payload.active.unwrap_or(current.4 != 0);
    let next_max_inventory = payload.max_inventory.or(current.5);

    if let Some(limit) = next_max_inventory {
        if limit < current.6 {
            return Err((
                StatusCode::BAD_REQUEST,
                "max_inventory cannot be lower than sold_inventory",
            ));
        }
    }

    sqlx::query("UPDATE nat_plans SET active = ?, max_inventory = ? WHERE id = ?")
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
        code: current.1,
        name: current.2,
        monthly_price: current.3,
        active: next_active,
        max_inventory: next_max_inventory,
        sold_inventory: current.6,
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
