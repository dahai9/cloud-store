use crate::auth;
use crate::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use tracing::error;

#[derive(Serialize)]
pub struct TicketItem {
    pub id: String,
    pub subject: String,
    pub category: String,
    pub priority: String,
    pub status: String,
}

pub async fn list_tickets(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TicketItem>>, (StatusCode, &'static str)> {
    let user = auth::require_auth(&headers, &state)?;

    let records = if user.role == "admin" {
        sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT id, subject, category, priority, status FROM support_tickets ORDER BY created_at DESC LIMIT 50",
        )
        .fetch_all(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, "failed to query support tickets");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load support tickets",
            )
        })?
    } else {
        sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT id, subject, category, priority, status FROM support_tickets WHERE user_id = ? ORDER BY created_at DESC LIMIT 50",
        )
        .bind(&user.id)
        .fetch_all(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, user_id = %user.id, "failed to query support tickets by user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load support tickets",
            )
        })?
    };

    let items = records
        .into_iter()
        .map(|(id, subject, category, priority, status)| TicketItem {
            id,
            subject,
            category,
            priority,
            status,
        })
        .collect();

    Ok(Json(items))
}
