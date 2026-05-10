# Contributing to polar-bear-rig-onchain

> **Polar Bear Systems** · Technology Lead: Murtaza Ali Imtiaz
> This repository is published under a restricted proprietary licence for
> portfolio and reference purposes. See [LICENSE-PBS](LICENSE-PBS) for
> permitted use.

---

## Development environment

### Prerequisites

| Tool | Version | Install |
|---|---|---|
| Rust stable toolchain | ≥ 1.85.0 | `rustup update stable` |
| `rustfmt` | (with toolchain) | `rustup component add rustfmt` |
| `clippy` | (with toolchain) | `rustup component add clippy` |

### Setup

```text
git clone https://github.com/murtazaai/polar-bear-rig-onchain
cd polar-bear-rig-onchain
cp .env.example .env
# Edit .env: set ANTHROPIC_API_KEY=sk-ant-...
```

---

## Workflow

### Build

```text
cargo build           # debug
cargo build --release # optimised
```

### Run

```text
# Full pipeline (dry-run)
cargo run --release -- --mode full --wallet <DEVNET_ADDRESS> --amount 0.1

# Individual subsystems
cargo run --release -- --mode balance
cargo run --release -- --mode quote
cargo run --release -- --mode signer
```

### Examples (no API key required for balance, quote, and signer)

```text
cargo run --example signer_demo
cargo run --example balance_demo
cargo run --example jupiter_dry_run
```

### Tests (no API key required)

```text
cargo test                                      # all deterministic tests
cargo test --test test_signer_context           # signer isolation only
cargo test --test test_balance                  # Lamports conversion only
cargo test --test test_jupiter                  # Jupiter constants only
```

### Live provider tests (API key required)

```text
ANTHROPIC_API_KEY=sk-ant-... cargo test --test providers -- --ignored --test-threads=1
```

### Format, lint, docs

```text
cargo fmt --all                                 # format
cargo clippy --all-targets -- -D warnings       # lint (zero warnings)
cargo doc --open                                # browse API docs
RUSTDOCFLAGS="--cfg docsrs" cargo doc           # with docsrs conditional items
```

---

## Code style

- **Edition**: Rust 2024
- **Max line width**: 100 characters (enforced by `rustfmt.toml`)
- **Imports**: always import both `rig::client::CompletionClient` and
  `rig::client::ProviderClient` when calling `.agent()` on any rig-core
  0.36+ Anthropic client
- **Doc comments**: `//!` for module-level docs; `///` for items; never `///`
  on macro invocation sites (`tokio::task_local!` etc.)
- **Error handling**: always `anyhow::Result`; propagate with `?`; no `unwrap`
  in library code
- **Signer storage**: always `tokio::task_local!` (never `thread_local!` in
  async code)

---

## Extending

### Adding a new tool

1. Add a new `struct MyTool` in `src/agent/tools.rs`.
2. Implement `rig::tool::Tool` for it.
3. Wire it into `agent::build()` in `src/agent/mod.rs` with `.tool(MyTool)`.
4. Add a test in the appropriate `tests/` file.

### Adding a new mode

1. Add a variant to the `Mode` enum in `src/main.rs`.
2. Add a `run_<mode>` async function.
3. Wire it into the `match args.mode` block.
4. Document it in `CHANGELOG.md`.
