use provider_adapter::{ComputeProvider, StubProvider};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .compact()
        .init();

    info!("worker booted");

    let provider = StubProvider;
    run_provisioning_tick(&provider).await?;
    run_renewal_tick().await;

    Ok(())
}

async fn run_provisioning_tick(provider: &dyn ComputeProvider) -> anyhow::Result<()> {
    let _ = provider;
    info!("provisioning tick placeholder");
    Ok(())
}

async fn run_renewal_tick() {
    info!("renewal tick placeholder");
}
