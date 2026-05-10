//! Shared on-chain newtypes and conversion helpers.
//!
//! | Type | Description |
//! |---|---|
//! | [`Lamports`] | Type-safe wrapper around the native SOL denomination |
//! | [`QuoteAmountSol`] | A validated SOL amount used in Jupiter quote requests |

/// Type-safe wrapper around lamports (the smallest SOL denomination).
///
/// 1 SOL = 1 000 000 000 lamports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Lamports(pub u64);

impl Lamports {
    /// The number of lamports in one SOL.
    pub const PER_SOL: u64 = 1_000_000_000;

    /// Convert to human-readable SOL with 9-decimal precision.
    #[must_use]
    pub fn to_sol(self) -> f64 {
        self.0 as f64 / Self::PER_SOL as f64
    }

    /// Construct from a floating-point SOL amount (truncating sub-lamport amounts).
    #[must_use]
    pub fn from_sol(sol: f64) -> Self {
        Self((sol * Self::PER_SOL as f64) as u64)
    }
}

impl std::fmt::Display for Lamports {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} lamports ({:.6} SOL)", self.0, self.to_sol())
    }
}
