use crate::auth;
use crate::AppState;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::Redirect;
use axum::Json;
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateOrderRequest {
    pub plan_code: String,
}

#[derive(Serialize)]
pub struct PaypalCreateOrderResponse {
    pub order_id: String,
    pub invoice_id: String,
    pub paypal_order_id: String,
    pub approval_url: String,
    pub amount: String,
    pub currency: String,
}

#[derive(Serialize)]
pub struct PaypalWebhookResponse {
    pub accepted: bool,
    pub note: &'static str,
}

#[derive(Deserialize, Serialize)]
struct PayPalWebhookEvent {
    id: String,
    event_type: String,
    #[serde(default)]
    create_time: Option<chrono::DateTime<chrono::Utc>>,
    resource: serde_json::Value,
}

#[derive(Deserialize)]
struct PayPalWebhookVerificationResponse {
    verification_status: String,
}

#[derive(Deserialize)]
struct PayPalAccessTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct PayPalCreateOrderLink {
    href: String,
    rel: String,
}

#[derive(Deserialize)]
struct PayPalCreateOrderResponseBody {
    id: String,
    links: Vec<PayPalCreateOrderLink>,
}

#[derive(Deserialize)]
struct PayPalCaptureOrderResponseBody {
    #[serde(rename = "id")]
    _id: String,
    status: String,
}

#[derive(Deserialize)]
pub struct PayPalReturnQuery {
    token: Option<String>,
    #[serde(rename = "payer_id")]
    _payer_id: Option<String>,
}

#[derive(Clone)]
struct CheckoutPlan {
    id: String,
    code: String,
    name: String,
    monthly_price: String,
}

pub async fn create_order(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<Json<PaypalCreateOrderResponse>, (StatusCode, &'static str)> {
    let user = auth::require_auth(&headers, &state).await?;
    let plan = load_plan(&state, &payload.plan_code).await?;

    ensure_inventory_available(&state, &plan.id).await?;

    let order_id = Uuid::new_v4();
    let invoice_id = Uuid::new_v4();
    let idempotency_key = Uuid::new_v4().to_string();
    let amount = plan.monthly_price.clone();
    let due_at = Utc::now() + Duration::hours(24);

    sqlx::query(
        "INSERT INTO orders (id, user_id, plan_id, status, total_amount, idempotency_key) VALUES (?, ?, ?, 'pending_payment', ?, ?)",
    )
    .bind(order_id.to_string())
    .bind(&user.id)
    .bind(&plan.id)
    .bind(&amount)
    .bind(&idempotency_key)
    .execute(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, user_id = %user.id, plan_code = %plan.code, "failed to create order");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to create order")
    })?;

    sqlx::query(
        "INSERT INTO invoices (id, user_id, order_id, amount, currency, status, due_at) VALUES (?, ?, ?, ?, 'USD', 'open', ?)",
    )
    .bind(invoice_id.to_string())
    .bind(&user.id)
    .bind(order_id.to_string())
    .bind(&amount)
    .bind(due_at.to_rfc3339())
    .execute(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, order_id = %order_id, user_id = %user.id, "failed to create invoice");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to create invoice")
    })?;

    let paypal_response = match issue_paypal_checkout(
        &state,
        &user.id,
        &plan,
        &order_id.to_string(),
        &invoice_id.to_string(),
        &amount,
    )
    .await
    {
        Ok(response) => response,
        Err(err) => {
            mark_checkout_failed(&state, &order_id, &invoice_id).await;
            return Err(err);
        }
    };

    Ok(Json(paypal_response))
}

pub async fn retry_invoice_payment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(invoice_id): Path<String>,
) -> Result<Json<PaypalCreateOrderResponse>, (StatusCode, &'static str)> {
    let user = auth::require_auth(&headers, &state).await?;
    let record = load_invoice_payment_context(&state, &invoice_id, &user).await?;

    if record.status.eq_ignore_ascii_case("paid") {
        return Err((StatusCode::CONFLICT, "invoice already paid"));
    }

    if invoice_is_overdue(&record.due_at) {
        expire_invoice_if_needed(&state, &record.invoice_id).await?;
        return Err((StatusCode::GONE, "invoice expired"));
    }

    if !record.status.eq_ignore_ascii_case("open") {
        return Err((StatusCode::CONFLICT, "invoice is not payable"));
    }

    let response = issue_paypal_checkout(
        &state,
        &user.id,
        &record.plan,
        &record.order_id,
        &record.invoice_id,
        &record.amount,
    )
    .await?;

    Ok(Json(response))
}

pub async fn paypal_return(
    State(state): State<AppState>,
    Query(query): Query<PayPalReturnQuery>,
) -> Result<Redirect, (StatusCode, &'static str)> {
    let Some(token) = query.token else {
        return Err((StatusCode::BAD_REQUEST, "missing PayPal token"));
    };

    let Some(invoice) = load_invoice_by_paypal_ref(&state, &token).await? else {
        return Err((StatusCode::NOT_FOUND, "checkout not found"));
    };

    if invoice.status == "paid" {
        info!(paypal_order_id = %token, order_id = %invoice.order_id, invoice_id = %invoice.invoice_id, "paypal checkout already finalized");
        return Ok(Redirect::to(&format!(
            "{}/app/balance",
            state.frontend_base_url
        )));
    }

    if invoice.status == "expired" || invoice_is_overdue(&invoice.due_at) {
        expire_invoice_if_needed(&state, &invoice.invoice_id).await?;
        warn!(paypal_order_id = %token, order_id = %invoice.order_id, invoice_id = %invoice.invoice_id, "paypal checkout expired before capture");
        return Err((StatusCode::GONE, "invoice expired"));
    }

    let capture = capture_paypal_order(&state, &token).await?;
    if capture.status != "COMPLETED" {
        warn!(paypal_order_id = %token, capture_status = %capture.status, payer_id = ?query._payer_id, "paypal capture did not complete");
        return Err((StatusCode::BAD_GATEWAY, "payment capture did not complete"));
    }

    finalize_paid_checkout(&state, &invoice.order_id, &invoice.invoice_id, &token).await?;

    info!(paypal_order_id = %token, order_id = %invoice.order_id, invoice_id = %invoice.invoice_id, "paypal sandbox payment completed");

    Ok(Redirect::to(&format!(
        "{}/app/balance",
        state.frontend_base_url
    )))
}

pub async fn paypal_cancel(State(state): State<AppState>) -> Redirect {
    Redirect::to(&format!("{}/order", state.frontend_base_url))
}

pub async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<PaypalWebhookResponse>, (StatusCode, &'static str)> {
    let raw_body = std::str::from_utf8(&body).map_err(|err| {
        error!(error = %err, "paypal webhook payload is not valid utf-8");
        (StatusCode::BAD_REQUEST, "invalid webhook payload")
    })?;

    let event: PayPalWebhookEvent = serde_json::from_str(raw_body).map_err(|err| {
        error!(error = %err, "failed to parse paypal webhook payload");
        (StatusCode::BAD_REQUEST, "invalid webhook payload")
    })?;

    verify_paypal_webhook(&state, &headers, raw_body).await?;

    if webhook_event_already_processed(&state, &event.id).await? {
        info!(event_id = %event.id, event_type = %event.event_type, "paypal webhook replay ignored");
        return Ok(Json(PaypalWebhookResponse {
            accepted: true,
            note: "paypal webhook already processed",
        }));
    }

    store_paypal_webhook_event(&state, &event, raw_body).await?;

    match event.event_type.as_str() {
        "PAYMENT.CAPTURE.COMPLETED" | "CHECKOUT.ORDER.COMPLETED" => {
            let Some(paypal_order_id) = extract_paypal_order_id(&event.resource) else {
                error!(event_id = %event.id, event_type = %event.event_type, "paypal webhook missing related order id");
                return Err((StatusCode::BAD_REQUEST, "missing related paypal order id"));
            };

            let Some(invoice) = load_invoice_by_paypal_ref(&state, &paypal_order_id).await? else {
                warn!(event_id = %event.id, event_type = %event.event_type, paypal_order_id = %paypal_order_id, "paypal webhook referenced unknown checkout");
                mark_paypal_webhook_event_processed(&state, &event.id).await?;
                return Ok(Json(PaypalWebhookResponse {
                    accepted: true,
                    note: "paypal webhook referenced unknown checkout",
                }));
            };

            if invoice.status == "expired" || invoice_is_overdue(&invoice.due_at) {
                expire_invoice_if_needed(&state, &invoice.invoice_id).await?;
                warn!(event_id = %event.id, event_type = %event.event_type, paypal_order_id = %paypal_order_id, order_id = %invoice.order_id, invoice_id = %invoice.invoice_id, "paypal webhook ignored because invoice expired before capture");
            } else if invoice.status != "paid" {
                finalize_paid_checkout(
                    &state,
                    &invoice.order_id,
                    &invoice.invoice_id,
                    &paypal_order_id,
                )
                .await?;
            }

            info!(event_id = %event.id, event_type = %event.event_type, paypal_order_id = %paypal_order_id, order_id = %invoice.order_id, invoice_id = %invoice.invoice_id, "paypal webhook finalized checkout");
        }
        "CHECKOUT.ORDER.APPROVED" => {
            let paypal_order_id = extract_paypal_order_id(&event.resource).unwrap_or_else(|| {
                event
                    .resource
                    .get("id")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string()
            });

            let Some(invoice) = load_invoice_by_paypal_ref(&state, &paypal_order_id).await? else {
                warn!(event_id = %event.id, event_type = %event.event_type, paypal_order_id = %paypal_order_id, "paypal approved webhook referenced unknown checkout");
                mark_paypal_webhook_event_processed(&state, &event.id).await?;
                return Ok(Json(PaypalWebhookResponse {
                    accepted: true,
                    note: "paypal webhook referenced unknown checkout",
                }));
            };

            info!(event_id = %event.id, event_type = %event.event_type, paypal_order_id = %paypal_order_id, "paypal checkout approved");

            let mut should_capture = true;
            if invoice.status == "expired" || invoice_is_overdue(&invoice.due_at) {
                expire_invoice_if_needed(&state, &invoice.invoice_id).await?;
                warn!(paypal_order_id = %paypal_order_id, order_id = %invoice.order_id, invoice_id = %invoice.invoice_id, "paypal approved webhook skipped because invoice expired before capture");
                should_capture = false;
            } else if let Some(create_time) = event.create_time {
                let age = Utc::now().signed_duration_since(create_time);
                if age > Duration::minutes(2) {
                    warn!(
                        paypal_order_id = %paypal_order_id,
                        age_seconds = age.num_seconds(),
                        "paypal checkout approved webhook is too old (> 2 mins), skipping auto-capture to prevent delayed conflicts"
                    );
                    should_capture = false;
                }
            }

            if should_capture {
                // Automatically attempt to capture the payment when webhook signals approval,
                // bridging the gap if the user closes the window before being redirected to the return URL.
                match capture_paypal_order(&state, &paypal_order_id).await {
                    Ok(capture) => {
                        info!(paypal_order_id = %paypal_order_id, capture_status = %capture.status, "paypal order automatically captured via webhook");
                    }
                    Err((status, msg)) => {
                        if status == StatusCode::CONFLICT {
                            info!(paypal_order_id = %paypal_order_id, "paypal order already captured, skipping webhook auto-capture");
                        } else {
                            warn!(
                                paypal_order_id = %paypal_order_id,
                                status_code = %status,
                                msg = %msg,
                                "paypal order capture attempt via webhook failed"
                            );
                        }
                    }
                }
            }
        }
        _ => {
            info!(event_id = %event.id, event_type = %event.event_type, "ignored paypal webhook event");
        }
    }

    mark_paypal_webhook_event_processed(&state, &event.id).await?;

    Ok(Json(PaypalWebhookResponse {
        accepted: true,
        note: "paypal webhook processed",
    }))
}

struct PaypalCheckoutRequest {
    payload: serde_json::Value,
}

async fn load_plan(
    state: &AppState,
    plan_code: &str,
) -> Result<CheckoutPlan, (StatusCode, &'static str)> {
    sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT id, code, name, CAST(monthly_price AS TEXT) FROM nat_plans WHERE code = ? AND active = 1",
    )
    .bind(plan_code)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, plan_code = %plan_code, "failed to query plan");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load plan")
    })?
    .map(|(id, code, name, monthly_price)| CheckoutPlan {
        id,
        code,
        name,
        monthly_price,
    })
    .ok_or((StatusCode::NOT_FOUND, "plan not found"))
}

fn build_paypal_create_order_request(
    state: &AppState,
    plan: &CheckoutPlan,
    order_id: &str,
    invoice_id: &str,
    amount: &str,
) -> PaypalCheckoutRequest {
    let return_url = format!("{}/api/payment/paypal/return", state.paypal_return_base_url);
    let cancel_url = format!("{}/api/payment/paypal/cancel", state.paypal_return_base_url);
    let payload = json!({
        "intent": "CAPTURE",
        "purchase_units": [{
            "reference_id": order_id,
            "custom_id": order_id,
            "invoice_id": invoice_id,
            "description": format!("{} ({})", plan.name, plan.code),
            "amount": {
                "currency_code": "USD",
                "value": amount,
            }
        }],
        "application_context": {
            "brand_name": "Cloud Store",
            "landing_page": "BILLING",
            "user_action": "PAY_NOW",
            "return_url": return_url,
            "cancel_url": cancel_url,
        }
    });

    PaypalCheckoutRequest { payload }
}

async fn issue_paypal_checkout(
    state: &AppState,
    user_id: &str,
    plan: &CheckoutPlan,
    order_id: &str,
    invoice_id: &str,
    amount: &str,
) -> Result<PaypalCreateOrderResponse, (StatusCode, &'static str)> {
    ensure_inventory_available(state, &plan.id).await?;

    let paypal_access_token = fetch_paypal_access_token(state).await?;
    let checkout_request =
        build_paypal_create_order_request(state, plan, order_id, invoice_id, amount);

    let paypal_response =
        create_paypal_order(state, &paypal_access_token, checkout_request).await?;

    let approval_url = find_paypal_approval_url(&paypal_response)
        .ok_or((StatusCode::BAD_GATEWAY, "missing sandbox approval url"))?;

    sqlx::query("UPDATE invoices SET external_payment_ref = ? WHERE id = ?")
        .bind(&paypal_response.id)
        .bind(invoice_id)
        .execute(&state.db)
        .await
        .map_err(|err| {
            error!(error = %err, invoice_id = %invoice_id, paypal_order_id = %paypal_response.id, "failed to store paypal reference");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to store payment reference")
        })?;

    info!(order_id = %order_id, invoice_id = %invoice_id, paypal_order_id = %paypal_response.id, user_id = %user_id, "created paypal sandbox checkout");

    Ok(PaypalCreateOrderResponse {
        order_id: order_id.to_string(),
        invoice_id: invoice_id.to_string(),
        paypal_order_id: paypal_response.id,
        approval_url,
        amount: amount.to_string(),
        currency: "USD".to_string(),
    })
}

async fn ensure_inventory_available(
    state: &AppState,
    plan_id: &str,
) -> Result<(), (StatusCode, &'static str)> {
    let available_capacity = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM nodes 
         JOIN nat_plans ON nat_plans.id = ? 
         WHERE nodes.active = 1 
         AND (nodes.memory_mb_total - nodes.memory_mb_used) >= nat_plans.memory_mb 
         AND (nodes.storage_gb_total - nodes.storage_gb_used) >= nat_plans.storage_gb",
    )
    .bind(plan_id)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, "failed to query inventory capacity");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to check inventory",
        )
    })?;

    if available_capacity <= 0 {
        warn!(plan_id = plan_id, "inventory unavailable for new payment");
        return Err((StatusCode::CONFLICT, "inventory unavailable"));
    }

    let plan_inventory = sqlx::query_as::<_, (Option<i64>, i64)>(
        "SELECT max_inventory, COALESCE(sold_inventory, 0) FROM nat_plans WHERE id = ? LIMIT 1",
    )
    .bind(plan_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, plan_id = %plan_id, "failed to query plan inventory");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to check plan inventory",
        )
    })?
    .ok_or((StatusCode::NOT_FOUND, "plan not found"))?;

    if let Some(limit) = plan_inventory.0 {
        if plan_inventory.1 >= limit {
            warn!(plan_id = %plan_id, sold_inventory = plan_inventory.1, max_inventory = limit, "plan inventory unavailable for new payment");
            return Err((StatusCode::CONFLICT, "plan inventory unavailable"));
        }
    }

    Ok(())
}

#[derive(Clone)]
struct InvoicePaymentContext {
    order_id: String,
    invoice_id: String,
    status: String,
    due_at: String,
    amount: String,
    plan: CheckoutPlan,
}

async fn load_invoice_payment_context(
    state: &AppState,
    invoice_id: &str,
    user: &auth::AuthUser,
) -> Result<InvoicePaymentContext, (StatusCode, &'static str)> {
    let record = if user.role == "admin" {
        sqlx::query_as::<_, (String, String, String, String, String, String, String, String, String, String)>(
            "SELECT invoices.order_id, invoices.id, invoices.status, invoices.due_at, CAST(invoices.amount AS TEXT), nat_plans.id, nat_plans.code, nat_plans.name, CAST(nat_plans.monthly_price AS TEXT), invoices.user_id FROM invoices INNER JOIN orders ON orders.id = invoices.order_id INNER JOIN nat_plans ON nat_plans.id = orders.plan_id WHERE invoices.id = ? LIMIT 1",
        )
        .bind(invoice_id)
        .fetch_optional(&state.db)
        .await
    } else {
        sqlx::query_as::<_, (String, String, String, String, String, String, String, String, String, String)>(
            "SELECT invoices.order_id, invoices.id, invoices.status, invoices.due_at, CAST(invoices.amount AS TEXT), nat_plans.id, nat_plans.code, nat_plans.name, CAST(nat_plans.monthly_price AS TEXT), invoices.user_id FROM invoices INNER JOIN orders ON orders.id = invoices.order_id INNER JOIN nat_plans ON nat_plans.id = orders.plan_id WHERE invoices.id = ? AND invoices.user_id = ? LIMIT 1",
        )
        .bind(invoice_id)
        .bind(&user.id)
        .fetch_optional(&state.db)
        .await
    }
    .map_err(|err| {
        error!(error = %err, invoice_id = %invoice_id, user_id = %user.id, "failed to load invoice payment context");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load invoice")
    })?;

    record
        .map(
            |(
                order_id,
                invoice_id,
                status,
                due_at,
                amount,
                plan_id,
                code,
                name,
                monthly_price,
                _owner_id,
            )| {
                InvoicePaymentContext {
                    order_id,
                    invoice_id,
                    status,
                    due_at,
                    amount,
                    plan: CheckoutPlan {
                        id: plan_id,
                        code,
                        name,
                        monthly_price,
                    },
                }
            },
        )
        .ok_or((StatusCode::NOT_FOUND, "invoice not found"))
}

async fn expire_invoice_if_needed(
    state: &AppState,
    invoice_id: &str,
) -> Result<(), (StatusCode, &'static str)> {
    sqlx::query(
        "UPDATE invoices SET status = 'expired' WHERE id = ? AND status = 'open' AND datetime(due_at) <= datetime('now')",
    )
    .bind(invoice_id)
    .execute(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, invoice_id = %invoice_id, "failed to expire invoice");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to expire invoice")
    })?;

    Ok(())
}

fn invoice_is_overdue(due_at: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(due_at)
        .map(|timestamp| timestamp.with_timezone(&Utc) <= Utc::now())
        .unwrap_or(false)
}

async fn fetch_paypal_access_token(state: &AppState) -> Result<String, (StatusCode, &'static str)> {
    let url = format!(
        "{}/v1/oauth2/token",
        state.paypal_base_url.trim_end_matches('/')
    );
    let response = state
        .http_client
        .post(url)
        .basic_auth(&state.paypal_client_id, Some(&state.paypal_client_secret))
        .form(&[("grant_type", "client_credentials")])
        .send()
        .await
        .map_err(|err| {
            error!(error = %err, "failed to request paypal access token");
            (StatusCode::BAD_GATEWAY, "failed to obtain sandbox token")
        })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_else(|_| String::new());
        error!(status = %status, body = %body, "paypal token request failed");
        if status == StatusCode::UNAUTHORIZED {
            return Err((
                StatusCode::BAD_GATEWAY,
                "sandbox token request failed: invalid client credentials or mismatched PayPal base URL",
            ));
        }

        return Err((StatusCode::BAD_GATEWAY, "sandbox token request failed"));
    }

    response
        .json::<PayPalAccessTokenResponse>()
        .await
        .map(|body| body.access_token)
        .map_err(|err| {
            error!(error = %err, "failed to parse paypal access token response");
            (StatusCode::BAD_GATEWAY, "failed to parse sandbox token")
        })
}

async fn create_paypal_order(
    state: &AppState,
    access_token: &str,
    request: PaypalCheckoutRequest,
) -> Result<PayPalCreateOrderResponseBody, (StatusCode, &'static str)> {
    let url = format!(
        "{}/v2/checkout/orders",
        state.paypal_base_url.trim_end_matches('/')
    );
    let response = state
        .http_client
        .post(url)
        .bearer_auth(access_token)
        .json(&request.payload)
        .send()
        .await
        .map_err(|err| {
            error!(error = %err, "failed to create paypal sandbox order");
            (StatusCode::BAD_GATEWAY, "failed to create sandbox order")
        })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_else(|_| String::new());
        error!(status = %status, body = %body, "paypal order creation failed");
        return Err((StatusCode::BAD_GATEWAY, "sandbox order creation failed"));
    }

    response
        .json::<PayPalCreateOrderResponseBody>()
        .await
        .map_err(|err| {
            error!(error = %err, "failed to parse paypal order response");
            (
                StatusCode::BAD_GATEWAY,
                "failed to parse sandbox order response",
            )
        })
}

async fn capture_paypal_order(
    state: &AppState,
    paypal_order_id: &str,
) -> Result<PayPalCaptureOrderResponseBody, (StatusCode, &'static str)> {
    let access_token = fetch_paypal_access_token(state).await?;
    let url = format!(
        "{}/v2/checkout/orders/{paypal_order_id}/capture",
        state.paypal_base_url.trim_end_matches('/')
    );
    let response = state
        .http_client
        .post(url)
        .bearer_auth(access_token)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body("{}")
        .send()
        .await
        .map_err(|err| {
            error!(error = %err, paypal_order_id = %paypal_order_id, "failed to capture paypal order");
            (StatusCode::BAD_GATEWAY, "failed to capture sandbox payment")
        })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_else(|_| String::new());
        if status.as_u16() == 422 && body.contains("ORDER_ALREADY_CAPTURED") {
            info!(paypal_order_id = %paypal_order_id, "paypal order was already captured");
            return Err((StatusCode::CONFLICT, "already captured"));
        }

        error!(status = %status, body = %body, paypal_order_id = %paypal_order_id, "paypal capture failed");
        return Err((StatusCode::BAD_GATEWAY, "sandbox payment capture failed"));
    }

    response
        .json::<PayPalCaptureOrderResponseBody>()
        .await
        .map_err(|err| {
            error!(error = %err, paypal_order_id = %paypal_order_id, "failed to parse paypal capture response");
            (StatusCode::BAD_GATEWAY, "failed to parse sandbox capture response")
        })
}

#[derive(Clone)]
struct InvoiceCheckoutRecord {
    order_id: String,
    invoice_id: String,
    status: String,
    due_at: String,
}

async fn load_invoice_by_paypal_ref(
    state: &AppState,
    paypal_order_id: &str,
) -> Result<Option<InvoiceCheckoutRecord>, (StatusCode, &'static str)> {
    sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT order_id, id, status, due_at FROM invoices WHERE external_payment_ref = ? LIMIT 1",
    )
    .bind(paypal_order_id)
    .fetch_optional(&state.db)
    .await
    .map(|record| {
        record.map(|(order_id, invoice_id, status, due_at)| InvoiceCheckoutRecord {
            order_id,
            invoice_id,
            status,
            due_at,
        })
    })
    .map_err(|err| {
        error!(error = %err, paypal_order_id = %paypal_order_id, "failed to load invoice by paypal reference");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load checkout record")
    })
}

async fn finalize_paid_checkout(
    state: &AppState,
    order_id: &str,
    invoice_id: &str,
    paypal_order_id: &str,
) -> Result<(), (StatusCode, &'static str)> {
    let mut tx = state.db.begin().await.map_err(|err| {
        error!(error = %err, order_id = %order_id, invoice_id = %invoice_id, "failed to begin checkout transaction");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to finalize checkout",
        )
    })?;

    let order_update = sqlx::query(
        "UPDATE orders SET status = 'paid', updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status != 'paid'",
    )
    .bind(order_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| {
        error!(error = %err, order_id = %order_id, "failed to mark order paid");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to mark order paid",
        )
    })?;

    if order_update.rows_affected() > 0 {
        let plan_inventory_update = sqlx::query(
            "UPDATE nat_plans SET sold_inventory = sold_inventory + 1 WHERE id = (SELECT plan_id FROM orders WHERE id = ?) AND (max_inventory IS NULL OR sold_inventory < max_inventory)",
        )
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| {
            error!(error = %err, order_id = %order_id, "failed to update sold inventory");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to update plan inventory",
            )
        })?;

        if plan_inventory_update.rows_affected() == 0 {
            return Err((StatusCode::CONFLICT, "plan inventory unavailable"));
        }
    }

    sqlx::query(
        "UPDATE invoices SET status = 'paid', paid_at = COALESCE(paid_at, CURRENT_TIMESTAMP), external_payment_ref = ? WHERE id = ?",
    )
    .bind(paypal_order_id)
    .bind(invoice_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| {
        error!(error = %err, invoice_id = %invoice_id, order_id = %order_id, "failed to mark invoice paid");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to mark invoice paid")
    })?;

    tx.commit().await.map_err(|err| {
        error!(error = %err, order_id = %order_id, invoice_id = %invoice_id, "failed to commit checkout transaction");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to finalize checkout",
        )
    })?;

    Ok(())
}

fn find_paypal_approval_url(response: &PayPalCreateOrderResponseBody) -> Option<String> {
    response
        .links
        .iter()
        .find(|link| link.rel == "approve" || link.rel == "payer-action")
        .map(|link| link.href.clone())
}

async fn mark_checkout_failed(state: &AppState, order_id: &Uuid, invoice_id: &Uuid) {
    let _ = sqlx::query(
        "UPDATE orders SET status = 'failed', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(order_id.to_string())
    .execute(&state.db)
    .await;

    let _ = sqlx::query("UPDATE invoices SET status = 'failed' WHERE id = ?")
        .bind(invoice_id.to_string())
        .execute(&state.db)
        .await;
}

async fn verify_paypal_webhook(
    state: &AppState,
    headers: &HeaderMap,
    raw_body: &str,
) -> Result<(), (StatusCode, &'static str)> {
    if state.paypal_webhook_id.trim().is_empty() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "paypal webhook id is not configured",
        ));
    }

    let transmission_id = required_header(headers, "PayPal-Transmission-Id")?;
    let transmission_time = required_header(headers, "PayPal-Transmission-Time")?;
    let cert_url = required_header(headers, "PayPal-Cert-Url")?;
    let auth_algo = required_header(headers, "PayPal-Auth-Algo")?;
    let transmission_sig = required_header(headers, "PayPal-Transmission-Sig")?;

    let event_value: serde_json::Value = serde_json::from_str(raw_body).map_err(|err| {
        error!(error = %err, "failed to parse webhook body to json value");
        (StatusCode::BAD_REQUEST, "invalid webhook json")
    })?;

    let event_id = event_value
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Construct the payload manually to prevent serde_json from sorting keys in raw_body and breaking the signature.
    let mut wrapper = serde_json::json!({
        "transmission_id": transmission_id,
        "transmission_time": transmission_time,
        "cert_url": cert_url,
        "auth_algo": auth_algo,
        "transmission_sig": transmission_sig,
        "webhook_id": state.paypal_webhook_id,
    })
    .to_string();

    // Remove the closing brace
    wrapper.pop();
    // Append the exact raw_body to ensure the signature target (webhook_event) is perfectly preserved
    let verify_payload = format!(r#"{},"webhook_event":{}}}"#, wrapper, raw_body);

    let url = format!(
        "{}/v1/notifications/verify-webhook-signature",
        state.paypal_base_url.trim_end_matches('/')
    );
    let response = state
        .http_client
        .post(url)
        .basic_auth(&state.paypal_client_id, Some(&state.paypal_client_secret))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(verify_payload)
        .send()
        .await
        .map_err(|err| {
            error!(error = %err, event_id = %event_id, "failed to verify paypal webhook signature");
            (
                StatusCode::BAD_GATEWAY,
                "failed to verify paypal webhook signature",
            )
        })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_else(|_| String::new());
        error!(status = %status, body = %body, event_id = %event_id, "paypal webhook verification request failed");
        return Err((
            StatusCode::BAD_GATEWAY,
            "paypal webhook verification failed",
        ));
    }

    let verification = response
        .json::<PayPalWebhookVerificationResponse>()
        .await
        .map_err(|err| {
            error!(error = %err, event_id = %event_id, "failed to parse paypal webhook verification response");
            (StatusCode::BAD_GATEWAY, "failed to parse webhook verification response")
        })?;

    if verification.verification_status != "SUCCESS" {
        warn!(event_id = %event_id, verification_status = %verification.verification_status, webhook_id = %state.paypal_webhook_id, "paypal webhook signature verification failed (ensure PAYPAL_WEBHOOK_ID in .env matches the webhook ID in PayPal dashboard)");
        return Err((
            StatusCode::BAD_REQUEST,
            "paypal webhook signature verification failed",
        ));
    }

    Ok(())
}

async fn store_paypal_webhook_event(
    state: &AppState,
    event: &PayPalWebhookEvent,
    payload: &str,
) -> Result<(), (StatusCode, &'static str)> {
    sqlx::query(
        "INSERT OR IGNORE INTO payment_webhook_events (id, gateway, event_id, payload) VALUES (?, 'paypal', ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&event.id)
    .bind(payload)
    .execute(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, event_id = %event.id, "failed to store paypal webhook event");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to store webhook event")
    })?;

    Ok(())
}

async fn webhook_event_already_processed(
    state: &AppState,
    event_id: &str,
) -> Result<bool, (StatusCode, &'static str)> {
    let processed_at = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT processed_at FROM payment_webhook_events WHERE gateway = 'paypal' AND event_id = ? LIMIT 1",
    )
    .bind(event_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, event_id = %event_id, "failed to load paypal webhook event state");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to load webhook state")
    })?;

    Ok(processed_at.and_then(|row| row.0).is_some())
}

async fn mark_paypal_webhook_event_processed(
    state: &AppState,
    event_id: &str,
) -> Result<(), (StatusCode, &'static str)> {
    sqlx::query(
        "UPDATE payment_webhook_events SET processed_at = CURRENT_TIMESTAMP WHERE gateway = 'paypal' AND event_id = ? AND processed_at IS NULL",
    )
    .bind(event_id)
    .execute(&state.db)
    .await
    .map_err(|err| {
        error!(error = %err, event_id = %event_id, "failed to mark paypal webhook event processed");
        (StatusCode::INTERNAL_SERVER_ERROR, "failed to update webhook state")
    })?;

    Ok(())
}

fn required_header(
    headers: &HeaderMap,
    name: &'static str,
) -> Result<String, (StatusCode, &'static str)> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
        .ok_or((StatusCode::BAD_REQUEST, "missing paypal webhook header"))
}

fn extract_paypal_order_id(resource: &serde_json::Value) -> Option<String> {
    resource
        .get("supplementary_data")
        .and_then(|value| value.get("related_ids"))
        .and_then(|value| value.get("order_id"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .or_else(|| {
            resource
                .get("id")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
        })
        .or_else(|| {
            resource
                .get("purchase_units")
                .and_then(|value| value.as_array())
                .and_then(|units| units.first())
                .and_then(|unit| unit.get("reference_id"))
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
        })
}
