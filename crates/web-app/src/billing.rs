use crate::auth;
use crate::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use tracing::error;

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
    let user = auth::require_auth(&headers, &state)?;

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
