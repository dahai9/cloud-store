use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use shared_domain::NatPlan;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionRequest {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub node_id: Uuid,
    pub plan: NatPlan,
    pub os_template: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConnection {
    pub endpoint: String,
    pub token: Option<String>,
}

#[async_trait]
pub trait ComputeProvider: Send + Sync {
    async fn provision_instance(
        &self,
        node: &NodeConnection,
        req: ProvisionRequest,
    ) -> anyhow::Result<ProvisionResult>;
    async fn attach_nat_ports(
        &self,
        node: &NodeConnection,
        req: PortBindingRequest,
    ) -> anyhow::Result<()>;
    async fn suspend_instance(
        &self,
        node: &NodeConnection,
        instance_id: &str,
    ) -> anyhow::Result<()>;
    async fn resume_instance(&self, node: &NodeConnection, instance_id: &str)
        -> anyhow::Result<()>;
    async fn destroy_instance(
        &self,
        node: &NodeConnection,
        instance_id: &str,
    ) -> anyhow::Result<()>;

    // New methods for Guest Management
    async fn start_instance(&self, node: &NodeConnection, instance_id: &str) -> anyhow::Result<()>;
    async fn stop_instance(&self, node: &NodeConnection, instance_id: &str) -> anyhow::Result<()>;
    async fn restart_instance(
        &self,
        node: &NodeConnection,
        instance_id: &str,
    ) -> anyhow::Result<()>;
    async fn reset_password(
        &self,
        node: &NodeConnection,
        instance_id: &str,
        new_password: &str,
    ) -> anyhow::Result<()>;
    async fn reinstall_instance(
        &self,
        node: &NodeConnection,
        instance_id: &str,
        os_template: &str,
    ) -> anyhow::Result<()>;
    async fn get_metrics(
        &self,
        node: &NodeConnection,
        instance_id: &str,
    ) -> anyhow::Result<InstanceMetrics>;
    async fn get_console_token(
        &self,
        node: &NodeConnection,
        instance_id: &str,
    ) -> anyhow::Result<ConsoleToken>;
}

pub struct IncusProvider {
    client_cert_pem: String,
    client_key_pem: String,
}

impl IncusProvider {
    pub fn new() -> anyhow::Result<Self> {
        let key_pair = rcgen::KeyPair::generate()?;
        let mut params = rcgen::CertificateParams::new(vec!["cloud-store-manager".to_string()])?;
        params
            .distinguished_name
            .push(rcgen::DnType::CommonName, "cloud-store-manager");
        let cert = params.self_signed(&key_pair)?;

        Ok(Self {
            client_cert_pem: cert.pem(),
            client_key_pem: key_pair.serialize_pem(),
        })
    }

    async fn get_client(&self) -> anyhow::Result<Client> {
        let identity = reqwest::Identity::from_pem(
            (self.client_cert_pem.clone() + &self.client_key_pem).as_bytes(),
        )?;

        let client = Client::builder()
            .identity(identity)
            .danger_accept_invalid_certs(true)
            .build()?;

        Ok(client)
    }

    async fn ensure_trusted(&self, node: &NodeConnection) -> anyhow::Result<()> {
        let client = self.get_client().await?;

        let res = client.get(format!("{}/1.0", node.endpoint)).send().await?;
        let body: serde_json::Value = res.json().await?;

        if body["metadata"]["auth"] == "trusted" {
            return Ok(());
        }

        if let Some(password) = &node.token {
            let trust_req = serde_json::json!({
                "type": "client",
                "password": password,
            });

            let res = client
                .post(format!("{}/1.0/certificates", node.endpoint))
                .json(&trust_req)
                .send()
                .await?;

            if !res.status().is_success() {
                anyhow::bail!(
                    "failed to trust client certificate on node: {}",
                    res.text().await?
                );
            }

            Ok(())
        } else {
            anyhow::bail!("node is not trusted and no trust password provided");
        }
    }
}

#[async_trait]
impl ComputeProvider for IncusProvider {
    async fn provision_instance(
        &self,
        node: &NodeConnection,
        req: ProvisionRequest,
    ) -> anyhow::Result<ProvisionResult> {
        self.ensure_trusted(node).await?;
        let client = self.get_client().await?;

        let instance_name = format!("cs-{}", req.order_id.to_string().split('-').next().unwrap());

        let create_req = serde_json::json!({
            "name": instance_name,
            "source": {
                "type": "image",
                "alias": req.os_template,
                "server": "https://images.linuxcontainers.org",
                "protocol": "simplestreams"
            },
            "config": {
                "limits.cpu": format!("{}", req.plan.cpu_cores),
                "limits.memory": format!("{}MB", req.plan.memory_mb),
            },
            "devices": {
                "root": {
                    "path": "/",
                    "pool": "default",
                    "size": format!("{}GB", req.plan.storage_gb),
                    "type": "disk"
                }
            }
        });

        let res = client
            .post(format!("{}/1.0/instances", node.endpoint))
            .json(&create_req)
            .send()
            .await?;

        if !res.status().is_success() {
            anyhow::bail!("failed to create incus instance: {}", res.text().await?);
        }

        let _ = self.start_instance(node, &instance_name).await;

        Ok(ProvisionResult {
            instance_id: instance_name,
            internal_ip: "auto".to_string(),
            node_id: req.node_id,
        })
    }

    async fn start_instance(&self, node: &NodeConnection, instance_id: &str) -> anyhow::Result<()> {
        self.ensure_trusted(node).await?;
        let client = self.get_client().await?;

        let res = client
            .put(format!(
                "{}/1.0/instances/{}/state",
                node.endpoint, instance_id
            ))
            .json(&serde_json::json!({"action": "start"}))
            .send()
            .await?;

        if !res.status().is_success() {
            anyhow::bail!("failed to start: {}", res.text().await?);
        }
        Ok(())
    }

    async fn stop_instance(&self, node: &NodeConnection, instance_id: &str) -> anyhow::Result<()> {
        self.ensure_trusted(node).await?;
        let client = self.get_client().await?;

        let res = client
            .put(format!(
                "{}/1.0/instances/{}/state",
                node.endpoint, instance_id
            ))
            .json(&serde_json::json!({"action": "stop", "force": true}))
            .send()
            .await?;

        if !res.status().is_success() {
            anyhow::bail!("failed to stop: {}", res.text().await?);
        }
        Ok(())
    }

    async fn restart_instance(
        &self,
        node: &NodeConnection,
        instance_id: &str,
    ) -> anyhow::Result<()> {
        self.ensure_trusted(node).await?;
        let client = self.get_client().await?;

        let res = client
            .put(format!(
                "{}/1.0/instances/{}/state",
                node.endpoint, instance_id
            ))
            .json(&serde_json::json!({"action": "restart"}))
            .send()
            .await?;

        if !res.status().is_success() {
            anyhow::bail!("failed to restart: {}", res.text().await?);
        }
        Ok(())
    }

    async fn reset_password(
        &self,
        node: &NodeConnection,
        instance_id: &str,
        new_password: &str,
    ) -> anyhow::Result<()> {
        self.ensure_trusted(node).await?;
        let client = self.get_client().await?;

        let cmd = format!("echo 'root:{}' | chpasswd", new_password);
        let res = client
            .post(format!(
                "{}/1.0/instances/{}/exec",
                node.endpoint, instance_id
            ))
            .json(&serde_json::json!({
                "command": ["bash", "-c", cmd],
                "wait-for-variable": true
            }))
            .send()
            .await?;

        if !res.status().is_success() {
            anyhow::bail!("failed to reset password: {}", res.text().await?);
        }
        Ok(())
    }

    async fn reinstall_instance(
        &self,
        node: &NodeConnection,
        instance_id: &str,
        _os_template: &str,
    ) -> anyhow::Result<()> {
        self.destroy_instance(node, instance_id).await?;
        Ok(())
    }

    async fn get_metrics(
        &self,
        node: &NodeConnection,
        instance_id: &str,
    ) -> anyhow::Result<InstanceMetrics> {
        self.ensure_trusted(node).await?;
        let client = self.get_client().await?;

        let res = client
            .get(format!(
                "{}/1.0/instances/{}/state",
                node.endpoint, instance_id
            ))
            .send()
            .await?;
        let body: serde_json::Value = res.json().await?;

        let cpu_usage = body["metadata"]["cpu"]["usage"].as_f64().unwrap_or(0.0) / 1_000_000_000.0;
        let mem_usage =
            body["metadata"]["memory"]["usage"].as_f64().unwrap_or(0.0) / 1024.0 / 1024.0;

        Ok(InstanceMetrics {
            cpu_usage_percent: cpu_usage,
            memory_used_mb: mem_usage,
            network_tx_bytes: 0,
            network_rx_bytes: 0,
        })
    }

    async fn get_console_token(
        &self,
        node: &NodeConnection,
        instance_id: &str,
    ) -> anyhow::Result<ConsoleToken> {
        self.ensure_trusted(node).await?;
        let client = self.get_client().await?;

        let res = client
            .post(format!(
                "{}/1.0/instances/{}/console",
                node.endpoint, instance_id
            ))
            .json(&serde_json::json!({"type": "vga"}))
            .send()
            .await?;

        let body: serde_json::Value = res.json().await?;
        let op_id = body["operation"]
            .as_str()
            .unwrap_or_default()
            .split('/')
            .last()
            .unwrap_or_default();
        let fds = &body["metadata"]["fds"];
        let control_secret = fds["control"].as_str().unwrap_or_default();

        Ok(ConsoleToken {
            url: format!(
                "ws{}/1.0/operations/{}/websocket?secret={}",
                &node.endpoint[4..],
                op_id,
                control_secret
            ),
            token: control_secret.to_string(),
        })
    }

    async fn attach_nat_ports(
        &self,
        _node: &NodeConnection,
        _req: PortBindingRequest,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn suspend_instance(
        &self,
        _node: &NodeConnection,
        _instance_id: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn resume_instance(
        &self,
        _node: &NodeConnection,
        _instance_id: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn destroy_instance(
        &self,
        node: &NodeConnection,
        instance_id: &str,
    ) -> anyhow::Result<()> {
        self.ensure_trusted(node).await?;
        let client = self.get_client().await?;
        let _ = self.stop_instance(node, instance_id).await;
        let res = client
            .delete(format!("{}/1.0/instances/{}", node.endpoint, instance_id))
            .send()
            .await?;
        if !res.status().is_success() {
            anyhow::bail!("failed to destroy: {}", res.text().await?);
        }
        Ok(())
    }
}

pub struct StubProvider;

#[async_trait]
impl ComputeProvider for StubProvider {
    async fn provision_instance(
        &self,
        _node: &NodeConnection,
        req: ProvisionRequest,
    ) -> anyhow::Result<ProvisionResult> {
        Ok(ProvisionResult {
            instance_id: format!("stub-{}", req.order_id),
            internal_ip: "10.10.0.10".to_string(),
            node_id: req.node_id,
        })
    }

    async fn attach_nat_ports(
        &self,
        _node: &NodeConnection,
        _req: PortBindingRequest,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn suspend_instance(
        &self,
        _node: &NodeConnection,
        _instance_id: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn resume_instance(
        &self,
        _node: &NodeConnection,
        _instance_id: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn destroy_instance(
        &self,
        _node: &NodeConnection,
        _instance_id: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn start_instance(
        &self,
        _node: &NodeConnection,
        _instance_id: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn stop_instance(
        &self,
        _node: &NodeConnection,
        _instance_id: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn restart_instance(
        &self,
        _node: &NodeConnection,
        _instance_id: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn reset_password(
        &self,
        _node: &NodeConnection,
        _instance_id: &str,
        _new_password: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn reinstall_instance(
        &self,
        _node: &NodeConnection,
        _instance_id: &str,
        _os_template: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn get_metrics(
        &self,
        _node: &NodeConnection,
        _instance_id: &str,
    ) -> anyhow::Result<InstanceMetrics> {
        Ok(InstanceMetrics {
            cpu_usage_percent: 5.0,
            memory_used_mb: 256.0,
            network_tx_bytes: 1024,
            network_rx_bytes: 2048,
        })
    }

    async fn get_console_token(
        &self,
        _node: &NodeConnection,
        _instance_id: &str,
    ) -> anyhow::Result<ConsoleToken> {
        Ok(ConsoleToken {
            url: "ws://localhost/stub-console".to_string(),
            token: "stub-token".to_string(),
        })
    }
}
