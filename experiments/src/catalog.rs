//! # Experiment Catalog
//!
//! Pre-built experiment templates for the 10 experiments described in
//! Ryan Guitard's MDB research papers, plus 5 additional critical
//! global problems.
//!
//! Each template defines:
//! - The exact input schema (what real-world data you need)
//! - The state space encoding
//! - The solver strategy
//! - Success criteria
//!
//! These are NOT toys. Each requires real data to run.
//! The templates ensure you provide EVERY required input.

use crate::problem::*;
use crate::schema::*;

/// Get all 15 pre-built experiment templates.
pub fn all_experiments() -> Vec<ExperimentDefinition> {
    vec![
        // ── Original 10 from the research papers ────────────────────
        protein_folding(),
        supply_chain_optimization(),
        climate_modeling(),
        drug_discovery(),
        cryptographic_analysis(),
        financial_portfolio_optimization(),
        quantum_error_correction(),
        network_routing(),
        materials_science(),
        genomic_sequence_alignment(),
        // ── Additional 5 critical global problems ───────────────────
        energy_grid_optimization(),
        pandemic_response_optimization(),
        water_resource_management(),
        traffic_flow_optimization(),
        agricultural_yield_optimization(),
    ]
}

/// Get a specific experiment by name.
pub fn get_experiment(name: &str) -> Option<ExperimentDefinition> {
    all_experiments().into_iter().find(|e| e.name == name)
}

/// List all experiment names and descriptions.
pub fn list_experiments() -> Vec<(String, String)> {
    all_experiments()
        .into_iter()
        .map(|e| (e.name, e.description))
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════
// EXPERIMENT 1: Protein Folding
// ═══════════════════════════════════════════════════════════════════════

pub fn protein_folding() -> ExperimentDefinition {
    ExperimentDefinition::builder("Protein Folding Discovery")
        .description(
            "Discover the minimum-energy 3D conformation of a protein from its \
             amino acid sequence. Uses MDB's state space to explore folding \
             pathways that classical methods cannot reach in feasible time."
        )
        .problem_type(ProblemType::ContinuousOptimization {
            maximize: false, // Minimize energy
            dimensions: 0,   // Set from input
            objective: "Minimize free energy of protein conformation".to_string(),
        })
        .encoding(StateSpaceEncoding::RealValued {
            dimensions: 300, // ~100 residues × 3 torsion angles
            bounds: vec![(-180.0, 180.0); 300], // Degrees
        })
        .strategy(SolverStrategy::Hybrid {
            strategies: vec![
                SolverStrategy::FitnessSearch { num_candidates: 1024 },
                SolverStrategy::Evolution { steps: 500 },
            ],
        })
        .schema(
            InputSchema::new("protein_folding", "Protein folding experiment inputs")
                .field(InputField::required_string("amino_acid_sequence",
                    "Full amino acid sequence (single-letter code, e.g., MKFLILLFNILCLFPVLAADNHGV...)"))
                .field(InputField::required_json("force_field_params",
                    "Force field parameters (AMBER, CHARMM, or OPLS format as JSON)")
                    .with_unit("kcal/mol"))
                .field(InputField::required_float("temperature",
                    "Simulation temperature").with_unit("kelvin").with_bounds(0.0, 1000.0))
                .field(InputField::required_float("ph",
                    "Solution pH").with_bounds(0.0, 14.0))
                .field(InputField::required_json("known_structures",
                    "Known experimental structures (PDB format as JSON) for validation"))
                .field(InputField::required_float_array("residue_contacts",
                    "Experimental contact map (from NMR or cross-linking mass spec)")
                    .no_provenance())
                .field(InputField::required_int("sequence_length",
                    "Number of amino acid residues").with_bounds(10.0, 10000.0))
        )
        .success_criteria(SuccessCriteria::OpenEnded {
            description: "Find a conformation with lower free energy than any known structure, \
                         or discover a novel folding pathway".to_string(),
        })
        .tag("biology")
        .tag("protein")
        .tag("structural")
        .reference("Guitard, R. (2026). MDB Experiment Papers — Protein Folding")
        .build()
        .expect("protein_folding definition")
}

// ═══════════════════════════════════════════════════════════════════════
// EXPERIMENT 2: Supply Chain Optimization
// ═══════════════════════════════════════════════════════════════════════

pub fn supply_chain_optimization() -> ExperimentDefinition {
    ExperimentDefinition::builder("Supply Chain Optimization")
        .description(
            "Optimize a multi-echelon supply chain network to minimize total cost \
             while satisfying demand constraints, lead times, and capacity limits. \
             Encodes the network as a combinatorial assignment problem."
        )
        .problem_type(ProblemType::CombinatorialOptimization {
            maximize: false,
            objective: "Minimize total supply chain cost (transport + inventory + penalties)".to_string(),
        })
        .encoding(StateSpaceEncoding::Integer {
            num_vars: 100, // Supplier-warehouse-retailer assignments
            domains: vec![(0, 50); 100],
        })
        .strategy(SolverStrategy::Hybrid {
            strategies: vec![
                SolverStrategy::Qaoa { depth: 4, max_iterations: 200 },
                SolverStrategy::FitnessSearch { num_candidates: 512 },
            ],
        })
        .schema(
            InputSchema::new("supply_chain", "Supply chain optimization inputs")
                .field(InputField::required_int("num_suppliers",
                    "Number of supplier nodes").with_bounds(1.0, 10000.0))
                .field(InputField::required_int("num_warehouses",
                    "Number of warehouse nodes").with_bounds(1.0, 1000.0))
                .field(InputField::required_int("num_retailers",
                    "Number of retail/demand points").with_bounds(1.0, 100000.0))
                .field(InputField::required_matrix("transport_costs",
                    "Transport cost matrix [suppliers+warehouses × warehouses+retailers]")
                    .with_unit("USD"))
                .field(InputField::required_float_array("supplier_capacities",
                    "Maximum supply capacity per supplier").with_unit("units"))
                .field(InputField::required_float_array("warehouse_capacities",
                    "Maximum throughput per warehouse").with_unit("units"))
                .field(InputField::required_float_array("demand",
                    "Demand at each retail point").with_unit("units"))
                .field(InputField::required_float_array("lead_times",
                    "Lead time for each route").with_unit("days"))
                .field(InputField::required_float("holding_cost_rate",
                    "Inventory holding cost per unit per day").with_unit("USD/unit/day"))
                .field(InputField::required_float("penalty_rate",
                    "Penalty for unmet demand per unit").with_unit("USD/unit"))
        )
        .success_criteria(SuccessCriteria::BeatBaseline {
            baseline: 0.0, // Set from input data
            description: "Beat the current supply chain cost or find a novel topology".to_string(),
        })
        .tag("logistics")
        .tag("optimization")
        .reference("Guitard, R. (2026). MDB Experiment Papers — Supply Chain")
        .build()
        .expect("supply_chain definition")
}

// ═══════════════════════════════════════════════════════════════════════
// EXPERIMENT 3: Climate Modeling
// ═══════════════════════════════════════════════════════════════════════

pub fn climate_modeling() -> ExperimentDefinition {
    ExperimentDefinition::builder("Climate Modeling")
        .description(
            "Model climate system dynamics by exploring parameter spaces of \
             coupled ocean-atmosphere models. Discover parameter regimes that \
             match observed data but reveal previously unknown sensitivity points."
        )
        .problem_type(ProblemType::Discovery {
            domain: "Climate science — coupled ocean-atmosphere dynamics".to_string(),
            novelty_metric: "Divergence from ensemble mean while matching observations".to_string(),
        })
        .encoding(StateSpaceEncoding::RealValued {
            dimensions: 50,
            bounds: vec![(0.0, 1.0); 50],
        })
        .strategy(SolverStrategy::Hybrid {
            strategies: vec![
                SolverStrategy::Evolution { steps: 1000 },
                SolverStrategy::FitnessSearch { num_candidates: 256 },
            ],
        })
        .schema(
            InputSchema::new("climate_modeling", "Climate modeling experiment inputs")
                .field(InputField::required_float_array("historical_temperatures",
                    "Historical global mean temperature anomalies (annual, °C)")
                    .with_unit("°C"))
                .field(InputField::required_float_array("co2_concentrations",
                    "Atmospheric CO₂ concentration time series (ppm)")
                    .with_unit("ppm"))
                .field(InputField::required_float_array("ocean_heat_content",
                    "Ocean heat content time series (10²² J)")
                    .with_unit("10²² J"))
                .field(InputField::required_float("climate_sensitivity_prior",
                    "Prior estimate of equilibrium climate sensitivity")
                    .with_unit("°C per CO₂ doubling").with_bounds(0.5, 10.0))
                .field(InputField::required_json("forcing_data",
                    "Radiative forcing data (volcanic, solar, aerosol, GHG)"))
                .field(InputField::required_int("projection_years",
                    "Number of years to project forward").with_bounds(1.0, 500.0))
                .field(InputField::required_float_array("sea_level_data",
                    "Historical sea level measurements (mm)")
                    .with_unit("mm"))
        )
        .success_criteria(SuccessCriteria::NovelDiscovery {
            novelty_threshold: 0.8,
        })
        .tag("climate")
        .tag("earth_science")
        .reference("Guitard, R. (2026). MDB Experiment Papers — Climate Modeling")
        .build()
        .expect("climate_modeling definition")
}

// ═══════════════════════════════════════════════════════════════════════
// EXPERIMENT 4: Drug Discovery
// ═══════════════════════════════════════════════════════════════════════

pub fn drug_discovery() -> ExperimentDefinition {
    ExperimentDefinition::builder("Drug Discovery")
        .description(
            "Discover novel molecular structures with high binding affinity to a \
             target protein. Explores chemical space using MDB's state space to \
             find drug candidates that traditional virtual screening misses."
        )
        .problem_type(ProblemType::Discovery {
            domain: "Medicinal chemistry — molecular binding optimization".to_string(),
            novelty_metric: "Tanimoto distance from known actives + binding affinity".to_string(),
        })
        .encoding(StateSpaceEncoding::Binary {
            bits: 256,
            bit_mapping: "Morgan fingerprint bits representing molecular substructures".to_string(),
        })
        .strategy(SolverStrategy::Hybrid {
            strategies: vec![
                SolverStrategy::FitnessSearch { num_candidates: 1024 },
                SolverStrategy::Qaoa { depth: 3, max_iterations: 150 },
            ],
        })
        .schema(
            InputSchema::new("drug_discovery", "Drug discovery experiment inputs")
                .field(InputField::required_json("target_protein",
                    "Target protein structure (PDB format as JSON, including binding site residues)"))
                .field(InputField::required_json("binding_site",
                    "Binding site definition (residue IDs, coordinates, pharmacophore features)"))
                .field(InputField::required_json("known_actives",
                    "Known active compounds (SMILES + activity values)"))
                .field(InputField::required_json("known_inactives",
                    "Known inactive compounds (SMILES) for contrast"))
                .field(InputField::required_float_array("pharmacophore_weights",
                    "Weights for pharmacophore features (H-bond donor/acceptor, hydrophobic, etc.)"))
                .field(InputField::required_json("admet_constraints",
                    "ADMET property constraints (solubility, permeability, toxicity thresholds)"))
                .field(InputField::required_float("activity_threshold",
                    "Minimum activity threshold (IC50/Ki) to consider 'active'")
                    .with_unit("nM").with_bounds(0.001, 100000.0))
        )
        .success_criteria(SuccessCriteria::OpenEnded {
            description: "Discover molecular structures with predicted binding affinity \
                         better than known actives, while satisfying ADMET constraints".to_string(),
        })
        .tag("pharma")
        .tag("chemistry")
        .tag("drug_design")
        .reference("Guitard, R. (2026). MDB Experiment Papers — Drug Discovery")
        .build()
        .expect("drug_discovery definition")
}

// ═══════════════════════════════════════════════════════════════════════
// EXPERIMENT 5: Cryptographic Analysis
// ═══════════════════════════════════════════════════════════════════════

pub fn cryptographic_analysis() -> ExperimentDefinition {
    ExperimentDefinition::builder("Cryptographic Analysis")
        .description(
            "Analyze cryptographic constructions by exploring the key space and \
             looking for structural weaknesses. Tests whether MDB's non-destructive \
             superposition can detect patterns that brute force cannot."
        )
        .problem_type(ProblemType::ConstraintSatisfaction {
            num_constraints: 0, // Determined by cipher structure
            constraint_description: "Find key bits satisfying ciphertext-plaintext relationship".to_string(),
        })
        .encoding(StateSpaceEncoding::Binary {
            bits: 128,
            bit_mapping: "Candidate key bits".to_string(),
        })
        .strategy(SolverStrategy::Hybrid {
            strategies: vec![
                SolverStrategy::FitnessSearch { num_candidates: 1024 },
                SolverStrategy::Evolution { steps: 200 },
            ],
        })
        .schema(
            InputSchema::new("crypto_analysis", "Cryptographic analysis inputs")
                .field(InputField::required_string("cipher_algorithm",
                    "Cipher algorithm name (e.g., AES-128, DES, custom)"))
                .field(InputField::required_int("key_length_bits",
                    "Key length in bits").with_bounds(8.0, 4096.0))
                .field(InputField::required_json("plaintext_ciphertext_pairs",
                    "Known plaintext-ciphertext pairs [{plaintext: hex, ciphertext: hex}]"))
                .field(InputField::required_json("cipher_structure",
                    "Cipher round structure definition (S-boxes, permutations, key schedule)"))
                .field(InputField::required_int("num_rounds",
                    "Number of cipher rounds").with_bounds(1.0, 256.0))
                .field(InputField::required_json("known_biases",
                    "Known statistical biases in the cipher (if any)")
                    .no_provenance())
        )
        .success_criteria(SuccessCriteria::FindFeasible)
        .tag("cryptography")
        .tag("security")
        .reference("Guitard, R. (2026). MDB Experiment Papers — Cryptographic Analysis")
        .build()
        .expect("crypto_analysis definition")
}

// ═══════════════════════════════════════════════════════════════════════
// EXPERIMENT 6: Financial Portfolio Optimization
// ═══════════════════════════════════════════════════════════════════════

pub fn financial_portfolio_optimization() -> ExperimentDefinition {
    ExperimentDefinition::builder("Financial Portfolio Optimization")
        .description(
            "Discover optimal portfolio allocations that maximize risk-adjusted \
             return on the efficient frontier. Uses real market data to find \
             allocations that classical mean-variance optimization misses due \
             to non-convexity from real-world constraints."
        )
        .problem_type(ProblemType::ContinuousOptimization {
            maximize: true,
            dimensions: 50, // Up to 50 assets
            objective: "Maximize Sharpe ratio with real-world constraints".to_string(),
        })
        .encoding(StateSpaceEncoding::RealValued {
            dimensions: 50,
            bounds: vec![(0.0, 1.0); 50], // Portfolio weights (0–100%)
        })
        .strategy(SolverStrategy::FitnessSearch { num_candidates: 512 })
        .schema(
            InputSchema::new("portfolio_optimization", "Portfolio optimization inputs")
                .field(InputField::required_int("num_assets",
                    "Number of assets in the universe").with_bounds(2.0, 10000.0))
                .field(InputField::required_float_array("expected_returns",
                    "Expected annual return per asset").with_unit("fraction"))
                .field(InputField::required_matrix("covariance_matrix",
                    "Asset return covariance matrix"))
                .field(InputField::required_float("risk_free_rate",
                    "Risk-free rate").with_unit("fraction").with_bounds(-0.1, 0.5))
                .field(InputField::required_float("max_position_size",
                    "Maximum allocation to any single asset").with_unit("fraction")
                    .with_bounds(0.01, 1.0))
                .field(InputField::required_float("target_return",
                    "Target annual return constraint").with_unit("fraction"))
                .field(InputField::required_json("sector_constraints",
                    "Sector exposure limits {sector: {min: float, max: float}}"))
                .field(InputField::required_float_array("transaction_costs",
                    "Transaction cost per asset").with_unit("fraction"))
        )
        .success_criteria(SuccessCriteria::BeatBaseline {
            baseline: 0.0,
            description: "Beat the Sharpe ratio of equal-weight and market-cap-weight portfolios".to_string(),
        })
        .tag("finance")
        .tag("portfolio")
        .reference("Guitard, R. (2026). MDB Experiment Papers — Portfolio Optimization")
        .build()
        .expect("portfolio_optimization definition")
}

// ═══════════════════════════════════════════════════════════════════════
// EXPERIMENT 7: Quantum Error Correction
// ═══════════════════════════════════════════════════════════════════════

pub fn quantum_error_correction() -> ExperimentDefinition {
    ExperimentDefinition::builder("Quantum Error Correction Code Discovery")
        .description(
            "Discover novel quantum error-correcting codes by exploring the space \
             of stabilizer codes. MDB's non-destructive inspection is uniquely \
             suited to this — it can evaluate code distance without collapsing \
             the code space."
        )
        .problem_type(ProblemType::Discovery {
            domain: "Quantum information — stabilizer codes".to_string(),
            novelty_metric: "Code distance × encoding rate, weighted by decoder complexity".to_string(),
        })
        .encoding(StateSpaceEncoding::Binary {
            bits: 200,
            bit_mapping: "Stabilizer generators as binary symplectic vectors".to_string(),
        })
        .strategy(SolverStrategy::Evolution { steps: 500 })
        .schema(
            InputSchema::new("qec_discovery", "Quantum error correction inputs")
                .field(InputField::required_int("num_physical_qubits",
                    "Number of physical qubits").with_bounds(3.0, 10000.0))
                .field(InputField::required_int("num_logical_qubits",
                    "Number of logical qubits to encode").with_bounds(1.0, 1000.0))
                .field(InputField::required_float("target_code_distance",
                    "Minimum desired code distance").with_bounds(1.0, 100.0))
                .field(InputField::required_json("noise_model",
                    "Physical noise model (depolarizing, amplitude damping, etc.)"))
                .field(InputField::required_float("error_rate",
                    "Physical error rate per gate").with_bounds(0.0, 1.0))
                .field(InputField::required_json("connectivity",
                    "Physical qubit connectivity graph (for implementability)")
                    .no_provenance())
        )
        .success_criteria(SuccessCriteria::NovelDiscovery {
            novelty_threshold: 0.9,
        })
        .tag("quantum")
        .tag("error_correction")
        .reference("Guitard, R. (2026). MDB Experiment Papers — QEC")
        .build()
        .expect("qec_discovery definition")
}

// ═══════════════════════════════════════════════════════════════════════
// EXPERIMENT 8: Network Routing
// ═══════════════════════════════════════════════════════════════════════

pub fn network_routing() -> ExperimentDefinition {
    ExperimentDefinition::builder("Network Routing Optimization")
        .description(
            "Optimize packet routing in large-scale networks (telecom, Internet, \
             data center). Discover routing tables that minimize latency and \
             maximize throughput under dynamic traffic patterns."
        )
        .problem_type(ProblemType::CombinatorialOptimization {
            maximize: false,
            objective: "Minimize maximum link utilization while meeting latency SLAs".to_string(),
        })
        .encoding(StateSpaceEncoding::Graph {
            num_nodes: 100,
            directed: true,
        })
        .strategy(SolverStrategy::Qaoa { depth: 5, max_iterations: 300 })
        .schema(
            InputSchema::new("network_routing", "Network routing inputs")
                .field(InputField::required_int("num_nodes",
                    "Number of network nodes").with_bounds(2.0, 100000.0))
                .field(InputField::required_matrix("link_capacities",
                    "Link capacity matrix [node × node]").with_unit("Gbps"))
                .field(InputField::required_matrix("link_latencies",
                    "Link latency matrix [node × node]").with_unit("ms"))
                .field(InputField::required_json("traffic_demands",
                    "Traffic demand matrix [{src, dst, demand_gbps, max_latency_ms}]"))
                .field(InputField::required_json("failure_scenarios",
                    "Link/node failure scenarios to be resilient against"))
                .field(InputField::required_float("max_link_utilization",
                    "Maximum allowed link utilization").with_bounds(0.1, 1.0))
        )
        .success_criteria(SuccessCriteria::BeatBaseline {
            baseline: 0.0,
            description: "Beat ECMP and OSPF routing in terms of max link utilization".to_string(),
        })
        .tag("networking")
        .tag("routing")
        .reference("Guitard, R. (2026). MDB Experiment Papers — Network Routing")
        .build()
        .expect("network_routing definition")
}

// ═══════════════════════════════════════════════════════════════════════
// EXPERIMENT 9: Materials Science
// ═══════════════════════════════════════════════════════════════════════

pub fn materials_science() -> ExperimentDefinition {
    ExperimentDefinition::builder("Novel Materials Discovery")
        .description(
            "Discover new material compositions with target properties (strength, \
             conductivity, thermal stability). Explores the compositional space \
             using MDB to find combinations that DFT calculations alone cannot \
             reach due to combinatorial explosion."
        )
        .problem_type(ProblemType::Discovery {
            domain: "Materials science — multi-component alloy/compound design".to_string(),
            novelty_metric: "Property score × compositional novelty vs. known materials DB".to_string(),
        })
        .encoding(StateSpaceEncoding::RealValued {
            dimensions: 20,
            bounds: vec![(0.0, 1.0); 20], // Element fractions
        })
        .strategy(SolverStrategy::Hybrid {
            strategies: vec![
                SolverStrategy::FitnessSearch { num_candidates: 512 },
                SolverStrategy::Evolution { steps: 300 },
            ],
        })
        .schema(
            InputSchema::new("materials_discovery", "Materials discovery inputs")
                .field(InputField::required_json("element_set",
                    "Candidate elements [{symbol, atomic_number, properties}]"))
                .field(InputField::required_int("max_components",
                    "Maximum number of elements in the compound").with_bounds(2.0, 20.0))
                .field(InputField::required_json("target_properties",
                    "Target material properties {tensile_strength_MPa, conductivity_S_m, melting_point_K, etc.}"))
                .field(InputField::required_json("known_compounds",
                    "Database of known compounds for novelty comparison"))
                .field(InputField::required_json("formation_energy_model",
                    "Model/data for predicting formation energy of candidate compounds"))
                .field(InputField::required_float_array("composition_constraints",
                    "Min/max fraction per element").no_provenance())
        )
        .success_criteria(SuccessCriteria::NovelDiscovery {
            novelty_threshold: 0.85,
        })
        .tag("materials")
        .tag("chemistry")
        .reference("Guitard, R. (2026). MDB Experiment Papers — Materials Science")
        .build()
        .expect("materials_discovery definition")
}

// ═══════════════════════════════════════════════════════════════════════
// EXPERIMENT 10: Genomic Sequence Alignment
// ═══════════════════════════════════════════════════════════════════════

pub fn genomic_sequence_alignment() -> ExperimentDefinition {
    ExperimentDefinition::builder("Genomic Sequence Alignment")
        .description(
            "Find optimal alignments between genomic sequences that reveal \
             evolutionary relationships, gene function, and structural features. \
             MDB explores the alignment space simultaneously, finding alignments \
             that dynamic programming misses due to gap penalty limitations."
        )
        .problem_type(ProblemType::CombinatorialOptimization {
            maximize: true,
            objective: "Maximize alignment score with biologically meaningful gap model".to_string(),
        })
        .encoding(StateSpaceEncoding::Integer {
            num_vars: 500,
            domains: vec![(-1, 3); 500], // -1=gap, 0-3=ACGT
        })
        .strategy(SolverStrategy::FitnessSearch { num_candidates: 1024 })
        .schema(
            InputSchema::new("genomic_alignment", "Genomic alignment inputs")
                .field(InputField::required_string("sequence_a",
                    "First DNA/RNA/protein sequence"))
                .field(InputField::required_string("sequence_b",
                    "Second DNA/RNA/protein sequence"))
                .field(InputField::required_matrix("substitution_matrix",
                    "Substitution scoring matrix (e.g., BLOSUM62, PAM250, or custom)"))
                .field(InputField::required_float("gap_open_penalty",
                    "Penalty for opening a gap").with_bounds(-1000.0, 0.0))
                .field(InputField::required_float("gap_extend_penalty",
                    "Penalty for extending a gap").with_bounds(-1000.0, 0.0))
                .field(InputField::required_string("sequence_type",
                    "Type of sequences: 'dna', 'rna', or 'protein'"))
                .field(InputField::required_json("structural_annotations",
                    "Known structural features (domains, motifs, regulatory regions)")
                    .no_provenance())
        )
        .success_criteria(SuccessCriteria::BeatBaseline {
            baseline: 0.0,
            description: "Beat BLAST/Smith-Waterman alignment score while maintaining biological validity".to_string(),
        })
        .tag("genomics")
        .tag("bioinformatics")
        .reference("Guitard, R. (2026). MDB Experiment Papers — Genomic Alignment")
        .build()
        .expect("genomic_alignment definition")
}

// ═══════════════════════════════════════════════════════════════════════
// EXPERIMENT 11: Energy Grid Optimization
// ═══════════════════════════════════════════════════════════════════════

pub fn energy_grid_optimization() -> ExperimentDefinition {
    ExperimentDefinition::builder("Energy Grid Optimization")
        .description(
            "Optimize the dispatch and topology of a power grid with mixed \
             renewable and conventional sources. Discover optimal scheduling \
             that balances cost, emissions, and reliability under uncertain \
             renewable generation."
        )
        .problem_type(ProblemType::CombinatorialOptimization {
            maximize: false,
            objective: "Minimize cost + emissions penalty subject to reliability constraints".to_string(),
        })
        .encoding(StateSpaceEncoding::Integer {
            num_vars: 200,
            domains: vec![(0, 100); 200],
        })
        .strategy(SolverStrategy::Hybrid {
            strategies: vec![
                SolverStrategy::Qaoa { depth: 4, max_iterations: 250 },
                SolverStrategy::FitnessSearch { num_candidates: 512 },
            ],
        })
        .schema(
            InputSchema::new("energy_grid", "Energy grid optimization inputs")
                .field(InputField::required_int("num_generators",
                    "Number of power generators").with_bounds(1.0, 10000.0))
                .field(InputField::required_json("generator_specs",
                    "Generator specifications [{type, capacity_mw, cost_per_mwh, co2_per_mwh, ramp_rate}]"))
                .field(InputField::required_float_array("demand_profile",
                    "24-hour demand profile (hourly)").with_unit("MW"))
                .field(InputField::required_float_array("solar_forecast",
                    "Solar generation forecast (hourly)").with_unit("MW"))
                .field(InputField::required_float_array("wind_forecast",
                    "Wind generation forecast (hourly)").with_unit("MW"))
                .field(InputField::required_matrix("transmission_capacity",
                    "Transmission line capacity matrix").with_unit("MW"))
                .field(InputField::required_float("reserve_margin",
                    "Required reserve margin").with_unit("fraction").with_bounds(0.0, 1.0))
                .field(InputField::required_float("carbon_price",
                    "Carbon price").with_unit("USD/ton CO2"))
        )
        .success_criteria(SuccessCriteria::BeatBaseline {
            baseline: 0.0,
            description: "Beat current dispatch schedule in total cost + emissions".to_string(),
        })
        .tag("energy")
        .tag("sustainability")
        .reference("Guitard, R. (2026). MDB — Critical Global Problems")
        .build()
        .expect("energy_grid definition")
}

// ═══════════════════════════════════════════════════════════════════════
// EXPERIMENT 12: Pandemic Response Optimization
// ═══════════════════════════════════════════════════════════════════════

pub fn pandemic_response_optimization() -> ExperimentDefinition {
    ExperimentDefinition::builder("Pandemic Response Optimization")
        .description(
            "Optimize resource allocation during a pandemic: vaccine distribution, \
             hospital capacity, testing sites, and quarantine zones. Discover \
             strategies that minimize total mortality and economic impact."
        )
        .problem_type(ProblemType::CombinatorialOptimization {
            maximize: false,
            objective: "Minimize weighted sum of mortality, hospitalizations, and economic cost".to_string(),
        })
        .encoding(StateSpaceEncoding::Integer {
            num_vars: 150,
            domains: vec![(0, 100); 150],
        })
        .strategy(SolverStrategy::Hybrid {
            strategies: vec![
                SolverStrategy::FitnessSearch { num_candidates: 512 },
                SolverStrategy::Evolution { steps: 400 },
            ],
        })
        .schema(
            InputSchema::new("pandemic_response", "Pandemic response optimization inputs")
                .field(InputField::required_int("num_regions",
                    "Number of geographic regions").with_bounds(2.0, 10000.0))
                .field(InputField::required_float_array("population",
                    "Population per region"))
                .field(InputField::required_json("sir_params",
                    "SIR/SEIR model parameters {beta, gamma, sigma, etc.} per region"))
                .field(InputField::required_float("total_vaccine_doses",
                    "Total available vaccine doses").with_bounds(0.0, 1e12))
                .field(InputField::required_float_array("hospital_beds",
                    "Hospital bed capacity per region"))
                .field(InputField::required_matrix("mobility_matrix",
                    "Inter-region mobility matrix (fraction of population traveling)"))
                .field(InputField::required_json("intervention_costs",
                    "Cost per intervention type {vaccination, testing, lockdown, etc.}").with_unit("USD"))
                .field(InputField::required_float("budget",
                    "Total budget").with_unit("USD"))
        )
        .success_criteria(SuccessCriteria::OpenEnded {
            description: "Discover resource allocation strategies that Pareto-dominate \
                         uniform distribution in mortality AND economic cost".to_string(),
        })
        .tag("public_health")
        .tag("epidemiology")
        .reference("Guitard, R. (2026). MDB — Critical Global Problems")
        .build()
        .expect("pandemic_response definition")
}

// ═══════════════════════════════════════════════════════════════════════
// EXPERIMENT 13: Water Resource Management
// ═══════════════════════════════════════════════════════════════════════

pub fn water_resource_management() -> ExperimentDefinition {
    ExperimentDefinition::builder("Water Resource Management")
        .description(
            "Optimize water allocation across agricultural, industrial, and \
             municipal users while maintaining ecological flow requirements. \
             Discover allocation strategies under climate uncertainty."
        )
        .problem_type(ProblemType::CombinatorialOptimization {
            maximize: false,
            objective: "Minimize total water deficit + ecological penalty".to_string(),
        })
        .encoding(StateSpaceEncoding::RealValued {
            dimensions: 100,
            bounds: vec![(0.0, 1.0); 100],
        })
        .strategy(SolverStrategy::FitnessSearch { num_candidates: 512 })
        .schema(
            InputSchema::new("water_management", "Water resource management inputs")
                .field(InputField::required_int("num_sources",
                    "Number of water sources (reservoirs, rivers, aquifers)").with_bounds(1.0, 1000.0))
                .field(InputField::required_int("num_users",
                    "Number of water user groups").with_bounds(1.0, 10000.0))
                .field(InputField::required_float_array("source_capacities",
                    "Available water per source").with_unit("m³/day"))
                .field(InputField::required_float_array("user_demands",
                    "Water demand per user group").with_unit("m³/day"))
                .field(InputField::required_float_array("ecological_flows",
                    "Minimum ecological flow requirements per river segment").with_unit("m³/day"))
                .field(InputField::required_matrix("conveyance_costs",
                    "Water transport cost matrix [source × user]").with_unit("USD/m³"))
                .field(InputField::required_json("climate_scenarios",
                    "Climate change scenarios affecting water availability"))
                .field(InputField::required_float_array("priority_weights",
                    "Priority weights per user group (municipal > agriculture > industrial)"))
        )
        .success_criteria(SuccessCriteria::BeatBaseline {
            baseline: 0.0,
            description: "Beat proportional allocation in total deficit reduction".to_string(),
        })
        .tag("water")
        .tag("environment")
        .reference("Guitard, R. (2026). MDB — Critical Global Problems")
        .build()
        .expect("water_management definition")
}

// ═══════════════════════════════════════════════════════════════════════
// EXPERIMENT 14: Traffic Flow Optimization
// ═══════════════════════════════════════════════════════════════════════

pub fn traffic_flow_optimization() -> ExperimentDefinition {
    ExperimentDefinition::builder("Traffic Flow Optimization")
        .description(
            "Optimize traffic signal timing and routing across a city-scale \
             road network. Discover signal coordination patterns that minimize \
             total travel time and emissions."
        )
        .problem_type(ProblemType::CombinatorialOptimization {
            maximize: false,
            objective: "Minimize total vehicle-hours + emissions penalty".to_string(),
        })
        .encoding(StateSpaceEncoding::Integer {
            num_vars: 200,
            domains: vec![(10, 120); 200], // Signal timings in seconds
        })
        .strategy(SolverStrategy::Qaoa { depth: 5, max_iterations: 300 })
        .schema(
            InputSchema::new("traffic_optimization", "Traffic flow optimization inputs")
                .field(InputField::required_int("num_intersections",
                    "Number of signalized intersections").with_bounds(1.0, 100000.0))
                .field(InputField::required_json("road_network",
                    "Road network graph {nodes: [{id, lat, lon}], edges: [{from, to, lanes, speed_limit, length}]}"))
                .field(InputField::required_json("traffic_demands",
                    "Origin-destination demand matrix [{origin, destination, vehicles_per_hour}]"))
                .field(InputField::required_json("signal_configs",
                    "Current signal configurations [{intersection_id, phases: [{green_s, yellow_s, directions}]}]"))
                .field(InputField::required_float_array("link_capacities",
                    "Capacity per link").with_unit("vehicles/hour"))
                .field(InputField::required_float("emission_factor",
                    "Average emission factor per vehicle-hour").with_unit("kg CO2/vehicle-hour"))
                .field(InputField::required_json("time_periods",
                    "Time periods to optimize [{name, start_hour, end_hour, demand_multiplier}]"))
        )
        .success_criteria(SuccessCriteria::BeatBaseline {
            baseline: 0.0,
            description: "Beat current signal timing in total travel time".to_string(),
        })
        .tag("transportation")
        .tag("urban")
        .reference("Guitard, R. (2026). MDB — Critical Global Problems")
        .build()
        .expect("traffic_optimization definition")
}

// ═══════════════════════════════════════════════════════════════════════
// EXPERIMENT 15: Agricultural Yield Optimization
// ═══════════════════════════════════════════════════════════════════════

pub fn agricultural_yield_optimization() -> ExperimentDefinition {
    ExperimentDefinition::builder("Agricultural Yield Optimization")
        .description(
            "Optimize crop selection, planting schedules, irrigation, and \
             fertilization to maximize yield while minimizing water usage \
             and environmental impact. Discover farming strategies that \
             account for soil variability and weather uncertainty."
        )
        .problem_type(ProblemType::CombinatorialOptimization {
            maximize: true,
            objective: "Maximize total yield × sustainability score".to_string(),
        })
        .encoding(StateSpaceEncoding::Integer {
            num_vars: 150,
            domains: vec![(0, 20); 150],
        })
        .strategy(SolverStrategy::Hybrid {
            strategies: vec![
                SolverStrategy::FitnessSearch { num_candidates: 512 },
                SolverStrategy::Evolution { steps: 300 },
            ],
        })
        .schema(
            InputSchema::new("agricultural_optimization", "Agricultural yield optimization inputs")
                .field(InputField::required_int("num_fields",
                    "Number of agricultural fields/parcels").with_bounds(1.0, 100000.0))
                .field(InputField::required_json("field_data",
                    "Per-field data [{area_ha, soil_type, soil_ph, organic_matter_pct, drainage_class}]"))
                .field(InputField::required_json("crop_options",
                    "Available crops [{name, water_need_mm, nutrient_need, growing_days, yield_per_ha, price_per_ton}]"))
                .field(InputField::required_float_array("monthly_rainfall",
                    "Monthly rainfall forecast (12 months)").with_unit("mm"))
                .field(InputField::required_float_array("monthly_temperature",
                    "Monthly average temperature forecast (12 months)").with_unit("°C"))
                .field(InputField::required_float("total_water_budget",
                    "Total irrigation water available").with_unit("m³"))
                .field(InputField::required_float("fertilizer_budget",
                    "Total fertilizer budget").with_unit("USD"))
                .field(InputField::required_json("rotation_constraints",
                    "Crop rotation constraints (which crops can follow which)"))
        )
        .success_criteria(SuccessCriteria::BeatBaseline {
            baseline: 0.0,
            description: "Beat monoculture baseline in total yield × sustainability".to_string(),
        })
        .tag("agriculture")
        .tag("sustainability")
        .reference("Guitard, R. (2026). MDB — Critical Global Problems")
        .build()
        .expect("agricultural_optimization definition")
}
