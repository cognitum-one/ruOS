//! ruos-sensors — opt-in sensor collector with ed25519 consent + brain storage.
//!
//! Collects data from 30+ sources, all gated by cryptographic consent.
//! Stores observations as brain memories for agent reasoning.

mod consent;
mod collectors;

use anyhow::Result;
use clap::{Parser, Subcommand};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "ruos-sensors", version = VERSION)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show consent status for all sensor categories
    Status,
    /// Grant consent for a sensor category (signed with ed25519)
    Grant { category: String },
    /// Revoke consent for a sensor category
    Revoke { category: String },
    /// Collect data from all consented sensors (one-shot)
    Collect,
    /// Run continuous collection (daemon mode)
    Daemon,
    /// List all available sensor categories
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status => consent::show_status(),
        Commands::Grant { category } => consent::grant(&category)?,
        Commands::Revoke { category } => consent::revoke(&category)?,
        Commands::Collect => collectors::collect_all().await?,
        Commands::Daemon => collectors::daemon().await?,
        Commands::List => list_categories(),
    }
    Ok(())
}

fn list_categories() {
    println!("ruos-sensors — Available Categories\n");
    println!("  Default ON (non-personal):");
    for c in &["system-metrics", "network-stats", "csi-sensing", "geo-satellite"] {
        println!("    {c}");
    }
    println!("\n  Opt-in required:");
    for c in &["camera-depth", "camera-objects", "audio-ambient",
               "bluetooth-proximity", "radar-vitals", "household-patterns"] {
        println!("    {c}");
    }
    println!("\n  Grant: ruos-sensors grant <category>");
    println!("  Revoke: ruos-sensors revoke <category>");
}
