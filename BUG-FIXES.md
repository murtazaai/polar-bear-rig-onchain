# BUG-FIXES — polar-bear-rig-onchain

Numbered log of every compiler error, lint failure, and dependency conflict
resolved during the Polar Bear Systems automated Rust development process.
Each entry records the error code, root cause, and before/after diff.

---

## Fix 01 — `thread_local!` replaced with `tokio::task_local!` in `signer.rs`

**Error:** Runtime data-race / security bug (no compiler error — latent bug).

**Root cause:** `thread_local!` scopes state to an OS thread. Tokio tasks can
migrate between OS threads at any `.await` point, so a signer installed on
thread T becomes visible to a different task that happens to run on thread T
next. In a concurrent HFT pipeline this is a keypair contamination bug.

**Before:**
```rust
thread_local! {
    static ACTIVE_SIGNER: RefCell<Option<Arc<SignerContextInner>>> = RefCell::new(None);
}
```

**After:**
```rust
tokio::task_local! {
    static CURRENT_SIGNER: Arc<SignerContextInner>;
}
```

**File:** `src/onchain/signer.rs`

---

## Fix 02 — `anthropic::ClientBuilder::new().build()` → `anthropic::Client::new()?`

**Error:** `E0308` — method not found; `Arc<Result<Client>>` has no `.agent()`.

**Root cause:** `Client::new` became fallible in rig-core 0.36+. Wrapping the
result in `Arc::new()` before propagating with `?` produces
`Arc<Result<Client>>`, on which `.agent()` does not exist.

**Before:**
```rust
let client = Arc::new(anthropic::ClientBuilder::new(api_key).build());
let agent = client.agent("claude-sonnet-4-6").build();
```

**After:**
```rust
let client = anthropic::Client::new(api_key)?;  // unwrap first, then use
let agent = client.agent("claude-sonnet-4-6").preamble(AGENT_PREAMBLE).build();
```

**File:** `src/agent/mod.rs`

---

## Fix 03 — `reqwest` feature `rustls-tls` renamed to `rustls` in 0.13

**Error:** `error: Package `reqwest` does not have feature `rustls-tls``.

**Root cause:** reqwest 0.13 reorganised TLS feature flags. The old name
`rustls-tls` no longer exists. rig-core 0.37 depends on reqwest ^0.13.

**Before:**
```toml
reqwest = { version = "^0.12", features = ["json", "rustls-tls"] }
```

**After:**
```toml
reqwest = { version = "^0.13", features = ["json", "rustls"] }
```

**File:** `Cargo.toml`

---

## Fix 04 — `dotenv` replaced with maintained fork `dotenvy`

**Error:** Unmaintained crate warning; potential supply-chain risk.

**Root cause:** The `dotenv` crate (0.15) is unmaintained. `dotenvy` is the
actively maintained fork with the same API.

**Before:**
```toml
dotenv = "0.15"
```
```rust
dotenv::dotenv().ok();
```

**After:**
```toml
dotenvy = "^0.15"
```
```rust
dotenvy::dotenv().ok();
```

**Files:** `Cargo.toml`, `src/main.rs`, all examples.

---

## Fix 05 — `///` outer doc comments converted to `//!` inner module docs

**Error:** `rustdoc::missing_doc_comment_on_item` / misattributed docs.

**Root cause:** `///` at the top of `src/*.rs` files are *item* doc comments
attached to the first `mod` declaration in `lib.rs`, not module-level docs.
Converted to `//!` inner doc comments so they describe the module itself.

**Before:**
```rust
/// Runtime configuration loaded from environment variables.
use anyhow::Result;
```

**After:**
```rust
//! Runtime configuration loaded from environment variables.
use anyhow::Result;
```

**Files:** `src/config.rs`, `src/main.rs`, `src/lib.rs`, all `src/**/*.rs`.

---

## Fix 06 — `ProviderClient` added alongside `CompletionClient` in agent imports

**Error:** `E0599: no method named 'agent' found for struct 'anthropic::Client'`.

**Root cause:** Calling `.agent()` on an `anthropic::Client` in rig-core ≥ 0.36
requires **both** `CompletionClient` **and** `ProviderClient` to be in scope.
Importing only `CompletionClient` causes this error. The crate is `rig-core`
(hyphen) but the Rust module path is `rig_core::` (underscore).

**Before:**
```rust
use rig::client::CompletionClient;
```

**After:**
```rust
use rig_core::{
    client::{CompletionClient, ProviderClient},
    completion::Prompt,
    providers::anthropic,
};
```

**Files:** `src/agent/mod.rs`, `src/main.rs`, `src/agent/tools.rs`,
`tests/providers/anthropic.rs`

---

## Fix 07 — Orphan files `src/signer_context.rs` and `src/solana_ops.rs` removed

**Error:** `warning[unused_doc_comments]`, `missing_docs` lint failures, dead code.

**Root cause:** Both files were leftover from the pre-module-refactor era.
`signer_context.rs` used `thread_local!` (see Fix 01) and referenced `uuid`
and `chrono` that were absent from `Cargo.toml`. `solana_ops.rs` was a
duplicate of `src/onchain/balance.rs`. Neither was referenced from `lib.rs`.

**Fix:** Deleted both orphan files. Correct implementations live in
`src/onchain/signer.rs` and `src/onchain/balance.rs`.

---

## Fix 08 — `rand ^0.8` → `rand ^0.9`; `random::<u64>()` API migration

**Error:** `error[E0670]: 'gen' is a keyword in Rust 2024`.

**Root cause:** `gen` is reserved in Rust 2024 (generator syntax). The `rand`
0.8 API exposed `rng.gen::<T>()` which clashes. Additionally `rand::thread_rng()`
was renamed to `rand::rng()` in rand 0.9. The `rand::random::<T>()` free
function still exists in 0.9 but the import must be removed to avoid confusion.

**Before:**
```toml
rand = "^0.8"
```
```rust
use rand::random;
let id = format!("{:016x}", random::<u64>());
```

**After:**
```toml
rand = "^0.9"
```
```rust
let id = format!("{:016x}", rand::rng().random::<u64>());
```

**Files:** `Cargo.toml`, `src/onchain/mod.rs`, `src/onchain/signer.rs`

---

## Fix 09 — `rig::` import path corrected to `rig_core::` throughout

**Error:** `error[E0432]: unresolved import 'rig'` (crate name is `rig-core`,
Rust module path uses underscores: `rig_core`).

**Root cause:** The Cargo dependency is named `rig-core` (hyphen), but Rust
resolves hyphens to underscores in `use` paths. Every `use rig::` must be
`use rig_core::`.

**Before:**
```rust
use rig::completion::Prompt;
use rig::{ client::{CompletionClient, ProviderClient}, ... };
```

**After:**
```rust
use rig_core::completion::Prompt;
use rig_core::{ client::{CompletionClient, ProviderClient}, ... };
```

**Files:** `src/main.rs`, `src/agent/mod.rs`, `src/agent/tools.rs`,
`tests/providers/anthropic.rs`

---

## Fix 10 — License changed from `License-PBS` to `MIT OR Apache-2.0` (valid SPDX)

**Error:** `cargo publish` rejects `license = "License-PBS"` — not a valid SPDX
identifier.

**Root cause:** `AND/OR` and non-SPDX strings are rejected by crates.io.
Requires both `LICENSE-MIT` and `LICENSE-APACHE` files present in the repo root.

**Before:**
```toml
license = "License-PBS"
```

**After:**
```toml
license = "MIT OR Apache-2.0"
```

**Files created:** `LICENSE-MIT`, `LICENSE-APACHE`
**File updated:** `Cargo.toml`

---

## Fix 11 — `rig-core` version bumped from `^0.36` to `^0.37`; `rustls` feature added

**Error:** API compatibility — `^0.36` misses fixes and stabilised APIs in 0.37.
`rig-core` does **not** have `features = ["anthropic"]` or `features = ["openai"]`
— all providers are compiled in unconditionally.

**Before:**
```toml
rig-core = "^0.36"
```

**After:**
```toml
rig-core = { version = "^0.37", features = ["rustls"] }
```

**File:** `Cargo.toml`

---

## Fix 12 — Release profile `lto = true` → `lto = "thin"`

**Error:** No compile error — build correctness. `lto = true` enables fat LTO
(full cross-crate bitcode merge), which significantly increases link time for
no practical runtime benefit over `"thin"` in a binary of this size.

**Before:**
```toml
[profile.release]
lto = true
```

**After:**
```toml
[profile.release]
lto = "thin"
```

**File:** `Cargo.toml`

---

*Generated by Polar Bear Systems Automated Rust Development Process V1 — May 2026*
