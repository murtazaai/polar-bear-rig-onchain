//! rig-core on-chain agent pipeline.
//!
//! Exposes [`build`] which assembles a `rig-core` [`rig::completion::Prompt`]
//! agent backed by `claude-sonnet-4-6` with three tools wired in:
//!
//! | Tool | Trigger |
//! |---|---|
//! | `solana_balance` | balance query step |
//! | `jupiter_quote` | swap quote step |
//! | `signer_isolation_log` | audit / isolation confirmation step |
//!
//! ## Rig client trait requirements (rig-core ≥ 0.36)
//!
//! Both [`rig::client::CompletionClient`] **and** [`rig::client::ProviderClient`]
//! must be in scope for the `.agent()` builder method to resolve on
//! `anthropic::Client`. `Client::new` is fallible in rig-core 0.36+ - always
//! propagate with `?`.
//!
//! ## API key requirement
//!
//! [`build`] requires `cfg.anthropic_api_key` to be `Some`. It returns a
//! descriptive error when the key is absent so that `--mode balance`,
//! `--mode quote`, and `--mode signer` can all run without the key - only
//! `--mode full` calls this function.

pub mod tools;

use std::sync::Arc;

use anyhow::{Context, Result};
use rig::{
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    providers::anthropic,
};

use crate::config::Config;
use crate::onchain::{balance::SolanaClient, jupiter::JupiterClient};
use tools::{JupiterQuoteTool, SignerIsolationTool, SolanaBalanceTool};

/// System preamble for the on-chain agent.
///
/// Scopes the agent to Solana devnet, enforces dry-run mode, and prescribes
/// the three-step PEV workflow the agent must follow.
const AGENT_PREAMBLE: &str = "\
You are an on-chain HFT agent for the polar-bear-rig-onchain pipeline. \
You operate on Solana DEVNET only - you must NEVER reference or attempt mainnet operations.

Your task workflow is:
1. PERCEIVE:  Receive wallet address and swap parameters.
2. EVALUATE:  Call solana_balance to check the wallet's SOL balance.
3. EVALUATE:  If balance >= requested swap amount, call jupiter_quote for a SOL→USDC dry-run quote.
4. ACT:       Call signer_isolation_log to capture the SignerContext audit record.
5. Summarise: balance found, quote received (DRY-RUN), isolation confirmed.

IMPORTANT: This is a DRY-RUN pipeline. jupiter_quote does NOT execute any swap. \
Always confirm dry_run=true in your summary. \
Output a structured summary with: wallet, balance (SOL), \
quote (SOL → USDC, price impact, routes), and isolation status.";

/// Build the rig-core on-chain agent with all three tools wired in.
///
/// # Errors
///
/// Returns `Err` if:
/// * `cfg.anthropic_api_key` is `None` (key absent from environment).
/// * `anthropic::Client::new` rejects the key format.
///
/// # Examples
///
/// ```rust,no_run
/// use polar_bear_rig_onchain::{agent, config::Config, onchain};
/// use std::sync::Arc;
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let cfg = Config::from_env()?;
/// let solana  = Arc::new(onchain::balance::SolanaClient::devnet());
/// let jupiter = Arc::new(onchain::jupiter::JupiterClient::dry_run());
/// let agent = agent::build(&cfg, solana, jupiter)?;
/// # Ok(()) }
/// ```
pub fn build(
    cfg: &Config,
    solana: Arc<SolanaClient>,
    jupiter: Arc<JupiterClient>,
) -> Result<impl Prompt> {
    // Validate the key here so that modes that don't call build() (balance,
    // quote, signer) can run freely without ANTHROPIC_API_KEY.
    let api_key = cfg
        .anthropic_api_key
        .as_deref()
        .context("ANTHROPIC_API_KEY is required for --mode full; set it in .env or the shell")?;

    // Client::new is fallible in rig-core 0.36+ - propagate with ?.
    // Both CompletionClient and ProviderClient must be in scope for .agent().
    let client = anthropic::Client::new(api_key)?;

    let agent = client
        .agent("claude-sonnet-4-6")
        .preamble(AGENT_PREAMBLE)
        .tool(SolanaBalanceTool::new(solana))
        .tool(JupiterQuoteTool::new(jupiter))
        .tool(SignerIsolationTool)
        .build();

    Ok(agent)
}
