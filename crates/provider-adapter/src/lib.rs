use anyhow::Context;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use shared_domain::NatPlan;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

const DEFAULT_DATA_DIR: &str = "data";
const INCUS_CLIENT_IDENTITY_FILE: &str = "incus-client.pem";
const INCUS_CLIENT_IDENTITY_LOCK_FILE: &str = "incus-client.pem.lock";
const INCUS_CLIENT_COMMON_NAME: &str = "cloud-store-manager";

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

#[derive(Debug, Deserialize)]
struct IncusResponseEnvelope {
    #[serde(rename = "type")]
    response_type: String,
    #[serde(default)]
    operation: Option<String>,
    #[serde(default)]
    error: String,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct IncusOperationState {
    status: String,
    #[serde(default)]
    err: String,
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
    client_identity_pem: String,
}

impl IncusProvider {
    pub fn new() -> anyhow::Result<Self> {
        let client_identity_pem = Self::load_or_create_identity()?;

        Ok(Self {
            client_identity_pem,
        })
    }

    fn data_dir() -> PathBuf {
        std::env::var("CLOUD_STORE_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_DATA_DIR))
    }

    fn identity_path() -> PathBuf {
        Self::data_dir().join(INCUS_CLIENT_IDENTITY_FILE)
    }

    fn lock_path() -> PathBuf {
        Self::data_dir().join(INCUS_CLIENT_IDENTITY_LOCK_FILE)
    }

    fn load_or_create_identity() -> anyhow::Result<String> {
        let identity_path = Self::identity_path();

        if let Some(identity_pem) = Self::read_valid_identity(&identity_path)? {
            return Ok(identity_pem);
        }

        Self::generate_and_persist_identity(&identity_path)
    }

    fn read_valid_identity(identity_path: &Path) -> anyhow::Result<Option<String>> {
        match fs::read_to_string(identity_path) {
            Ok(identity_pem) => {
                if reqwest::Identity::from_pem(identity_pem.as_bytes()).is_ok() {
                    Ok(Some(identity_pem))
                } else {
                    Ok(None)
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err).with_context(|| {
                format!(
                    "failed to read persisted Incus client identity from {}",
                    identity_path.display()
                )
            }),
        }
    }

    fn generate_and_persist_identity(identity_path: &Path) -> anyhow::Result<String> {
        if let Some(parent) = identity_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create data directory for {}",
                    identity_path.display()
                )
            })?;
        }

        let lock_path = Self::lock_path();
        let started_at = Instant::now();

        loop {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(lock_file) => {
                    let identity_pem = Self::generate_identity_pem()?;
                    let temp_path = identity_path.with_extension("tmp");

                    fs::write(&temp_path, identity_pem.as_bytes()).with_context(|| {
                        format!(
                            "failed to write temporary Incus client identity to {}",
                            temp_path.display()
                        )
                    })?;

                    fs::rename(&temp_path, identity_path).with_context(|| {
                        format!(
                            "failed to persist Incus client identity to {}",
                            identity_path.display()
                        )
                    })?;

                    drop(lock_file);
                    let _ = fs::remove_file(&lock_path);

                    return Ok(identity_pem);
                }
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                    if let Some(identity_pem) = Self::read_valid_identity(identity_path)? {
                        return Ok(identity_pem);
                    }

                    if started_at.elapsed() > Duration::from_secs(5) {
                        let _ = fs::remove_file(&lock_path);
                    } else {
                        thread::sleep(Duration::from_millis(50));
                    }
                }
                Err(err) => {
                    return Err(err).with_context(|| {
                        format!(
                            "failed to acquire Incus client identity lock at {}",
                            lock_path.display()
                        )
                    });
                }
            }
        }
    }

    fn generate_identity_pem() -> anyhow::Result<String> {
        let key_pair = rcgen::KeyPair::generate()?;
        let mut params = rcgen::CertificateParams::new(vec![INCUS_CLIENT_COMMON_NAME.to_string()])?;
        params
            .distinguished_name
            .push(rcgen::DnType::CommonName, INCUS_CLIENT_COMMON_NAME);
        let cert = params.self_signed(&key_pair)?;

        Ok(format!("{}{}", cert.pem(), key_pair.serialize_pem()))
    }

    fn resolve_incus_url(base_endpoint: &str, url: &str) -> String {
        if url.starts_with("http://") || url.starts_with("https://") {
            url.to_owned()
        } else if url.starts_with('/') {
            format!("{}{}", base_endpoint.trim_end_matches('/'), url)
        } else {
            format!("{}/{}", base_endpoint.trim_end_matches('/'), url)
        }
    }

    async fn wait_for_operation(
        &self,
        client: &Client,
        node_endpoint: &str,
        operation_url: &str,
        action: &str,
    ) -> anyhow::Result<()> {
        let wait_url = format!(
            "{}/wait?timeout=-1",
            Self::resolve_incus_url(node_endpoint, operation_url).trim_end_matches('/')
        );

        let res = client
            .get(&wait_url)
            .send()
            .await
            .with_context(|| format!("failed to wait for {action} operation"))?;

        if !res.status().is_success() {
            anyhow::bail!(
                "failed to wait for {action} operation: {}",
                res.text().await?
            );
        }

        let response: IncusResponseEnvelope = res
            .json()
            .await
            .with_context(|| format!("failed to parse {action} operation response"))?;

        let metadata = response.metadata.ok_or_else(|| {
            anyhow::anyhow!("{action} operation did not return final operation state")
        })?;
        let operation: IncusOperationState = serde_json::from_value(metadata)
            .with_context(|| format!("failed to decode {action} operation state"))?;

        if operation.status != "Success" {
            let err = if operation.err.is_empty() {
                operation.status
            } else {
                operation.err
            };

            anyhow::bail!("{action} operation failed: {err}");
        }

        Ok(())
    }

    async fn submit_operation_request(
        &self,
        client: &Client,
        request: reqwest::RequestBuilder,
        node_endpoint: &str,
        action: &str,
    ) -> anyhow::Result<()> {
        let res = request
            .send()
            .await
            .with_context(|| format!("failed to send {action} request"))?;

        if !res.status().is_success() {
            anyhow::bail!("failed to {action}: {}", res.text().await?);
        }

        let response: IncusResponseEnvelope = res
            .json()
            .await
            .with_context(|| format!("failed to parse {action} response"))?;

        match response.response_type.as_str() {
            "sync" => {
                if response.error.is_empty() {
                    Ok(())
                } else {
                    anyhow::bail!("failed to {action}: {}", response.error)
                }
            }
            "async" => {
                let operation_url = response.operation.ok_or_else(|| {
                    anyhow::anyhow!("{action} response did not include an operation URL")
                })?;

                self.wait_for_operation(client, node_endpoint, &operation_url, action)
                    .await
            }
            "error" => {
                let error = if response.error.is_empty() {
                    format!("{action} returned an error response")
                } else {
                    response.error
                };

                anyhow::bail!("failed to {action}: {error}");
            }
            other => anyhow::bail!("unexpected Incus response type '{other}' while {action}"),
        }
    }

    async fn get_client(&self) -> anyhow::Result<Client> {
        let identity = reqwest::Identity::from_pem(self.client_identity_pem.as_bytes())
            .context("failed to load Incus client identity")?;

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

        if let Some(trust_token) = &node.token {
            let trust_req = serde_json::json!({
                "type": "client",
                "trust_token": trust_token,
            });

            let untrusted_url = format!("{}/1.0/certificates?public", node.endpoint);
            let untrusted_res = client
                .post(&untrusted_url)
                .json(&trust_req)
                .send()
                .await
                .with_context(|| format!("failed to submit trust request to {untrusted_url}"))?;

            if untrusted_res.status().is_success() {
                return Ok(());
            }

            let untrusted_status = untrusted_res.status();
            let untrusted_body = untrusted_res
                .text()
                .await
                .unwrap_or_else(|_| "<failed to read response body>".to_string());

            anyhow::bail!(
                "failed to trust client certificate on node; trust_token attempt (status {untrusted_status}) returned: {untrusted_body}"
            );
        } else {
            anyhow::bail!("node is not trusted and no trust token provided");
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
                "limits.cpu.allowance": format!("{}%", req.plan.cpu_allowance_pct),
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

        self.submit_operation_request(
            &client,
            client
                .post(format!("{}/1.0/instances", node.endpoint))
                .json(&create_req),
            &node.endpoint,
            "create incus instance",
        )
        .await?;

        self.start_instance(node, &instance_name).await?;

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
            .json(&serde_json::json!({"action": "start"}));

        self.submit_operation_request(&client, res, &node.endpoint, "start instance")
            .await
    }

    async fn stop_instance(&self, node: &NodeConnection, instance_id: &str) -> anyhow::Result<()> {
        self.ensure_trusted(node).await?;
        let client = self.get_client().await?;

        let res = client
            .put(format!(
                "{}/1.0/instances/{}/state",
                node.endpoint, instance_id
            ))
            .json(&serde_json::json!({"action": "stop", "force": true}));

        self.submit_operation_request(&client, res, &node.endpoint, "stop instance")
            .await
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
            .json(&serde_json::json!({"action": "restart"}));

        self.submit_operation_request(&client, res, &node.endpoint, "restart instance")
            .await
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
            .next_back()
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
        let res = client.delete(format!("{}/1.0/instances/{}", node.endpoint, instance_id));

        self.submit_operation_request(&client, res, &node.endpoint, "destroy instance")
            .await
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
