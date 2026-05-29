//! Solana balance queries - devnet only.
//!
//! Exposes [`SolanaClient`], a thin wrapper around
//! [`solana_client::rpc_client::RpcClient`] that is permanently bound to
//! the Solana devnet RPC URL. There is intentionally no `mainnet()` or
//! `from_url()` constructor - this crate is a testnet-only
//! repository, and the type system enforces that invariant.
//!
//! ## Pipeline position
//!
//! ```text
//! SignerContext::install → [ SolanaClient::query_balance ] → JupiterClient
//! ```

use std::str::FromStr;

use anyhow::{Context, Result};
use serde::Serialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use tracing::info;

use crate::onchain::types::Lamports;

/// Solana devnet RPC endpoint.
///
/// This constant is the only RPC URL permitted in this crate.
const DEVNET_RPC_URL: &str = "https://api.devnet.solana.com";

// ── Client ────────────────────────────────────────────────────────────────────

/// Thin RPC wrapper permanently bound to Solana devnet.
///
/// The only constructor is [`SolanaClient::devnet`]; mainnet is not exposed.
pub struct SolanaClient {
    rpc: RpcClient,
}

impl SolanaClient {
    /// Construct a devnet-only client.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use polar_bear_rig_onchain::onchain::balance::SolanaClient;
    ///
    /// let client = SolanaClient::devnet();
    /// ```
    #[must_use]
    pub fn devnet() -> Self {
        info!(
            rpc_url = %DEVNET_RPC_URL,
            "[SolanaClient::devnet] connecting to Solana devnet (testnet - no mainnet ops)"
        );
        Self {
            rpc: RpcClient::new(DEVNET_RPC_URL.to_string()),
        }
    }

    /// Query the SOL balance for a given base-58 wallet address.
    ///
    /// Returns a [`BalanceResult`] carrying lamports, SOL denomination, and
    /// network tag - ready to be serialised into a `rig-core` tool response.
    ///
    /// # Arguments
    ///
    /// * `address` - Base-58 encoded Solana wallet address to query.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `address` is not a valid base-58 public key, or if
    /// the devnet RPC call fails (network error, rate-limit).
    pub fn query_balance(&self, address: &str) -> Result<BalanceResult> {
        let pubkey = Pubkey::from_str(address)
            .with_context(|| format!("invalid base-58 pubkey: '{address}'"))?;

        info!(
            address,
            "[SolanaClient::query_balance] querying devnet balance"
        );

        let lamports = self
            .rpc
            .get_balance(&pubkey)
            .with_context(|| format!("RPC get_balance failed for '{address}'"))?;

        let sol = Lamports(lamports).to_sol();

        info!(
            address,
            lamports,
            sol,
            network = "devnet",
            "[SolanaClient::query_balance] ✓ balance retrieved"
        );

        Ok(BalanceResult {
            address: address.to_string(),
            lamports,
            sol,
            network: "devnet".to_string(),
        })
    }

    /// Return `true` when the wallet balance meets `min_sol`.
    ///
    /// Convenience wrapper used by the agent to gate Jupiter quote calls on
    /// sufficient balance.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`SolanaClient::query_balance`].
    pub fn has_minimum_balance(&self, address: &str, min_sol: f64) -> Result<bool> {
        Ok(self.query_balance(address)?.sol >= min_sol)
    }
}

// ── Result type ───────────────────────────────────────────────────────────────

/// Balance query result - serialisable for `rig-core` tool output.
#[derive(Debug, Clone, Serialize)]
pub struct BalanceResult {
    /// Base-58 wallet address that was queried.
    pub address: String,
    /// Balance in lamports (raw on-chain denomination).
    pub lamports: u64,
    /// Balance in SOL (lamports ÷ 1 000 000 000).
    pub sol: f64,
    /// RPC network tag (`"devnet"` in all cases for this crate).
    pub network: String,
}

impl std::fmt::Display for BalanceResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BalanceResult {{ address={}, lamports={}, sol={:.6}, network={} }}",
            self.address, self.lamports, self.sol, self.network
        )
    }
}
