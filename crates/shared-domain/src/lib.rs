use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const DEFAULT_OS_TEMPLATE: &str = "debian/13";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserRole {
    User,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderStatus {
    PendingPayment,
    Paid,
    Provisioning,
    Active,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubscriptionStatus {
    Active,
    GracePeriod,
    Suspended,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InvoiceStatus {
    Open,
    Paid,
    Failed,
    Refunded,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TicketCategory {
    Sales,
    AfterSales,
    Billing,
    Network,
    Technical,
    Abuse,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatPlan {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub memory_mb: i32,
    pub storage_gb: i32,
    pub cpu_cores: i32,
    pub cpu_allowance_pct: i32,
    pub bandwidth_mbps: i32,
    pub traffic_gb: i32,
    pub monthly_price: Decimal,
    pub nat_port_limit: i32,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstanceStatus {
    Pending,
    Starting,
    Running,
    Stopped,
    Suspended,
    Deleted,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: Uuid,
    pub name: String,
    pub region: String,
    pub cpu_cores_total: i32,
    pub memory_mb_total: i32,
    pub storage_gb_total: i32,
    pub cpu_cores_used: i32,
    pub memory_mb_used: i32,
    pub storage_gb_used: i32,
    pub api_endpoint: Option<String>,
    pub api_token: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: Uuid,
    pub user_id: Uuid,
    pub node_id: Uuid,
    pub order_id: Uuid,
    pub plan_id: Uuid,
    pub provider_instance_id: Option<String>,
    pub root_password: Option<String>,
    pub status: InstanceStatus,
    pub os_template: String,
    pub auto_renew: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatMapping {
    pub id: Uuid,
    pub instance_id: Uuid,
    pub internal_port: i32,
    pub external_port: i32,
    pub protocol: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatPortLease {
    pub id: Uuid,
    pub node_id: Uuid,
    pub public_ip: String,
    pub start_port: i32,
    pub end_port: i32,
    pub reserved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub plan_id: Uuid,
    pub status: OrderStatus,
    pub total_amount: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: Uuid,
    pub user_id: Uuid,
    pub order_id: Option<Uuid>,
    pub external_payment_ref: Option<String>,
    pub amount: Decimal,
    pub currency: String,
    pub status: InvoiceStatus,
    pub due_at: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportTicket {
    pub id: Uuid,
    pub user_id: Uuid,
    pub category: TicketCategory,
    pub priority: TicketPriority,
    pub subject: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("invalid state transition")]
    InvalidStateTransition,
    #[error("invalid amount")]
    InvalidAmount,
    #[error("resource not found")]
    NotFound,
}

pub fn validate_positive_amount(amount: Decimal) -> Result<(), DomainError> {
    if amount <= Decimal::ZERO {
        return Err(DomainError::InvalidAmount);
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BalanceTransactionType {
    Recharge,
    Refund,
    AutoRenew,
    AdminAdjustment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceTransaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub amount: Decimal,
    pub r#type: BalanceTransactionType,
    pub description: String,
    pub created_at: DateTime<Utc>,
}
