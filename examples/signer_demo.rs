//! `signer_demo` — demonstrate `SignerContext` task-local isolation.
//!
//! Spawns three concurrent Tokio tasks; each installs its own
//! [`polar_bear_rig_onchain::onchain::signer::LocalSolanaSigner`] context and
//! logs its public key. Because `tokio::task_local!` scopes the slot to the
//! owning task (not the OS thread), no public key leaks between tasks even
//! when they overlap in time.
//!
//! ```text
//! cargo run --example signer_demo
//! ```

use anyhow::Result;
use polar_bear_rig_onchain::onchain::signer::{LocalSolanaSigner, with_signer};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("polar_bear_rig_onchain=info,signer_demo=info"))
        .init();

    info!("╔══════════════════════════════════════════════════════════╗");
    info!("║  SIGNER DEMO  ·  task-local SignerContext isolation      ║");
    info!("╚══════════════════════════════════════════════════════════╝");

    let handles: Vec<_> = (0..3_usize)
        .map(|i| {
            tokio::spawn(async move {
                let label = format!("demo-task-{i}");
                let signer = LocalSolanaSigner::ephemeral(&label);
                let pubkey = signer.pubkey().to_string();

                with_signer(signer, move || async move {
                    info!(task = i, %pubkey, "task running in isolated context");
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

                    // Snapshot is visible inside the scope.
                    let snap = polar_bear_rig_onchain::onchain::signer::snapshot_active();
                    assert!(snap.is_some(), "snapshot must be Some inside with_signer");
                    assert_eq!(snap.unwrap().pubkey, pubkey);

                    info!(task = i, "task complete — signer context isolated ✓");
                    Ok::<(), anyhow::Error>(())
                })
                .await
            })
        })
        .collect();

    for h in handles {
        h.await??;
    }

    info!("SignerContext isolation verified across 3 concurrent tasks ✓");
    Ok(())
}
