use gloo_net::http::Request;
use crate::models::*;

pub fn default_api_base() -> String {
    option_env!("ADMIN_API_BASE_URL")
        .unwrap_or("http://127.0.0.1:8082")
        .to_string()
}

pub async fn login(api_base: &str, payload: &AuthPayload) -> Result<AuthTokenResponse, String> {
    Request::post(&format!("{api_base}/api/auth/login"))
        .header("Content-Type", "application/json")
        .json(payload)
        .map_err(|e| format!("Failed to build request: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .json::<AuthTokenResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {e}"))
}

pub async fn get_profile(api_base: &str, token: &str) -> Result<AuthProfileResponse, String> {
    Request::get(&format!("{api_base}/api/auth/me"))
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .json::<AuthProfileResponse>()
        .await
        .map_err(|e| format!("Failed to parse profile: {e}"))
}

pub async fn get_nodes(api_base: &str, token: &str) -> Result<Vec<NodeItem>, String> {
    Request::get(&format!("{api_base}/api/admin/nodes"))
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .json::<Vec<NodeItem>>()
        .await
        .map_err(|e| format!("Failed to parse nodes: {e}"))
}

pub async fn get_plans(api_base: &str, token: &str) -> Result<Vec<AdminPlanItem>, String> {
    Request::get(&format!("{api_base}/api/admin/plans"))
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .json::<Vec<AdminPlanItem>>()
        .await
        .map_err(|e| format!("Failed to parse plans: {e}"))
}

pub async fn update_plan(
    api_base: &str,
    token: &str,
    plan_id: &str,
    payload: &AdminPlanUpdateRequest,
) -> Result<(), String> {
    let resp = Request::patch(&format!("{api_base}/api/admin/plans/{plan_id}"))
        .header("Authorization", &format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .json(payload)
        .map_err(|e| format!("Failed to build request: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.ok() {
        return Err(format!("Update failed: {}", resp.status()));
    }
    Ok(())
}

pub async fn get_guests(api_base: &str, token: &str) -> Result<Vec<GuestItem>, String> {
    Request::get(&format!("{api_base}/api/admin/guests"))
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .json::<Vec<GuestItem>>()
        .await
        .map_err(|e| format!("Failed to parse guests: {e}"))
}

pub async fn update_guest(
    api_base: &str,
    token: &str,
    user_id: &str,
    payload: &GuestUpdateRequest,
) -> Result<(), String> {
    let resp = Request::patch(&format!("{api_base}/api/admin/guests/{user_id}"))
        .header("Authorization", &format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .json(payload)
        .map_err(|e| format!("Failed to build request: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.ok() {
        return Err(format!("Update failed: {}", resp.status()));
    }
    Ok(())
}

pub async fn get_tickets(api_base: &str, token: &str) -> Result<Vec<TicketItem>, String> {
    Request::get(&format!("{api_base}/api/admin/tickets"))
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?
        .json::<Vec<TicketItem>>()
        .await
        .map_err(|e| format!("Failed to parse tickets: {e}"))
}

pub async fn update_ticket_status(
    api_base: &str,
    token: &str,
    ticket_id: &str,
    payload: &TicketStatusUpdateRequest,
) -> Result<(), String> {
    let resp = Request::patch(&format!("{api_base}/api/admin/tickets/{ticket_id}/status"))
        .header("Authorization", &format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .json(payload)
        .map_err(|e| format!("Failed to build request: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.ok() {
        return Err(format!("Update failed: {}", resp.status()));
    }
    Ok(())
}

pub async fn reply_ticket(
    api_base: &str,
    token: &str,
    ticket_id: &str,
    payload: &TicketReplyRequest,
) -> Result<(), String> {
    let resp = Request::post(&format!("{api_base}/api/admin/tickets/{ticket_id}/reply"))
        .header("Authorization", &format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .json(payload)
        .map_err(|e| format!("Failed to build request: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.ok() {
        return Err(format!("Reply failed: {}", resp.status()));
    }
    Ok(())
}
