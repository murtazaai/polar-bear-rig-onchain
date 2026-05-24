# Changelog

All notable changes to `polar-bear-rig-onchain` are documented here.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.2.0] - 2026-05-25

### Removed

- `src/signer_context.rs` — orphan file from the pre-module-refactor era. Used
  `thread_local!` (the security bug documented in Fix 1) with `uuid` and
  `chrono` dependencies absent from `Cargo.toml`. The correct implementation
  lives in `src/onchain/signer.rs` via `tokio::task_local!`.
- `src/solana_ops.rs` — orphan duplicate of `src/onchain/balance.rs`. Neither
  file was referenced from `lib.rs`; removing them eliminates dead code and
  resolves the `missing_docs` lint warnings they triggered.

### Fixed

- **Fix 6 (complete)** — `rig::client::ProviderClient` added alongside
  `CompletionClient` in `src/agent/mod.rs`. Both traits must be in scope for
  `.agent()` to resolve on `anthropic::Client` in rig-core ≥ 0.36; only
  `CompletionClient` was present in the previous commit.
- **README badges** — corrected `rust-1.93.1+` → `rust-1.85.0+` and
  `rig-core-0.37` → `rig-core-0.36+` to match `Cargo.toml` values.

## [Unreleased]

### Added

- `src/lib.rs` - crate root; re-exports `agent`, `config`, `onchain` for integration tests.
- `src/config.rs` - `Config::from_env()`; mirrors `polar-bear-rig-hft` config pattern.
- `src/onchain/mod.rs` - `execute_pipeline()` and `demo_signer()` public entry points.
- `src/onchain/types.rs` - `Lamports` newtype with `to_sol()` / `from_sol()` helpers.
- `src/agent/mod.rs` - `agent::build()` with corrected `Client::new(api_key)?` pattern.
- `src/agent/tools.rs` - `SolanaBalanceTool`, `JupiterQuoteTool`, `SignerIsolationTool`.
- `examples/signer_demo.rs` - three-task SignerContext isolation demo (no API key needed).
- `examples/balance_demo.rs` - Solana devnet balance query demo.
- `examples/jupiter_dry_run.rs` - Jupiter dry-run quote + IsolationReport demo.
- `tests/test_signer_context.rs` - task-local isolation, snapshot, seal tests.
- `tests/test_balance.rs` - `Lamports` conversion and display tests.
- `tests/test_jupiter.rs` - mint address constants and `dry_run` constructor tests.
- `tests/providers/anthropic.rs` - live provider tests gated behind `#[ignore]`.
- `docs/architecture.md` - system architecture deep-dive.
- `docs/star_story.md` - STAR story.
- `docs/screen_capture_guide.md` - demo run instructions.
- `rustfmt.toml` - code style (100 cols, 2024 edition, crate-level imports).
- `.clippy.toml` - Clippy MSRV 1.85.0, cognitive complexity thresholds.
- `.zed/tasks.json` / `.zed/debug.json` - Zed IDE task runner and debugger config.
- `CHANGELOG.md` - this file.
- `CONTRIBUTING.md` - development workflow guide.
- `FILE_STRUCTURE.md` - annotated repository layout.

### Changed

- `Cargo.toml` - **edition `2021` → `2024`**; `rust-version` `1.75` → `1.85.0`.
- `Cargo.toml` - added `[lints.rust]` and `[lints.clippy]` tables; `[package.metadata.docs.rs]`.
- `Cargo.toml` - `reqwest "0.12"` → `"^0.13"`, feature `rustls-tls` → `rustls` (Fix 3).
- `Cargo.toml` - `dotenv = "0.15"` → `dotenvy = "^0.15"` (Fix 4).
- `Cargo.toml` - added `thiserror = "^2"`, `rand = "^0.8"`, `clap = "^4"`.
- `Cargo.toml` - added `panic = "abort"`, `strip = "debuginfo"` to release profile.
- `Cargo.toml` - added `[dev-dependencies]`: `tokio-test`, `mockall`.
- Flat `src/` → module hierarchy: `src/onchain/`, `src/agent/`, `src/config.rs`.
- `src/main.rs` - `///` module docs → `//!`; `dotenv` → `dotenvy`; added `clap` CLI.
- `src/main.rs` - added `--mode [full|balance|quote|signer]`, `--wallet`, `--amount` flags.
- `.gitignore` - replaced broad ignore file with focused Rust-only version (mirrors HFT).

### Fixed

- **Fix 1** - `thread_local! { static ACTIVE_SIGNER }` replaced with
  `tokio::task_local! { static CURRENT_SIGNER }` in `onchain/signer.rs`.
  `thread_local!` is **unsafe in async code**: Tokio tasks can migrate between
  OS threads at any `await` point, causing one task's keypair to leak into
  another task's context. `task_local!` scopes the slot to the Tokio task,
  not the thread.

- **Fix 2** - `anthropic::ClientBuilder::new(api_key).build()` replaced with
  `anthropic::Client::new(api_key)?`. `Client::new` is **fallible** in
  rig-core ≥ 0.36 and must be propagated with `?`; `ClientBuilder` was the
  pre-0.36 API.

- **Fix 3** - `reqwest` feature `rustls-tls` renamed to `rustls` in 0.13.
  The old feature name causes a compile error.

- **Fix 4** - crate `dotenv` is unmaintained; replaced with the maintained
  fork `dotenvy`. Call site updated: `dotenv::dotenv()` → `dotenvy::dotenv()`.

- **Fix 5** - `///` at the top of every `src/*.rs` file were item doc comments
  attached to the `mod` declarations in `main.rs`, not module-level docs.
  Converted to `//!` inner doc comments in the respective files.

- **Fix 6** - `rig::client::CompletionClient` and `rig::client::ProviderClient`
  were absent from the `use` statement in `agent/mod.rs`. Both traits must be
  in scope to call `.agent()` on an `anthropic::Client` in rig-core ≥ 0.36.

---

## [0.1.0] - 2026-05-07

Initial implementation:

- rig-core agent pipeline with three tool implementations.
- Solana devnet balance query via `solana-client` RPC.
- Jupiter V6 dry-run swap quote (SOL → USDC).
- SignerContext RAII boundary.
- README with architecture diagram and STAR story section.
