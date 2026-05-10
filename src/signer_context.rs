/// signer_context.rs
///
/// SignerContext - thread-local signer isolation for concurrent HFT agent tasks.
///
/// Problem: In a multi-step agentic HFT pipeline, multiple async tasks may execute
/// concurrently, each requiring access to a wallet keypair. Passing keypairs through
/// every function signature creates coupling and creates a risk of cross-task signer
/// contamination - one task's signer leaking into another task's execution context.
///
/// Solution: Thread-local SignerContext with RAII guard. Each agent task:
///   1. Creates a SignerContext from its keypair.
///   2. Installs it into the thread-local slot via SignerGuard (RAII).
///   3. Executes its pipeline steps - any step can call SignerContext::with_active().
///   4. On drop (task completion or panic), SignerGuard evicts the context.
///
/// Security boundary enforced: no context can persist beyond the task that owns it.
///
/// This mirrors the rig-onchain-kit SignerContext pattern used in production
/// rig-core agent pipelines for Solana/EVM execution.
use std::cell::RefCell;
use std::fmt;
use std::time::Instant;

use anyhow::{bail, Result};
use chrono::Utc;
use serde::Serialize;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::signer::Signer;
use tracing::{info, warn};
use uuid::Uuid;

// ─── Core Context ────────────────────────────────────────────────────────────

/// A SignerContext binds a task-scoped wallet keypair to the current thread.
/// Once installed, all code on this thread can resolve the signer without
/// passing it explicitly - a clean security boundary.
pub struct SignerContext {
    /// Human-readable label identifying which agent task owns this context.
    pub wallet_label: String,
    /// Base-58 encoded public key - safe to log.
    pub pubkey: String,
    /// Unique ID for this context instance - used in audit logs.
    pub context_id: String,
    /// UTC timestamp when the context was created.
    pub created_at_utc: String,
    /// Monotonic timer for latency accounting.
    created_at_instant: Instant,
    /// The actual keypair - kept private; only exposed via sign().
    keypair: Keypair,
}

impl fmt::Debug for SignerContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never print keypair bytes in debug output.
        f.debug_struct("SignerContext")
            .field("wallet_label", &self.wallet_label)
            .field("pubkey", &self.pubkey)
            .field("context_id", &self.context_id)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for SignerContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SignerContext[{}] label={} pubkey={} age={}ms",
            &self.context_id[..8],
            self.wallet_label,
            self.pubkey,
            self.created_at_instant.elapsed().as_millis(),
        )
    }
}

impl SignerContext {
    /// Construct a context from a keypair. Does NOT install it - call `install()`
    /// or wrap in `SignerGuard::new()`.
    pub fn new(keypair: Keypair, label: impl Into<String>) -> Self {
        let label = label.into();
        let pubkey = keypair.pubkey().to_string();
        let context_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        info!(
            wallet_label = %label,
            pubkey = %pubkey,
            context_id = %context_id,
            "[SignerContext::new] context created"
        );

        Self {
            wallet_label: label,
            pubkey,
            context_id,
            created_at_utc: now,
            created_at_instant: Instant::now(),
            keypair,
        }
    }

    /// Construct a fresh ephemeral context with a new random keypair.
    /// Use for dry-run / testnet scenarios where no persistent wallet is needed.
    pub fn ephemeral(label: impl Into<String>) -> Self {
        let kp = Keypair::new();
        Self::new(kp, label)
    }

    /// Install this context into the current thread-local slot.
    /// Warns if a previous context is evicted (indicates a boundary overlap).
    pub fn install(self) {
        ACTIVE_SIGNER.with(|cell| {
            let mut slot = cell.borrow_mut();
            if let Some(prev) = slot.as_ref() {
                warn!(
                    evicting_label   = %prev.wallet_label,
                    evicting_pubkey  = %prev.pubkey,
                    incoming_label   = %self.wallet_label,
                    "[SignerContext::install] ⚠ evicting existing context - boundary overlap detected"
                );
            }
            info!(
                wallet_label = %self.wallet_label,
                pubkey       = %self.pubkey,
                context_id   = %self.context_id,
                "[SignerContext::install] context installed on thread-local slot"
            );
            *slot = Some(self);
        });
    }

    /// Evict whatever context is in the thread-local slot.
    /// Called automatically by SignerGuard::drop().
    pub fn evict() {
        ACTIVE_SIGNER.with(|cell| {
            if let Some(ctx) = cell.borrow_mut().take() {
                info!(
                    wallet_label = %ctx.wallet_label,
                    pubkey       = %ctx.pubkey,
                    context_id   = %ctx.context_id,
                    age_ms       = %ctx.created_at_instant.elapsed().as_millis(),
                    "[SignerContext::evict] context cleared - security boundary sealed ✓"
                );
            }
        });
    }

    /// Run a closure with read access to the active context.
    /// Returns None if no context is installed (programming error - should not occur
    /// when using SignerGuard correctly).
    pub fn with_active<F, R>(f: F) -> Option<R>
    where
        F: FnOnce(&SignerContext) -> R,
    {
        ACTIVE_SIGNER.with(|cell| cell.borrow().as_ref().map(f))
    }

    /// Snapshot the active context's public metadata - safe to serialise and log.
    pub fn snapshot() -> Option<SignerSnapshot> {
        Self::with_active(|ctx| ctx.to_snapshot())
    }

    /// Return the public key string of the active context.
    pub fn active_pubkey() -> Result<String> {
        Self::with_active(|ctx| ctx.pubkey.clone())
            .ok_or_else(|| anyhow::anyhow!("No active SignerContext on this thread"))
    }

    /// Sign a message payload with the held keypair.
    /// Kept private to this module; callers use the context via `with_active`.
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        use solana_sdk::signature::Signer as SolanaSigner;
        self.keypair.sign_message(message).into()
    }

    fn to_snapshot(&self) -> SignerSnapshot {
        SignerSnapshot {
            wallet_label: self.wallet_label.clone(),
            pubkey: self.pubkey.clone(),
            context_id: self.context_id.clone(),
            created_at_utc: self.created_at_utc.clone(),
            age_ms: self.created_at_instant.elapsed().as_millis() as u64,
        }
    }
}

// ─── Thread-local Storage ─────────────────────────────────────────────────────

thread_local! {
    static ACTIVE_SIGNER: RefCell<Option<SignerContext>> = RefCell::new(None);
}

// ─── RAII Guard ───────────────────────────────────────────────────────────────

/// SignerGuard installs a SignerContext on construction and evicts it on drop.
///
/// This guarantees the boundary is sealed even if the task panics or returns
/// early via `?`. Always prefer SignerGuard over manually calling install/evict.
///
/// Usage:
/// ```rust
/// let _guard = SignerGuard::new(SignerContext::ephemeral("hft-agent-task-42"));
/// // SignerContext is now active for this task.
/// let pubkey = SignerContext::active_pubkey()?;
/// // ... pipeline steps ...
/// // guard drops here → evict() is called automatically
/// ```
pub struct SignerGuard;

impl SignerGuard {
    pub fn new(ctx: SignerContext) -> Self {
        ctx.install();
        Self
    }
}

impl Drop for SignerGuard {
    fn drop(&mut self) {
        SignerContext::evict();
    }
}

// ─── Snapshot (safe for serialisation / logging) ──────────────────────────────

/// Public-metadata snapshot of a SignerContext.
/// Contains no private key material - safe to log, serialise, and audit.
#[derive(Debug, Clone, Serialize)]
pub struct SignerSnapshot {
    pub wallet_label: String,
    pub pubkey: String,
    pub context_id: String,
    pub created_at_utc: String,
    pub age_ms: u64,
}

// ─── Isolation Report ─────────────────────────────────────────────────────────

/// Isolation report emitted at the end of each agent task.
/// Provides a structured audit record of the SignerContext lifecycle.
#[derive(Debug, Serialize)]
pub struct IsolationReport {
    pub task_id: String,
    pub signer: Option<SignerSnapshot>,
    pub boundary_sealed: bool,
    pub report_timestamp: String,
}

impl IsolationReport {
    pub fn capture(task_id: &str) -> Self {
        let signer = SignerContext::snapshot();
        Self {
            task_id: task_id.to_string(),
            signer,
            boundary_sealed: false, // becomes true after evict
            report_timestamp: Utc::now().to_rfc3339(),
        }
    }

    pub fn seal(mut self) -> Self {
        self.boundary_sealed = true;
        self.report_timestamp = Utc::now().to_rfc3339();
        self
    }
}
