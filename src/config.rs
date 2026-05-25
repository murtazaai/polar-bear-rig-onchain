//! Runtime configuration loaded from environment variables.
//!
//! All fields are read at startup via [`Config::from_env`]. `ANTHROPIC_API_KEY`
//! is optional at the config level - it is only required when `--mode full`
//! actually constructs the rig-core agent. All other modes (`balance`, `quote`,
//! `signer`) work without it.
//!
//! ## Environment variables
//!
//! | Variable | Required | Default |
//! |---|---|---|
//! | `ANTHROPIC_API_KEY` | ❌ (only for `--mode full`) | - |
//! | `SOLANA_RPC_URL` | ❌ | `https://api.devnet.solana.com` |
//! | `WALLET_ADDRESS` | ❌ | (public devnet address) |
//! | `DRY_RUN` | ❌ | `true` |

use anyhow::Result;

/// Default Solana devnet wallet used when `WALLET_ADDRESS` is absent.
///
/// A publicly-visible devnet address with no mainnet balance.
pub const DEFAULT_DEVNET_WALLET: &str = "4Nd1mBQtrMJVYVfKf2PX99kkXz36o2gWHa9zSX6TQKL";

/// Global runtime configuration for the on-chain agent platform.
///
/// Constructed once at startup and shared by reference across all subsystems.
/// Cloning is cheap - all fields are either `String`, `Option<String>`, or `bool`.
#[derive(Debug, Clone)]
pub struct Config {
    /// Anthropic API key forwarded to `rig-core` when building the agent.
    ///
    /// `None` when `ANTHROPIC_API_KEY` is absent from the environment.
    /// Required only for `--mode full`; [`crate::agent::build`] returns a
    /// clear error if it is called without a key. All other modes never
    /// construct the agent and therefore work without the key.
    pub anthropic_api_key: Option<String>,

    /// Solana JSON-RPC endpoint. Defaults to Solana devnet.
    pub solana_rpc_url: String,

    /// Base-58 encoded Solana wallet address to query.
    ///
    /// This address is used for balance queries only. No private key is
    /// required - the demo generates an ephemeral keypair for signing context.
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
    /// This function always succeeds. `ANTHROPIC_API_KEY` is captured as
    /// `Some(key)` when present and `None` when absent; validation is deferred
    /// to [`crate::agent::build`], which is only called for `--mode full`.
    ///
    /// # Errors
    ///
    /// Currently infallible; `Result` is kept for forward-compatibility.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use polar_bear_rig_onchain::config::Config;
    ///
    /// // Works even without ANTHROPIC_API_KEY in the environment.
    /// let cfg = Config::from_env().expect("config must load");
    /// ```
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            solana_rpc_url: std::env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string()),
            wallet_address: std::env::var("WALLET_ADDRESS")
                .unwrap_or_else(|_| DEFAULT_DEVNET_WALLET.to_string()),
            dry_run: std::env::var("DRY_RUN").map_or(true, |v| v == "true" || v == "1"),
        })
    }
}
