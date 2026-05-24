//! # polar-bear-rig-onchain
//!
//! **Polar Bear Systems** - rig-onchain-kit agent demonstrating Solana on-chain
//! operations with `SignerContext` task-local signer isolation.
//!
//! Technology Lead: Murtaza Ali Imtiaz (July 2019 – present).
//!
//! ## Pipeline
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                 polar-bear-rig-onchain                      │
//! ├──────────┬──────────────────────┬────────────────────────── ┤
//! │  onchain │       agent          │        config             │
//! │ signer   │ SolanaBalanceTool    │ Config::from_env()        │
//! │ balance  │ JupiterQuoteTool     │ ANTHROPIC_API_KEY         │
//! │ jupiter  │ SignerIsolationTool  │ WALLET_ADDRESS            │
//! │ types    │ build_onchain_agent  │ SOLANA_RPC_URL            │
//! └──────────┴──────────────────────┴───────────────────────────┘
//! ```
//!
//! ## Full pipeline
//!
//! 1. **`SignerContext`** ([`onchain::signer`]) - a random ephemeral keypair is
//!    bound to the current Tokio task via `tokio::task_local!`. The
//!    `onchain::signer::SignerGuard` RAII wrapper guarantees the context is
//!    evicted even if the task panics.
//! 2. **Balance query** ([`onchain::balance`]) - Solana devnet RPC
//!    `get_balance` call; returns lamports and SOL denomination.
//! 3. **Jupiter quote** ([`onchain::jupiter`]) - read-only GET to the Jupiter
//!    V6 `/quote` endpoint; `dry_run = true` is baked in and a runtime
//!    `assert!` prevents any accidental swap execution.
//! 4. **Isolation log** ([`onchain::signer`]) - `SignerContext::snapshot()`
//!    emits an [`onchain::signer::SignerSnapshot`] for the Reactor GUI audit
//!    trail.
//!
//! ## Quick start
//!
//! ```text
//! cp .env.example .env   # set ANTHROPIC_API_KEY
//! cargo run --release -- --mode full --wallet <DEVNET_ADDRESS> --amount 0.1
//! ```

pub mod agent;
pub mod config;
pub mod onchain;
