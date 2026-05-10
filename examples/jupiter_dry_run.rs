//! `jupiter_dry_run` - fetch a dry-run Jupiter V6 swap quote and emit an
//! isolation audit log.
//!
//! Wraps the quote call inside a [`with_signer`] scope to demonstrate the
//! complete on-chain execution pattern: signer context installed → quote
//! fetched → isolation log captured → context evicted.
//!
//! ```text
//! cargo run --example jupiter_dry_run
//! ```
//!
//! No API key is required (Jupiter quote is a public read-only endpoint).

use anyhow::Result;
use polar_bear_rig_onchain::onchain::{
    jupiter::{DEFAULT_SLIPPAGE_BPS, JupiterClient, SOL_MINT, USDC_MINT},
    signer::{IsolationReport, LocalSolanaSigner, snapshot_active, with_signer},
    types::Lamports,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(
            "polar_bear_rig_onchain=info,jupiter_dry_run=info",
        ))
        .init();

    info!("╔══════════════════════════════════════════════════════════╗");
    info!("║  JUPITER DRY-RUN  ·  SOL → USDC quote + isolation log   ║");
    info!("╚══════════════════════════════════════════════════════════╝");

    let signer = LocalSolanaSigner::ephemeral("jupiter-dry-run-demo");
    let task_id = signer.context_id().to_string();

    let result = with_signer(signer, || async {
        let client = JupiterClient::dry_run();
        let lamports = Lamports::from_sol(0.1).0;

        let quote = client
            .get_quote(SOL_MINT, USDC_MINT, lamports, DEFAULT_SLIPPAGE_BPS)
            .await?;

        // Capture isolation report while context is still active.
        let report = IsolationReport::capture(&task_id);

        Ok::<_, anyhow::Error>((quote, report))
    })
    .await?;

    let (quote, report) = result;
    let sealed = report.seal();

    println!("\n── Jupiter Quote (DRY-RUN) ──────────────────────────────────");
    println!("{quote}");
    for (i, route) in quote.routes.iter().enumerate() {
        println!(
            "  Route[{i}]: {} ({}%) via {}",
            route.label, route.percent, route.amm_key
        );
    }

    println!("\n── Isolation Report ─────────────────────────────────────────");
    println!("{}", serde_json::to_string_pretty(&sealed)?);

    Ok(())
}
