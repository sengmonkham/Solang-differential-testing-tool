# solang-diff

A prototype differential testing tool for [Hyperledger Solang](https://github.com/hyperledger-solang/solang) — compares the runtime cost of Solidity contracts (compiled to Soroban WASM via Solang) against equivalent Rust Soroban SDK contracts on the mock VM.

## What it does

```
[1/4] Compiling Solidity counter (via solang CLI)...
      ✓ WASM size: 1,392 bytes   ← Solang output
[2/4] Loading Rust SDK counter WASM...
      ✓ WASM size: 2,870 bytes   ← Rust SDK output

┌─────────────────────────┬────────────────────┬────────────────────┐
│  increment()            │  Solang (Solidity)  │  Rust SDK          │
├─────────────────────────┼────────────────────┼────────────────────┤
│  CPU Instructions       │  450,295            │  552,493            │
│  Memory (bytes)         │  1,166,373          │  1,176,867          │
│  Return value match     │  ✓ yes                                  │
├─────────────────────────┴────────────────────┴────────────────────┤
│  Overhead:  0.82× CPU   0.99× memory                           │
└──────────────────────────────────────────────────────────────────────┘
```

Both contracts are deployed to the same `soroban-sdk` mock VM. The `testutils` budget API measures CPU instructions and memory bytes consumed per invocation.

> **Key finding:** In the mock VM environment, the Solang-compiled Solidity counter uses *fewer* CPU instructions than the Rust SDK equivalent (~0.82×). This is an interesting result for the mentorship evaluation — it suggests Solang's optimizer produces efficient WASM for simple storage patterns, though the delta may change under more complex contracts or on the real network.

## Prerequisites

```sh
# 1. solang binary (macOS ARM)
curl -L https://github.com/hyperledger-solang/solang/releases/download/v0.3.4/solang-mac-arm \
  -o /usr/local/bin/solang && chmod +x /usr/local/bin/solang

# 2. wasm32 target (for building the Rust reference contract)
rustup target add wasm32-unknown-unknown
```

## Build & Run

```sh
# One-shot: builds Rust counter WASM, then runs the diff tool
./build-and-run.sh
```

Or step by step:

```sh
# 1. Build the Rust reference contract (workspace build)
cargo build -p counter-rs --target wasm32-unknown-unknown --release

# 2. Run solang-diff
cargo run --bin solang-diff --features soroban
```

## Project structure

```
solang-diff/
├── src/main.rs                            ← solang-diff binary (CLI approach)
├── solang-diff-contracts/
│   ├── counter.sol                        ← Solidity contract (also inlined in main.rs)
│   └── counter_rs/                        ← Rust reference contract
│       ├── Cargo.toml
│       └── src/lib.rs
├── build-and-run.sh                       ← one-shot helper
├── guide.md                               ← implementation guide
└── README.md
```

## How it works

1. **Compile Solidity**: Writes the inline Solidity source to a temp file, calls `solang compile --target soroban` via `std::process::Command`, reads the output WASM.
2. **Load Rust WASM**: Reads the pre-built `counter_rs.wasm` from the workspace target.
3. **Register both** on a single shared `Env::default()` (the soroban-sdk mock VM).
4. **Measure**: For each function (`increment`, `get`), calls `budget.reset_default()` then `env.invoke_contract()`, then reads `budget.cpu_instruction_cost()` and `budget.memory_bytes_cost()`.
5. **Report**: Prints a formatted table with ratio (`Overhead: Nx CPU`).

## Dependency notes

| Crate | Version | Purpose |
|-------|---------|---------|
| `soroban-sdk` | `22.0.7` | Mock VM + `testutils` budget API |
| `tempfile` | `3` | Temp file for passing Solidity to the CLI |
| `solang` (binary) | `0.3.4` | Compiles Solidity → Soroban WASM (via CLI) |

The `solang` Rust *library* is intentionally **not** used as a dependency — it requires a Solana-fork of LLVM 16 to link, which is not available via Homebrew. Using the pre-built `solang` binary sidesteps this entirely.

