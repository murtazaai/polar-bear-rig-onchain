//! Jupiter V6 quote API — dry-run only.
//!
//! Calls the Jupiter Aggregator V6 `/quote` endpoint to fetch the best swap
//! route and expected output amount for a given SOL → USDC swap. **No swap
//! transaction is constructed or submitted.** The `dry_run = true` flag is
//! baked into the only constructor, and a runtime `assert!` prevents any
//! accidental live-mode execution.
//!
//! In production, replace this module with
//! `rig_onchain_kit::tools::JupiterSwap` once `rig-onchain-kit` is published
//! to crates.io.
//!
//! ## Pipeline position
//!
//! ```text
//! SignerContext → SolanaBalance → [ JupiterClient::get_quote (dry-run) ] → IsolationLog
//! ```

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::onchain::types::Lamports;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Jupiter V6 quote API endpoint.
const JUPITER_QUOTE_URL: &str = "https://quote-api.jup.ag/v6/quote";

/// Wrapped SOL mint address (mainnet — used for price discovery; no txns sent).
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// USDC mint address (mainnet — used for price discovery; no txns sent).
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// Default slippage tolerance for HFT quote requests (50 bps = 0.5 %).
pub const DEFAULT_SLIPPAGE_BPS: u16 = 50;

// ── Client ────────────────────────────────────────────────────────────────────

/// Jupiter quote client — dry-run mode only.
///
/// [`JupiterClient::dry_run`] is the only constructor; the `dry_run` flag is
/// baked in. A `debug_assert!` in [`JupiterClient::get_quote`] fires
/// immediately if `dry_run` is somehow `false`, preventing any swap execution.
pub struct JupiterClient {
    http: Client,
    dry_run: bool,
}

impl JupiterClient {
    /// Construct a dry-run-only Jupiter client.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use polar_bear_rig_onchain::onchain::jupiter::JupiterClient;
    ///
    /// let client = JupiterClient::dry_run();
    /// ```
    #[must_use]
    pub fn dry_run() -> Self {
        info!(
            quote_url = %JUPITER_QUOTE_URL,
            "[JupiterClient::dry_run] client initialised — DRY-RUN mode (no transactions submitted)"
        );
        Self {
            http: Client::new(),
            dry_run: true,
        }
    }

    /// Fetch the best swap quote from Jupiter V6 for a given token pair and
    /// amount.
    ///
    /// This is a read-only GET to `/v6/quote` — no transaction is constructed
    /// and no wallet is accessed.
    ///
    /// # Arguments
    ///
    /// * `input_mint`   — Base-58 mint address of the token being sold.
    /// * `output_mint`  — Base-58 mint address of the token being bought.
    /// * `amount`       — Amount in the smallest unit of `input_mint`
    ///   (lamports for SOL).
    /// * `slippage_bps` — Allowed slippage in basis points (100 bps = 1 %).
    ///
    /// # Errors
    ///
    /// Returns `Err` on network failure, a non-2xx HTTP response, or a
    /// response body that cannot be deserialised as a Jupiter quote.
    pub async fn get_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<JupiterQuote> {
        // Safety: this assert fires if someone constructs a non-dry-run client
        // and attempts to reach the swap execution path.
        assert!(
            self.dry_run,
            "SAFETY ABORT: attempted live swap execution in dry-run JupiterClient"
        );

        let url = format!(
            "{JUPITER_QUOTE_URL}?inputMint={input_mint}\
             &outputMint={output_mint}&amount={amount}&slippageBps={slippage_bps}"
        );

        info!(
            input_mint,
            output_mint,
            amount,
            slippage_bps,
            dry_run = self.dry_run,
            "[JupiterClient::get_quote] → fetching price quote (no swap executed)"
        );

        let response = self
            .http
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Jupiter API request failed — check network connectivity")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Jupiter API error HTTP {status}: {body}");
        }

        let raw: JupiterQuoteRaw = response
            .json()
            .await
            .context("failed to parse Jupiter quote JSON response")?;

        let quote = JupiterQuote::from_raw(raw);

        info!(
            out_amount     = quote.out_amount,
            out_amount_ui  = quote.out_amount_ui,
            price_impact   = %quote.price_impact_pct,
            routes         = quote.routes.len(),
            dry_run        = self.dry_run,
            "[JupiterClient::get_quote] ✓ quote received [DRY-RUN — NOT executed]"
        );

        Ok(quote)
    }
}

// ── Raw API types (Jupiter V6 response schema) ────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JupiterQuoteRaw {
    input_mint: String,
    in_amount: String,
    output_mint: String,
    out_amount: String,
    #[allow(dead_code)]
    other_amount_threshold: String,
    #[allow(dead_code)]
    swap_mode: String,
    slippage_bps: u16,
    price_impact_pct: String,
    route_plan: Vec<RoutePlanRaw>,
    context_slot: Option<u64>,
    #[allow(dead_code)]
    time_taken: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RoutePlanRaw {
    swap_info: SwapInfoRaw,
    percent: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwapInfoRaw {
    amm_key: String,
    label: Option<String>,
    #[allow(dead_code)]
    input_mint: String,
    #[allow(dead_code)]
    output_mint: String,
    in_amount: String,
    out_amount: String,
    fee_amount: String,
    #[allow(dead_code)]
    fee_mint: String,
}

// ── Domain types (serialisable for rig-core tool output) ─────────────────────

/// Cleaned Jupiter quote — serialisable as a `rig-core` tool response.
#[derive(Debug, Clone, Serialize)]
pub struct JupiterQuote {
    /// Input token mint address.
    pub input_mint: String,
    /// Output token mint address.
    pub output_mint: String,
    /// Input amount in lamports.
    pub in_amount: u64,
    /// Output amount in smallest unit of output mint (USDC micro-units).
    pub out_amount: u64,
    /// Human-readable USDC output (out_amount ÷ 1 000 000).
    pub out_amount_ui: f64,
    /// Price impact as a percentage string from the Jupiter API.
    pub price_impact_pct: String,
    /// Slippage tolerance in basis points.
    pub slippage_bps: u16,
    /// Individual swap route legs selected by the Jupiter aggregator.
    pub routes: Vec<SwapRoute>,
    /// Always `true` in this crate — guarantees no swap was executed.
    pub dry_run: bool,
    /// Solana slot at which the quote was valid.
    pub context_slot: Option<u64>,
}

/// A single aggregated route leg.
#[derive(Debug, Clone, Serialize)]
pub struct SwapRoute {
    /// AMM label (e.g. `"Raydium"`, `"Orca"`, `"Whirlpool"`).
    pub label: String,
    /// On-chain AMM program address.
    pub amm_key: String,
    /// Input amount for this leg.
    pub in_amount: u64,
    /// Output amount for this leg.
    pub out_amount: u64,
    /// Fee charged by this AMM in the input token.
    pub fee_amount: u64,
    /// Percentage of the total input routed through this leg.
    pub percent: u8,
}

impl JupiterQuote {
    fn from_raw(raw: JupiterQuoteRaw) -> Self {
        let out_amount: u64 = raw.out_amount.parse().unwrap_or(0);
        let out_amount_ui = out_amount as f64 / 1_000_000.0; // USDC has 6 decimals

        let routes = raw
            .route_plan
            .into_iter()
            .map(|r| SwapRoute {
                label: r.swap_info.label.unwrap_or_else(|| "Unknown".into()),
                amm_key: r.swap_info.amm_key,
                in_amount: r.swap_info.in_amount.parse().unwrap_or(0),
                out_amount: r.swap_info.out_amount.parse().unwrap_or(0),
                fee_amount: r.swap_info.fee_amount.parse().unwrap_or(0),
                percent: r.percent,
            })
            .collect();

        Self {
            input_mint: raw.input_mint,
            output_mint: raw.output_mint,
            in_amount: raw.in_amount.parse().unwrap_or(0),
            out_amount,
            out_amount_ui,
            price_impact_pct: raw.price_impact_pct,
            slippage_bps: raw.slippage_bps,
            routes,
            dry_run: true,
            context_slot: raw.context_slot,
        }
    }
}

impl std::fmt::Display for JupiterQuote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "JupiterQuote {{ in={} lamports → out={:.6} USDC, \
             impact={}, routes={}, DRY-RUN=true }}",
            Lamports(self.in_amount),
            self.out_amount_ui,
            self.price_impact_pct,
            self.routes.len(),
        )
    }
}
