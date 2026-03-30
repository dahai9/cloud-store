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
    pub due_at: String,
}

pub async fn list_invoices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<InvoiceItem>>, (StatusCode, &'static str)> {
    let user = auth::require_auth(&headers, &state)?;

    let records = if user.role == "admin" {
        sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT id, CAST(amount AS TEXT), status, due_at FROM invoices ORDER BY created_at DESC LIMIT 50",
        )
        .fetch_all(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, "failed to query invoices");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to load invoices")
        })?
    } else {
        sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT id, CAST(amount AS TEXT), status, due_at FROM invoices WHERE user_id = ? ORDER BY created_at DESC LIMIT 50",
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
        .map(|(id, amount, status, due_at)| InvoiceItem {
            id,
            amount,
            status,
            due_at,
        })
        .collect();

    Ok(Json(items))
}
