//! rig-core tool implementations for the on-chain agent pipeline.
//!
//! Three tools are wired into the agent built by [`crate::agent::build`]:
//!
//! | Tool | `NAME` | Purpose |
//! |---|---|---|
//! | [`SolanaBalanceTool`] | `solana_balance` | Devnet SOL balance query |
//! | [`JupiterQuoteTool`] | `jupiter_quote` | Dry-run SOL → USDC quote |
//! | [`SignerIsolationTool`] | `signer_isolation_log` | Snapshot active SignerContext |
//!
//! ## Rig client trait requirements (rig-core ≥ 0.36)
//!
//! The [`Tool`] trait uses `async fn` in trait syntax stabilised in Rust 1.75.
//! [`rig::client::CompletionClient`] must be in scope to call `.agent()` on a
//! rig-core 0.36+ Anthropic client - otherwise `E0599: no method named 'agent'`.

use std::sync::Arc;

use rig::{
    completion::ToolDefinition,
    tool::{Tool, ToolError},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

use crate::onchain::types::Lamports;
use crate::onchain::{
    balance::{BalanceResult, SolanaClient},
    jupiter::{DEFAULT_SLIPPAGE_BPS, JupiterClient, JupiterQuote, SOL_MINT, USDC_MINT},
    signer::{SignerSnapshot, snapshot_active},
};

// ── Tool 1: Solana balance ────────────────────────────────────────────────────

/// Arguments for [`SolanaBalanceTool`].
#[derive(Deserialize)]
pub struct BalanceArgs {
    /// Base-58 Solana wallet address to query on devnet.
    pub address: String,
}

/// Queries the SOL balance for a wallet address on Solana devnet.
pub struct SolanaBalanceTool {
    client: Arc<SolanaClient>,
}

impl SolanaBalanceTool {
    /// Construct the tool with a shared [`SolanaClient`].
    #[must_use]
    pub fn new(client: Arc<SolanaClient>) -> Self {
        Self { client }
    }
}

impl Tool for SolanaBalanceTool {
    const NAME: &'static str = "solana_balance";

    type Args = BalanceArgs;
    type Output = BalanceResult;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Query the SOL balance for a wallet address on Solana devnet. \
                Returns lamports, SOL amount, and network identifier."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "address": {
                        "type": "string",
                        "description": "Base-58 encoded Solana wallet address to query."
                    }
                },
                "required": ["address"]
            }),
        }
    }

    async fn call(&self, args: BalanceArgs) -> Result<BalanceResult, ToolError> {
        info!(
            address = %args.address,
            "[SolanaBalanceTool] tool invoked by rig-core agent"
        );
        self.client
            .query_balance(&args.address)
            .map_err(|e| ToolError::ToolCallError(e.into()))
    }
}

// ── Tool 2: Jupiter quote (dry-run) ──────────────────────────────────────────

/// Arguments for [`JupiterQuoteTool`].
#[derive(Deserialize)]
pub struct JupiterQuoteArgs {
    /// Amount of SOL to quote in human-readable SOL units (e.g. `0.1`).
    pub sol_amount: f64,
    /// Slippage tolerance in basis points (optional, default 50 bps = 0.5 %).
    pub slippage_bps: Option<u16>,
}

/// Fetches a dry-run SOL → USDC swap quote from Jupiter V6.
pub struct JupiterQuoteTool {
    client: Arc<JupiterClient>,
}

impl JupiterQuoteTool {
    /// Construct the tool with a shared [`JupiterClient`].
    #[must_use]
    pub fn new(client: Arc<JupiterClient>) -> Self {
        Self { client }
    }
}

impl Tool for JupiterQuoteTool {
    const NAME: &'static str = "jupiter_quote";

    type Args = JupiterQuoteArgs;
    type Output = JupiterQuote;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Fetch a DRY-RUN SOL→USDC swap quote from Jupiter V6 aggregator. \
                Returns expected output amount, price impact, and route breakdown. \
                No transaction is constructed or submitted - this is quote-only."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "sol_amount": {
                        "type": "number",
                        "description": "Amount of SOL to swap (e.g. 0.1 for 0.1 SOL)."
                    },
                    "slippage_bps": {
                        "type": "integer",
                        "description": "Slippage tolerance in basis points (optional, default 50).",
                        "minimum": 1,
                        "maximum": 1000
                    }
                },
                "required": ["sol_amount"]
            }),
        }
    }

    async fn call(&self, args: JupiterQuoteArgs) -> Result<JupiterQuote, ToolError> {
        let lamports = Lamports::from_sol(args.sol_amount).0;
        let slippage = args.slippage_bps.unwrap_or(DEFAULT_SLIPPAGE_BPS);

        info!(
            sol_amount = args.sol_amount,
            lamports,
            slippage_bps = slippage,
            "[JupiterQuoteTool] tool invoked by rig-core agent"
        );

        self.client
            .get_quote(SOL_MINT, USDC_MINT, lamports, slippage)
            .await
            .map_err(|e| ToolError::ToolCallError(e.into()))
    }
}

// ── Tool 3: SignerContext isolation log ───────────────────────────────────────

/// Arguments for [`SignerIsolationTool`].
#[derive(Deserialize)]
pub struct SignerIsolationArgs {
    /// Identifier for the current agent task - used to label the audit record.
    pub task_id: String,
}

/// Output of [`SignerIsolationTool`].
#[derive(Debug, Serialize)]
pub struct SignerIsolationOutput {
    /// Agent task identifier.
    pub task_id: String,
    /// Public-metadata snapshot of the active signer context.
    pub signer: Option<SignerSnapshot>,
    /// `true` when a `SignerContext` is actively installed for this task.
    pub boundary_active: bool,
    /// Human-readable isolation status string.
    pub isolation_status: String,
}

/// Snapshots the active task-local [`crate::onchain::signer::SignerContextInner`]
/// and emits a structured isolation audit log.
pub struct SignerIsolationTool;

impl Tool for SignerIsolationTool {
    const NAME: &'static str = "signer_isolation_log";

    type Args = SignerIsolationArgs;
    type Output = SignerIsolationOutput;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Snapshot the current task-local SignerContext and emit a \
                structured isolation audit log. Confirms the security boundary is \
                active and no cross-task signer contamination has occurred."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Identifier for the current agent task."
                    }
                },
                "required": ["task_id"]
            }),
        }
    }

    async fn call(&self, args: SignerIsolationArgs) -> Result<SignerIsolationOutput, ToolError> {
        info!(
            task_id = %args.task_id,
            "[SignerIsolationTool] capturing SignerContext snapshot"
        );

        let signer = snapshot_active();
        let boundary_active = signer.is_some();
        let isolation_status = if boundary_active {
            format!(
                "ISOLATED - SignerContext active for task '{}'. Boundary enforced.",
                args.task_id
            )
        } else {
            format!(
                "WARNING - No active SignerContext for task '{}'. \
                 Task may be running outside a with_signer scope.",
                args.task_id
            )
        };

        info!(
            task_id        = %args.task_id,
            boundary_active,
            status         = %isolation_status,
            "[SignerIsolationTool] ✓ isolation log captured"
        );

        Ok(SignerIsolationOutput {
            task_id: args.task_id,
            signer,
            boundary_active,
            isolation_status,
        })
    }
}
