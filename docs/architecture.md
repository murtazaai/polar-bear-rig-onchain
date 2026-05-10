# polar-bear-rig-onchain

## High-Level Architecture

```
┌──────────────────────────────────────────────────────────────────────────┐
│              polar-bear-rig-onchain  ·  Polar Bear Systems               │
│         rig-onchain-kit  ·  SignerContext  ·  Solana devnet              │
└──────────────────────────────────────────────────────────────────────────┘

            CLI Entry Point (main.rs)
            --mode [full|balance|quote|signer]
                         │
         ┌───────────────┴────────────────────────────────┐
         │                                                │
         ▼                                                ▼
┌─────────────────────┐                      ┌────────────────────────────┐
│  onchain::signer    │                      │  agent (rig-core)          │
│                     │                      │                            │
│ tokio::task_local!  │                      │ SolanaBalanceTool          │
│ LocalSolanaSigner   │◄─── installed via ───│   → onchain::balance       │
│ with_signer(f)      │    SignerGuard        │                            │
│ snapshot_active()   │                      │ JupiterQuoteTool           │
│ IsolationReport     │                      │   → onchain::jupiter       │
└──────────┬──────────┘                      │                            │
           │                                 │ SignerIsolationTool         │
           │                                 │   → onchain::signer        │
           ▼                                 └──────────────┬─────────────┘
┌──────────────────────────────────────────────────────────▼─────────────┐
│  rig-core Agent Pipeline  (claude-sonnet-4-6)                          │
│                                                                        │
│  PERCEIVE → EVALUATE (balance + quote) → ACT (isolation log)           │
│                                                                        │
│  PEV Loop governance across the Agentic Web                            │
└─────────────────────────────────────────────────────────────────────────┘
                         │
          ┌──────────────┴──────────────────────┐
          │                                     │
          ▼                                     ▼
┌──────────────────────┐            ┌───────────────────────────────────┐
│  onchain::balance    │            │  onchain::jupiter                 │
│                      │            │                                   │
│  SolanaClient        │            │  JupiterClient                    │
│  (devnet-only)       │            │  (dry_run = true, always)         │
│  get_balance(pubkey) │            │  GET /v6/quote                    │
│  → BalanceResult     │            │  → JupiterQuote                   │
│                      │            │  runtime assert! guards           │
│  Solana devnet RPC   │            │  any live swap attempt            │
└──────────────────────┘            └───────────────────────────────────┘
```

## Security boundary: task-local vs thread-local

The original `thread_local!` pattern is **unsafe** in an async runtime:

- Tokio tasks can be *moved* between OS threads at any `await` point.
- A `thread_local!` signer installed on thread T may be read by a different
  task that later runs on thread T - data contamination.

This repo uses `tokio::task_local!` via `with_signer(signer, f)`:

- The slot is scoped to the **Tokio task**, not the OS thread.
- Multiple tasks can run on the same thread pool concurrently; each sees only
  its own signer.
- When the `with_signer` scope exits (normally, via `?`, or via panic), the
  task-local slot is automatically cleared.

## Module map

```
src/
├── lib.rs                crate root; re-exports agent, config, onchain
├── main.rs               binary entry; clap CLI; mode dispatch
├── config.rs             Config::from_env(); ANTHROPIC_API_KEY etc.
├── agent/
│   ├── mod.rs            build() → impl Prompt; rig-core agent assembly
│   └── tools.rs          SolanaBalanceTool, JupiterQuoteTool, SignerIsolationTool
└── onchain/
    ├── mod.rs            execute_pipeline(), demo_signer() - public entry points
    ├── signer.rs         tokio::task_local! SignerContext; with_signer; IsolationReport
    ├── balance.rs        SolanaClient::devnet(); query_balance; BalanceResult
    ├── jupiter.rs        JupiterClient::dry_run(); get_quote; JupiterQuote
    └── types.rs          Lamports newtype; conversion helpers
```
