# Repository File Structure

```
polar-bear-rig-onchain/
│
│  ── Root tooling & meta ────────────────────────────────────────────
├── Cargo.toml             Rust 2024 edition; all dependencies; lint + metadata tables
├── Cargo.lock             Committed (binary crate); delete + regenerate on dep changes
├── rustfmt.toml           Code-style rules (100 cols, 2024 edition, crate-level imports)
├── .clippy.toml           Clippy config (MSRV 1.85.0, complexity thresholds)
├── .gitignore             Focused Rust-only ignore file
├── .env.example           Template for ANTHROPIC_API_KEY and other env vars
├── LICENSE-PBS            Polar Bear Systems proprietary licence
├── README.md              Project overview and quick-start
├── CHANGELOG.md           Version history (all bug fixes documented)
├── CONTRIBUTING.md        Dev setup, workflow, code-style, test instructions
├── FILE_STRUCTURE.md      This file
│
│  ── Zed IDE config ─────────────────────────────────────────────────
├── .zed/
│   ├── tasks.json         Task runner (build, run, test, clippy, fmt, examples)
│   └── debug.json         LLDB debugger launch configurations
│
│  ── Documentation ──────────────────────────────────────────────────
├── docs/
│   ├── architecture.md    System architecture; module map; task-local vs thread-local
│   ├── star_story.md      STAR story
│   └── screen_capture_guide.md   Demo run instructions for screen capture
│
│  ── Standalone runnable examples ───────────────────────────────────
│  (cargo run --example <name>; no API key needed except full-pipeline)
├── examples/
│   ├── signer_demo.rs       SignerContext task-local isolation across 3 tasks
│   ├── balance_demo.rs      Solana devnet balance query
│   └── jupiter_dry_run.rs   Jupiter V6 dry-run quote + IsolationReport
│
│  ── Library source ─────────────────────────────────────────────────
├── src/
│   ├── lib.rs               Crate root; re-exports agent, config, onchain
│   ├── main.rs              Binary entry point; clap CLI arg parsing; mode dispatch
│   ├── config.rs            Config::from_env(); reads ANTHROPIC_API_KEY etc.
│   │
│   ├── agent/               rig-core agent layer
│   │   ├── mod.rs           build() → impl Prompt; AGENT_PREAMBLE; rig-core 0.36 imports
│   │   └── tools.rs         SolanaBalanceTool, JupiterQuoteTool, SignerIsolationTool
│   │
│   └── onchain/             On-chain execution layer
│       ├── mod.rs           execute_pipeline(), demo_signer() — public entry points
│       ├── signer.rs        tokio::task_local! SignerContext; with_signer; IsolationReport
│       ├── balance.rs       SolanaClient::devnet(); query_balance; BalanceResult
│       ├── jupiter.rs       JupiterClient::dry_run(); get_quote; JupiterQuote
│       └── types.rs         Lamports newtype; conversion helpers
│
│  ── Integration tests (no API key required) ─────────────────────────
├── tests/
│   ├── test_signer_context.rs   task-local isolation, snapshot_active, IsolationReport
│   ├── test_balance.rs          Lamports conversion, display
│   ├── test_jupiter.rs          mint address constants, dry_run constructor
│   │
│   └── providers/               Live provider tests - gated behind #[ignore]
│       └── anthropic.rs         Requires ANTHROPIC_API_KEY; run with --ignored
```

## Key design decisions

| Decision | Rationale |
|---|---|
| `tokio::task_local!` not `thread_local!` | Tokio tasks can migrate between OS threads at `await` points; `task_local!` scopes the signer slot to the task, not the thread |
| Lib + bin targets | Integration tests are external crates; lib exposes `polar_bear_rig_onchain::` |
| Rust 2024 edition | Matches the rig upstream repository and `polar-bear-rig-hft` |
| `Client::new(api_key)?` | `Client::new` is **fallible** in rig-core ≥ 0.36; `ClientBuilder` is the pre-0.36 API |
| Both `CompletionClient` + `ProviderClient` | Both traits must be in scope for `.agent()` to resolve in rig-core ≥ 0.36 |
| `dotenvy` not `dotenv` | `dotenv` crate is unmaintained; `dotenvy` is the maintained fork |
| `reqwest "^0.13"` + `rustls` feature | `rustls-tls` was renamed to `rustls` in reqwest 0.13; old feature name causes compile error |
| `#[ignore]` on live tests | Prevents CI failures when `ANTHROPIC_API_KEY` is absent |
| `strip = "debuginfo"` in release | Reduces binary size; mirrors rig release profile |
| `//!` not `///` at file tops | `///` at file scope with no following item triggers `unused_doc_comments` lint |
