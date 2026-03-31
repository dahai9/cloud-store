#[cfg(target_arch = "wasm32")]
use crate::pages::{
    BalancePage, LoginPage, OrderPage, ProfilePage, ServicesPage, StorefrontPage, TicketsPage,
};
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
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
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, PartialEq)]
pub enum DashboardTab {
    Profile,
    Services,
    Tickets,
    Balance,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
pub struct ProductPlan {
    pub code: &'static str,
    pub name: &'static str,
    pub spec: &'static str,
    pub monthly_price: &'static str,
    pub badge: &'static str,
}

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Serialize)]
pub struct AuthPayload {
    pub email: String,
    pub password: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Deserialize)]
pub struct AuthTokenResponse {
    pub token: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Deserialize, Serialize)]
pub struct AuthProfileResponse {
    pub user_id: String,
    pub email: String,
    pub role: String,
}

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Deserialize)]
pub struct TicketItem {
    pub id: String,
    pub subject: String,
    pub category: String,
    pub priority: String,
    pub status: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Serialize)]
pub struct PayPalCreateOrderRequest {
    pub plan_code: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Deserialize)]
pub struct PayPalCreateOrderResponse {
    pub order_id: String,
    pub invoice_id: String,
    pub paypal_order_id: String,
    pub approval_url: String,
    pub amount: String,
    pub currency: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
pub struct SessionState {
    pub api_base: String,
    pub token: Option<String>,
    pub profile: Option<AuthProfileResponse>,
    pub invoices: Vec<InvoiceItem>,
    pub tickets: Vec<TicketItem>,
    pub loading: bool,
    pub error: Option<String>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, PartialEq)]
pub enum AuthTransportRisk {
    Secure,
    LoopbackDev,
    InsecureRemote,
}

#[cfg(target_arch = "wasm32")]
impl SessionState {
    pub fn new(api_base: String) -> Self {
        Self {
            api_base,
            token: None,
            profile: None,
            invoices: vec![],
            tickets: vec![],
            loading: false,
            error: None,
        }
    }
}
