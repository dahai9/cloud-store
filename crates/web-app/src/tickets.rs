use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct TicketItem {
    pub id: &'static str,
    pub subject: &'static str,
    pub category: &'static str,
    pub priority: &'static str,
    pub status: &'static str,
}

pub async fn list_tickets() -> Json<Vec<TicketItem>> {
    Json(vec![TicketItem {
        id: "demo-ticket-1",
        subject: "NAT port forwarding setup",
        category: "technical",
        priority: "high",
        status: "open",
    }])
}
