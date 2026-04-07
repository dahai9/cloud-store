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

#[derive(Serialize)]
pub struct UserBalanceInfo {
    pub balance: String,
}

#[derive(Serialize)]
pub struct BalanceTransactionItem {
    pub id: String,
    pub amount: String,
    pub r#type: String,
    pub description: String,
    pub created_at: String,
}

pub async fn get_balance(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<UserBalanceInfo>, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    let balance = sqlx::query_scalar::<_, String>("SELECT CAST(balance AS TEXT) FROM users WHERE id = ?")
        .bind(&user.id)
        .fetch_one(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, user_id = %user.id, "failed to query user balance");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to load balance")
        })?;

    Ok(Json(UserBalanceInfo { balance }))
}

pub async fn list_balance_transactions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<BalanceTransactionItem>>, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT id, CAST(amount AS TEXT), type, description, created_at FROM balance_transactions WHERE user_id = ? ORDER BY created_at DESC LIMIT 100",
    )
    .bind(&user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, user_id = %user.id, "failed to query balance transactions");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load transactions")
    })?;

    let items = rows
        .into_iter()
        .map(|(id, amount, r#type, description, created_at)| BalanceTransactionItem {
            id,
            amount,
            r#type,
            description,
            created_at,
        })
        .collect();

    Ok(Json(items))
}

pub async fn recharge_balance(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<UserBalanceInfo>, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    // Mock recharge: add $100
    let amount = 100.00;
    let description = "Mock recharge (manual trigger)".to_string();
    let tx_id = uuid::Uuid::new_v4().to_string();

    let mut tx = state.db.begin().await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    sqlx::query("UPDATE users SET balance = balance + ? WHERE id = ?")
        .bind(amount)
        .bind(&user.id)
        .execute(&mut *tx)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to update balance"))?;

    sqlx::query(
        "INSERT INTO balance_transactions (id, user_id, amount, type, description) VALUES (?, ?, ?, 'recharge', ?)"
    )
    .bind(&tx_id)
    .bind(&user.id)
    .bind(amount)
    .bind(&description)
    .execute(&mut *tx)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to create transaction"))?;

    tx.commit().await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to commit"))?;

    let new_balance = sqlx::query_scalar::<_, String>("SELECT CAST(balance AS TEXT) FROM users WHERE id = ?")
        .bind(&user.id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    Ok(Json(UserBalanceInfo { balance: new_balance }))
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
