//! `polar-bear-rig-onchain` - CLI entry point.
//!
//! **Polar Bear Systems** | Technology Lead: Murtaza Ali Imtiaz
//!
//! Platform: Rig (Rust Inference Gateway / ARC) · rig-onchain-kit ·
//! SignerContext (task-local) · Jupiter V6 (dry-run) · Solana devnet
//!
//! ## Usage
//!
//! ```text
//! # Full pipeline (default)
//! cargo run --release -- --mode full --wallet <DEVNET_ADDRESS> --amount 0.1
//!
//! # Individual subsystems
//! cargo run --release -- --mode balance
//! cargo run --release -- --mode quote
//! cargo run --release -- --mode signer
//! ```
//!
//! Set `ANTHROPIC_API_KEY` in `.env` or the shell environment before running.

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
    /// Run the full pipeline: SignerContext → balance → Jupiter quote → isolation log.
    Full,
    /// Run only the Solana devnet balance query.
    Balance,
    /// Run only the Jupiter V6 dry-run swap quote.
    Quote,
    /// Run the SignerContext task-local isolation demo (3 concurrent tasks).
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

    /// Solana devnet wallet address to query (overrides WALLET_ADDRESS env var).
    #[arg(short, long)]
    wallet: Option<String>,

    /// SOL amount to quote via Jupiter (in SOL units, e.g. `0.1`).
    #[arg(short, long, default_value_t = 0.1)]
    amount: f64,
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
    let mut cfg = Config::from_env()?;

    // CLI --wallet overrides WALLET_ADDRESS from env.
    if let Some(w) = args.wallet {
        cfg.wallet_address = w;
    }

    info!("╔══════════════════════════════════════════════════════════╗");
    info!("║  POLAR BEAR RIG ONCHAIN  ·  rig-onchain-kit Platform    ║");
    info!("╚══════════════════════════════════════════════════════════╝");
    info!(
        mode   = ?args.mode,
        wallet = %cfg.wallet_address,
        amount = args.amount,
        "Starting platform"
    );

    match args.mode {
        Mode::Full => run_full(&cfg, args.amount).await?,
        Mode::Balance => run_balance(&cfg).await?,
        Mode::Quote => run_quote(&cfg, args.amount).await?,
        Mode::Signer => onchain::demo_signer(&cfg).await?,
    }

    info!("Platform run complete. All operations logged.");
    Ok(())
}

// ── Mode implementations ──────────────────────────────────────────────────────

/// Full pipeline: SignerContext → rig-core agent → balance → quote → isolation log.
async fn run_full(cfg: &Config, amount: f64) -> Result<()> {
    let solana = Arc::new(onchain::balance::SolanaClient::devnet());
    let jupiter = Arc::new(onchain::jupiter::JupiterClient::dry_run());

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

/// Balance-only mode: query and print Solana devnet balance.
async fn run_balance(cfg: &Config) -> Result<()> {
    let client = onchain::balance::SolanaClient::devnet();
    let result = client.query_balance(&cfg.wallet_address)?;
    info!(balance = %result, "[BALANCE] query complete");
    println!("{result}");
    Ok(())
}

/// Quote-only mode: fetch a Jupiter dry-run quote and print it.
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
