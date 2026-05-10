//! Integration tests for [`polar_bear_rig_onchain::onchain::balance`].
//!
//! Verifies conversion helpers and display formatting. Network calls are
//! not made in these tests — all assertions operate on pure Rust code paths.

use polar_bear_rig_onchain::onchain::types::Lamports;

// ── Lamports conversion ───────────────────────────────────────────────────────

/// `Lamports::to_sol` must return exactly 1.0 for 1 000 000 000 lamports.
#[test]
fn test_lamports_to_sol_one_sol() {
    let l = Lamports(1_000_000_000);
    assert!(
        (l.to_sol() - 1.0).abs() < f64::EPSILON,
        "1_000_000_000 lamports must equal 1.0 SOL"
    );
}

/// `Lamports::from_sol` must round-trip 0.1 SOL correctly.
#[test]
fn test_lamports_from_sol_round_trip() {
    let original_sol = 0.1_f64;
    let lamports = Lamports::from_sol(original_sol);
    let recovered = lamports.to_sol();
    assert!(
        (recovered - original_sol).abs() < 1e-9,
        "round-trip 0.1 SOL → lamports → SOL must be lossless within floating-point tolerance"
    );
}

/// `Lamports(0)` must display as zero with the correct format.
#[test]
fn test_lamports_display_zero() {
    let display = format!("{}", Lamports(0));
    assert!(display.contains("0 lamports"), "display must mention lamports");
    assert!(display.contains("SOL"), "display must mention SOL");
}

/// `Lamports::PER_SOL` must equal exactly 1 000 000 000.
#[test]
fn test_lamports_per_sol_constant() {
    assert_eq!(Lamports::PER_SOL, 1_000_000_000, "PER_SOL constant must be 10^9");
}
