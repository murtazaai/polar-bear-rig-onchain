//! Runtime configuration loaded from environment variables.
//!
//! All fields are read at startup via [`Config::from_env`]. The only required
//! variable is `ANTHROPIC_API_KEY`; every other variable has a safe default
//! that keeps the binary in dry-run mode against Solana devnet.
//!
//! ## Environment variables
//!
//! | Variable | Required | Default |
//! |---|---|---|
//! | `ANTHROPIC_API_KEY` | ✅ | — |
//! | `SOLANA_RPC_URL` | ❌ | `https://api.devnet.solana.com` |
//! | `WALLET_ADDRESS` | ❌ | (public devnet address) |
//! | `DRY_RUN` | ❌ | `true` |

use anyhow::{Context, Result};

/// Default Solana devnet wallet used when `WALLET_ADDRESS` is absent.
///
/// A publicly-visible devnet address with no mainnet balance.
pub const DEFAULT_DEVNET_WALLET: &str = "4Nd1mBQtrMJVYVfKf2PX99kkXz36o2gWHa9zSX6TQKL";

/// Global runtime configuration for the on-chain agent platform.
///
/// Constructed once at startup and shared by reference across all subsystems.
/// Cloning is cheap — all fields are either `String` or `bool`.
#[derive(Debug, Clone)]
pub struct Config {
    /// Anthropic API key forwarded to every `rig-core` client.
    pub anthropic_api_key: String,

    /// Solana JSON-RPC endpoint. Defaults to Solana devnet.
    pub solana_rpc_url: String,

    /// Base-58 encoded Solana wallet address to query.
    ///
    /// This address is used for balance queries only. No private key is
    /// required — the demo generates an ephemeral keypair for signing context.
    pub wallet_address: String,

    /// When `true` (the default), all on-chain operations are simulated and no
    /// real transactions are signed or broadcast.
    ///
    /// Jupiter quote calls are always read-only regardless of this flag.
    /// A runtime `assert!` in [`crate::onchain::jupiter::JupiterClient`]
    /// prevents any swap execution even when `dry_run = false`.
    pub dry_run: bool,
}

impl Config {
    /// Construct a [`Config`] from the process environment.
    ///
    /// Call [`dotenvy::dotenv`] before this function to load variables from a
    /// `.env` file; `main` does this automatically.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `ANTHROPIC_API_KEY` is not present in the environment.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use polar_bear_rig_onchain::config::Config;
    ///
    /// let cfg = Config::from_env().expect("ANTHROPIC_API_KEY must be set");
    /// ```
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY")
                .context("ANTHROPIC_API_KEY not set — copy .env.example to .env")?,
            solana_rpc_url: std::env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string()),
            wallet_address: std::env::var("WALLET_ADDRESS")
                .unwrap_or_else(|_| DEFAULT_DEVNET_WALLET.to_string()),
            dry_run: std::env::var("DRY_RUN")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
        })
    }
}
