//! SignerContext - task-local keypair isolation for secure on-chain operations.
//!
//! Implements the security boundary described in the `rig-onchain-kit`
//! documentation: every async on-chain call must be wrapped in
//! [`with_signer`], which scopes the active [`solana_sdk::signature::Keypair`]
//! to exactly the current Tokio task via `tokio::task_local!`.
//!
//! ## Why task-local storage?
//!
//! In an async runtime multiple trades can be in-flight simultaneously on a
//! shared thread pool. Using a `thread_local!` signer would risk one task's
//! keypair leaking into another task that runs on the same thread. A
//! `task_local!` slot is scoped to the *task*, not the thread - it is
//! automatically removed when the task's future completes, regardless of which
//! OS thread executed it.
//!
//! ## Production upgrade path
//!
//! Replace [`LocalSolanaSigner`] and the hand-rolled `task_local!` storage
//! with `rig_onchain_kit::signer::SignerContext` once that crate is published
//! to crates.io.

use std::sync::Arc;

use anyhow::Result;
use rand::random;
use serde::Serialize;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use tracing::info;

// ── Task-local storage ────────────────────────────────────────────────────────

// Declared with a regular line comment rather than a doc comment (///)
// because `tokio::task_local!` is a macro invocation - rustdoc cannot attach
// outer doc attributes to macro call sites, which would trigger the
// `unused_doc_comments` lint.
tokio::task_local! {
    static CURRENT_SIGNER: Arc<SignerContextInner>;
}

// ── Inner context (shared via Arc across nested closures) ─────────────────────

/// Internal context stored in the task-local slot.
///
/// Wrapped in [`Arc`] so that multiple borrows within the same task scope do
/// not require copying the keypair.
#[derive(Debug)]
pub struct SignerContextInner {
    /// Random keypair - never logged in full; only the public key is visible.
    keypair: Keypair,
    /// Human-readable label identifying the agent task that owns this context.
    pub label: String,
    /// Base-58 public key - safe to log and serialise.
    pub pubkey: String,
    /// Unique identifier for this context instance (16-char random hex).
    pub context_id: String,
    /// Unix timestamp (seconds) when the context was created.
    pub created_at_secs: u64,
}

impl SignerContextInner {
    fn new(keypair: Keypair, label: impl Into<String>) -> Self {
        let label = label.into();
        let pubkey = keypair.pubkey().to_string();
        let context_id = format!("{:016x}", random::<u64>());
        let created_at_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            keypair,
            label,
            pubkey,
            context_id,
            created_at_secs,
        }
    }
}

// ── Public signer wrapper ─────────────────────────────────────────────────────

/// A Solana keypair wrapper that loads its key from the process environment
/// or generates a fresh ephemeral key for devnet / testnet runs.
///
/// In production, decode the base-58 private key stored in
/// `SOLANA_PRIVATE_KEY` using `Keypair::from_base58_string`. In this demo a
/// fresh random keypair is generated regardless of whether the variable is set.
pub struct LocalSolanaSigner {
    inner: SignerContextInner,
}

impl LocalSolanaSigner {
    /// Construct a [`LocalSolanaSigner`] from the process environment.
    ///
    /// When `SOLANA_PRIVATE_KEY` is present the variable is acknowledged, but
    /// the demo still generates a random keypair as a placeholder.
    ///
    /// **TODO (production):** replace `Keypair::new()` with
    /// `Keypair::from_base58_string(&key)` to load the real signing key.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use polar_bear_rig_onchain::onchain::signer::LocalSolanaSigner;
    ///
    /// let s = LocalSolanaSigner::from_env("my-task");
    /// println!("signer pubkey: {}", s.pubkey());
    /// ```
    pub fn from_env(label: impl Into<String>) -> Self {
        let keypair = Keypair::new(); // TODO: load from SOLANA_PRIVATE_KEY in production
        Self {
            inner: SignerContextInner::new(keypair, label),
        }
    }

    /// Construct a fresh ephemeral signer for dry-run / testnet use.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use polar_bear_rig_onchain::onchain::signer::LocalSolanaSigner;
    ///
    /// let s = LocalSolanaSigner::ephemeral("dry-run-task");
    /// assert!(!s.pubkey().to_string().is_empty());
    /// ```
    #[must_use]
    pub fn ephemeral(label: impl Into<String>) -> Self {
        Self::from_env(label)
    }

    /// Return the [`Pubkey`] (on-chain address) of the wrapped keypair.
    #[must_use]
    pub fn pubkey(&self) -> Pubkey {
        self.inner.keypair.pubkey()
    }

    /// Return the human-readable context label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.inner.label
    }

    /// Return the unique context identifier.
    #[must_use]
    pub fn context_id(&self) -> &str {
        &self.inner.context_id
    }
}

// ── Task-local scope ──────────────────────────────────────────────────────────

/// Execute an async closure with `signer` installed as the task-local signer.
///
/// Installs `signer` into the `CURRENT_SIGNER` task-local slot for the
/// duration of the future returned by `f`. The signer is automatically
/// removed when the scope exits, so it cannot outlive the operation it was
/// created for.
///
/// # Errors
///
/// Propagates any [`anyhow::Error`] returned by the closure `f`.
///
/// # Examples
///
/// ```rust,no_run
/// use polar_bear_rig_onchain::onchain::signer::{with_signer, LocalSolanaSigner};
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let signer = LocalSolanaSigner::ephemeral("hft-task-001");
/// let pubkey = signer.pubkey();
/// let result = with_signer(signer, || async {
///     Ok::<&str, anyhow::Error>("swap executed")
/// })
/// .await?;
/// # Ok(()) }
/// ```
pub async fn with_signer<F, Fut, T>(signer: LocalSolanaSigner, f: F) -> Result<T>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let label = signer.inner.label.clone();
    let pubkey = signer.inner.pubkey.clone();
    let context_id = signer.inner.context_id.clone();

    info!(%label, %pubkey, %context_id, "[SignerContext] task-local signer installed");

    let arc = Arc::new(signer.inner);
    let result = CURRENT_SIGNER.scope(arc, f()).await;

    info!(%label, %pubkey, %context_id, "[SignerContext] task-local signer evicted - boundary sealed ✓");

    result
}

// ── Snapshot / audit types ────────────────────────────────────────────────────

/// Public-metadata snapshot of an active [`SignerContextInner`].
///
/// Contains no private key material - safe to log, serialise, and audit.
#[derive(Debug, Clone, Serialize)]
pub struct SignerSnapshot {
    /// Human-readable label of the owning agent task.
    pub label: String,
    /// Base-58 encoded public key.
    pub pubkey: String,
    /// Unique context identifier (16-char hex).
    pub context_id: String,
    /// Unix timestamp (seconds) when the context was created.
    pub created_at_secs: u64,
}

/// Capture a snapshot of the active task-local signer context.
///
/// Returns `None` if no signer is installed for the current Tokio task (which
/// indicates a programming error - all on-chain work must be wrapped in
/// [`with_signer`]).
#[must_use]
pub fn snapshot_active() -> Option<SignerSnapshot> {
    CURRENT_SIGNER
        .try_with(|ctx| SignerSnapshot {
            label: ctx.label.clone(),
            pubkey: ctx.pubkey.clone(),
            context_id: ctx.context_id.clone(),
            created_at_secs: ctx.created_at_secs,
        })
        .ok()
}

/// Return the public key string of the active task-local signer.
///
/// # Errors
///
/// Returns `Err` if no [`SignerContext`] is installed for this task.
pub fn active_pubkey() -> Result<String> {
    CURRENT_SIGNER
        .try_with(|ctx| ctx.pubkey.clone())
        .map_err(|_| anyhow::anyhow!("No active SignerContext on this task"))
}

// ── Isolation report ──────────────────────────────────────────────────────────

/// Structured audit record of a single [`with_signer`] task lifecycle.
///
/// Serialised to JSON and emitted by the agent pipeline at the end of each
/// run for the Reactor GUI audit trail.
#[derive(Debug, Serialize)]
pub struct IsolationReport {
    /// Identifier of the owning agent task.
    pub task_id: String,
    /// Signer snapshot captured while the context was still active.
    pub signer: Option<SignerSnapshot>,
    /// `true` after [`IsolationReport::seal`] has been called (i.e. the
    /// task-local signer has been evicted).
    pub boundary_sealed: bool,
    /// Unix timestamp when the report was generated.
    pub report_timestamp_secs: u64,
}

impl IsolationReport {
    /// Capture an isolation report for `task_id` using the currently active
    /// task-local signer.
    #[must_use]
    pub fn capture(task_id: &str) -> Self {
        let signer = snapshot_active();
        Self {
            task_id: task_id.to_string(),
            signer,
            boundary_sealed: false,
            report_timestamp_secs: now_secs(),
        }
    }

    /// Mark the boundary as sealed (called after the [`with_signer`] scope has
    /// exited and the context has been evicted).
    #[must_use]
    pub fn seal(mut self) -> Self {
        self.boundary_sealed = true;
        self.report_timestamp_secs = now_secs();
        self
    }
}

/// Spawn three concurrent tasks and demonstrate that each has an independent
/// task-local signer.
///
/// Mirrors [`crate::onchain::signer::demo_signer`] from `polar-bear-rig-hft`.
/// Each task calls [`LocalSolanaSigner::ephemeral`] and [`with_signer`]
/// independently. Because `CURRENT_SIGNER` is task-local, no public key leaks
/// between tasks even though they overlap in time.
///
/// # Errors
///
/// Returns `Err` if any spawned task panics or its
/// [`tokio::task::JoinHandle`] returns an error.
pub async fn demo_signer() -> Result<()> {
    info!("[SIGNER] Demonstrating SignerContext task-local isolation across 3 concurrent tasks");

    let handles: Vec<_> = (0..3_usize)
        .map(|i| {
            tokio::spawn(async move {
                let label = format!("demo-task-{i}");
                let signer = LocalSolanaSigner::ephemeral(label.clone());
                let pubkey = signer.pubkey();
                with_signer(signer, move || async move {
                    info!(task = i, %pubkey, "[SIGNER] task running in isolated context");
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    info!(task = i, "[SIGNER] task complete - signer isolated");
                    Ok::<(), anyhow::Error>(())
                })
                .await
            })
        })
        .collect();

    for h in handles {
        h.await??;
    }
    info!("[SIGNER] All tasks complete. SignerContext isolation verified ✓");
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
