use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct PaypalCreateOrderResponse {
    pub message: &'static str,
}

#[derive(Serialize)]
pub struct PaypalWebhookResponse {
    pub accepted: bool,
    pub note: &'static str,
}

pub async fn create_order() -> Json<PaypalCreateOrderResponse> {
    Json(PaypalCreateOrderResponse {
        message: "paypal create order placeholder",
    })
}

pub async fn webhook_stub() -> Json<PaypalWebhookResponse> {
    Json(PaypalWebhookResponse {
        accepted: true,
        note: "paypal webhook placeholder - add signature verification and idempotency",
    })
}
