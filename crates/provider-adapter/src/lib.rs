use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use shared_domain::NatPlan;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionRequest {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub node_id: Uuid,
    pub plan: NatPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionResult {
    pub instance_id: String,
    pub internal_ip: String,
    pub node_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortBindingRequest {
    pub instance_id: String,
    pub public_ip: String,
    pub start_port: i32,
    pub end_port: i32,
}

#[async_trait]
pub trait ComputeProvider: Send + Sync {
    async fn provision_instance(&self, req: ProvisionRequest) -> anyhow::Result<ProvisionResult>;
    async fn attach_nat_ports(&self, req: PortBindingRequest) -> anyhow::Result<()>;
    async fn suspend_instance(&self, instance_id: &str) -> anyhow::Result<()>;
    async fn resume_instance(&self, instance_id: &str) -> anyhow::Result<()>;
    async fn destroy_instance(&self, instance_id: &str) -> anyhow::Result<()>;
}

pub struct StubProvider;

#[async_trait]
impl ComputeProvider for StubProvider {
    async fn provision_instance(&self, req: ProvisionRequest) -> anyhow::Result<ProvisionResult> {
        Ok(ProvisionResult {
            instance_id: format!("stub-{}", req.order_id),
            internal_ip: "10.10.0.10".to_string(),
            node_id: req.node_id,
        })
    }

    async fn attach_nat_ports(&self, _req: PortBindingRequest) -> anyhow::Result<()> {
        Ok(())
    }

    async fn suspend_instance(&self, _instance_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn resume_instance(&self, _instance_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn destroy_instance(&self, _instance_id: &str) -> anyhow::Result<()> {
        Ok(())
    }
}
