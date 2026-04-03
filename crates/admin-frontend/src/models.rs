use crate::pages::{
    DashboardLayout, GuestsPage, InstancesPage, LoginPage, NodesPage, OverviewPage, PlansPage,
    TicketsPage,
};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Routable, Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum Route {
    #[layout(DashboardLayout)]
    #[route("/")]
    OverviewPage {},
    #[route("/nodes")]
    NodesPage {},
    #[route("/instances")]
    InstancesPage {},
    #[route("/plans")]
    PlansPage {},
    #[route("/guests")]
    GuestsPage {},
    #[route("/tickets")]
    TicketsPage {},
    #[end_layout]
    #[route("/login")]
    LoginPage {},
}

#[derive(Clone, Serialize)]
pub struct AuthPayload {
    pub email: String,
    pub password: String,
}

#[derive(Clone, Deserialize)]
pub struct AuthTokenResponse {
    pub token: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AuthProfileResponse {
    pub user_id: String,
    pub email: String,
    pub role: String,
}

#[derive(Clone, Deserialize, PartialEq, Serialize)]
pub struct NodeItem {
    pub id: String,
    pub name: String,
    pub region: String,
    pub cpu_cores_total: i64,
    pub memory_mb_total: i64,
    pub storage_gb_total: i64,
    pub cpu_cores_used: i64,
    pub memory_mb_used: i64,
    pub storage_gb_used: i64,
    pub api_endpoint: Option<String>,
    pub api_token: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct NodeCreateRequest {
    pub name: String,
    pub region: String,
    pub cpu_cores_total: i64,
    pub memory_mb_total: i64,
    pub storage_gb_total: i64,
    pub api_endpoint: Option<String>,
    pub api_token: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct NodeUpdateRequest {
    pub name: Option<String>,
    pub region: Option<String>,
    pub cpu_cores_total: Option<i64>,
    pub memory_mb_total: Option<i64>,
    pub storage_gb_total: Option<i64>,
    pub api_endpoint: Option<String>,
    pub api_token: Option<String>,
}

#[derive(Clone, Deserialize, PartialEq)]
pub struct InstanceItem {
    pub id: String,
    pub user_email: String,
    pub node_name: String,
    pub plan_name: String,
    pub status: String,
    pub os_template: String,
    pub created_at: String,
}

#[derive(Clone, Deserialize, PartialEq)]
pub struct AdminPlanItem {
    pub id: String,
    pub code: String,
    pub name: String,
    pub monthly_price: String,
    pub memory_mb: i64,
    pub storage_gb: i64,
    pub cpu_cores: i64,
    pub bandwidth_mbps: i64,
    pub traffic_gb: i64,
    pub active: bool,
    pub max_inventory: Option<i64>,
    pub sold_inventory: i64,
}

#[derive(Clone, Serialize)]
pub struct AdminPlanCreateRequest {
    pub code: String,
    pub name: String,
    pub monthly_price: String,
    pub memory_mb: i64,
    pub storage_gb: i64,
    pub cpu_cores: i64,
    pub bandwidth_mbps: i64,
    pub traffic_gb: i64,
}

#[derive(Clone, Serialize)]
pub struct AdminPlanUpdateRequest {
    pub code: Option<String>,
    pub name: Option<String>,
    pub monthly_price: Option<String>,
    pub memory_mb: Option<i64>,
    pub storage_gb: Option<i64>,
    pub cpu_cores: Option<i64>,
    pub bandwidth_mbps: Option<i64>,
    pub traffic_gb: Option<i64>,
    pub active: Option<bool>,
    pub max_inventory: Option<i64>,
}

#[derive(Clone, Deserialize, PartialEq)]
pub struct GuestItem {
    pub id: String,
    pub email: String,
    pub disabled: bool,
    pub created_at: String,
}

#[derive(Clone, Serialize)]
pub struct GuestUpdateRequest {
    pub disabled: bool,
}

#[derive(Clone, Deserialize, PartialEq)]
pub struct TicketItem {
    pub id: String,
    pub subject: String,
    pub category: String,
    pub priority: String,
    pub status: String,
}

#[derive(Clone, Serialize)]
pub struct TicketStatusUpdateRequest {
    pub status: String,
}

#[derive(Clone, Serialize)]
pub struct TicketReplyRequest {
    pub message: String,
}

#[derive(Clone, Default)]
pub struct AdminSessionState {
    pub api_base: String,
    pub token: Option<String>,
    pub profile: Option<AuthProfileResponse>,
    pub nodes: Vec<NodeItem>,
    pub instances: Vec<InstanceItem>,
    pub plans: Vec<AdminPlanItem>,
    pub guests: Vec<GuestItem>,
    pub tickets: Vec<TicketItem>,
    pub loading: bool,
    pub notice: Option<String>,
    pub error: Option<String>,
}

impl AdminSessionState {
    pub fn new(api_base: String) -> Self {
        Self {
            api_base,
            ..Default::default()
        }
    }
}
