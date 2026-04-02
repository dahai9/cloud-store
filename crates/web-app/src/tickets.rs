use crate::auth;
use crate::AppState;
use axum::extract::Path;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use tracing::error;
use uuid::Uuid;

#[derive(Serialize)]
pub struct TicketItem {
    pub id: String,
    pub subject: String,
    pub category: String,
    pub priority: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct TicketMessageItem {
    pub id: String,
    pub sender_user_id: Option<String>,
    pub message: String,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct TicketStatusUpdateRequest {
    pub status: String,
}

#[derive(Deserialize)]
pub struct TicketReplyRequest {
    pub message: String,
}

pub async fn list_tickets(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TicketItem>>, (StatusCode, &'static str)> {
    let user = auth::require_auth(&headers, &state).await?;

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

pub async fn admin_update_ticket_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ticket_id): Path<String>,
    Json(payload): Json<TicketStatusUpdateRequest>,
) -> Result<Json<TicketItem>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    if !is_valid_ticket_status(payload.status.as_str()) {
        return Err((StatusCode::BAD_REQUEST, "invalid ticket status"));
    }

    let found = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM support_tickets WHERE id = ?")
        .bind(&ticket_id)
        .fetch_one(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, ticket_id = %ticket_id, "failed to check support ticket");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to update support ticket",
            )
        })?;

    if found == 0 {
        return Err((StatusCode::NOT_FOUND, "ticket not found"));
    }

    sqlx::query(
        "UPDATE support_tickets SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(payload.status.trim().to_lowercase())
    .bind(&ticket_id)
    .execute(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, ticket_id = %ticket_id, "failed to update support ticket status");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to update support ticket",
        )
    })?;

    let item = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT id, subject, category, priority, status FROM support_tickets WHERE id = ? LIMIT 1",
    )
    .bind(&ticket_id)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, ticket_id = %ticket_id, "failed to reload support ticket");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to load support ticket",
        )
    })?;

    Ok(Json(TicketItem {
        id: item.0,
        subject: item.1,
        category: item.2,
        priority: item.3,
        status: item.4,
    }))
}

pub async fn admin_list_ticket_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ticket_id): Path<String>,
) -> Result<Json<Vec<TicketMessageItem>>, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM support_tickets WHERE id = ?")
        .bind(&ticket_id)
        .fetch_one(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, ticket_id = %ticket_id, "failed to check ticket existence");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load ticket messages",
            )
        })?;

    if exists == 0 {
        return Err((StatusCode::NOT_FOUND, "ticket not found"));
    }

    let rows = sqlx::query_as::<_, (String, Option<String>, String, String)>(
        "SELECT id, sender_user_id, message, created_at FROM support_messages WHERE ticket_id = ? ORDER BY created_at ASC",
    )
    .bind(&ticket_id)
    .fetch_all(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, ticket_id = %ticket_id, "failed to query support messages");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to load ticket messages",
        )
    })?;

    let items = rows
        .into_iter()
        .map(
            |(id, sender_user_id, message, created_at)| TicketMessageItem {
                id,
                sender_user_id,
                message,
                created_at,
            },
        )
        .collect();

    Ok(Json(items))
}

pub async fn admin_reply_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ticket_id): Path<String>,
    Json(payload): Json<TicketReplyRequest>,
) -> Result<Json<TicketMessageItem>, (StatusCode, &'static str)> {
    let admin = auth::require_admin(&headers, &state).await?;

    if payload.message.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "message is required"));
    }

    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM support_tickets WHERE id = ?")
        .bind(&ticket_id)
        .fetch_one(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, ticket_id = %ticket_id, "failed to check ticket existence");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to reply ticket")
        })?;

    if exists == 0 {
        return Err((StatusCode::NOT_FOUND, "ticket not found"));
    }

    let message_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO support_messages (id, ticket_id, sender_user_id, message) VALUES (?, ?, ?, ?)",
    )
    .bind(&message_id)
    .bind(&ticket_id)
    .bind(&admin.id)
    .bind(payload.message.trim())
    .execute(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, ticket_id = %ticket_id, admin_id = %admin.id, "failed to insert support message");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to reply ticket")
    })?;

    sqlx::query("UPDATE support_tickets SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&ticket_id)
        .execute(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, ticket_id = %ticket_id, "failed to touch support ticket");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to reply ticket")
        })?;

    let created = sqlx::query_as::<_, (String, Option<String>, String, String)>(
        "SELECT id, sender_user_id, message, created_at FROM support_messages WHERE id = ? LIMIT 1",
    )
    .bind(&message_id)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, message_id = %message_id, "failed to load created support message");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to reply ticket")
    })?;

    Ok(Json(TicketMessageItem {
        id: created.0,
        sender_user_id: created.1,
        message: created.2,
        created_at: created.3,
    }))
}

fn is_valid_ticket_status(status: &str) -> bool {
    matches!(
        status.trim().to_lowercase().as_str(),
        "open" | "in_progress" | "resolved" | "closed"
    )
}
