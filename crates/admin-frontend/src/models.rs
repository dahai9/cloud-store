use serde::{Deserialize, Serialize};
use dioxus::prelude::*;
use crate::pages::{
    LoginPage, OverviewPage, NodesPage, PlansPage, GuestsPage, TicketsPage, DashboardLayout,
};

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[layout(DashboardLayout)]
        #[route("/")]
        OverviewPage {},
        #[route("/nodes")]
        NodesPage {},
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

#[derive(Clone, Deserialize, PartialEq)]
pub struct NodeItem {
    pub id: String,
    pub name: String,
    pub region: String,
    pub total_capacity: i64,
    pub used_capacity: i64,
}

#[derive(Clone, Deserialize, PartialEq)]
pub struct AdminPlanItem {
    pub id: String,
    pub code: String,
    pub name: String,
    pub monthly_price: String,
    pub active: bool,
    pub max_inventory: Option<i64>,
    pub sold_inventory: i64,
}

#[derive(Clone, Serialize)]
pub struct AdminPlanUpdateRequest {
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
