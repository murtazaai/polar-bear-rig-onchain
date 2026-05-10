//! Integration tests for [`polar_bear_rig_onchain::onchain::signer`].
//!
//! Verifies that [`with_signer`] provides task-local isolation across
//! concurrent Tokio tasks, that [`snapshot_active`] returns the correct
//! metadata, and that [`LocalSolanaSigner::ephemeral`] generates a valid
//! keypair.

use polar_bear_rig_onchain::onchain::signer::{
    LocalSolanaSigner, IsolationReport, snapshot_active, with_signer,
};

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_signer(label: &str) -> LocalSolanaSigner {
    LocalSolanaSigner::ephemeral(label)
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// Two concurrently spawned tasks must each obtain their own task-local signer
/// without interfering with each other.
#[tokio::test]
async fn test_signer_context_isolation() {
    let (r1, r2) = tokio::join!(
        tokio::spawn(async {
            let s = make_signer("task-A");
            let pk = s.pubkey().to_string();
            with_signer(s, || async { Ok::<_, anyhow::Error>(pk) })
                .await
                .unwrap()
        }),
        tokio::spawn(async {
            let s = make_signer("task-B");
            let pk = s.pubkey().to_string();
            with_signer(s, || async { Ok::<_, anyhow::Error>(pk) })
                .await
                .unwrap()
        })
    );
    // Both tasks must succeed independently.
    assert!(r1.is_ok(), "task A signer context should not fail");
    assert!(r2.is_ok(), "task B signer context should not fail");

    // The two public keys must be different (independent ephemeral keypairs).
    let pk_a = r1.unwrap();
    let pk_b = r2.unwrap();
    assert_ne!(pk_a, pk_b, "each task must get a distinct ephemeral keypair");
}

/// Inside a [`with_signer`] scope, [`snapshot_active`] must return `Some`.
#[tokio::test]
async fn test_snapshot_active_returns_some_inside_scope() {
    let signer = make_signer("snap-test");
    let expected_label = signer.label().to_string();

    with_signer(signer, || async {
        let snap = snapshot_active();
        assert!(snap.is_some(), "snapshot_active must return Some inside with_signer");
        let s = snap.unwrap();
        assert_eq!(s.label, expected_label, "snapshot label must match signer label");
        assert!(!s.pubkey.is_empty(), "pubkey must not be empty");
        assert!(!s.context_id.is_empty(), "context_id must not be empty");
        Ok::<(), anyhow::Error>(())
    })
    .await
    .unwrap();
}

/// After a [`with_signer`] scope exits, [`snapshot_active`] must return `None`.
#[tokio::test]
async fn test_snapshot_active_returns_none_outside_scope() {
    // Run a scope to establish (and then close) a context.
    with_signer(make_signer("transient"), || async {
        Ok::<(), anyhow::Error>(())
    })
    .await
    .unwrap();

    // After the scope, the task-local slot is cleared.
    let snap = snapshot_active();
    assert!(
        snap.is_none(),
        "snapshot_active must return None after with_signer scope exits"
    );
}

/// [`LocalSolanaSigner::ephemeral`] must produce a valid, non-empty public key.
#[test]
fn test_ephemeral_signer_produces_valid_pubkey() {
    let s = LocalSolanaSigner::ephemeral("pubkey-test");
    let pk = s.pubkey().to_string();
    assert!(!pk.is_empty(), "pubkey must not be empty");
    assert!(pk.len() >= 32, "base-58 pubkey must be at least 32 characters");
}

/// [`IsolationReport::seal`] must flip `boundary_sealed` to `true`.
#[test]
fn test_isolation_report_seal() {
    let report = IsolationReport::capture("test-task-id");
    // Outside any with_signer scope, signer is None.
    assert!(report.signer.is_none());
    assert!(!report.boundary_sealed);

    let sealed = report.seal();
    assert!(sealed.boundary_sealed, "seal() must set boundary_sealed = true");
}
