use crate::auth;
use crate::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use tracing::error;

#[derive(Serialize)]
pub struct PublicPlanItem {
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
    pub max_inventory: Option<i64>,
    pub sold_inventory: i64,
}

pub async fn list_public_plans(
    State(state): State<AppState>,
) -> Result<Json<Vec<PublicPlanItem>>, (StatusCode, &'static str)> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, i64, i64, i64, i64, i64, Option<i64>, i64)>(
        "SELECT id, code, name, CAST(monthly_price AS TEXT), memory_mb, storage_gb, cpu_cores, cpu_allowance_pct, bandwidth_mbps, traffic_gb, max_inventory, sold_inventory FROM nat_plans WHERE active = 1 ORDER BY monthly_price ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, "failed to query public plans");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to load plans",
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
                max_inventory,
                sold_inventory,
            )| {
                PublicPlanItem {
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
                    max_inventory,
                    sold_inventory,
                }
            },
        )
        .collect();

    Ok(Json(items))
}

#[derive(Serialize)]
pub struct InvoiceItem {
    pub id: String,
    pub amount: String,
    pub status: String,
    pub order_id: Option<String>,
    pub external_payment_ref: Option<String>,
    pub due_at: String,
    pub created_at: String,
    pub paid_at: Option<String>,
}

async fn expire_overdue_invoices(state: &AppState) -> Result<(), (StatusCode, &'static str)> {
    sqlx::query(
        "UPDATE invoices SET status = 'expired' WHERE status = 'open' AND datetime(due_at) <= datetime('now')",
    )
    .execute(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, "failed to expire overdue invoices");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to expire invoices")
    })?;

    Ok(())
}

pub async fn list_invoices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<InvoiceItem>>, (StatusCode, &'static str)> {
    let user = auth::require_auth(&headers, &state).await?;

    expire_overdue_invoices(&state).await?;

    let records = if user.role == "admin" {
        sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, String, String, Option<String>)>(
            "SELECT id, CAST(amount AS TEXT), status, order_id, external_payment_ref, due_at, created_at, paid_at FROM invoices ORDER BY created_at DESC LIMIT 50",
        )
        .fetch_all(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, "failed to query invoices");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to load invoices")
        })?
    } else {
        sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, String, String, Option<String>)>(
            "SELECT id, CAST(amount AS TEXT), status, order_id, external_payment_ref, due_at, created_at, paid_at FROM invoices WHERE user_id = ? ORDER BY created_at DESC LIMIT 50",
        )
        .bind(&user.id)
        .fetch_all(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, user_id = %user.id, "failed to query invoices by user");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to load invoices")
        })?
    };

    let items = records
        .into_iter()
        .map(
            |(id, amount, status, order_id, external_payment_ref, due_at, created_at, paid_at)| {
                InvoiceItem {
                    id,
                    amount,
                    status,
                    order_id,
                    external_payment_ref,
                    due_at,
                    created_at,
                    paid_at,
                }
            },
        )
        .collect();

    Ok(Json(items))
}
