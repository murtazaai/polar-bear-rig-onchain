//! Integration tests for [`polar_bear_rig_onchain::onchain::jupiter`].
//!
//! Verifies token mint address constants, the default slippage constant, and
//! the `JupiterClient::dry_run` constructor. Network calls are not made; all
//! tests are purely deterministic.

use polar_bear_rig_onchain::onchain::jupiter::{
    DEFAULT_SLIPPAGE_BPS, SOL_MINT, USDC_MINT,
};

// ── Constants ─────────────────────────────────────────────────────────────────

/// The wrapped-SOL mint address must match the well-known mainnet address.
#[test]
fn test_sol_mint_address() {
    assert_eq!(
        SOL_MINT,
        "So11111111111111111111111111111111111111112",
        "SOL_MINT must be the wrapped-SOL mainnet address"
    );
}

/// The USDC mint address must match the well-known mainnet address.
#[test]
fn test_usdc_mint_address() {
    assert_eq!(
        USDC_MINT,
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "USDC_MINT must be the mainnet USDC address"
    );
}

/// The default slippage tolerance must be 50 basis points (0.5 %).
#[test]
fn test_default_slippage_bps() {
    assert_eq!(
        DEFAULT_SLIPPAGE_BPS, 50,
        "DEFAULT_SLIPPAGE_BPS must be 50 (0.5 %)"
    );
}

/// `JupiterClient::dry_run` must construct without panicking.
#[test]
fn test_dry_run_constructor_does_not_panic() {
    let _ = polar_bear_rig_onchain::onchain::jupiter::JupiterClient::dry_run();
}
