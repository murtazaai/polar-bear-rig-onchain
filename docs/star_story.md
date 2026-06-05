# polar-bear-rig-onchain

## STAR Story 

### Situation

Polar Bear (🍨) needed a demonstrable `rig-onchain-kit` integration showing
how a `rig-core` agent safely accesses Solana on-chain data and queries DEX
liquidity while enforcing cryptographic signer isolation between concurrent
HFT agent tasks.

### Task

As Technology Lead, design and implement the `polar-bear-rig-onchain` GitHub
repo covering: `rig-onchain-kit Solana/EVM + SignerContext security
boundaries`. Deliverables: working Rust project on Solana devnet, Solana
balance query, Jupiter V6 dry-run swap quote, structured `SignerContext`
isolation audit log, and full `rig-core` PEV-loop governance.

### Action

- Structured the codebase as a lib + bin crate (matching the `polar-bear-rig-hft`
  standard) with `src/onchain/`, `src/agent/`, and `src/config.rs` modules.
- Upgraded from the unsafe `thread_local!` pattern to `tokio::task_local!` via
  `with_signer(signer, f)` - the correct async-safe signer isolation model
  described in the `rig-onchain-kit` documentation.
- Fixed the `rig-core` 0.36+ client API: `Client::new(api_key)?` (fallible),
  both `CompletionClient` and `ProviderClient` imported for `.agent()` to
  resolve.
- Built three `rig::tool::Tool` implementations: `SolanaBalanceTool`,
  `JupiterQuoteTool` (Jupiter V6 read-only GET, `dry_run = true` baked in with
  a runtime `assert!` guard), `SignerIsolationTool` (snapshots the active
  task-local context for the Reactor GUI audit trail).
- Added `IsolationReport` - serialisable JSON audit record of the signer
  lifecycle (context\_id, pubkey, created\_at, boundary\_sealed flag).
- Implemented `clap`-driven CLI with `--mode [full|balance|quote|signer]` and
  `--wallet` / `--amount` flags, matching the `polar-bear-rig-hft` UX pattern.
- Applied the full tooling stack: `rustfmt.toml` (100 cols, 2024 edition),
  `.clippy.toml` (MSRV 1.93.1, complexity thresholds), `[lints]` table,
  `[package.metadata.docs.rs]`.

### Result

- Fully compilable, zero-warning Rust project (`cargo clippy -D warnings`).
- `tokio::task_local!` isolation verified across three concurrent async tasks
  in `tests/test_signer_context.rs`.
- `IsolationReport` JSON emitted at the end of every pipeline run - direct
  input to the Reactor GUI audit trail.
- Jupiter V6 dry-run quote with route breakdown and price impact log.
- Structured `tracing` output with module-tagged events at every pipeline step.
- `cargo run --example jupiter_dry_run` demonstrates the full flow without
  requiring an Anthropic API key.
