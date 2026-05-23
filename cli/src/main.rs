//! # MDB CLI — Interactive Multidimensional Binary Shell
//!
//! ```text
//! $ mdb                     # Start interactive REPL
//! $ mdb bench               # Run benchmark suite
//! $ mdb circuit bell        # Run a preset circuit
//! $ mdb shor 15             # Factor a number
//! $ mdb grover 4 7          # Search 4-qubit space for target 7
//! ```

use mdb_core::algorithms;
use mdb_core::benchmarks;
use mdb_core::circuit;
use mdb_core::coordinates::DimensionalCascade;
use mdb_core::evolution;
use mdb_core::persistence::Workspace;
use mdb_core::register::QuantumRegister;
use mdb_core::sparse_register::SparseQuantumRegister;
use mdb_core::superbit::SuperBit;
use std::io::{self, BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        repl();
        return;
    }

    match args[1].as_str() {
        "bench" | "benchmark" => {
            if args.get(2).map(|s| s.as_str()) == Some("sparse") {
                cmd_bench_sparse();
            } else {
                cmd_bench();
            }
        }
        "sparse" => cmd_sparse(&args[2..]),
        "circuit" => cmd_circuit(&args[2..]),
        "shor" => cmd_shor(&args[2..]),
        "grover" => cmd_grover(&args[2..]),
        "superbit" | "cascade" => cmd_superbit(&args[2..]),
        "help" | "--help" | "-h" => cmd_help(),
        "version" | "--version" | "-v" => {
            println!("MDB-OS v{}", mdb_core::VERSION);
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            cmd_help();
            std::process::exit(1);
        }
    }
}

// ═════════════════════════════════════════════════════════════════════
// REPL
// ═════════════════════════════════════════════════════════════════════

fn repl() {
    println!("+-------------------------------------------------------+");
    println!("|  MDB-OS v{}  --  Multidimensional Binary Shell     |", mdb_core::VERSION);
    println!("|  Type 'help' for commands, 'quit' to exit.          |");
    println!("+-------------------------------------------------------+");
    println!();

    let stdin = io::stdin();
    let mut workspace = Workspace::new();
    let mut registers: Vec<(String, QuantumRegister)> = Vec::new();

    loop {
        print!("mdb> ");
        io::stdout().flush().ok();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() || line.is_empty() {
            break;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        let cmd = parts[0].to_lowercase();

        match cmd.as_str() {
            "quit" | "exit" | "q" => {
                println!("Goodbye.");
                break;
            }
            "help" | "h" | "?" => repl_help(),

            // -- SuperBit commands --
            "superbit" | "sb" => repl_superbit(&parts[1..], &mut workspace),

            // -- Register commands --
            "register" | "reg" | "r" => repl_register(&parts[1..], &mut registers),

            // -- Circuit commands --
            "circuit" | "circ" | "c" => repl_circuit(&parts[1..]),

            // -- Algorithm commands --
            "shor" => {
                if parts.len() < 2 {
                    println!("Usage: shor <number>");
                } else if let Ok(n) = parts[1].parse::<u64>() {
                    match algorithms::shors_factor(n) {
                        Some(r) => println!("  {} = {} x {}", n, r.factors.0, r.factors.1),
                        None => println!("  Cannot factor {}", n),
                    }
                } else {
                    println!("Invalid number: {}", parts[1]);
                }
            }
            "grover" => {
                if parts.len() < 3 {
                    println!("Usage: grover <qubits> <target>");
                } else if let (Ok(n), Ok(t)) = (parts[1].parse::<usize>(), parts[2].parse::<usize>()) {
                    let target_bits: Vec<u8> = (0..n).map(|k| ((t >> (n - 1 - k)) & 1) as u8).collect();
                    let result = algorithms::grovers_search(
                        n,
                        &|bits: &[u8]| bits == target_bits.as_slice(),
                        None,
                    );
                    println!("  Search space: {} items", 1u64 << n);
                    println!("  Target: {}", t);
                    println!("  Found:  {} {}", result.index, if result.index == t { "PASS" } else { "FAIL" });
                    println!("  Iterations: {}", result.iterations);
                } else {
                    println!("Invalid arguments");
                }
            }
            "deutsch-jozsa" | "dj" => {
                if parts.len() < 2 {
                    println!("Usage: dj <qubits>  (tests balanced parity function)");
                } else if let Ok(n) = parts[1].parse::<usize>() {
                    let result = algorithms::deutsch_jozsa(n, &|x: usize| {
                        let mut p = 0u8;
                        let mut v = x;
                        while v > 0 { p ^= (v & 1) as u8; v >>= 1; }
                        p
                    });
                    println!("  {:?} -- determined in ONE query (classically needs up to {} queries)",
                             result, (1u64 << n) / 2 + 1);
                } else {
                    println!("Invalid number: {}", parts[1]);
                }
            }

            // -- Cascade --
            "cascade" => {
                if parts.len() < 2 {
                    println!("Usage: cascade <binary_string> [max_dim]");
                } else {
                    let bits: Vec<u8> = parts[1].chars().filter_map(|c| {
                        if c == '0' { Some(0) } else if c == '1' { Some(1) } else { None }
                    }).collect();
                    let max_dim = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(7);
                    if bits.is_empty() {
                        println!("Provide a binary string, e.g.: cascade 10110");
                    } else {
                        let c = DimensionalCascade::from_bits(&bits, max_dim);
                        let dim_names = ["D1 Value", "D2 Space", "D3 Time", "D4 Spacetime", "D5 Momentum", "D6 Energy"];
                        println!("  Bits: {:?}", bits);
                        println!("  n = {}", c.n);
                        for (i, dim_vec) in c.dims.iter().enumerate() {
                            let name = dim_names.get(i).unwrap_or(&"D?");
                            println!("  {:<14} {:?}", name, dim_vec);
                        }
                    }
                }
            }

            // -- Benchmarks --
            "bench" | "benchmark" => {
                println!("{}", benchmarks::report());
            }

            // -- Persistence --
            "save" => {
                if parts.len() < 2 {
                    println!("Usage: save <filepath>");
                } else {
                    match workspace.save(parts[1]) {
                        Ok(()) => println!("  Saved workspace to {}", parts[1]),
                        Err(e) => println!("  Error: {}", e),
                    }
                }
            }
            "load" => {
                if parts.len() < 2 {
                    println!("Usage: load <filepath>");
                } else {
                    match Workspace::load(parts[1]) {
                        Ok(ws) => {
                            println!("  Loaded: {} SuperBits, {} registers", ws.superbits.len(), ws.registers.len());
                            workspace = ws;
                        }
                        Err(e) => println!("  Error: {}", e),
                    }
                }
            }

            // -- Teleportation --
            "teleport" => {
                let (ar, ai) = (1.0 / 2.0_f64.sqrt(), 0.0);
                let (br, bi) = (1.0 / 2.0_f64.sqrt(), 0.0);
                let result = algorithms::teleport(ar, ai, br, bi, 42);
                println!("  Quantum Teleportation:");
                println!("  Input:  α=({:.4}+{:.4}i), β=({:.4}+{:.4}i)", ar, ai, br, bi);
                println!("  Output: α=({:.4}+{:.4}i), β=({:.4}+{:.4}i)",
                    result.received_alpha.0, result.received_alpha.1,
                    result.received_beta.0, result.received_beta.1);
                println!("  Fidelity: {:.6}", result.fidelity);
                println!("  Alice measured: ({}, {})", result.alice_bits.0, result.alice_bits.1);
            }

            // -- Sparse register --
            "sparse" => {
                if parts.len() < 2 {
                    println!("Usage: sparse ghz <n> | sparse new <n>");
                } else {
                    match parts[1] {
                        "ghz" => {
                            let n: usize = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(20);
                            if n > 50 {
                                println!("  Max 50 qubits in sparse mode");
                            } else {
                                println!("  Creating {}-qubit GHZ state (sparse)...", n);
                                let start = std::time::Instant::now();
                                let mut r = SparseQuantumRegister::new(n, &format!("ghz_{}", n));
                                r.hadamard(0);
                                for i in 1..n { r.cnot(i-1, i); }
                                let elapsed = start.elapsed();
                                println!("  Population: {} states", r.population());
                                println!("  Memory: {} bytes", r.memory_bytes());
                                println!("  Time: {:.1}ms", elapsed.as_secs_f64() * 1000.0);
                                if n < 64 {
                                    println!("  Dense equivalent: {} bytes", 16u128 * (1u128 << n));
                                } else {
                                    println!("  Dense equivalent: 2^{} × 16 bytes (impossible)", n);
                                }
                            }
                        }
                        _ => println!("Unknown sparse command: {}", parts[1]),
                    }
                }
            }

            // -- Evolution --
            "evolve" => {
                if workspace.superbits.is_empty() {
                    println!("  No SuperBits. Use 'sb new <bits>' first.");
                } else {
                    let steps = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(3);
                    println!("  Evolving last SuperBit ({} steps)...", steps);
                    let snap = workspace.superbits.last().unwrap();
                    let mut sb = SuperBit::from_bits(snap.sigma.clone());
                    println!("  Original: {:?}", snap.sigma);
                    let results = evolution::evolve_dimensional_n(&mut sb, steps);
                    for (i, res) in results.iter().enumerate() {
                        let pos = res.modified_position.map(|p| format!("pos {}", p)).unwrap_or("(blocked)".into());
                        println!("  Step {}: gen={} modified={} blocked={}",
                            i + 1, res.generation, pos, res.blocked_by_anchor);
                    }
                    let addr = sb.address();
                    let bits: String = sb.sigma().iter().map(|b| if *b == 1 { '1' } else { '0' }).collect();
                    println!("  Final:   {}", bits);
                    println!("  Address: d4={:.4}  d5={}", addr.d4_spacetime, addr.d5_momentum);
                }
            }

            // -- Clear --
            "clear" | "cls" => {
                print!("\x1B[2J\x1B[1;1H");
                io::stdout().flush().ok();
            }

            // -- Info --
            "version" | "v" => {
                println!("  MDB-OS v{}", mdb_core::VERSION);
            }
            "status" => {
                println!("  MDB-OS v{}", mdb_core::VERSION);
                println!("  Workspace: {} SuperBits, {} registers", workspace.superbits.len(), registers.len());
                println!("  Available: core engine, sparse register, WASM, CLI");
            }

            _ => {
                println!("Unknown command: '{}'. Type 'help' for available commands.", cmd);
            }
        }
    }
}

fn repl_help() {
    println!("
  ╔═══════════════════════════════════════════════════════╗
  ║  MDB-OS v{:<43} ║
  ╠═══════════════════════════════════════════════════════╣
  ║                                                       ║
  ║  SUPERBIT                                             ║
  ║    sb new <bits>          Create a SuperBit            ║
  ║    sb peek                List SuperBits in workspace  ║
  ║    cascade <bits> [dim]   Show dimensional cascade     ║
  ║    evolve                 Evolve last SuperBit         ║
  ║                                                       ║
  ║  QUANTUM                                              ║
  ║    reg new <n>            Create n-qubit register      ║
  ║    reg h/x/cnot <args>    Apply gates                  ║
  ║    reg peek               View state                   ║
  ║    reg measure            Measure register             ║
  ║    circuit <name> [n]     Run preset (bell/ghz/qft)    ║
  ║    sparse ghz [n]         Sparse register GHZ state    ║
  ║                                                       ║
  ║  ALGORITHMS                                           ║
  ║    shor <N>               Factor a number              ║
  ║    grover <qubits> <tgt>  Search for target            ║
  ║    dj <qubits>            Deutsch-Jozsa                ║
  ║    teleport               Quantum teleportation        ║
  ║                                                       ║
  ║  SYSTEM                                               ║
  ║    bench                  Run benchmark suite          ║
  ║    save <file>            Save workspace               ║
  ║    load <file>            Load workspace               ║
  ║    status                 Show system status            ║
  ║    version                Show version                  ║
  ║    clear                  Clear screen                  ║
  ║    quit                   Exit                          ║
  ╚═══════════════════════════════════════════════════════╝
", mdb_core::VERSION);
}

fn repl_superbit(args: &[&str], workspace: &mut Workspace) {
    if args.is_empty() {
        println!("Usage: sb new <bits> | sb peek");
        return;
    }
    match args[0] {
        "new" | "create" => {
            if args.len() < 2 {
                println!("Usage: sb new 10110");
                return;
            }
            let bits: Vec<u8> = args[1].chars().filter_map(|c| {
                if c == '0' { Some(0) } else if c == '1' { Some(1) } else { None }
            }).collect();
            if bits.is_empty() {
                println!("Provide bits, e.g.: sb new 10110");
                return;
            }
            let sb = SuperBit::from_bits(bits.clone());
            workspace.add_superbit(&sb, &format!("sb_{}", workspace.superbits.len()));
            let addr = sb.address();
            println!("  Created SuperBit: {:?}", bits);
            println!("  D4 Spacetime: {:.4}", addr.d4_spacetime);
            println!("  D5 Momentum:  {}", addr.d5_momentum);
            println!("  Stored as #{}", workspace.superbits.len() - 1);
        }
        "peek" | "list" => {
            if workspace.superbits.is_empty() {
                println!("  No SuperBits in workspace. Use 'sb new <bits>' to create one.");
                return;
            }
            for (i, snap) in workspace.superbits.iter().enumerate() {
                println!("  [{}] {} -- {} states, sigma: {:?}",
                    i, snap.label, snap.state_patterns.len(), snap.sigma);
            }
        }
        _ => println!("Unknown sb command: {}", args[0]),
    }
}

fn repl_register(args: &[&str], registers: &mut Vec<(String, QuantumRegister)>) {
    if args.is_empty() {
        println!("Usage: reg new <n> | reg h <pos> | reg cnot <c> <t> | reg peek");
        return;
    }
    match args[0] {
        "new" | "create" => {
            if args.len() < 2 {
                println!("Usage: reg new <n_qubits>");
                return;
            }
            if let Ok(n) = args[1].parse::<usize>() {
                if n > 24 {
                    println!("Max 24 qubits (16M amplitudes)");
                    return;
                }
                let name = format!("reg_{}", registers.len());
                registers.push((name.clone(), QuantumRegister::new(n, &name)));
                println!("  Created {}-qubit register #{}", n, registers.len() - 1);
            }
        }
        "h" | "hadamard" => {
            if let Some((_, reg)) = registers.last_mut() {
                if let Some(Ok(pos)) = args.get(1).map(|s| s.parse::<usize>()) {
                    reg.hadamard(pos);
                    println!("  Applied H to position {}", pos);
                } else {
                    println!("Usage: reg h <position>");
                }
            } else {
                println!("  No register. Use 'reg new <n>' first.");
            }
        }
        "x" | "pauli_x" => {
            if let Some((_, reg)) = registers.last_mut() {
                if let Some(Ok(pos)) = args.get(1).map(|s| s.parse::<usize>()) {
                    reg.pauli_x(pos);
                    println!("  Applied X to position {}", pos);
                }
            }
        }
        "cnot" => {
            if let Some((_, reg)) = registers.last_mut() {
                if args.len() >= 3 {
                    if let (Ok(c), Ok(t)) = (args[1].parse::<usize>(), args[2].parse::<usize>()) {
                        reg.cnot(c, t);
                        println!("  Applied CNOT({} -> {})", c, t);
                    }
                } else {
                    println!("Usage: reg cnot <control> <target>");
                }
            }
        }
        "peek" | "view" => {
            if let Some((name, reg)) = registers.last() {
                let view = reg.peek();
                println!("  Register '{}': {} qubits, {} non-zero states", name, reg.n, view.nonzero_states);
                for sv in &view.states {
                    if sv.probability > 0.001 {
                        let bits_str: String = sv.bits.iter().map(|b| if *b == 0 { '0' } else { '1' }).collect();
                        println!("    |{}>  p={:.4}  phase={:.4}", bits_str, sv.probability, sv.phase);
                    }
                }
            } else {
                println!("  No register. Use 'reg new <n>' first.");
            }
        }
        "measure" => {
            if let Some((name, reg)) = registers.last_mut() {
                let result = reg.measure_all(42);
                let bits_str: String = result.bits.iter().map(|b| if *b == 0 { '0' } else { '1' }).collect();
                println!("  Measured '{}': |{}>  (p={:.4})", name, bits_str, result.probability);
            }
        }
        _ => println!("Unknown reg command: {}", args[0]),
    }
}

fn repl_circuit(args: &[&str]) {
    let name = args.first().copied().unwrap_or("bell");
    match name {
        "bell" => {
            let circ = circuit::bell_pair(2);
            println!("{}", circ.to_ascii());
            let result = circ.measure_all().execute();
            println!("  Measurement: {:?}", result.measurements);
        }
        "ghz" => {
            let n = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(3);
            let circ = circuit::ghz_state(n);
            println!("{}", circ.to_ascii());
            let result = circ.measure_all().execute();
            println!("  Measurement: {:?}", result.measurements);
        }
        "qft" => {
            let n = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(4);
            let circ = circuit::qft_circuit(n);
            println!("{}", circ.to_ascii());
        }
        _ => {
            println!("Available circuits: bell, ghz [n], qft [n]");
        }
    }
}

// ═════════════════════════════════════════════════════════════════════
// One-shot commands
// ═════════════════════════════════════════════════════════════════════

fn cmd_bench() {
    println!("{}", benchmarks::report());
}

fn cmd_bench_sparse() {
    println!("{}", benchmarks::sparse_report());
}

fn cmd_sparse(args: &[String]) {
    let subcmd = args.first().map(|s| s.as_str()).unwrap_or("help");
    match subcmd {
        "ghz" => {
            let n: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(30);
            println!("Creating {}-qubit GHZ state (sparse mode)...", n);
            let start = std::time::Instant::now();
            let mut r = SparseQuantumRegister::new(n, &format!("ghz_{}", n));
            r.hadamard(0);
            for i in 1..n {
                r.cnot(i - 1, i);
            }
            let elapsed = start.elapsed();
            println!("{}", r);
            println!("Population: {} states", r.population());
            println!("Memory: {} bytes", r.memory_bytes());
            println!("Time: {:.1}ms", elapsed.as_secs_f64() * 1000.0);
            if n < 64 {
                println!(
                    "Dense would need: {} bytes (2^{} × 16)",
                    16u128 * (1u128 << n),
                    n
                );
            } else {
                println!("Dense would need: 2^{} × 16 bytes (impossible)", n);
            }
            println!("\nCascade snapshot:");
            for entry in r.cascade_snapshot() {
                println!(
                    "  |{:0width$b}⟩  p={:.6}  {}",
                    entry.index,
                    entry.probability,
                    entry.address,
                    width = n.min(64)
                );
            }
        }
        "bench" => {
            println!("{}", benchmarks::sparse_report());
        }
        _ => {
            println!("MDB Sparse Register Commands:");
            println!("  mdb sparse ghz [n]    Create n-qubit GHZ state (default: 30)");
            println!("  mdb sparse bench      Run sparse benchmarks");
            println!();
            println!("The sparse register stores only populated basis states,");
            println!("addressed by the dimensional cascade. Memory scales with");
            println!("entanglement complexity, not 2^n.");
        }
    }
}

fn cmd_circuit(args: &[String]) {
    let name = args.first().map(|s| s.as_str()).unwrap_or("bell");
    repl_circuit(&[name]);
}

fn cmd_shor(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: mdb shor <number>");
        std::process::exit(1);
    }
    let n: u64 = args[0].parse().expect("Invalid number");
    match algorithms::shors_factor(n) {
        Some(r) => println!("{} = {} x {}", n, r.factors.0, r.factors.1),
        None => println!("Cannot factor {}", n),
    }
}

fn cmd_grover(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: mdb grover <qubits> <target>");
        std::process::exit(1);
    }
    let n: usize = args[0].parse().expect("Invalid qubit count");
    let target: usize = args[1].parse().expect("Invalid target");
    let target_bits: Vec<u8> = (0..n).map(|k| ((target >> (n - 1 - k)) & 1) as u8).collect();
    let result = algorithms::grovers_search(
        n,
        &|bits: &[u8]| bits == target_bits.as_slice(),
        None,
    );
    println!("Search space: {} items", 1u64 << n);
    println!("Target: {}", target);
    println!("Found:  {} {}", result.index, if result.index == target { "PASS" } else { "FAIL" });
    println!("Iterations: {}", result.iterations);
}

fn cmd_superbit(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: mdb superbit <binary_string>");
        std::process::exit(1);
    }
    let bits: Vec<u8> = args[0].chars().filter_map(|c| {
        if c == '0' { Some(0) } else if c == '1' { Some(1) } else { None }
    }).collect();
    let sb = SuperBit::from_bits(bits.clone());
    let addr = sb.address();
    let c = DimensionalCascade::from_bits(&bits, 7);
    let dim_names = ["D1 Value", "D2 Space", "D3 Time", "D4 Spacetime", "D5 Momentum", "D6 Energy", "D7"];
    println!("SuperBit: {:?}", bits);
    println!("Address:  d4={:.4}  d5={}", addr.d4_spacetime, addr.d5_momentum);
    for (i, dim_vec) in c.dims.iter().enumerate() {
        let name = dim_names.get(i).unwrap_or(&"D?");
        println!("  {:<14} {:?}", name, dim_vec);
    }
}

fn cmd_help() {
    println!("MDB-OS v{} — Multidimensional Binary Operating System", mdb_core::VERSION);
    println!("Invented by Ryan Guitard.");
    println!();
    println!("USAGE:");
    println!("  mdb                         Interactive REPL");
    println!("  mdb bench                   Run benchmark suite");
    println!("  mdb bench sparse            Run sparse vs dense benchmarks");
    println!("  mdb sparse ghz [n]          Create n-qubit GHZ state (sparse mode)");
    println!("  mdb sparse bench            Run sparse register benchmarks");
    println!("  mdb shor <N>                Factor a number with Shor's algorithm");
    println!("  mdb grover <qubits> <tgt>   Grover's search");
    println!("  mdb circuit <name>          Run a preset circuit (bell, ghz, qft)");
    println!("  mdb superbit <bits>         Compute dimensional cascade for binary string");
    println!("  mdb help                    This message");
    println!("  mdb version                 Show version");
    println!();
    println!("BROWSER:");
    println!("  python3 -m http.server 8080 && open http://localhost:8080/web/desktop/");
    println!("  Full desktop OS in the browser — Terminal, Experiment Lab, Circuit Designer.");
    println!();
    println!("EXAMPLES:");
    println!("  mdb shor 15                 => 15 = 3 x 5");
    println!("  mdb grover 4 7              => Search 16 items, find 7");
    println!("  mdb superbit 10110          => Dimensional cascade for 10110");
    println!("  mdb sparse ghz 30           => 30-qubit GHZ state (sparse register)");
    println!("  mdb bench                   => Full benchmark report");
}
