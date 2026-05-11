use soroban_sdk::testutils::budget::Budget;
use soroban_sdk::{vec as soroban_vec, Address, Env, Symbol, Val};
use std::io::Write;

// A simple Solidity counter contract used for comparative testing
const COUNTER_SOL: &str = r#"
contract counter {
    uint64 public count = 0;

    function increment() public returns (uint64) {
        count += 1;
        return count;
    }

    function get() public view returns (uint64) {
        return count;
    }
}
"#;

/// Compiles a Solidity source string into a WASM binary using the Solang compiler.
/// This function creates a temporary file for the source, invokes the `solang` CLI,
/// and reads the resulting compiled WebAssembly.
fn compile_solidity(src: &str) -> (Vec<u8>, String) {
    // Create a temporary file to hold the Solidity source code
    let mut sol_file =
        tempfile::NamedTempFile::with_suffix(".sol").expect("could not create temp file");
    sol_file
        .write_all(src.as_bytes())
        .expect("could not write source");
    let sol_path = sol_file.path().to_path_buf();

    // Set up a temporary directory for the compiler output
    let out_dir = tempfile::tempdir().expect("could not create output dir");
    let out_path = out_dir.path().to_path_buf();

    // Locate the solang binary, defaulting to "solang" if not specified in env
    let solang_bin = std::env::var("SOLANG_BIN").unwrap_or_else(|_| "solang".to_string());

    // Execute the solang compiler, targeting the soroban environment
    let output = std::process::Command::new(&solang_bin)
        .args(["compile", "--target", "soroban",
               sol_path.to_str().unwrap(),
               "-o", out_path.to_str().unwrap()])
        .output()
        .unwrap_or_else(|e| {
            eprintln!("ERROR: Could not execute '{solang_bin}': {e}");
            eprintln!("Install: curl -L https://github.com/hyperledger-solang/solang/releases/download/v0.3.4/solang-mac-arm -o /usr/local/bin/solang && chmod +x /usr/local/bin/solang");
            std::process::exit(1);
        });

    if !output.status.success() {
        eprintln!(
            "ERROR: solang compile failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::process::exit(1);
    }
    if !output.stderr.is_empty() {
        eprintln!(
            "[solang warnings]\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Parse the output directory to find the generated .wasm file
    let wasm_entry = std::fs::read_dir(&out_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().map_or(false, |x| x == "wasm"))
        .unwrap_or_else(|| {
            eprintln!("ERROR: solang produced no .wasm output");
            std::process::exit(1);
        });

    let filename = wasm_entry.file_name().to_string_lossy().into_owned();
    let bytes = std::fs::read(wasm_entry.path()).expect("could not read WASM");
    (bytes, filename)
}

/// Helper function to load a pre-compiled Rust WASM binary from the file system.
fn load_wasm(path: &str) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("ERROR: Could not read WASM at '{path}': {e}");
        eprintln!("Build: cargo build -p counter-rs --target wasm32-unknown-unknown --release");
        std::process::exit(1);
    })
}

struct Metrics {
    cpu_insns: u64,
    mem_bytes: u64,
    return_val: Val,
}

/// Measures the execution cost (CPU instructions and memory) of a smart contract function.
/// It uses the Soroban SDK's mock environment to estimate the budget required for the invocation.
fn measure(env: &Env, addr: &Address, func_name: &str, extra_args: &[Val]) -> Metrics {
    let func = Symbol::new(env, func_name);
    let mut soroban_args = soroban_vec![env];
    for arg in extra_args {
        soroban_args.push_back(arg.clone());
    }

    // Reset the mock VM's budget counters to 0 to measure only this specific invocation
    let mut budget: Budget = env.cost_estimate().budget();
    budget.reset_default();

    // Invoke the contract function and capture its return value
    let return_val: Val = env.invoke_contract(addr, &func, soroban_args);

    Metrics {
        cpu_insns: budget.cpu_instruction_cost(),
        mem_bytes: budget.memory_bytes_cost(),
        return_val,
    }
}

/// Main entry point for the solang-diff prototype.
/// Orchestrates the compilation, loading, execution, and cost comparison between
/// the Solidity and Rust implementations of a contract.
fn main() {
    // Take the path to the rust SDK wasm from arguments or default to the target directory
    let rust_wasm_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/wasm32-unknown-unknown/release/counter_rs.wasm".to_string());

    println!("┌──────────────────────────────────────────────────────-───┐");
    println!("│              solang-diff  ·  Counter prototype           │");
    println!("│   Soroban mock-VM cost comparison: Solidity vs Rust SDK  │");
    println!("└───────────────────────────────────────────────────────-──┘");

    let env = Env::default();

    println!("\n[1/4] Compiling Solidity counter (via solang CLI)...");
    let (sol_wasm, sol_filename) = compile_solidity(COUNTER_SOL);
    println!("      Compiled from: {sol_filename}");
    println!(
        "      ✓ WASM size: {} bytes",
        format_num(sol_wasm.len() as u64)
    );
    #[allow(deprecated)]
    let sol_addr = env.register_contract_wasm(None, sol_wasm.as_slice());
    println!("      ✓ Registered at: {:?}", sol_addr);

    println!("\n[2/4] Loading Rust SDK WASM from:\n      {rust_wasm_path}");
    let rust_wasm = load_wasm(&rust_wasm_path);
    println!(
        "      ✓ WASM size: {} bytes",
        format_num(rust_wasm.len() as u64)
    );
    #[allow(deprecated)]
    let rust_addr = env.register_contract_wasm(None, rust_wasm.as_slice());
    println!("      ✓ Registered at: {:?}", rust_addr);

    println!("\n[3/4] Measuring increment()...");
    let sol_inc = measure(&env, &sol_addr, "increment", &[]);
    let rust_inc = measure(&env, &rust_addr, "increment", &[]);

    println!("[4/4] Measuring get()...");
    let sol_get = measure(&env, &sol_addr, "get", &[]);
    let rust_get = measure(&env, &rust_addr, "get", &[]);

    println!();
    print_report("increment()", &sol_inc, &rust_inc);
    println!();
    print_report("get()", &sol_get, &rust_get);
    println!("\nNote: numbers reflect the soroban-sdk mock VM, not on-chain costs.");
}

/// Prints a formatted comparative report of the CPU and memory costs.
fn print_report(func: &str, sol: &Metrics, rust: &Metrics) {
    let cpu_ratio = sol.cpu_insns as f64 / rust.cpu_insns.max(1) as f64;
    let mem_ratio = sol.mem_bytes as f64 / rust.mem_bytes.max(1) as f64;
    let match_str = if sol.return_val.shallow_eq(&rust.return_val) {
        "✓ yes"
    } else {
        "✗ MISMATCH"
    };

    println!("┌─────────────────────────┬────────────────────┬────────────────────┐");
    println!(
        "│  {:<23} │  Solang (Solidity)  │  Rust SDK          │",
        func
    );
    println!("├─────────────────────────┼────────────────────┼────────────────────┤");
    println!(
        "│  CPU Instructions       │  {:<18} │  {:<18} │",
        format_num(sol.cpu_insns),
        format_num(rust.cpu_insns)
    );
    println!(
        "│  Memory (bytes)         │  {:<18} │  {:<18} │",
        format_num(sol.mem_bytes),
        format_num(rust.mem_bytes)
    );
    println!("│  Return value match     │  {:<38} │", match_str);
    println!("├─────────────────────────┴────────────────────┴────────────────────┤");
    println!(
        "│  Overhead:  {:.2}× CPU   {:.2}× memory                           │",
        cpu_ratio, mem_ratio
    );
    if cpu_ratio > 1.5 {
        println!("│  ⚠  Significant CPU overhead detected (>1.5×)                    │");
    }
    println!("└──────────────────────────────────────────────────────────────────────┘");
}

fn format_num(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}
