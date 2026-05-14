# polar-bear-rig-onchain

**rig-onchain-kit Agent: Solana Balance → Jupiter Swap (dry-run) → SignerContext Isolation**

> Technology Lead: Murtaza Ali Imtiaz · Polar Bear Systems · July 2019–Present

A production-grade Rust implementation of the `rig-onchain-kit` on-chain agent
pattern, powered by [Rig (ARC)](https://rig.rs). Demonstrates Solana devnet
balance queries, Jupiter V6 dry-run swap quotes, and **task-local
`SignerContext`** signer isolation via `tokio::task_local!` - the correct
async-safe implementation of the `rig-onchain-kit` security boundary pattern.

---

## Architecture

```
  CLI Entry (main.rs)
  --mode [full|balance|quote|signer]
       │
  ┌────┴───────────────────────────────┐
  │  onchain::signer                   │──── tokio::task_local! SignerContext
  │  with_signer(signer, f)            │     (task-scoped, not thread-scoped)
  └────┬───────────────────────────────┘
       │
  ┌────┴───────────────────────────────┐
  │  rig-core Agent (claude-sonnet-4-6)│──── SolanaBalanceTool
  │  PEV loop governance               │     JupiterQuoteTool (dry-run)
  │                                    │     SignerIsolationTool
  └────┬───────────────────────────────┘
       │
  ┌────┴───────────────────────────────┐
  │  onchain::balance                  │──── Solana devnet RPC get_balance
  │  onchain::jupiter                  │──── Jupiter V6 /quote (read-only)
  └────────────────────────────────────┘
```

---

## Tech stack

| Layer | Technology |
|---|---|
| AI Agent Framework | [rig-core](https://crates.io/crates/rig-core) (Rig / ARC) ≥ 0.36 |
| On-chain bridge | rig-onchain-kit pattern (tokio::task_local! SignerContext) |
| Async runtime | Tokio |
| Blockchain | Solana (devnet) |
| DEX aggregator | Jupiter V6 `/quote` (dry-run, no txns) |
| CLI | clap |
| Logging | tracing + tracing-subscriber |

---

## Build

```bash
# Prerequisites: Rust 1.85.0+ (rustup.rs)
git clone https://github.com/murtazaai/polar-bear-rig-onchain
cd polar-bear-rig-onchain
cp .env.example .env
# Add your ANTHROPIC_API_KEY to .env

# Build (release)
cargo build --release

# Run full pipeline (dry-run by default)
cargo run --release -- --mode full --amount 0.1

# Run individual modes
cargo run --release -- --mode balance
cargo run --release -- --mode quote
cargo run --release -- --mode signer

# Run examples (no API key needed for balance, quote, signer)
cargo run --example signer_demo
cargo run --example balance_demo
cargo run --example jupiter_dry_run

# Tests (no API key needed)
cargo test
cargo clippy --all-targets -- -D warnings
```

---

## Why task_local! not thread_local!

Tokio tasks can be **moved between OS threads** at any `await` point. Using
`thread_local!` would cause a signer installed on thread T to be visible to a
different task that later runs on thread T - key contamination in a concurrent
HFT pipeline. `tokio::task_local!` via `with_signer(signer, f)` scopes the
keypair slot to the **Tokio task**, giving each in-flight trade its own
isolated signing context with zero locking overhead.

---

## Related

- [Architecture Diagram](./docs/architecture.md)
- [STAR Story](./docs/star_story.md)
- [Screen Capture Guide](./docs/screen_capture_guide.md)
- [File Structure](./FILE_STRUCTURE.md)

- [Rig Framework](https://rig.rs) · [0xPlaygrounds](https://github.com/0xPlaygrounds/rig)
- [Jupiter](https://jup.ag/) · [Solana Program Library](https://spl.solana.com/)
- [polar-bear-rig-hft](https://github.com/murtazaai/polar-bear-rig-hft)

---

## 📝 License

PBS License: [PBS License](./LICENSE-PBS)

---

## 👤 Author

**Murtaza Ali Imtiaz**

- LinkedIn: [LinkedIn](https://linkedin.com/in/murtazai)
- GitHub: [@murtazaai](https://github.com/murtazaai)
- Portfolio: [murtazai.com](https://murtazai.com)
