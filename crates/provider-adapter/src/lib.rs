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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceMetrics {
    pub cpu_usage_percent: f64,
    pub memory_used_mb: f64,
    pub network_tx_bytes: u64,
    pub network_rx_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleToken {
    pub url: String,
    pub token: String,
}

#[async_trait]
pub trait ComputeProvider: Send + Sync {
    async fn provision_instance(&self, req: ProvisionRequest) -> anyhow::Result<ProvisionResult>;
    async fn attach_nat_ports(&self, req: PortBindingRequest) -> anyhow::Result<()>;
    async fn suspend_instance(&self, instance_id: &str) -> anyhow::Result<()>;
    async fn resume_instance(&self, instance_id: &str) -> anyhow::Result<()>;
    async fn destroy_instance(&self, instance_id: &str) -> anyhow::Result<()>;
    
    // New methods for Guest Management
    async fn start_instance(&self, instance_id: &str) -> anyhow::Result<()>;
    async fn stop_instance(&self, instance_id: &str) -> anyhow::Result<()>;
    async fn restart_instance(&self, instance_id: &str) -> anyhow::Result<()>;
    async fn reset_password(&self, instance_id: &str, new_password: &str) -> anyhow::Result<()>;
    async fn reinstall_instance(&self, instance_id: &str, os_template: &str) -> anyhow::Result<()>;
    async fn get_metrics(&self, instance_id: &str) -> anyhow::Result<InstanceMetrics>;
    async fn get_console_token(&self, instance_id: &str) -> anyhow::Result<ConsoleToken>;
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

    async fn start_instance(&self, _instance_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn stop_instance(&self, _instance_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn restart_instance(&self, _instance_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn reset_password(&self, _instance_id: &str, _new_password: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn reinstall_instance(&self, _instance_id: &str, _os_template: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn get_metrics(&self, _instance_id: &str) -> anyhow::Result<InstanceMetrics> {
        Ok(InstanceMetrics {
            cpu_usage_percent: 5.0,
            memory_used_mb: 256.0,
            network_tx_bytes: 1024,
            network_rx_bytes: 2048,
        })
    }

    async fn get_console_token(&self, _instance_id: &str) -> anyhow::Result<ConsoleToken> {
        Ok(ConsoleToken {
            url: "ws://localhost/stub-console".to_string(),
            token: "stub-token".to_string(),
        })
    }
}
