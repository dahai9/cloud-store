use crate::pages::{
    BalancePage, ConsolePage, InstanceDetailPage, LoginPage, OrderPage, ProfilePage, ServicesPage,
    StorefrontPage, TicketsPage,
};

use dioxus::prelude::*;

use serde::{Deserialize, Serialize};

#[derive(Clone, Routable, Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum Route {
    #[route("/")]
    StorefrontPage {},
    #[route("/order?:plan")]
    OrderPage { plan: String },
    #[route("/login?:source&:plan")]
    LoginPage {
        source: Option<String>,
        plan: Option<String>,
    },
    #[route("/app/profile")]
    ProfilePage {},
    #[route("/app/services")]
    ServicesPage {},
    #[route("/app/tickets")]
    TicketsPage {},
    #[route("/app/balance")]
    BalancePage {},
    #[route("/app/instances/:id")]
    InstanceDetailPage { id: String },
    #[route("/app/instances/:id/console")]
    ConsolePage { id: String },
}

#[derive(Clone, Copy, PartialEq)]
pub enum DashboardTab {
    Profile,
    Services,
    Tickets,
    Balance,
}

#[derive(Clone, Deserialize, PartialEq)]
pub struct PublicPlanItem {
    pub id: String,
    pub code: String,
    pub name: String,
    pub monthly_price: String,
    pub memory_mb: i64,
    pub storage_gb: i64,
    pub cpu_cores: i64,
    pub cpu_allowance_pct: i64,
    pub bandwidth_mbps: i64,
    pub traffic_gb: i64,
    pub max_inventory: Option<i64>,
    pub sold_inventory: i64,
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

#[derive(Clone, Deserialize)]
pub struct InvoiceItem {
    pub id: String,
    pub amount: String,
    pub status: String,
    pub order_id: Option<String>,
    pub external_payment_ref: Option<String>,
    pub due_at: String,
    pub created_at: String,
    pub paid_at: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct TicketItem {
    pub id: String,
    pub subject: String,
    pub category: String,
    pub priority: String,
    pub status: String,
}

#[derive(Clone, Serialize)]
pub struct PayPalCreateOrderRequest {
    pub plan_code: String,
}

#[derive(Clone, Deserialize)]
#[allow(dead_code)]
pub struct PayPalCreateOrderResponse {
    pub order_id: String,
    pub invoice_id: String,
    pub paypal_order_id: String,
    pub approval_url: String,
    pub amount: String,
    pub currency: String,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct InstanceItem {
    pub id: String,
    pub node_id: String,
    pub plan_id: String,
    pub status: String,
    pub os_template: String,
    pub root_password: Option<String>,
    pub created_at: String,
    pub nat_ip: Option<String>,
    pub nat_port_range: Option<String>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct NatMappingItem {
    pub id: String,
    pub internal_port: i32,
    pub external_port: i32,
    pub protocol: String,
    pub created_at: String,
}

#[derive(Clone, Serialize)]
pub struct CreateNatMappingRequest {
    pub internal_port: i32,
    pub external_port: i32,
    pub protocol: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstanceAction {
    Start,
    Stop,
    Restart,
    ResetPassword { new_password: Option<String> },
    Reinstall { os_template: Option<String> },
}

#[derive(Clone, Serialize)]
pub struct ActionRequest {
    pub action: InstanceAction,
}

#[derive(Clone, Deserialize, PartialEq)]
pub struct InstanceMetrics {
    pub status: String,
    pub cpu_usage_percent: f64,
    pub memory_used_mb: f64,
    pub network_tx_bytes: u64,
    pub network_rx_bytes: u64,
}

#[derive(Clone, Deserialize)]
#[allow(dead_code)]
pub struct ConsoleToken {
    pub url: String,
    pub token: String,
}

#[derive(Clone)]
pub struct SessionState {
    pub api_base: String,
    pub token: Option<String>,
    pub profile: Option<AuthProfileResponse>,
    pub public_plans: Vec<PublicPlanItem>,
    pub invoices: Vec<InvoiceItem>,
    pub tickets: Vec<TicketItem>,
    pub instances: Vec<InstanceItem>,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AuthTransportRisk {
    Secure,
    LoopbackDev,
    InsecureRemote,
}

impl SessionState {
    pub fn new(api_base: String) -> Self {
        Self {
            api_base,
            token: None,
            profile: None,
            public_plans: vec![],
            invoices: vec![],
            tickets: vec![],
            instances: vec![],
            loading: false,
            error: None,
        }
    }
}
