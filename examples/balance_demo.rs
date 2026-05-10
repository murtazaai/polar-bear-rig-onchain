//! `balance_demo` — query the SOL balance of a devnet wallet.
//!
//! ```text
//! cargo run --example balance_demo
//! ```
//!
//! Optionally override the wallet address:
//!
//! ```text
//! WALLET_ADDRESS=<base-58-address> cargo run --example balance_demo
//! ```

use anyhow::Result;
use polar_bear_rig_onchain::{config::DEFAULT_DEVNET_WALLET, onchain::balance::SolanaClient};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("polar_bear_rig_onchain=info,balance_demo=info"))
        .init();

    info!("╔══════════════════════════════════════════════════════════╗");
    info!("║  BALANCE DEMO  ·  Solana devnet balance query            ║");
    info!("╚══════════════════════════════════════════════════════════╝");

    // Read wallet address from env; fall back to the public devnet address.
    // No ANTHROPIC_API_KEY required for this example.
    let wallet = std::env::var("WALLET_ADDRESS")
        .unwrap_or_else(|_| DEFAULT_DEVNET_WALLET.to_string());

    let client = SolanaClient::devnet();
    let result = client.query_balance(&wallet)?;

    println!("\nWallet:   {}", result.address);
    println!("Balance:  {:.6} SOL ({} lamports)", result.sol, result.lamports);
    println!("Network:  {}", result.network);

    Ok(())
}
