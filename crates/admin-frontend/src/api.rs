use crate::models::{
    AdminPlanCreateRequest, AdminPlanItem, AdminPlanUpdateRequest, AuthPayload,
    AuthProfileResponse, AuthTokenResponse, GuestItem, GuestUpdateRequest, InstanceItem,
    NatPortLeaseCreateRequest, NatPortLeaseItem, NodeCreateRequest, NodeItem, NodeUpdateRequest,
    TicketItem, TicketReplyRequest, TicketStatusUpdateRequest,
};
use reqwest::Client;

pub fn default_api_base() -> String {
    option_env!("ADMIN_API_BASE_URL")
        .unwrap_or("http://127.0.0.1:8082")
        .to_string()
}

pub async fn login(api_base: &str, payload: &AuthPayload) -> Result<AuthTokenResponse, String> {
    let client = Client::new();
    let resp = client
        .post(format!("{api_base}/api/auth/login"))
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Login failed: {}", resp.status()));
    }

    resp.json::<AuthTokenResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {e}"))
}

pub async fn get_profile(api_base: &str, token: &str) -> Result<AuthProfileResponse, String> {
    let client = Client::new();
    let resp = client
        .get(format!("{api_base}/api/auth/me"))
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Profile request failed: {}", resp.status()));
    }

    resp.json::<AuthProfileResponse>()
        .await
        .map_err(|e| format!("Failed to parse profile: {e}"))
}

pub async fn get_nodes(api_base: &str, token: &str) -> Result<Vec<NodeItem>, String> {
    let client = Client::new();
    let resp = client
        .get(format!("{api_base}/api/admin/nodes"))
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Nodes request failed: {}", resp.status()));
    }

    resp.json::<Vec<NodeItem>>()
        .await
        .map_err(|e| format!("Failed to parse nodes: {e}"))
}

pub async fn create_node(
    api_base: &str,
    token: &str,
    payload: &NodeCreateRequest,
) -> Result<NodeItem, String> {
    let client = Client::new();
    let resp = client
        .post(format!("{api_base}/api/admin/nodes"))
        .header("Authorization", &format!("Bearer {token}"))
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Create failed: {}", resp.status()));
    }
    resp.json::<NodeItem>()
        .await
        .map_err(|e| format!("Failed to parse response: {e}"))
}

pub async fn get_nat_port_leases(
    api_base: &str,
    token: &str,
) -> Result<Vec<NatPortLeaseItem>, String> {
    let client = Client::new();
    let resp = client
        .get(format!("{api_base}/api/admin/nat-port-leases"))
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Nat port lease request failed: {}", resp.status()));
    }

    resp.json::<Vec<NatPortLeaseItem>>()
        .await
        .map_err(|e| format!("Failed to parse nat port leases: {e}"))
}

pub async fn create_nat_port_lease(
    api_base: &str,
    token: &str,
    payload: &NatPortLeaseCreateRequest,
) -> Result<NatPortLeaseItem, String> {
    let client = Client::new();
    let resp = client
        .post(format!("{api_base}/api/admin/nat-port-leases"))
        .header("Authorization", &format!("Bearer {token}"))
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Create failed: {}", resp.status()));
    }

    resp.json::<NatPortLeaseItem>()
        .await
        .map_err(|e| format!("Failed to parse response: {e}"))
}

pub async fn delete_nat_port_lease(
    api_base: &str,
    token: &str,
    lease_id: &str,
) -> Result<(), String> {
    let client = Client::new();
    let resp = client
        .delete(format!("{api_base}/api/admin/nat-port-leases/{lease_id}"))
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Delete failed: {}", resp.status()));
    }

    Ok(())
}

pub async fn update_node(
    api_base: &str,
    token: &str,
    node_id: &str,
    payload: &NodeUpdateRequest,
) -> Result<NodeItem, String> {
    let client = Client::new();
    let resp = client
        .patch(format!("{api_base}/api/admin/nodes/{node_id}"))
        .header("Authorization", &format!("Bearer {token}"))
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Update failed: {}", resp.status()));
    }
    resp.json::<NodeItem>()
        .await
        .map_err(|e| format!("Failed to parse response: {e}"))
}

pub async fn get_instances(api_base: &str, token: &str) -> Result<Vec<InstanceItem>, String> {
    let client = Client::new();
    let resp = client
        .get(format!("{api_base}/api/admin/instances"))
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Instances request failed: {}", resp.status()));
    }

    resp.json::<Vec<InstanceItem>>()
        .await
        .map_err(|e| format!("Failed to parse instances: {e}"))
}

pub async fn get_plans(api_base: &str, token: &str) -> Result<Vec<AdminPlanItem>, String> {
    let client = Client::new();
    let resp = client
        .get(format!("{api_base}/api/admin/plans"))
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Plans request failed: {}", resp.status()));
    }

    resp.json::<Vec<AdminPlanItem>>()
        .await
        .map_err(|e| format!("Failed to parse plans: {e}"))
}

pub async fn create_plan(
    api_base: &str,
    token: &str,
    payload: &AdminPlanCreateRequest,
) -> Result<AdminPlanItem, String> {
    let client = Client::new();
    let resp = client
        .post(format!("{api_base}/api/admin/plans"))
        .header("Authorization", &format!("Bearer {token}"))
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Create failed: {}", resp.status()));
    }
    resp.json::<AdminPlanItem>()
        .await
        .map_err(|e| format!("Failed to parse response: {e}"))
}

pub async fn update_plan(
    api_base: &str,
    token: &str,
    plan_id: &str,
    payload: &AdminPlanUpdateRequest,
) -> Result<(), String> {
    let client = Client::new();
    let resp = client
        .patch(format!("{api_base}/api/admin/plans/{plan_id}"))
        .header("Authorization", &format!("Bearer {token}"))
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Update failed: {}", resp.status()));
    }
    Ok(())
}

pub async fn get_guests(api_base: &str, token: &str) -> Result<Vec<GuestItem>, String> {
    let client = Client::new();
    let resp = client
        .get(format!("{api_base}/api/admin/guests"))
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Guests request failed: {}", resp.status()));
    }

    resp.json::<Vec<GuestItem>>()
        .await
        .map_err(|e| format!("Failed to parse guests: {e}"))
}

pub async fn update_guest(
    api_base: &str,
    token: &str,
    user_id: &str,
    payload: &GuestUpdateRequest,
) -> Result<(), String> {
    let client = Client::new();
    let resp = client
        .patch(format!("{api_base}/api/admin/guests/{user_id}"))
        .header("Authorization", &format!("Bearer {token}"))
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Update failed: {}", resp.status()));
    }
    Ok(())
}

pub async fn get_tickets(api_base: &str, token: &str) -> Result<Vec<TicketItem>, String> {
    let client = Client::new();
    let resp = client
        .get(format!("{api_base}/api/admin/tickets"))
        .header("Authorization", &format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Tickets request failed: {}", resp.status()));
    }

    resp.json::<Vec<TicketItem>>()
        .await
        .map_err(|e| format!("Failed to parse tickets: {e}"))
}

pub async fn update_ticket_status(
    api_base: &str,
    token: &str,
    ticket_id: &str,
    payload: &TicketStatusUpdateRequest,
) -> Result<(), String> {
    let client = Client::new();
    let resp = client
        .patch(format!("{api_base}/api/admin/tickets/{ticket_id}/status"))
        .header("Authorization", &format!("Bearer {token}"))
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
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
    let client = Client::new();
    let resp = client
        .post(format!("{api_base}/api/admin/tickets/{ticket_id}/reply"))
        .header("Authorization", &format!("Bearer {token}"))
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Reply failed: {}", resp.status()));
    }
    Ok(())
}
