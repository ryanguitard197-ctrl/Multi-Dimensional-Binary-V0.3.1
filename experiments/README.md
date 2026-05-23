# MDB Experiment Framework

**Invented by Ryan Guitard**

A rigorous experimentation framework for the MDB (Multidimensional Binary) computing paradigm. This is not a toy — it's built for real scientific discovery.

---

## What This Does

MDB is a **discovery engine**. It finds *new* solutions to problems that are currently unsolvable or computationally intractable. The Experiment Framework gives you everything needed to:

1. **Define** an experiment — what problem you're solving, what data you need, how to encode it
2. **Validate** all inputs — refuses to run with missing data, wrong types, or unverified sources
3. **Run** the experiment — connects to MDB's SuperBit engine (fitness search, QAOA, evolution)
4. **Reproduce** results — every run creates a manifest with hashes, provenance, and environment info

---

## Quick Start

### Option A: Use a Pre-Built Template

The framework ships with **15 experiment templates** (from Ryan's research papers + critical global problems). Pick one, feed it real data, and run.

```rust
use mdb_experiments::{ExperimentRunner, DataSource};
use mdb_experiments::catalog;
use mdb_experiments::schema::InputValue;
use std::collections::HashMap;

fn main() {
    // 1. Pick an experiment
    let experiment = catalog::protein_folding();

    // 2. See what data you need
    let required = ExperimentRunner::required_fields(&experiment);
    for (name, field_type, description) in &required {
        println!("  {} ({}) — {}", name, field_type, description);
    }

    // 3. Provide real data with provenance
    let mut inputs = HashMap::new();
    inputs.insert("amino_acid_sequence".to_string(),
        InputValue::new(serde_json::json!("MKFLILLFNILCLFPVLAADNHGV..."))
    );
    inputs.insert("temperature".to_string(),
        InputValue::new(serde_json::json!(310.15))
            .with_source(DataSource::doi("10.1016/j.jmb.2024.168123"))
    );
    // ... fill ALL required fields ...

    // 4. Run
    let runner = ExperimentRunner::new();
    let result = runner.run(&experiment, inputs, None);

    if result.success {
        println!("Best score: {}", result.discovery.unwrap().best_score);
        // Save the reproducibility manifest
        result.manifest.unwrap().save("protein_run_001.json").unwrap();
    } else {
        // Shows exactly what's missing
        println!("{}", result.message);
    }
}
```

### Option B: Build a Custom Experiment

Not limited to the templates. Define any experiment with any fields:

```rust
use mdb_experiments::custom::CustomExperiment;
use mdb_experiments::schema::{FieldType, InputField};
use mdb_experiments::problem::*;

fn main() {
    let experiment = CustomExperiment::new("My Research Problem")
        .description("Discovering optimal configurations for...")
        .problem_type(ProblemType::CombinatorialOptimization {
            maximize: true,
            objective: "Maximize throughput under constraints".to_string(),
        })
        .encoding(StateSpaceEncoding::Binary {
            bits: 64,
            bit_mapping: "Each bit = whether node i is included".to_string(),
        })
        .strategy(SolverStrategy::Hybrid {
            strategies: vec![
                SolverStrategy::FitnessSearch { num_candidates: 512 },
                SolverStrategy::Qaoa { depth: 3, max_iterations: 100 },
            ],
        })
        // Add whatever input fields you need
        .add_field(InputField::required_float("throughput_target", "Target throughput in ops/sec")
            .with_unit("ops/sec")
            .with_bounds(0.0, 1e9))
        .add_field(InputField::required_matrix("adjacency", "Network adjacency matrix"))
        .add_field(InputField::required_json("constraints", "Problem constraints as JSON"))
        .add_field(InputField::required_float("budget", "Total budget")
            .with_unit("USD"))
        // Optional fields with defaults
        .add_field(InputField::required_int("max_iterations", "Max solver iterations")
            .optional_with_default("1000"))
        .build()
        .expect("valid experiment definition");

    // Now run it like any other experiment
    let runner = ExperimentRunner::new();
    // ... provide inputs and run ...
}
```

### Option C: Let the AI Assistant Help You

Don't know what data you need? Describe your problem in plain English and the AI assistant figures it out:

```rust
use mdb_experiments::assistant::ExperimentAssistant;

fn main() {
    let assistant = ExperimentAssistant::new();

    // Describe what you want to solve
    let plan = assistant.plan_experiment(
        "I want to find the optimal placement of 50 cell towers across a city \
         to maximize coverage while minimizing cost. I have terrain data, \
         population density maps, and a budget of $10M."
    );

    // The assistant tells you exactly what you need:
    println!("Experiment: {}", plan.name);
    println!("Problem type: {:?}", plan.problem_type);
    println!("\nData you need to provide:");
    for req in &plan.required_data {
        println!("  • {} — {}", req.field_name, req.description);
        println!("    Type: {:?}", req.field_type);
        println!("    Where to get it: {}", req.suggested_source);
    }

    // Build the experiment from the plan
    let experiment = plan.to_experiment_definition();

    // Or let the assistant auto-fill from your data sources
    let inputs = assistant.auto_fill(&experiment, &[
        ("terrain_data", "/path/to/terrain.geotiff"),
        ("population_density", "/path/to/census_data.csv"),
        ("tower_specs", "/path/to/tower_catalog.json"),
    ]);
}
```

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                     User / AI Assistant                       │
│          "I want to solve X" → ExperimentPlan                │
├──────────────────────────────────────────────────────────────┤
│                  Experiment Definition                        │
│   Problem Type · Encoding · Strategy · Input Schema          │
├──────────────────────────────────────────────────────────────┤
│                    Input Validation                           │
│   Type check · Bounds · Provenance · Completeness            │
│         *** REFUSES TO RUN IF ANYTHING IS MISSING ***        │
├──────────────────────────────────────────────────────────────┤
│                    Discovery Engine                           │
│   fitness_search · QAOA · evolution · hybrid                 │
│                  ↕ mdb-core SuperBit ↕                       │
├──────────────────────────────────────────────────────────────┤
│                    Result Analysis                            │
│   Best solution · Ranked alternatives · Dimensional data     │
├──────────────────────────────────────────────────────────────┤
│                Reproducibility Manifest                       │
│   Input hashes · Provenance chain · Environment · Results    │
└──────────────────────────────────────────────────────────────┘
```

---

## Modules

| Module | Lines | Description |
|--------|-------|-------------|
| `schema.rs` | 611 | Input field types, validation, provenance requirements |
| `provenance.rs` | 320 | Data source tracking (DOI, URL, file hash, API, database) |
| `problem.rs` | 299 | Problem types, state space encodings, solver strategies |
| `engine.rs` | 633 | Discovery engine connecting to mdb-core primitives |
| `manifest.rs` | 243 | Reproducibility manifest generation and verification |
| `catalog.rs` | 843 | 15 pre-built experiment templates |
| `runner.rs` | 306 | Top-level orchestrator (validate → run → manifest) |
| `custom.rs` | — | Custom experiment builder (any fields, any problem) |
| `assistant.rs` | — | AI-powered experiment planning and auto-fill |

---

## Pre-Built Experiment Templates

### From Ryan Guitard's Research Papers (10)

| # | Experiment | Problem Type | Key Inputs |
|---|-----------|-------------|------------|
| 1 | Protein Folding Discovery | Continuous Opt | Amino acid sequence, force field, temperature, pH |
| 2 | Supply Chain Optimization | Combinatorial Opt | Suppliers, warehouses, costs, demand, capacities |
| 3 | Climate Modeling | Discovery | Historical temps, CO₂, ocean heat, forcing data |
| 4 | Drug Discovery | Discovery | Target protein, binding site, known actives, ADMET |
| 5 | Cryptographic Analysis | Constraint Sat | Cipher structure, plaintext-ciphertext pairs |
| 6 | Financial Portfolio Optimization | Continuous Opt | Returns, covariance, constraints, risk-free rate |
| 7 | Quantum Error Correction | Discovery | Physical/logical qubits, noise model, connectivity |
| 8 | Network Routing Optimization | Combinatorial Opt | Topology, capacities, latencies, traffic demands |
| 9 | Novel Materials Discovery | Discovery | Element set, target properties, known compounds |
| 10 | Genomic Sequence Alignment | Combinatorial Opt | Sequences, substitution matrix, gap penalties |

### Critical Global Problems (5)

| # | Experiment | Problem Type | Key Inputs |
|---|-----------|-------------|------------|
| 11 | Energy Grid Optimization | Combinatorial Opt | Generators, demand, renewables, transmission |
| 12 | Pandemic Response Optimization | Combinatorial Opt | Regions, SIR params, vaccines, hospital capacity |
| 13 | Water Resource Management | Combinatorial Opt | Sources, users, ecological flows, climate scenarios |
| 14 | Traffic Flow Optimization | Combinatorial Opt | Intersections, road network, signal configs |
| 15 | Agricultural Yield Optimization | Combinatorial Opt | Fields, crops, weather, water/fertilizer budgets |

---

## Input Validation — The Gatekeeper

The framework *refuses to run* if:

- ❌ Any required field is missing
- ❌ Any field has the wrong type (e.g., string where float is expected)
- ❌ Any numeric value is out of bounds
- ❌ Any array has too few or too many elements
- ❌ Any value isn't in the allowed set
- ❌ Any field requiring provenance doesn't have a source

This is intentional. Anonymous data and missing inputs produce meaningless results.

### Supported Field Types

| Type | Description | Example |
|------|-------------|---------|
| `Integer` | Whole number | `42` |
| `Float` | Decimal number | `3.14159` |
| `String` | Text | `"MKFLIL..."` |
| `Boolean` | True/false | `true` |
| `FloatArray` | Array of decimals | `[1.0, 2.5, 3.7]` |
| `IntArray` | Array of integers | `[1, 5, 10, 20]` |
| `FloatMatrix` | 2D matrix | `[[1.0, 0.5], [0.5, 1.0]]` |
| `Json` | Structured object | `{"key": "value"}` |
| `Binary` | Raw bytes (base64) | `"SGVsbG8="` |

### Field Configuration

```rust
// Required float with unit, bounds, and provenance
InputField::required_float("temperature", "Reaction temperature")
    .with_unit("kelvin")
    .with_bounds(200.0, 500.0)

// Required array with length constraints
InputField::required_float_array("measurements", "Sensor readings")
    .with_unit("mV")
    .with_length_bounds(100, 10000)

// Optional with default (won't block execution if missing)
InputField::required_int("max_iterations", "Solver iterations")
    .optional_with_default("1000")

// No provenance required (e.g., configuration, not data)
InputField::required_json("solver_config", "Solver configuration")
    .no_provenance()
```

---

## Data Provenance

Every data input should have a source. The framework supports:

| Source Type | Description | Example |
|------------|-------------|---------|
| `DOI` | Published paper | `DataSource::doi("10.1038/s41586-024-07487-w")` |
| `URL` | Web resource | `DataSource::url("https://data.gov/dataset/...")` |
| `File` | Local file (auto-hashed) | `DataSource::file("/data/measurements.csv")` |
| `Database` | Database query | `DataSource::database("PostgreSQL", "SELECT ...")` |
| `API` | API endpoint | `DataSource::api("NOAA Climate API", "v3/data/...")` |
| `Computed` | Derived from other data | `DataSource::computed("Averaged from 3 runs")` |
| `Manual` | Hand-entered | `DataSource::manual("Lab notebook p.42")` |

Each source includes:
- Timestamp (when accessed)
- Optional SHA-256 hash (for file/binary data)
- Optional citation (title, authors, year)

---

## Reproducibility Manifests

Every successful run produces a JSON manifest:

```json
{
  "manifest_id": "a1b2c3d4-...",
  "created_at": "2026-05-22T18:30:00Z",
  "mdb_version": "0.3.0",
  "definition": { ... },
  "input_hashes": {
    "amino_acid_sequence": "sha256:3a7f...",
    "temperature": "sha256:b2c1..."
  },
  "provenance": {
    "chain": [
      {"field": "temperature", "source": {"DOI": "10.1016/..."}}
    ],
    "combined_hash": "sha256:9e4d..."
  },
  "environment": {
    "os": "linux",
    "arch": "x86_64",
    "cpu_cores": 16,
    "memory_bytes": 68719476736,
    "rustc_version": "1.79.0"
  },
  "results": {
    "best_score": 0.947832,
    "best_solution_hex": "4f2a1b...",
    "candidates_evaluated": 1024,
    "superposition_intact": true
  },
  "manifest_hash": "sha256:7c3e..."
}
```

To verify: `manifest.verify_integrity()` recomputes the hash and confirms nothing was tampered with.

---

## Solver Strategies

| Strategy | Best For | MDB Primitive |
|----------|----------|---------------|
| `FitnessSearch` | Optimization with known fitness function | `mdb_core::search::fitness_search` |
| `Qaoa` | Combinatorial optimization (MaxCut, TSP) | `mdb_core::variational::qaoa` |
| `Evolution` | Exploring emergent behavior, open-ended | `mdb_core::evolution::evolve_dimensional_n` |
| `Hybrid` | When you're not sure — tries multiple | Runs all, keeps best |
| `Custom` | When you want to orchestrate manually | Use engine methods directly |

---

## AI Assistant

The AI assistant helps you go from "I have a problem" to "I have a running experiment":

1. **`plan_experiment(description)`** — Describe your problem in plain English. Gets back a structured plan: problem type, encoding, required data fields, suggested data sources.

2. **`refine_plan(plan, feedback)`** — Iterate on the plan. "Add a constraint for budget" → updated plan.

3. **`auto_fill(experiment, data_sources)`** — Point it at your data files/APIs and it fills in the input fields automatically with proper provenance.

4. **`suggest_experiments(domain)`** — Given a domain ("materials science", "logistics"), suggests experiments you could run.

5. **`explain_results(result)`** — Takes a DiscoveryResult and explains what was found in plain English.

---

## Building

```bash
# From the MDB-OS root
cargo build -p mdb-experiments

# Run tests
cargo test -p mdb-experiments

# Build with all optimizations (for actual experiments)
cargo build -p mdb-experiments --release
```

---

## Example: Running a Full Experiment

```rust
use mdb_experiments::*;
use mdb_experiments::catalog;
use mdb_experiments::schema::InputValue;
use mdb_experiments::provenance::DataSource;
use std::collections::HashMap;

fn main() {
    // Pick the supply chain optimization experiment
    let experiment = catalog::supply_chain_optimization();

    // Check what we need
    println!("Required inputs for '{}':", experiment.name);
    for (name, ftype, desc) in ExperimentRunner::required_fields(&experiment) {
        println!("  {} ({}) — {}", name, ftype, desc);
    }

    // Prepare inputs with real data and provenance
    let mut inputs = HashMap::new();

    inputs.insert("num_suppliers".to_string(),
        InputValue::new(serde_json::json!(25))
            .with_source(DataSource::manual("Company ERP system export 2026-05")));

    inputs.insert("num_warehouses".to_string(),
        InputValue::new(serde_json::json!(8))
            .with_source(DataSource::manual("Company ERP system export 2026-05")));

    inputs.insert("num_retailers".to_string(),
        InputValue::new(serde_json::json!(150))
            .with_source(DataSource::database("SAP", "SELECT COUNT(*) FROM retail_locations")));

    inputs.insert("transport_costs".to_string(),
        InputValue::new(serde_json::json!([[10.5, 20.3], [15.2, 8.7]]))
            .with_source(DataSource::file("/data/transport_costs_2026.csv")));

    inputs.insert("supplier_capacities".to_string(),
        InputValue::new(serde_json::json!([1000.0, 2000.0, 1500.0]))
            .with_source(DataSource::api("Supplier Portal API", "/v2/capacities")));

    inputs.insert("warehouse_capacities".to_string(),
        InputValue::new(serde_json::json!([5000.0, 3000.0]))
            .with_source(DataSource::file("/data/warehouse_specs.json")));

    inputs.insert("demand".to_string(),
        InputValue::new(serde_json::json!([100.0, 200.0, 150.0]))
            .with_source(DataSource::database("Salesforce", "Q2 2026 forecast")));

    inputs.insert("lead_times".to_string(),
        InputValue::new(serde_json::json!([2.0, 3.5, 1.0]))
            .with_source(DataSource::manual("Historical average from 2025 data")));

    inputs.insert("holding_cost_rate".to_string(),
        InputValue::new(serde_json::json!(0.15))
            .with_source(DataSource::manual("Finance team estimate")));

    inputs.insert("penalty_rate".to_string(),
        InputValue::new(serde_json::json!(50.0))
            .with_source(DataSource::manual("SLA penalty clause")));

    // Validate first (optional — runner does this anyway)
    let runner = ExperimentRunner::new().with_max_candidates(512);
    let validation = runner.validate_only(&experiment, &inputs);
    if !validation.valid {
        for err in &validation.errors {
            eprintln!("ERROR: [{}] {}", err.field, err.message);
        }
        return;
    }

    // Run with progress callback
    let result = runner.run(
        &experiment,
        inputs,
        Some(Box::new(|update| {
            println!("[{:.0}%] {} — {}", update.progress * 100.0, update.phase, update.message);
        })),
    );

    // Check results
    if result.success {
        let discovery = result.discovery.unwrap();
        println!("\n=== DISCOVERY ===");
        println!("Best score: {:.6}", discovery.best_score);
        println!("Solution: {}", discovery.best_solution.interpretation);
        println!("Candidates evaluated: {}", discovery.candidates_evaluated);
        println!("Superposition intact: {}", discovery.superposition_intact);

        // Save the manifest
        let manifest = result.manifest.unwrap();
        manifest.save("supply_chain_run_001.json").unwrap();
        println!("\nManifest saved: {}", manifest.manifest_id);
        println!("Integrity verified: {}", manifest.verify_integrity());
    } else {
        eprintln!("Experiment failed: {}", result.message);
    }
}
```

---

## FAQ

**Q: Can I define my own experiment from scratch?**
A: Yes. Use `CustomExperiment::new()` or `ExperimentDefinition::builder()`. You can add any fields, any types, any constraints. The 15 templates are starting points, not limits.

**Q: What if I don't know what data I need?**
A: Use the AI assistant. Describe your problem in plain English and it tells you exactly what data to gather, what type each field should be, and where to find the data.

**Q: Why does it refuse to run with missing inputs?**
A: Because this is real science. Running an experiment with missing or anonymous data produces meaningless results. The framework forces rigor.

**Q: Can I run without provenance?**
A: Individual fields can be marked `.no_provenance()` if they're configuration rather than data. But for real data inputs, provenance is required by default. You can override per-field.

**Q: How do I reproduce someone else's results?**
A: Load their manifest (`ReproducibilityManifest::load("run.json")`), get the same input data (hashes are in the manifest for verification), and re-run. The manifest captures everything: definition, inputs, environment, and results.

**Q: What if I want to run the same experiment with different parameters?**
A: Clone the experiment definition, modify what you want, and run again. Each run gets its own manifest.

---

## License

Part of MDB-OS. Invented by Ryan Guitard. All rights reserved.
