/* tslint:disable */
/* eslint-disable */

/**
 * A quantum circuit — declarative gate composition.
 */
export class WasmCircuit {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * ASCII visualization of the circuit.
     */
    ascii(): string;
    /**
     * Create a Bell pair circuit.
     */
    static bell(n: number): WasmCircuit;
    cnot(control: number, target: number): WasmCircuit;
    cz(a: number, b: number): WasmCircuit;
    /**
     * Circuit depth.
     */
    depth(): number;
    /**
     * Gate count.
     */
    gateCount(): number;
    /**
     * Create a GHZ state circuit.
     */
    static ghz(n: number): WasmCircuit;
    h(k: number): WasmCircuit;
    hAll(): WasmCircuit;
    measureAll(): WasmCircuit;
    /**
     * Create an empty n-qubit circuit.
     */
    constructor(n: number, name: string);
    /**
     * Create a QFT circuit.
     */
    static qft(n: number): WasmCircuit;
    /**
     * Execute the circuit and return results (JSON).
     */
    run(): string;
    sGate(k: number): WasmCircuit;
    swap(a: number, b: number): WasmCircuit;
    tGate(k: number): WasmCircuit;
    toffoli(c1: number, c2: number, target: number): WasmCircuit;
    x(k: number): WasmCircuit;
    y(k: number): WasmCircuit;
    z(k: number): WasmCircuit;
}

/**
 * A quantum register — full statevector simulator.
 */
export class WasmRegister {
    free(): void;
    [Symbol.dispose](): void;
    cnot(control: number, target: number): void;
    controlledPhase(control: number, target: number, theta: number): void;
    cz(a: number, b: number): void;
    /**
     * Dimension (2^n).
     */
    dim(): number;
    /**
     * Fidelity between two registers.
     */
    fidelity(other: WasmRegister): number;
    /**
     * Fork (clone) the register. Non-destructive — another MDB advantage.
     */
    fork(): WasmRegister;
    fredkin(control: number, t1: number, t2: number): void;
    /**
     * Create from integer value.
     */
    static fromInt(n: number, value: number, name: string): WasmRegister;
    hadamard(k: number): void;
    inverseQft(positions: Uint32Array): void;
    /**
     * Destructive measurement of all qubits.
     */
    measureAll(seed: bigint): string;
    /**
     * Create an n-qubit register initialized to |0...0⟩.
     */
    constructor(n: number, name: string);
    pauliX(k: number): void;
    pauliY(k: number): void;
    pauliZ(k: number): void;
    /**
     * Non-destructive peek at the full state (JSON). MDB advantage.
     */
    peek(): string;
    phase(k: number, theta: number): void;
    /**
     * Get probability distribution as a flat array.
     */
    probabilities(): Float64Array;
    qft(positions: Uint32Array): void;
    /**
     * Reset to |0...0⟩.
     */
    reset(): void;
    rx(k: number, theta: number): void;
    ry(k: number, theta: number): void;
    rz(k: number, theta: number): void;
    sGate(k: number): void;
    /**
     * Non-destructive sample (doesn't collapse state).
     */
    sampleAll(seed: bigint): string;
    swap(a: number, b: number): void;
    tGate(k: number): void;
    toffoli(c1: number, c2: number, target: number): void;
    /**
     * Number of qubits.
     */
    readonly n: number;
}

/**
 * A SuperBit — multidimensional binary data in superposition.
 */
export class WasmSuperBit {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Add a new state to the superposition.
     */
    addState(bits: Uint8Array, label: string, weight: number): void;
    /**
     * Get the dimensional address (JSON).
     */
    address(): string;
    /**
     * Number of bits.
     */
    bitLength(): number;
    /**
     * Compute the dimensional cascade up to max_dim (JSON array of dimension vectors).
     */
    cascade(max_dim: number): string;
    /**
     * Collapse to a specific state by index. Returns JSON of the collapsed state.
     */
    collapseTo(index: number): string;
    /**
     * Deserialize a SuperBit from bytes.
     */
    static decode(data: Uint8Array): WasmSuperBit;
    /**
     * Serialize the SuperBit to bytes.
     */
    encode(): Uint8Array;
    /**
     * Fork (clone) the SuperBit.
     */
    fork(): WasmSuperBit;
    /**
     * Create a SuperBit from a binary string like "10110".
     */
    static fromString(s: string): WasmSuperBit;
    /**
     * Create a SuperBit from a bit array (e.g., [1, 0, 1, 1]).
     */
    constructor(bits: Uint8Array);
    /**
     * Non-destructive peek at all superposition states (JSON).
     */
    peek(): string;
    /**
     * Number of superposition states.
     */
    stateCount(): number;
    /**
     * Get distances between all superposition states (JSON).
     */
    stateDistances(): string;
}

/**
 * Deutsch-Jozsa algorithm on a parity function. Returns "Constant" or "Balanced".
 */
export function deutschJozsa(n_qubits: number): string;

/**
 * Compute a dimensional cascade for a binary string. Returns JSON.
 */
export function dimensionalCascade(bits_str: string, max_dim: number): string;

/**
 * Grover's search. Finds target in 2^n_qubits search space. Returns JSON.
 */
export function grover(n_qubits: number, target: number): string;

/**
 * Run the benchmark suite and return a text report.
 */
export function runBenchmarks(): string;

/**
 * Shor's factoring algorithm. Returns JSON with factors, or null if can't factor.
 */
export function shor(n: bigint): string;

/**
 * Quantum teleportation. Teleports state alpha|0⟩ + beta|1⟩.
 * Takes alpha_re, alpha_im, beta_re, beta_im.
 */
export function teleport(alpha_re: number, alpha_im: number, beta_re: number, beta_im: number, seed: bigint): string;

/**
 * Returns the MDB-OS version string.
 */
export function version(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmcircuit_free: (a: number, b: number) => void;
    readonly __wbg_wasmregister_free: (a: number, b: number) => void;
    readonly __wbg_wasmsuperbit_free: (a: number, b: number) => void;
    readonly deutschJozsa: (a: number) => [number, number];
    readonly dimensionalCascade: (a: number, b: number, c: number) => [number, number];
    readonly grover: (a: number, b: number) => [number, number];
    readonly runBenchmarks: () => [number, number];
    readonly shor: (a: bigint) => [number, number];
    readonly teleport: (a: number, b: number, c: number, d: number, e: bigint) => [number, number];
    readonly version: () => [number, number];
    readonly wasmcircuit_ascii: (a: number) => [number, number];
    readonly wasmcircuit_bell: (a: number) => number;
    readonly wasmcircuit_cnot: (a: number, b: number, c: number) => number;
    readonly wasmcircuit_cz: (a: number, b: number, c: number) => number;
    readonly wasmcircuit_depth: (a: number) => number;
    readonly wasmcircuit_gateCount: (a: number) => number;
    readonly wasmcircuit_ghz: (a: number) => number;
    readonly wasmcircuit_h: (a: number, b: number) => number;
    readonly wasmcircuit_hAll: (a: number) => number;
    readonly wasmcircuit_measureAll: (a: number) => number;
    readonly wasmcircuit_new: (a: number, b: number, c: number) => number;
    readonly wasmcircuit_qft: (a: number) => number;
    readonly wasmcircuit_run: (a: number) => [number, number];
    readonly wasmcircuit_sGate: (a: number, b: number) => number;
    readonly wasmcircuit_swap: (a: number, b: number, c: number) => number;
    readonly wasmcircuit_tGate: (a: number, b: number) => number;
    readonly wasmcircuit_toffoli: (a: number, b: number, c: number, d: number) => number;
    readonly wasmcircuit_x: (a: number, b: number) => number;
    readonly wasmcircuit_y: (a: number, b: number) => number;
    readonly wasmcircuit_z: (a: number, b: number) => number;
    readonly wasmregister_cnot: (a: number, b: number, c: number) => void;
    readonly wasmregister_controlledPhase: (a: number, b: number, c: number, d: number) => void;
    readonly wasmregister_cz: (a: number, b: number, c: number) => void;
    readonly wasmregister_dim: (a: number) => number;
    readonly wasmregister_fidelity: (a: number, b: number) => number;
    readonly wasmregister_fork: (a: number) => number;
    readonly wasmregister_fredkin: (a: number, b: number, c: number, d: number) => void;
    readonly wasmregister_fromInt: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly wasmregister_hadamard: (a: number, b: number) => void;
    readonly wasmregister_inverseQft: (a: number, b: number, c: number) => void;
    readonly wasmregister_measureAll: (a: number, b: bigint) => [number, number];
    readonly wasmregister_n: (a: number) => number;
    readonly wasmregister_new: (a: number, b: number, c: number) => [number, number, number];
    readonly wasmregister_pauliX: (a: number, b: number) => void;
    readonly wasmregister_pauliY: (a: number, b: number) => void;
    readonly wasmregister_pauliZ: (a: number, b: number) => void;
    readonly wasmregister_peek: (a: number) => [number, number];
    readonly wasmregister_phase: (a: number, b: number, c: number) => void;
    readonly wasmregister_probabilities: (a: number) => [number, number];
    readonly wasmregister_qft: (a: number, b: number, c: number) => void;
    readonly wasmregister_reset: (a: number) => void;
    readonly wasmregister_rx: (a: number, b: number, c: number) => void;
    readonly wasmregister_ry: (a: number, b: number, c: number) => void;
    readonly wasmregister_rz: (a: number, b: number, c: number) => void;
    readonly wasmregister_sGate: (a: number, b: number) => void;
    readonly wasmregister_sampleAll: (a: number, b: bigint) => [number, number];
    readonly wasmregister_swap: (a: number, b: number, c: number) => void;
    readonly wasmregister_tGate: (a: number, b: number) => void;
    readonly wasmregister_toffoli: (a: number, b: number, c: number, d: number) => void;
    readonly wasmsuperbit_addState: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly wasmsuperbit_address: (a: number) => [number, number];
    readonly wasmsuperbit_bitLength: (a: number) => number;
    readonly wasmsuperbit_cascade: (a: number, b: number) => [number, number];
    readonly wasmsuperbit_collapseTo: (a: number, b: number) => [number, number];
    readonly wasmsuperbit_decode: (a: number, b: number) => [number, number, number];
    readonly wasmsuperbit_encode: (a: number) => [number, number];
    readonly wasmsuperbit_fork: (a: number) => number;
    readonly wasmsuperbit_fromString: (a: number, b: number) => number;
    readonly wasmsuperbit_new: (a: number, b: number) => number;
    readonly wasmsuperbit_peek: (a: number) => [number, number];
    readonly wasmsuperbit_stateCount: (a: number) => number;
    readonly wasmsuperbit_stateDistances: (a: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
