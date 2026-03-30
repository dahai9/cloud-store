use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct InvoiceItem {
    pub id: &'static str,
    pub amount: &'static str,
    pub status: &'static str,
    pub due_at: &'static str,
}

pub async fn list_invoices() -> Json<Vec<InvoiceItem>> {
    Json(vec![InvoiceItem {
        id: "demo-invoice-1",
        amount: "5.99",
        status: "open",
        due_at: "2026-04-01T00:00:00Z",
    }])
}
