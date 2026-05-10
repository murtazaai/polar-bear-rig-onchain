/// solana_ops.rs
///
/// Solana balance query for the polar-bear-rig-onchain agent pipeline.
///
/// Connects to Solana DEVNET only. All mainnet RPC URLs are explicitly blocked
/// at the type level - you must call SolanaClient::devnet() to construct a client.
/// This is a testnet-only repo; no mainnet operations are permitted.
///
/// Pipeline position:
///   SignerContext::install → [ SolanaBalance::query ] → JupiterQuote → log
use anyhow::{Context, Result};
use serde::Serialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tracing::info;

/// Devnet RPC endpoint. Only this URL is permitted in this crate.
const DEVNET_RPC_URL: &str = "https://api.devnet.solana.com";

/// SOL has 9 decimal places (1 SOL = 1_000_000_000 lamports).
const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

// ─── Client ──────────────────────────────────────────────────────────────────

/// Thin wrapper around solana_client::RpcClient restricted to devnet.
pub struct SolanaClient {
    rpc: RpcClient,
}

impl SolanaClient {
    /// Construct a devnet-only client.
    /// This is the only constructor - mainnet is not exposed.
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
    /// Returns a `BalanceResult` with lamports, SOL, and metadata suitable
    /// for passing to a rig-core agent tool response.
    pub fn query_balance(&self, address: &str) -> Result<BalanceResult> {
        let pubkey = Pubkey::from_str(address)
            .with_context(|| format!("Invalid base-58 pubkey: '{}'", address))?;

        info!(
            address = %address,
            "[SolanaClient::query_balance] querying devnet balance"
        );

        let lamports = self
            .rpc
            .get_balance(&pubkey)
            .with_context(|| format!("RPC get_balance failed for '{}'", address))?;

        let sol = lamports_to_sol(lamports);

        info!(
            address   = %address,
            lamports  = %lamports,
            sol       = %sol,
            network   = "devnet",
            "[SolanaClient::query_balance] ✓ balance retrieved"
        );

        Ok(BalanceResult {
            address: address.to_string(),
            lamports,
            sol,
            network: "devnet".to_string(),
        })
    }

    /// Check whether an address has a minimum balance (in SOL).
    /// Used by the HFT agent to confirm sufficient funds before quoting a swap.
    pub fn has_minimum_balance(&self, address: &str, min_sol: f64) -> Result<bool> {
        let result = self.query_balance(address)?;
        Ok(result.sol >= min_sol)
    }
}

// ─── Result Types ─────────────────────────────────────────────────────────────

/// Balance query result - serialisable for rig-core tool output.
#[derive(Debug, Clone, Serialize)]
pub struct BalanceResult {
    pub address: String,
    pub lamports: u64,
    pub sol: f64,
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

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Convert lamports to SOL with 9-decimal precision.
#[inline]
pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / LAMPORTS_PER_SOL as f64
}

/// Convert SOL to lamports (truncating sub-lamport amounts).
#[inline]
pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * LAMPORTS_PER_SOL as f64) as u64
}
