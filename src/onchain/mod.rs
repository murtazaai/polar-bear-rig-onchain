//! On-chain execution layer.
//!
//! Exposes two public entry points consumed by `crate::main`:
//!
//! * [`execute_pipeline`] - wraps a Solana balance query and a Jupiter quote inside an isolated
//!   [`signer::LocalSolanaSigner`] context and returns an [`OnchainResult`].
//! * [`demo_signer`] - spawns three concurrent tasks to demonstrate that the task-local signer
//!   storage is fully isolated per Tokio task.
//!
//! ## Security boundary
//!
//! All on-chain operations are wrapped in [`signer::with_signer`], which uses
//! `tokio::task_local!` to scope the active keypair to exactly one async task.
//! This mirrors the `rig-onchain-kit` `SignerContext` pattern and prevents
//! concurrent tasks from accidentally sharing or overwriting each other's
//! signing credentials.

pub mod balance;
pub mod jupiter;
pub mod signer;
pub mod types;

use anyhow::Result;
use balance::{BalanceResult, SolanaClient};
use jupiter::{DEFAULT_SLIPPAGE_BPS, JupiterClient, JupiterQuote, SOL_MINT, USDC_MINT};
use signer::{LocalSolanaSigner, with_signer};
use tracing::info;
use types::Lamports;

use crate::config::Config;

/// Aggregated output of one full on-chain pipeline run.
#[derive(Debug)]
pub struct OnchainResult {
    /// Balance query result.
    pub balance: BalanceResult,
    /// Jupiter swap quote (always dry-run).
    pub quote: JupiterQuote,
    /// Public key of the ephemeral task-local signer.
    pub signer_pubkey: String,
}

/// Run the on-chain pipeline: balance query → Jupiter quote, all inside an
/// isolated [`signer::LocalSolanaSigner`] context.
///
/// # Arguments
///
/// * `cfg`    - Runtime configuration (provides wallet address and RPC URL).
/// * `amount` - SOL amount to quote (in SOL units, e.g. `0.1`).
///
/// # Errors
///
/// Returns `Err` if the balance query or Jupiter API call fails.
pub async fn execute_pipeline(cfg: &Config, amount: f64) -> Result<OnchainResult> {
    let signer =
        LocalSolanaSigner::ephemeral(format!("hft-task-{:016x}", rand::rng().random::<u64>()));
    let signer_pubkey = signer.pubkey().to_string();

    info!(%signer_pubkey, "[ONCHAIN] SignerContext: task-local signer loaded");

    with_signer(signer, || async {
        let solana = SolanaClient::devnet();
        let jupiter = JupiterClient::dry_run();

        // ── Balance query ─────────────────────────────────────────
        let balance = solana.query_balance(&cfg.wallet_address)?;
        info!(sol = balance.sol, "[ONCHAIN] balance query complete");

        // ── Jupiter quote (dry-run) ───────────────────────────────
        let lamports = Lamports::from_sol(amount).0;
        let quote = jupiter
            .get_quote(SOL_MINT, USDC_MINT, lamports, DEFAULT_SLIPPAGE_BPS)
            .await?;
        info!(
            out_usdc = quote.out_amount_ui,
            impact   = %quote.price_impact_pct,
            "[ONCHAIN] Jupiter quote complete [DRY-RUN]"
        );

        Ok(OnchainResult {
            balance,
            quote,
            signer_pubkey: signer_pubkey.clone(),
        })
    })
    .await
}

/// Demonstrate [`signer::with_signer`] isolation across three concurrent tasks.
///
/// # Errors
///
/// Returns `Err` if any spawned task panics.
pub async fn demo_signer(_cfg: &Config) -> Result<()> {
    signer::demo_signer().await
}
