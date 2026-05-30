# polar-bear-rig-onchain

**rig-onchain-kit Agent: Solana Balance → Jupiter Swap (dry-run) → SignerContext Isolation**

[![Crates.io](https://img.shields.io/crates/v/polar-bear-rig-onchain.svg)](https://crates.io/crates/polar-bear-rig-onchain)
[![Docs.rs](https://docs.rs/polar-bear-rig-onchain/badge.svg)](https://docs.rs/polar-bear-rig-onchain)
[![Rust](https://img.shields.io/badge/rust-1.93.1+-orange)](https://www.rust-lang.org/)
[![Edition](https://img.shields.io/badge/Edition-2024-blue)](https://doc.rust-lang.org/edition-guide/)
[![rig-core](https://img.shields.io/badge/rig--core-0.37+-purple)](https://rig.rs)
[![Solana](https://img.shields.io/badge/Solana-Devnet-9945FF)](https://solana.com)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE-MIT)

> Built by [Murtaza Ali Imtiaz](https://github.com/murtazaai) · Technology Lead · **Polar Bear Systems** · July 2019 – Present

A production-grade Rust implementation of the `rig-onchain-kit` on-chain agent pattern, powered
by [Rig (ARC)](https://rig.rs). Demonstrates Solana devnet balance queries, Jupiter V6 dry-run
swap quotes, and **task-local `SignerContext`** signer isolation via `tokio::task_local!` - the
correct async-safe implementation of the `rig-onchain-kit` security boundary pattern.

---

## API key requirement

`ANTHROPIC_API_KEY` is **only required for `--mode full` without `--no-agent`**. Every other
invocation - including `--mode full --no-agent` - works without it.

| Invocation | API key needed? |
|---|---|
| `--mode full` | ✅ Yes - constructs the rig-core agent |
| `--mode full --no-agent` | ❌ No - runs balance → quote → signer directly |
| `--mode balance` | ❌ No |
| `--mode quote` | ❌ No |
| `--mode signer` | ❌ No |
| `cargo test` | ❌ No - live provider tests are `#[ignore]`-gated |
| `cargo run --example *` | ❌ No |

---

## `--no-agent` flag

Pass `--no-agent` alongside `--mode full` to bypass the rig-core agent call entirely. The
pipeline still runs all three on-chain subsystems in sequence (balance → quote → signer) but
skips constructing the Anthropic client, so **no API key is needed**.

```bash
# Full pipeline output, no API key required
cargo run --release -- --mode full --no-agent --amount 0.1
```

Useful for:
- **CI / CD** - smoke-test the on-chain subsystems without injecting secrets
- **Local development** - iterate on balance/quote/signer logic before wiring the agent
- **Demos** - run the complete pipeline output in a keyless environment

When the flag is absent, `--mode full` behaves as before: it requires `ANTHROPIC_API_KEY`
and returns a clear error if it is missing.

---

## Architecture

```
  CLI Entry (main.rs)
  --mode [full|balance|quote|signer]  [--no-agent]
       │
       ├─ --mode full (default) ──────────────────────────── requires API key
       │   ┌──────────────────────────────────────────────┐
       │   │  rig-core Agent (claude-sonnet-4-6)          │
       │   │  PEV loop: SolanaBalanceTool                 │
       │   │            JupiterQuoteTool (dry-run)        │
       │   │            SignerIsolationTool               │
       │   └──────────────────────────────────────────────┘
       │
       └─ --mode full --no-agent ────────────────────────── no API key needed
           balance → quote → signer (direct, no LLM)
       │
  ┌────┴───────────────────────────────┐
  │  onchain::signer                   │──── tokio::task_local! SignerContext
  │  with_signer(signer, f)            │     (task-scoped, not thread-scoped)
  ├────────────────────────────────────┤
  │  onchain::balance                  │──── Solana devnet RPC get_balance
  │  onchain::jupiter                  │──── Jupiter V6 /quote (read-only)
  └────────────────────────────────────┘
```

---

## Tech stack

| Layer | Technology |
|---|---|
| AI Agent Framework | [rig-core](https://crates.io/crates/rig-core) (Rig / ARC) ≥ 0.37 |
| On-chain bridge | rig-onchain-kit pattern (`tokio::task_local!` SignerContext) |
| Async runtime | Tokio |
| Blockchain | Solana (devnet) |
| DEX aggregator | Jupiter V6 `/quote` (dry-run, no txns) |
| CLI | clap |
| Logging | tracing + tracing-subscriber |
| IDE | Zed (tasks, debugger, rust-analyzer LSP) |

---

## Build & run

```bash
# Prerequisites: Rust 1.93.1+ (rustup.rs)
git clone https://github.com/murtazaai/polar-bear-rig-onchain
cd polar-bear-rig-onchain

# Build - no API key needed
cargo build --release

# ── No API key needed ──────────────────────────────────────────────────────

# Full pipeline, agent skipped (balance → quote → signer direct)
cargo run --release -- --mode full --no-agent

# Solana devnet balance query
cargo run --release -- --mode balance

# Jupiter V6 dry-run swap quote
cargo run --release -- --mode quote --amount 0.1

# SignerContext task-local isolation demo (3 concurrent tasks)
cargo run --release -- --mode signer

# Standalone examples
cargo run --example balance_demo
cargo run --example signer_demo
cargo run --example jupiter_dry_run

# ── Full pipeline with rig-core agent (requires ANTHROPIC_API_KEY) ─────────

cp .env.example .env          # then set ANTHROPIC_API_KEY=sk-ant-...
cargo run --release -- --mode full --wallet <DEVNET_ADDRESS> --amount 0.1
```

---

## Tests

```bash
# All deterministic unit tests - no API key needed
cargo test

# Per-module (all keyless, --nocapture for tracing output)
cargo test --test test_balance         -- --nocapture
cargo test --test test_jupiter         -- --nocapture
cargo test --test test_signer_context  -- --nocapture

# Lint
cargo clippy --all-targets -- -D warnings

# Live provider tests - require ANTHROPIC_API_KEY, opt-in only
ANTHROPIC_API_KEY=sk-ant-... cargo test --test providers -- --ignored --test-threads=1 --nocapture
```

The live tests in `tests/providers/anthropic.rs` are gated with `#[ignore]` and are never
included in a plain `cargo test` run. They must be invoked explicitly with `--ignored` and a
valid key in the environment.

---

## Zed IDE

The repository ships a fully configured `.zed/` workspace with three files that wire up the
entire development workflow inside [Zed](https://zed.dev).

### `.zed/tasks.json` - 23 tasks across 6 groups

Open the task picker with **Cmd/Ctrl + Shift + P → "task: spawn"** (or bind to a key).
Tasks are tagged so you can filter by group in the picker.

| Tag | Tasks included |
|---|---|
| `build` | `check`, `build: debug`, `build: release`, `clean` |
| `lint` | `clippy`, `fmt: check`, `doc: CI check` |
| `fmt` | `fmt`, `fmt: check` |
| `docs` | `doc: open`, `doc: open (private items)`, `doc: CI check` |
| `test` | `test: all`, `test: balance`, `test: jupiter`, `test: signer context`, `test: providers ⚠` |
| `run` | `run: full --no-agent`, `run: balance`, `run: quote`, `run: signer demo`, `run: full pipeline ⚠` |
| `example` | `example: balance_demo`, `example: signer_demo`, `example: jupiter_dry_run` |
| `live` | Tasks marked ⚠ that require `ANTHROPIC_API_KEY` |

Every task sets `cwd = $ZED_WORKTREE_ROOT`, `RUST_LOG = polar_bear_rig_onchain=debug`, and
`RUST_BACKTRACE` so tracing output is always visible. The `doc: CI check` task sets
`RUSTDOCFLAGS = "--cfg docsrs -D warnings"` to mirror the docs.rs build exactly.

### `.zed/debug.json` - 9 CodeLLDB launch configurations

Install the **CodeLLDB** extension via **Zed → Extensions** if not already present.
Every debug config calls `"build_task": "build: debug"` before attaching, so the binary
is always fresh. Set breakpoints in the gutter before launching.

| Config | API key? | Notes |
|---|---|---|
| `Debug: --mode full --no-agent` | ❌ | Recommended starting point - steps through all three subsystems |
| `Debug: --mode balance` | ❌ | Isolates the Solana RPC call |
| `Debug: --mode quote` | ❌ | Isolates the Jupiter HTTP call and JSON deserialisation |
| `Debug: --mode signer` | ❌ | Steps through the 3-task `tokio::task_local!` demo |
| `Debug: --mode full ⚠` | ✅ | Full PEV loop; breakpoints in `src/agent/mod.rs` or `tools.rs` |
| `Debug: --mode full --wallet --amount 0.5 ⚠` | ✅ | Custom wallet and amount pre-filled |
| `Debug: example - balance_demo` | ❌ | |
| `Debug: example - signer_demo` | ❌ | |
| `Debug: example - jupiter_dry_run` | ❌ | |

All configs set `RUST_LOG = polar_bear_rig_onchain=debug` and `RUST_BACKTRACE = full` so
structured log output appears in the Zed debug console.

### `.zed/settings.json` - project-level Zed settings

Applies only to this workspace (overrides `~/.config/zed/settings.json`):

- **Format on save** - delegates to rust-analyzer → rustfmt, so `rustfmt.toml`
  (edition 2024, max_width 100, crate-level imports) is respected automatically
- **rust-analyzer check command** - runs `clippy --all-targets -- -D warnings` on save,
  keeping the Zed Problems panel in sync with the `clippy` task
- **Inlay hints** - type hints, parameter hints, chaining hints, closure capture hints,
  lifetime elision hints, reborrow hints
- **Proc macros** - enabled for clap `#[derive(Parser/ValueEnum)]`, serde `#[derive]`,
  and `tokio::task_local!` (with a suppression for false-positive "not expanded" warnings)
- **Completion** - autoimport, fill-arguments snippets, private-editable members
- **Import granularity** - `crate` + `StdExternalCrate` grouping, matching `rustfmt.toml`
- **Code lens** - run/debug/references/implementations rendered above functions and structs
- **Semantic highlighting** - strings, operators, punctuation

---

## Why `task_local!` not `thread_local!`

Tokio tasks can be **moved between OS threads** at any `await` point. Using `thread_local!`
would cause a signer installed on thread T to be visible to a different task that later runs
on thread T - key contamination in a concurrent HFT pipeline. `tokio::task_local!` via
`with_signer(signer, f)` scopes the keypair slot to the **Tokio task**, giving each in-flight
trade its own isolated signing context with zero locking overhead.

---

## Related

- [Architecture Diagram](./docs/architecture.md)
- [STAR Story](./docs/star_story.md)
- [Screen Capture Guide](./docs/screen_capture_guide.md)
- [File Structure](./FILE_STRUCTURE.md)
- [Changelog](./CHANGELOG.md)
- [Contributing](./CONTRIBUTING.md)

- [Rig Framework](https://rig.rs) · [0xPlaygrounds](https://github.com/0xPlaygrounds/rig)
- [Jupiter](https://jup.ag/) · [Solana Program Library](https://spl.solana.com/)
- [polar-bear-rig-hft](https://github.com/murtazaai/polar-bear-rig-hft)

---

## License

Proprietary - © 2026 Murtaza Ali Imtiaz / Polar Bear Systems  
See [LICENSE-PBS](LICENSE-PBS) for permitted use.

Licensed under:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

---

## Author

**Murtaza Ali Imtiaz** · Technology Lead · **Polar Bear Systems** · (July 2019 – Present)

- GitHub: [@murtazaai](https://github.com/murtazaai)
- LinkedIn: [linkedin.com/in/murtazai](https://linkedin.com/in/murtazai)
- Portfolio: [murtazai.com](https://murtazai.com)
