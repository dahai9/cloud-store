
use crate::pages::{
    BalancePage, InstanceDetailPage, LoginPage, OrderPage, ProfilePage, ServicesPage,
    StorefrontPage, TicketsPage,
};

use dioxus::prelude::*;

use serde::{Deserialize, Serialize};


#[derive(Clone, Routable, Debug, PartialEq)]
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
}


#[derive(Clone, Copy, PartialEq)]
pub enum DashboardTab {
    Profile,
    Services,
    Tickets,
    Balance,
}


#[derive(Clone, Copy)]
pub struct ProductPlan {
    pub code: &'static str,
    pub name: &'static str,
    pub spec: &'static str,
    pub monthly_price: &'static str,
    pub badge: &'static str,
}


pub const PLANS: [ProductPlan; 3] = [
    ProductPlan {
        code: "nat-mini",
        name: "NAT Mini",
        spec: "1GB RAM / 50GB SSD / Shared NAT",
        monthly_price: "$5.99",
        badge: "Starter",
    },
    ProductPlan {
        code: "nat-standard",
        name: "NAT Standard",
        spec: "1GB RAM / 50GB SSD / Better traffic priority",
        monthly_price: "$7.99",
        badge: "Most Popular",
    },
    ProductPlan {
        code: "nat-pro",
        name: "NAT Pro",
        spec: "1GB RAM / 50GB SSD / Priority support",
        monthly_price: "$9.99",
        badge: "Business",
    },
];


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
    pub created_at: String,
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

#[derive(Clone, Deserialize)]
pub struct InstanceMetrics {
    pub cpu_usage_percent: f64,
    pub memory_used_mb: f64,
    pub network_tx_bytes: u64,
    pub network_rx_bytes: u64,
}

#[derive(Clone, Deserialize)]
pub struct ConsoleToken {
    pub url: String,
    pub token: String,
}


#[derive(Clone)]
pub struct SessionState {
    pub api_base: String,
    pub token: Option<String>,
    pub profile: Option<AuthProfileResponse>,
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
            invoices: vec![],
            tickets: vec![],
            instances: vec![],
            loading: false,
            error: None,
        }
    }
}
