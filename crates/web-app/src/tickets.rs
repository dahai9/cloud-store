use crate::auth;
use crate::AppState;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use async_stream::stream;
use futures_util::stream::Stream;
use std::convert::Infallible;
use serde::Deserialize;
use serde::Serialize;
use tracing::error;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SseQuery {
    pub token: String,
}

#[derive(Serialize)]
pub struct TicketItem {
    pub id: String,
    pub user_id: String,
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

#[derive(Deserialize)]
pub struct CreateTicketRequest {
    pub subject: String,
    pub category: String,
    pub priority: String,
    pub message: String,
}

pub async fn create_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateTicketRequest>,
) -> Result<Json<TicketItem>, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    if payload.subject.trim().is_empty() || payload.message.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "subject and message are required"));
    }

    let ticket_id = Uuid::new_v4().to_string();
    let message_id = Uuid::new_v4().to_string();

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    sqlx::query(
        "INSERT INTO support_tickets (id, user_id, category, priority, subject, status) VALUES (?, ?, ?, ?, ?, 'open')",
    )
    .bind(&ticket_id)
    .bind(&user.id)
    .bind(payload.category.trim())
    .bind(payload.priority.trim())
    .bind(payload.subject.trim())
    .execute(&mut *tx)
    .await
    .map_err(|err| {
        error!(error = %err, user_id = %user.id, "failed to create support ticket");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to create ticket")
    })?;

    sqlx::query(
        "INSERT INTO support_messages (id, ticket_id, sender_user_id, message) VALUES (?, ?, ?, ?)",
    )
    .bind(&message_id)
    .bind(&ticket_id)
    .bind(&user.id)
    .bind(payload.message.trim())
    .execute(&mut *tx)
    .await
    .map_err(|err| {
        error!(error = %err, ticket_id = %ticket_id, user_id = %user.id, "failed to create initial ticket message");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to create ticket")
    })?;

    tx.commit()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to commit transaction"))?;

    Ok(Json(TicketItem {
        id: ticket_id,
        user_id: user.id,
        subject: payload.subject,
        category: payload.category,
        priority: payload.priority,
        status: "open".to_string(),
    }))
}

pub async fn list_tickets(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TicketItem>>, (StatusCode, &'static str)> {
    let user = auth::require_auth(&headers, &state).await?;

    let records = if user.role == "admin" {
        sqlx::query_as::<_, (String, String, String, String, String, String)>(
            "SELECT id, user_id, subject, category, priority, status FROM support_tickets ORDER BY updated_at DESC LIMIT 50",
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
        sqlx::query_as::<_, (String, String, String, String, String, String)>(
            "SELECT id, user_id, subject, category, priority, status FROM support_tickets WHERE user_id = ? ORDER BY updated_at DESC LIMIT 50",
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
        .map(|(id, user_id, subject, category, priority, status)| TicketItem {
            id,
            user_id,
            subject,
            category,
            priority,
            status,
        })
        .collect();

    Ok(Json(items))
}

pub async fn ticket_messages_stream(
    State(state): State<AppState>,
    Path(ticket_id): Path<String>,
    Query(query): Query<SseQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, &'static str)> {
    let user = auth::require_user_from_token(&query.token, &state).await?;

    let ticket_owner_id = sqlx::query_scalar::<_, String>(
        "SELECT user_id FROM support_tickets WHERE id = ?"
    )
    .bind(&ticket_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
    .ok_or((StatusCode::NOT_FOUND, "ticket not found"))?;

    if ticket_owner_id != user.id {
        return Err((StatusCode::FORBIDDEN, "access denied"));
    }

    let db = state.db.clone();
    let s = stream! {
        let mut last_created_at = "1970-01-01 00:00:00".to_string();
        let mut last_status = String::new();

        loop {
            // Check for new messages
            let q = "SELECT id, sender_user_id, message, created_at FROM support_messages WHERE ticket_id = ? AND created_at > ? ORDER BY created_at ASC";
            let rows = sqlx::query_as::<_, (String, Option<String>, String, String)>(q)
                .bind(&ticket_id)
                .bind(&last_created_at)
                .fetch_all(&db)
                .await;

            if let Ok(rows) = rows {
                for (id, sender_user_id, message, created_at) in rows {
                    let item = TicketMessageItem {
                        id,
                        sender_user_id,
                        message,
                        created_at: created_at.clone(),
                    };
                    
                    last_created_at = created_at;

                    if let Ok(json) = serde_json::to_string(&item) {
                        yield Ok(Event::default().event("message").data(json));
                    }
                }
            }

            // Check for status change
            if let Ok(current_status) = sqlx::query_scalar::<_, String>("SELECT status FROM support_tickets WHERE id = ?")
                .bind(&ticket_id)
                .fetch_one(&db)
                .await 
            {
                if current_status != last_status {
                    last_status = current_status.clone();
                    yield Ok(Event::default().event("status").data(current_status));
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    };

    Ok(Sse::new(s).keep_alive(KeepAlive::default()))
}

pub async fn admin_ticket_messages_stream(
    State(state): State<AppState>,
    Path(ticket_id): Path<String>,
    Query(query): Query<SseQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, &'static str)> {
    let _ = auth::require_admin_from_token(&query.token, &state).await?;

    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM support_tickets WHERE id = ?")
        .bind(&ticket_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?;

    if exists == 0 {
        return Err((StatusCode::NOT_FOUND, "ticket not found"));
    }

    let db = state.db.clone();
    let s = stream! {
        let mut last_created_at = "1970-01-01 00:00:00".to_string();
        let mut last_status = String::new();

        loop {
            // Check for new messages
            let q = "SELECT id, sender_user_id, message, created_at FROM support_messages WHERE ticket_id = ? AND created_at > ? ORDER BY created_at ASC";
            let rows = sqlx::query_as::<_, (String, Option<String>, String, String)>(q)
                .bind(&ticket_id)
                .bind(&last_created_at)
                .fetch_all(&db)
                .await;

            if let Ok(rows) = rows {
                for (id, sender_user_id, message, created_at) in rows {
                    let item = TicketMessageItem {
                        id,
                        sender_user_id,
                        message,
                        created_at: created_at.clone(),
                    };
                    
                    last_created_at = created_at;

                    if let Ok(json) = serde_json::to_string(&item) {
                        yield Ok(Event::default().event("message").data(json));
                    }
                }
            }

            // Check for status change
            if let Ok(current_status) = sqlx::query_scalar::<_, String>("SELECT status FROM support_tickets WHERE id = ?")
                .bind(&ticket_id)
                .fetch_one(&db)
                .await 
            {
                if current_status != last_status {
                    last_status = current_status.clone();
                    yield Ok(Event::default().event("status").data(current_status));
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    };

    Ok(Sse::new(s).keep_alive(KeepAlive::default()))
}

pub async fn reply_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ticket_id): Path<String>,
    Json(payload): Json<TicketReplyRequest>,
) -> Result<Json<TicketMessageItem>, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    if payload.message.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "message is required"));
    }

    let ticket_owner_id = sqlx::query_scalar::<_, String>(
        "SELECT user_id FROM support_tickets WHERE id = ?"
    )
    .bind(&ticket_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
    .ok_or((StatusCode::NOT_FOUND, "ticket not found"))?;

    if ticket_owner_id != user.id {
        return Err((StatusCode::FORBIDDEN, "access denied"));
    }

    let message_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO support_messages (id, ticket_id, sender_user_id, message) VALUES (?, ?, ?, ?)",
    )
    .bind(&message_id)
    .bind(&ticket_id)
    .bind(&user.id)
    .bind(payload.message.trim())
    .execute(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, ticket_id = %ticket_id, user_id = %user.id, "failed to insert support message");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to reply ticket")
    })?;

    sqlx::query("UPDATE support_tickets SET status = 'open', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
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

    let item = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT id, user_id, subject, category, priority, status FROM support_tickets WHERE id = ? LIMIT 1",
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
        user_id: item.1,
        subject: item.2,
        category: item.3,
        priority: item.4,
        status: item.5,
    }))
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

pub async fn close_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ticket_id): Path<String>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let user = auth::require_user(&headers, &state).await?;

    let ticket_owner_id = sqlx::query_scalar::<_, String>(
        "SELECT user_id FROM support_tickets WHERE id = ?"
    )
    .bind(&ticket_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
    .ok_or((StatusCode::NOT_FOUND, "ticket not found"))?;

    if ticket_owner_id != user.id {
        return Err((StatusCode::FORBIDDEN, "access denied"));
    }

    sqlx::query("UPDATE support_tickets SET status = 'closed', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&ticket_id)
        .execute(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, ticket_id = %ticket_id, "failed to close support ticket");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to close ticket")
        })?;

    Ok(StatusCode::OK)
}

pub async fn admin_close_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ticket_id): Path<String>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let _ = auth::require_admin(&headers, &state).await?;

    sqlx::query("UPDATE support_tickets SET status = 'closed', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&ticket_id)
        .execute(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, ticket_id = %ticket_id, "failed to close support ticket");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to close ticket")
        })?;

    Ok(StatusCode::OK)
}

fn is_valid_ticket_status(status: &str) -> bool {
    matches!(
        status.trim().to_lowercase().as_str(),
        "open" | "in_progress" | "resolved" | "closed"
    )
}
