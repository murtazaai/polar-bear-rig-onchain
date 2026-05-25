//! `polar-bear-rig-onchain` - CLI entry point.
//!
//! **Polar Bear Systems** | Technology Lead: Murtaza Ali Imtiaz
//!
//! Platform: Rig (Rust Inference Gateway / ARC) · rig-onchain-kit ·
//! `SignerContext` (task-local) · Jupiter V6 (dry-run) · Solana devnet
//!
//! ## Usage
//!
//! ```text
//! # Full pipeline with rig-core agent (requires ANTHROPIC_API_KEY)
//! cargo run --release -- --mode full --wallet <DEVNET_ADDRESS> --amount 0.1
//!
//! # Full pipeline, skip the agent (no API key needed)
//! cargo run --release -- --mode full --no-agent
//!
//! # Individual subsystems (no API key needed)
//! cargo run --release -- --mode balance
//! cargo run --release -- --mode quote
//! cargo run --release -- --mode signer
//! ```
//!
//! `ANTHROPIC_API_KEY` is only required for `--mode full` **without** `--no-agent`.
//! Set it in `.env` (copy `.env.example`) or export it in the shell.

use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use rig::completion::Prompt;
use tracing::info;
use tracing_subscriber::EnvFilter;

use polar_bear_rig_onchain::{agent, config::Config, onchain};

// ── CLI ───────────────────────────────────────────────────────────────────────

/// CLI operating mode - selects which subsystem to exercise.
#[derive(Debug, Clone, ValueEnum)]
enum Mode {
    /// Run the full pipeline: `SignerContext` → balance → Jupiter quote → isolation log.
    ///
    /// Requires `ANTHROPIC_API_KEY` unless `--no-agent` is also passed.
    Full,
    /// Query Solana devnet balance only. No API key needed.
    Balance,
    /// Fetch a Jupiter V6 dry-run swap quote only. No API key needed.
    Quote,
    /// Run the `SignerContext` task-local isolation demo (3 concurrent tasks). No API key needed.
    Signer,
}

/// CLI arguments parsed by [`clap`].
#[derive(Parser, Debug)]
#[command(name = "polar-bear-rig-onchain")]
#[command(
    about = "rig-onchain-kit agent: Solana balance → Jupiter swap (dry-run) - Polar Bear Systems"
)]
struct Args {
    /// Operating mode (default: `full`).
    #[arg(short, long, default_value = "full")]
    mode: Mode,

    /// Solana devnet wallet address to query (overrides `WALLET_ADDRESS` env var).
    #[arg(short, long)]
    wallet: Option<String>,

    /// SOL amount to quote via Jupiter (in SOL units, e.g. `0.1`).
    #[arg(short, long, default_value_t = 0.1)]
    amount: f64,

    /// Skip the rig-core agent call even in `--mode full`.
    ///
    /// When set, the full pipeline runs balance → quote → signer directly
    /// without constructing the Anthropic client, so `ANTHROPIC_API_KEY` is
    /// not required. Useful for local development, CI, or smoke-testing
    /// the on-chain subsystems without an API key.
    #[arg(long, default_value_t = false)]
    no_agent: bool,
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("polar_bear_rig_onchain=debug".parse()?),
        )
        .init();

    let args = Args::parse();

    // Config::from_env() is infallible: ANTHROPIC_API_KEY becomes Option<String>.
    // Validation against None happens inside agent::build(), which is only
    // called for --mode full without --no-agent.
    let mut cfg = Config::from_env()?;

    // CLI --wallet overrides WALLET_ADDRESS from env.
    if let Some(w) = args.wallet {
        cfg.wallet_address = w;
    }

    info!("╔══════════════════════════════════════════════════════════╗");
    info!("║  POLAR BEAR RIG ONCHAIN  ·  rig-onchain-kit Platform    ║");
    info!("╚══════════════════════════════════════════════════════════╝");
    info!(
        mode          = ?args.mode,
        wallet        = %cfg.wallet_address,
        amount        = args.amount,
        no_agent      = args.no_agent,
        api_key_present = cfg.anthropic_api_key.is_some(),
        "Starting platform"
    );

    match args.mode {
        Mode::Full => {
            if args.no_agent {
                run_full_no_agent(&cfg, args.amount).await?;
            } else {
                run_full(&cfg, args.amount).await?;
            }
        }
        Mode::Balance => run_balance(&cfg).await?,
        Mode::Quote => run_quote(&cfg, args.amount).await?,
        Mode::Signer => onchain::demo_signer(&cfg).await?,
    }

    info!("Platform run complete. All operations logged.");
    Ok(())
}

// ── Mode implementations ──────────────────────────────────────────────────────

/// Full pipeline via rig-core agent: `SignerContext` → balance → quote → isolation log.
///
/// Requires `ANTHROPIC_API_KEY`. For a keyless run use `--no-agent`.
async fn run_full(cfg: &Config, amount: f64) -> Result<()> {
    let solana = Arc::new(onchain::balance::SolanaClient::devnet());
    let jupiter = Arc::new(onchain::jupiter::JupiterClient::dry_run());

    // agent::build validates cfg.anthropic_api_key is Some and returns a clear
    // error message when it is not.
    let agent = agent::build(cfg, Arc::clone(&solana), Arc::clone(&jupiter))?;

    let prompt = format!(
        "Execute the on-chain agent pipeline:\n\
         - Wallet:  {}\n\
         - Network: Solana devnet\n\
         - Swap:    {amount} SOL → USDC (dry-run, do NOT execute)\n\n\
         Steps:\n\
         1. Call solana_balance for the wallet.\n\
         2. If balance >= {amount} SOL, call jupiter_quote for {amount} SOL → USDC.\n\
         3. Call signer_isolation_log.\n\
         4. Output a structured summary (confirm DRY-RUN=true).",
        cfg.wallet_address
    );

    info!("[FULL] dispatching task to rig-core agent (PEV loop)");
    let response = agent.prompt(&prompt).await?;

    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  AGENT PIPELINE RESULT                                   ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");
    println!("{response}");
    println!("\n✓  polar-bear-rig-onchain pipeline complete.");
    println!("   Network: devnet | Swap: DRY-RUN | Boundary: SEALED");

    Ok(())
}

/// Full pipeline **without** the rig-core agent. No `ANTHROPIC_API_KEY` needed.
///
/// Runs balance → quote → signer directly and prints each result. The output
/// is equivalent to running the three keyless modes in sequence; use this to
/// smoke-test the on-chain subsystems in CI or local dev without a key.
async fn run_full_no_agent(cfg: &Config, amount: f64) -> Result<()> {
    info!("[FULL --no-agent] running pipeline without rig-core agent (no API key required)");

    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  FULL PIPELINE  ·  --no-agent  ·  No API key required   ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    println!("── Step 1: Solana devnet balance ──────────────────────────");
    run_balance(cfg).await?;

    println!("\n── Step 2: Jupiter V6 dry-run quote ───────────────────────");
    run_quote(cfg, amount).await?;

    println!("\n── Step 3: SignerContext isolation demo ────────────────────");
    onchain::demo_signer(cfg).await?;

    println!("\n✓  polar-bear-rig-onchain pipeline complete (agent skipped).");
    println!("   Network: devnet | Swap: DRY-RUN | Boundary: SEALED | Agent: SKIPPED");

    Ok(())
}

/// Balance-only mode: query and print Solana devnet balance. No API key needed.
#[allow(clippy::unused_async)]
async fn run_balance(cfg: &Config) -> Result<()> {
    let client = onchain::balance::SolanaClient::devnet();
    let result = client.query_balance(&cfg.wallet_address)?;
    info!(balance = %result, "[BALANCE] query complete");
    println!("{result}");
    Ok(())
}

/// Quote-only mode: fetch a Jupiter dry-run quote and print it. No API key needed.
async fn run_quote(cfg: &Config, amount: f64) -> Result<()> {
    use onchain::jupiter::{DEFAULT_SLIPPAGE_BPS, JupiterClient, SOL_MINT, USDC_MINT};
    use onchain::types::Lamports;

    let _ = cfg; // wallet not needed for price discovery
    let client = JupiterClient::dry_run();
    let lamports = Lamports::from_sol(amount).0;
    let quote = client
        .get_quote(SOL_MINT, USDC_MINT, lamports, DEFAULT_SLIPPAGE_BPS)
        .await?;
    info!(quote = %quote, "[QUOTE] dry-run quote complete");
    println!("{quote}");
    Ok(())
}
