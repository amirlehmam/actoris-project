//! Actoris Sidecar - Pingora-based proxy with eBPF metering

mod metering;
mod proxy;
mod telemetry;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Actoris Sidecar...");

    // TODO: Initialize Pingora proxy
    // TODO: Initialize eBPF metering
    // TODO: Connect to NATS for telemetry

    tracing::info!("Actoris Sidecar started successfully");

    // Keep running
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down...");

    Ok(())
}
