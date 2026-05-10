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
//! Calling `.agent()` on `anthropic::Client` requires [`rig::client::CompletionClient`]
//! in scope so the `.agent()` builder method resolves.
//!
//! `Client::new` is fallible in rig-core 0.36+ - always propagate with `?`.

pub mod tools;

use std::sync::Arc;

use anyhow::Result;
use rig::{client::CompletionClient, completion::Prompt, providers::anthropic};

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
/// Returns `Err` if `anthropic::Client::new` fails (invalid API key format).
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
    // Client::new is fallible in rig-core 0.36+ - propagate with ?.
    let client = anthropic::Client::new(&cfg.anthropic_api_key)?;

    // CompletionClient must be in scope for `.agent()` to resolve (rig-core ≥ 0.36).
    let agent = client
        .agent("claude-sonnet-4-6")
        .preamble(AGENT_PREAMBLE)
        .tool(SolanaBalanceTool::new(solana))
        .tool(JupiterQuoteTool::new(jupiter))
        .tool(SignerIsolationTool)
        .build();

    Ok(agent)
}
