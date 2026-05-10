# polar-bear-rig-onchain

## Screen Capture Guide

Running `cargo run --release -- --mode full` produces structured logs showing:

1. **SignerContext**: task-local ephemeral keypair installed; pubkey, context_id,
   and label logged at INFO level.
2. **Solana balance**: devnet RPC `get_balance` call; lamports and SOL amount logged.
3. **Jupiter quote** (DRY-RUN): GET `/v6/quote` → expected USDC output, price
   impact %, and route breakdown (Raydium / Orca / Whirlpool legs).
4. **Agent pipeline result**: `rig-core` agent (claude-sonnet-4-6) structured
   summary of all three steps.
5. **SignerContext eviction**: `boundary sealed ✓` logged when `with_signer`
   scope exits.
6. **IsolationReport JSON**: `boundary_sealed = true` in final audit output.

### No-API-key demo

```bash
# Runs Jupiter quote + isolation log without an Anthropic key:
cargo run --example jupiter_dry_run
```

### Full pipeline demo

```bash
cp .env.example .env
# Edit .env: set ANTHROPIC_API_KEY=sk-ant-...
cargo run --release -- --mode full
```

### Signer isolation demo (no API key)

```bash
cargo run --release -- --mode signer
# or
cargo run --example signer_demo
```
