use crate::models::{
    ActionRequest, AuthPayload, AuthProfileResponse, AuthTokenResponse, AuthTransportRisk,
    InstanceAction, InstanceItem, InstanceMetrics, InvoiceItem, PayPalCreateOrderRequest,
    PayPalCreateOrderResponse, PublicPlanItem, SessionState, TicketItem,
};

use reqwest::Client;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
const AUTH_TOKEN_KEY: &str = "cloud_store.auth.token";
#[cfg(target_arch = "wasm32")]
const AUTH_PROFILE_KEY: &str = "cloud_store.auth.profile";

pub fn default_api_base() -> String {
    option_env!("API_BASE_URL")
        .unwrap_or("http://127.0.0.1:8081")
        .to_string()
}

pub async fn get_public_plans(api_base: &str) -> Result<Vec<PublicPlanItem>, String> {
    let client = Client::new();
    let resp = client
        .get(format!("{api_base}/api/plans"))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Failed to load plans: {}", resp.status()));
    }

    resp.json::<Vec<PublicPlanItem>>()
        .await
        .map_err(|e| format!("Parse error: {e}"))
}

pub fn auth_transport_risk(api_base: &str) -> AuthTransportRisk {
    if api_base.starts_with("https://") {
        AuthTransportRisk::Secure
    } else if api_base.contains("127.0.0.1") || api_base.contains("localhost") {
        AuthTransportRisk::LoopbackDev
    } else {
        AuthTransportRisk::InsecureRemote
    }
}

pub fn auth_transport_notice(api_base: &str) -> Option<&'static str> {
    match auth_transport_risk(api_base) {
        AuthTransportRisk::Secure => None,
        AuthTransportRisk::LoopbackDev => {
            Some("当前是本地开发地址，注册/登录请求仍会通过 HTTP 发送。生产环境请改成 HTTPS。")
        }
        AuthTransportRisk::InsecureRemote => {
            Some("当前 API 不是 HTTPS，出于安全原因已禁止提交账号信息。请先切换到 HTTPS。")
        }
    }
}

pub fn build_console_ws_url(api_base: &str, id: &str, token: &str) -> String {
    let base = api_base
        .replace("http://", "ws://")
        .replace("https://", "wss://");
    format!("{base}/api/instances/{id}/console/ws?token={token}")
}

pub fn load_initial_session() -> SessionState {
    let mut initial = SessionState::new(default_api_base());

    if let Some((token, profile)) = load_persisted_session() {
        initial.token = Some(token);
        initial.profile = Some(profile);
    }

    initial
}

pub fn persist_authenticated_session(_session: &SessionState) {
    #[cfg(target_arch = "wasm32")]
    {
        use web_sys::window;
        let storage = window().and_then(|w| w.local_storage().ok()).flatten();
        if let Some(storage) = storage {
            if let Some(token) = &_session.token {
                let _ = storage.set_item(AUTH_TOKEN_KEY, token);
            }
            if let Some(profile) = &_session.profile {
                if let Ok(serialized) = serde_json::to_string(profile) {
                    let _ = storage.set_item(AUTH_PROFILE_KEY, &serialized);
                }
            }
        }
    }
}

pub fn clear_persisted_session() {
    #[cfg(target_arch = "wasm32")]
    {
        use web_sys::window;
        let storage = window().and_then(|w| w.local_storage().ok()).flatten();
        if let Some(storage) = storage {
            let _ = storage.remove_item(AUTH_TOKEN_KEY);
            let _ = storage.remove_item(AUTH_PROFILE_KEY);
        }
    }
}

pub async fn authenticate_and_load(
    api_base: &str,
    endpoint: &str,
    email: &str,
    password: &str,
) -> Result<BootstrapBundle, String> {
    if email.trim().is_empty() || password.trim().is_empty() {
        return Err("email and password are required".to_string());
    }

    if matches!(
        auth_transport_risk(api_base),
        AuthTransportRisk::InsecureRemote
    ) {
        return Err(
            "当前 API 不是 HTTPS，出于安全原因已禁止提交账号信息。请先切换到 HTTPS。".to_string(),
        );
    }

    let auth = AuthPayload {
        email: email.trim().to_string(),
        password: password.to_string(),
    };

    let client = Client::new();
    let url = format!("{api_base}/api/auth/{endpoint}");

    let resp = client
        .post(&url)
        .json(&auth)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "auth failed".to_string());
        return Err(format!("auth failed ({status}): {body}"));
    }

    let auth_result = resp
        .json::<AuthTokenResponse>()
        .await
        .map_err(|e| format!("failed to parse auth response: {e}"))?;

    let token = auth_result.token;
    let profile = fetch_profile(api_base, &token).await?;
    let invoices = fetch_invoices(api_base, &token).await?;
    let tickets = fetch_tickets(api_base, &token).await?;
    let instances = fetch_instances(api_base, &token).await?;
    let balance = fetch_balance(api_base, &token).await?;
    let balance_transactions = fetch_balance_transactions(api_base, &token).await?;

    Ok(BootstrapBundle {
        token,
        profile,
        invoices,
        tickets,
        instances,
        balance,
        balance_transactions,
    })
}

#[cfg(test)]
mod tests {
    use super::build_console_ws_url;

    #[test]
    fn builds_ws_url_from_http_base() {
        let url = build_console_ws_url("http://127.0.0.1:8081", "inst-123", "tok-abc");

        assert_eq!(
            url,
            "ws://127.0.0.1:8081/api/instances/inst-123/console/ws?token=tok-abc"
        );
    }

    #[test]
    fn builds_ws_url_from_https_base() {
        let url = build_console_ws_url("https://example.com", "inst-123", "tok-abc");

        assert_eq!(
            url,
            "wss://example.com/api/instances/inst-123/console/ws?token=tok-abc"
        );
    }
}

pub async fn load_authenticated_bundle(
    api_base: &str,
    token: &str,
) -> Result<BootstrapBundle, String> {
    let profile = fetch_profile(api_base, token).await?;
    let invoices = fetch_invoices(api_base, token).await?;
    let tickets = fetch_tickets(api_base, token).await?;
    let instances = fetch_instances(api_base, token).await?;
    let balance = fetch_balance(api_base, token).await?;
    let balance_transactions = fetch_balance_transactions(api_base, token).await?;

    Ok(BootstrapBundle {
        token: token.to_string(),
        profile,
        invoices,
        tickets,
        instances,
        balance,
        balance_transactions,
    })
}

pub async fn create_paypal_checkout(
    api_base: &str,
    token: &str,
    plan_code: &str,
    payment_method: Option<String>,
) -> Result<PayPalCreateOrderResponse, String> {
    if matches!(
        auth_transport_risk(api_base),
        AuthTransportRisk::InsecureRemote
    ) {
        return Err(
            "当前 API 不是 HTTPS，出于安全原因已禁止发起支付。请先切换到 HTTPS。".to_string(),
        );
    }

    let payload = PayPalCreateOrderRequest {
        plan_code: plan_code.to_string(),
        payment_method,
    };

    let client = Client::new();
    let url = format!("{api_base}/api/payment/paypal/create");
    let resp = client
        .post(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("payment request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "payment failed".to_string());
        return Err(format!("payment failed ({status}): {body}"));
    }

    resp.json::<PayPalCreateOrderResponse>()
        .await
        .map_err(|e| format!("failed to parse payment response: {e}"))
}

pub async fn retry_paypal_invoice(
    api_base: &str,
    token: &str,
    invoice_id: &str,
) -> Result<PayPalCreateOrderResponse, String> {
    if matches!(
        auth_transport_risk(api_base),
        AuthTransportRisk::InsecureRemote
    ) {
        return Err(
            "当前 API 不是 HTTPS，出于安全原因已禁止发起支付。请先切换到 HTTPS。".to_string(),
        );
    }

    let client = Client::new();
    let url = format!("{api_base}/api/payment/paypal/retry/{invoice_id}");
    let resp = client
        .post(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("payment retry request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "payment retry failed".to_string());
        return Err(format!("payment retry failed ({status}): {body}"));
    }

    resp.json::<PayPalCreateOrderResponse>()
        .await
        .map_err(|e| format!("failed to parse retry response: {e}"))
}

pub async fn fetch_balance(api_base: &str, token: &str) -> Result<String, String> {
    let client = Client::new();
    let url = format!("{api_base}/api/user/balance");
    let resp = client
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("failed to load balance: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("balance request failed: {}", resp.status()));
    }

    let result = resp
        .json::<crate::models::UserBalanceInfo>()
        .await
        .map_err(|e| format!("failed to parse balance: {e}"))?;
    Ok(result.balance)
}

pub async fn fetch_balance_transactions(
    api_base: &str,
    token: &str,
) -> Result<Vec<crate::models::BalanceTransactionItem>, String> {
    let client = Client::new();
    let url = format!("{api_base}/api/user/balance/transactions");
    let resp = client
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("failed to load balance transactions: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "balance transactions request failed: {}",
            resp.status()
        ));
    }

    resp.json::<Vec<crate::models::BalanceTransactionItem>>()
        .await
        .map_err(|e| format!("failed to parse balance transactions: {e}"))
}

pub async fn recharge_balance(
    api_base: &str,
    token: &str,
    amount: &str,
) -> Result<PayPalCreateOrderResponse, String> {
    let client = Client::new();
    let url = format!("{api_base}/api/user/balance/recharge");
    let payload = crate::models::RechargeRequest {
        amount: amount.to_string(),
    };

    let resp = client
        .post(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("recharge request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("recharge failed: {}", resp.status()));
    }

    resp.json::<PayPalCreateOrderResponse>()
        .await
        .map_err(|e| format!("failed to parse recharge response: {e}"))
}

pub async fn update_auto_renew(
    api_base: &str,
    token: &str,
    instance_id: &str,
    auto_renew: bool,
) -> Result<InstanceItem, String> {
    let client = Client::new();
    let url = format!("{api_base}/api/instances/{instance_id}/auto-renew");
    let payload = crate::models::UpdateAutoRenewRequest { auto_renew };

    let resp = client
        .patch(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("auto-renew update request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("auto-renew update failed: {}", resp.status()));
    }

    resp.json::<InstanceItem>()
        .await
        .map_err(|e| format!("failed to parse instance response: {e}"))
}

async fn fetch_profile(api_base: &str, token: &str) -> Result<AuthProfileResponse, String> {
    let client = Client::new();
    let url = format!("{api_base}/api/auth/me");
    let resp = client
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("failed to load profile: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "profile request failed with status {}",
            resp.status()
        ));
    }

    resp.json::<AuthProfileResponse>()
        .await
        .map_err(|e| format!("failed to parse profile response: {e}"))
}

async fn fetch_invoices(api_base: &str, token: &str) -> Result<Vec<InvoiceItem>, String> {
    let client = Client::new();
    let url = format!("{api_base}/api/invoices");
    let resp = client
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("failed to load invoices: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "invoice request failed with status {}",
            resp.status()
        ));
    }

    resp.json::<Vec<InvoiceItem>>()
        .await
        .map_err(|e| format!("failed to parse invoices response: {e}"))
}

async fn fetch_tickets(api_base: &str, token: &str) -> Result<Vec<TicketItem>, String> {
    let client = Client::new();
    let url = format!("{api_base}/api/tickets");
    let resp = client
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("failed to load tickets: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "ticket request failed with status {}",
            resp.status()
        ));
    }

    resp.json::<Vec<TicketItem>>()
        .await
        .map_err(|e| format!("failed to parse tickets response: {e}"))
}

#[allow(dead_code)]
pub async fn create_ticket(
    api_base: &str,
    token: &str,
    payload: &crate::models::CreateTicketRequest,
) -> Result<TicketItem, String> {
    let client = Client::new();
    let url = format!("{api_base}/api/tickets");
    let resp = client
        .post(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("failed to create ticket: {e}"))?;

    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(format!("create ticket failed: {err_text}"));
    }

    resp.json::<TicketItem>()
        .await
        .map_err(|e| format!("failed to parse created ticket: {e}"))
}

#[allow(dead_code)]
pub async fn reply_ticket(
    api_base: &str,
    token: &str,
    ticket_id: &str,
    message: &str,
) -> Result<crate::models::TicketMessageItem, String> {
    let client = Client::new();
    let url = format!("{api_base}/api/tickets/{ticket_id}/reply");
    let payload = serde_json::json!({ "message": message });

    let resp = client
        .post(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("reply request failed: {e}"))?;

    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(format!("reply failed: {err_text}"));
    }

    resp.json::<crate::models::TicketMessageItem>()
        .await
        .map_err(|e| format!("failed to parse message: {e}"))
}

pub async fn close_ticket(api_base: &str, token: &str, ticket_id: &str) -> Result<(), String> {
    let client = Client::new();
    let url = format!("{api_base}/api/tickets/{ticket_id}/close");
    let resp = client
        .post(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("close request failed: {e}"))?;

    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(format!("close failed: {err_text}"));
    }

    Ok(())
}

pub async fn fetch_instances(api_base: &str, token: &str) -> Result<Vec<InstanceItem>, String> {
    let client = Client::new();
    let url = format!("{api_base}/api/instances");
    let resp = client
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("failed to load instances: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "instances request failed with status {}",
            resp.status()
        ));
    }

    resp.json::<Vec<InstanceItem>>()
        .await
        .map_err(|e| format!("failed to parse instances response: {e}"))
}

pub async fn fetch_instance_details(
    api_base: &str,
    token: &str,
    id: &str,
) -> Result<InstanceItem, String> {
    let client = Client::new();
    let url = format!("{api_base}/api/instances/{id}");
    let resp = client
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("failed to load instance details: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "instance details request failed with status {}",
            resp.status()
        ));
    }

    resp.json::<InstanceItem>()
        .await
        .map_err(|e| format!("failed to parse instance details response: {e}"))
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct ActionResponse {
    pub message: String,
    pub new_password: Option<String>,
}

pub async fn perform_instance_action(
    api_base: &str,
    token: &str,
    id: &str,
    action: InstanceAction,
) -> Result<ActionResponse, String> {
    let client = Client::new();
    let url = format!("{api_base}/api/instances/{id}/action");
    let payload = ActionRequest { action };

    let resp = client
        .post(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("action request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "action failed".to_string());
        return Err(format!("action failed ({status}): {body}"));
    }

    resp.json::<ActionResponse>()
        .await
        .map_err(|e| format!("failed to parse action response: {e}"))
}

pub async fn fetch_instance_metrics(
    api_base: &str,
    token: &str,
    id: &str,
) -> Result<InstanceMetrics, String> {
    let client = Client::new();
    let url = format!("{api_base}/api/instances/{id}/metrics");
    let resp = client
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("failed to load metrics: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "metrics request failed with status {}",
            resp.status()
        ));
    }

    resp.json::<InstanceMetrics>()
        .await
        .map_err(|e| format!("failed to parse metrics response: {e}"))
}

pub async fn fetch_nat_mappings(
    api_base: &str,
    token: &str,
    instance_id: &str,
) -> Result<Vec<crate::models::NatMappingItem>, String> {
    let client = Client::new();
    let url = format!("{api_base}/api/instances/{instance_id}/nat-mappings");
    let resp = client
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("failed to load nat mappings: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "nat mappings request failed with status {}",
            resp.status()
        ));
    }

    resp.json::<Vec<crate::models::NatMappingItem>>()
        .await
        .map_err(|e| format!("failed to parse nat mappings response: {e}"))
}

pub async fn create_nat_mapping(
    api_base: &str,
    token: &str,
    instance_id: &str,
    payload: &crate::models::CreateNatMappingRequest,
) -> Result<crate::models::NatMappingItem, String> {
    let client = Client::new();
    let url = format!("{api_base}/api/instances/{instance_id}/nat-mappings");
    let resp = client
        .post(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("failed to create nat mapping: {e}"))?;

    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(format!("create nat mapping failed: {err_text}"));
    }

    resp.json::<crate::models::NatMappingItem>()
        .await
        .map_err(|e| format!("failed to parse created nat mapping: {e}"))
}

pub async fn remove_nat_mapping(
    api_base: &str,
    token: &str,
    instance_id: &str,
    mapping_id: &str,
) -> Result<(), String> {
    let client = Client::new();
    let url = format!("{api_base}/api/instances/{instance_id}/nat-mappings/{mapping_id}");
    let resp = client
        .delete(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("failed to remove nat mapping: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("remove nat mapping failed: {}", resp.status()));
    }

    Ok(())
}

pub async fn refund_failed_order(
    api_base: &str,
    token: &str,
    order_id: &str,
) -> Result<(), String> {
    let client = Client::new();
    let url = format!("{api_base}/api/user/balance/refund/{order_id}");
    let resp = client
        .post(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("refund request failed: {e}"))?;

    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(format!("refund failed: {err_text}"));
    }

    Ok(())
}

fn load_persisted_session() -> Option<(String, AuthProfileResponse)> {
    #[cfg(target_arch = "wasm32")]
    {
        use web_sys::window;
        let storage = window().and_then(|w| w.local_storage().ok()).flatten()?;
        let token = storage.get_item(AUTH_TOKEN_KEY).ok().flatten()?;
        let profile = storage.get_item(AUTH_PROFILE_KEY).ok().flatten()?;
        let profile = serde_json::from_str::<AuthProfileResponse>(&profile).ok()?;
        Some((token, profile))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

pub struct BootstrapBundle {
    pub token: String,
    pub profile: AuthProfileResponse,
    pub invoices: Vec<InvoiceItem>,
    pub tickets: Vec<TicketItem>,
    pub instances: Vec<InstanceItem>,
    pub balance: String,
    pub balance_transactions: Vec<crate::models::BalanceTransactionItem>,
}
