/* @ts-self-types="./mdb_wasm.d.ts" */

/**
 * A quantum circuit — declarative gate composition.
 */
export class WasmCircuit {
    static __wrap(ptr) {
        const obj = Object.create(WasmCircuit.prototype);
        obj.__wbg_ptr = ptr;
        WasmCircuitFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmCircuitFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmcircuit_free(ptr, 0);
    }
    /**
     * ASCII visualization of the circuit.
     * @returns {string}
     */
    ascii() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmcircuit_ascii(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Create a Bell pair circuit.
     * @param {number} n
     * @returns {WasmCircuit}
     */
    static bell(n) {
        const ret = wasm.wasmcircuit_bell(n);
        return WasmCircuit.__wrap(ret);
    }
    /**
     * @param {number} control
     * @param {number} target
     * @returns {WasmCircuit}
     */
    cnot(control, target) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.wasmcircuit_cnot(ptr, control, target);
        return WasmCircuit.__wrap(ret);
    }
    /**
     * @param {number} a
     * @param {number} b
     * @returns {WasmCircuit}
     */
    cz(a, b) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.wasmcircuit_cz(ptr, a, b);
        return WasmCircuit.__wrap(ret);
    }
    /**
     * Circuit depth.
     * @returns {number}
     */
    depth() {
        const ret = wasm.wasmcircuit_depth(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Gate count.
     * @returns {number}
     */
    gateCount() {
        const ret = wasm.wasmcircuit_gateCount(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Create a GHZ state circuit.
     * @param {number} n
     * @returns {WasmCircuit}
     */
    static ghz(n) {
        const ret = wasm.wasmcircuit_ghz(n);
        return WasmCircuit.__wrap(ret);
    }
    /**
     * @param {number} k
     * @returns {WasmCircuit}
     */
    h(k) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.wasmcircuit_h(ptr, k);
        return WasmCircuit.__wrap(ret);
    }
    /**
     * @returns {WasmCircuit}
     */
    hAll() {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.wasmcircuit_hAll(ptr);
        return WasmCircuit.__wrap(ret);
    }
    /**
     * @returns {WasmCircuit}
     */
    measureAll() {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.wasmcircuit_measureAll(ptr);
        return WasmCircuit.__wrap(ret);
    }
    /**
     * Create an empty n-qubit circuit.
     * @param {number} n
     * @param {string} name
     */
    constructor(n, name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmcircuit_new(n, ptr0, len0);
        this.__wbg_ptr = ret;
        WasmCircuitFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Create a QFT circuit.
     * @param {number} n
     * @returns {WasmCircuit}
     */
    static qft(n) {
        const ret = wasm.wasmcircuit_qft(n);
        return WasmCircuit.__wrap(ret);
    }
    /**
     * Execute the circuit and return results (JSON).
     * @returns {string}
     */
    run() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ptr = this.__destroy_into_raw();
            const ret = wasm.wasmcircuit_run(ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {number} k
     * @returns {WasmCircuit}
     */
    sGate(k) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.wasmcircuit_sGate(ptr, k);
        return WasmCircuit.__wrap(ret);
    }
    /**
     * @param {number} a
     * @param {number} b
     * @returns {WasmCircuit}
     */
    swap(a, b) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.wasmcircuit_swap(ptr, a, b);
        return WasmCircuit.__wrap(ret);
    }
    /**
     * @param {number} k
     * @returns {WasmCircuit}
     */
    tGate(k) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.wasmcircuit_tGate(ptr, k);
        return WasmCircuit.__wrap(ret);
    }
    /**
     * @param {number} c1
     * @param {number} c2
     * @param {number} target
     * @returns {WasmCircuit}
     */
    toffoli(c1, c2, target) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.wasmcircuit_toffoli(ptr, c1, c2, target);
        return WasmCircuit.__wrap(ret);
    }
    /**
     * @param {number} k
     * @returns {WasmCircuit}
     */
    x(k) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.wasmcircuit_x(ptr, k);
        return WasmCircuit.__wrap(ret);
    }
    /**
     * @param {number} k
     * @returns {WasmCircuit}
     */
    y(k) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.wasmcircuit_y(ptr, k);
        return WasmCircuit.__wrap(ret);
    }
    /**
     * @param {number} k
     * @returns {WasmCircuit}
     */
    z(k) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.wasmcircuit_z(ptr, k);
        return WasmCircuit.__wrap(ret);
    }
}
if (Symbol.dispose) WasmCircuit.prototype[Symbol.dispose] = WasmCircuit.prototype.free;

/**
 * A quantum register — full statevector simulator.
 */
export class WasmRegister {
    static __wrap(ptr) {
        const obj = Object.create(WasmRegister.prototype);
        obj.__wbg_ptr = ptr;
        WasmRegisterFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmRegisterFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmregister_free(ptr, 0);
    }
    /**
     * @param {number} control
     * @param {number} target
     */
    cnot(control, target) {
        wasm.wasmregister_cnot(this.__wbg_ptr, control, target);
    }
    /**
     * @param {number} control
     * @param {number} target
     * @param {number} theta
     */
    controlledPhase(control, target, theta) {
        wasm.wasmregister_controlledPhase(this.__wbg_ptr, control, target, theta);
    }
    /**
     * @param {number} a
     * @param {number} b
     */
    cz(a, b) {
        wasm.wasmregister_cz(this.__wbg_ptr, a, b);
    }
    /**
     * Dimension (2^n).
     * @returns {number}
     */
    dim() {
        const ret = wasm.wasmregister_dim(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Fidelity between two registers.
     * @param {WasmRegister} other
     * @returns {number}
     */
    fidelity(other) {
        _assertClass(other, WasmRegister);
        const ret = wasm.wasmregister_fidelity(this.__wbg_ptr, other.__wbg_ptr);
        return ret;
    }
    /**
     * Fork (clone) the register. Non-destructive — another MDB advantage.
     * @returns {WasmRegister}
     */
    fork() {
        const ret = wasm.wasmregister_fork(this.__wbg_ptr);
        return WasmRegister.__wrap(ret);
    }
    /**
     * @param {number} control
     * @param {number} t1
     * @param {number} t2
     */
    fredkin(control, t1, t2) {
        wasm.wasmregister_fredkin(this.__wbg_ptr, control, t1, t2);
    }
    /**
     * Create from integer value.
     * @param {number} n
     * @param {number} value
     * @param {string} name
     * @returns {WasmRegister}
     */
    static fromInt(n, value, name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmregister_fromInt(n, value, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmRegister.__wrap(ret[0]);
    }
    /**
     * @param {number} k
     */
    hadamard(k) {
        wasm.wasmregister_hadamard(this.__wbg_ptr, k);
    }
    /**
     * @param {Uint32Array} positions
     */
    inverseQft(positions) {
        const ptr0 = passArray32ToWasm0(positions, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmregister_inverseQft(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * Destructive measurement of all qubits.
     * @param {bigint} seed
     * @returns {string}
     */
    measureAll(seed) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmregister_measureAll(this.__wbg_ptr, seed);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Number of qubits.
     * @returns {number}
     */
    get n() {
        const ret = wasm.wasmregister_n(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Create an n-qubit register initialized to |0...0⟩.
     * @param {number} n
     * @param {string} name
     */
    constructor(n, name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmregister_new(n, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        WasmRegisterFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @param {number} k
     */
    pauliX(k) {
        wasm.wasmregister_pauliX(this.__wbg_ptr, k);
    }
    /**
     * @param {number} k
     */
    pauliY(k) {
        wasm.wasmregister_pauliY(this.__wbg_ptr, k);
    }
    /**
     * @param {number} k
     */
    pauliZ(k) {
        wasm.wasmregister_pauliZ(this.__wbg_ptr, k);
    }
    /**
     * Non-destructive peek at the full state (JSON). MDB advantage.
     * @returns {string}
     */
    peek() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmregister_peek(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {number} k
     * @param {number} theta
     */
    phase(k, theta) {
        wasm.wasmregister_phase(this.__wbg_ptr, k, theta);
    }
    /**
     * Get probability distribution as a flat array.
     * @returns {Float64Array}
     */
    probabilities() {
        const ret = wasm.wasmregister_probabilities(this.__wbg_ptr);
        var v1 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
        return v1;
    }
    /**
     * @param {Uint32Array} positions
     */
    qft(positions) {
        const ptr0 = passArray32ToWasm0(positions, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmregister_qft(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * Reset to |0...0⟩.
     */
    reset() {
        wasm.wasmregister_reset(this.__wbg_ptr);
    }
    /**
     * @param {number} k
     * @param {number} theta
     */
    rx(k, theta) {
        wasm.wasmregister_rx(this.__wbg_ptr, k, theta);
    }
    /**
     * @param {number} k
     * @param {number} theta
     */
    ry(k, theta) {
        wasm.wasmregister_ry(this.__wbg_ptr, k, theta);
    }
    /**
     * @param {number} k
     * @param {number} theta
     */
    rz(k, theta) {
        wasm.wasmregister_rz(this.__wbg_ptr, k, theta);
    }
    /**
     * @param {number} k
     */
    sGate(k) {
        wasm.wasmregister_sGate(this.__wbg_ptr, k);
    }
    /**
     * Non-destructive sample (doesn't collapse state).
     * @param {bigint} seed
     * @returns {string}
     */
    sampleAll(seed) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmregister_sampleAll(this.__wbg_ptr, seed);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {number} a
     * @param {number} b
     */
    swap(a, b) {
        wasm.wasmregister_swap(this.__wbg_ptr, a, b);
    }
    /**
     * @param {number} k
     */
    tGate(k) {
        wasm.wasmregister_tGate(this.__wbg_ptr, k);
    }
    /**
     * @param {number} c1
     * @param {number} c2
     * @param {number} target
     */
    toffoli(c1, c2, target) {
        wasm.wasmregister_toffoli(this.__wbg_ptr, c1, c2, target);
    }
}
if (Symbol.dispose) WasmRegister.prototype[Symbol.dispose] = WasmRegister.prototype.free;

/**
 * A sparse quantum register — stores only populated basis states.
 * No position limit.  Memory = O(populated states).
 */
export class WasmSparseRegister {
    static __wrap(ptr) {
        const obj = Object.create(WasmSparseRegister.prototype);
        obj.__wbg_ptr = ptr;
        WasmSparseRegisterFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmSparseRegisterFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmsparseregister_free(ptr, 0);
    }
    /**
     * Auto-prune using the register's default threshold.
     * @returns {number}
     */
    autoPrune() {
        const ret = wasm.wasmsparseregister_autoPrune(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get the dimensional cascade snapshot — maps each populated state
     * through the Fibonacci coordinate system.
     * @returns {string}
     */
    cascadeSnapshot() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsparseregister_cascadeSnapshot(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {number} control
     * @param {number} target
     */
    cnot(control, target) {
        wasm.wasmsparseregister_cnot(this.__wbg_ptr, control, target);
    }
    /**
     * @param {number} control
     * @param {number} target
     * @param {number} theta
     */
    controlledPhase(control, target, theta) {
        wasm.wasmsparseregister_controlledPhase(this.__wbg_ptr, control, target, theta);
    }
    /**
     * @param {number} a
     * @param {number} b
     */
    cz(a, b) {
        wasm.wasmsparseregister_cz(this.__wbg_ptr, a, b);
    }
    /**
     * Fidelity (overlap) between this register and another.
     * Returns |⟨ψ|φ⟩|² — 1.0 means identical states, 0.0 means orthogonal.
     * @param {WasmSparseRegister} other
     * @returns {number}
     */
    fidelity(other) {
        _assertClass(other, WasmSparseRegister);
        const ret = wasm.wasmsparseregister_fidelity(this.__wbg_ptr, other.__wbg_ptr);
        return ret;
    }
    /**
     * fork() — Independent deep copy of the register.
     * Impossible on quantum hardware (violates no-cloning theorem).
     * Modifying the fork cannot affect the original.
     * @returns {WasmSparseRegister}
     */
    fork() {
        const ret = wasm.wasmsparseregister_fork(this.__wbg_ptr);
        return WasmSparseRegister.__wrap(ret);
    }
    /**
     * @param {number} control
     * @param {number} t1
     * @param {number} t2
     */
    fredkin(control, t1, t2) {
        wasm.wasmsparseregister_fredkin(this.__wbg_ptr, control, t1, t2);
    }
    /**
     * Create from a bit string (e.g. [1, 0, 1, 1]).
     * @param {Uint8Array} bits
     * @param {string} name
     * @returns {WasmSparseRegister}
     */
    static fromBits(bits, name) {
        const ptr0 = passArray8ToWasm0(bits, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsparseregister_fromBits(ptr0, len0, ptr1, len1);
        return WasmSparseRegister.__wrap(ret);
    }
    /**
     * Create from an integer basis state |value⟩.
     * @param {number} n
     * @param {bigint} value
     * @param {string} name
     * @returns {WasmSparseRegister}
     */
    static fromInt(n, value, name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsparseregister_fromInt(n, value, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmSparseRegister.__wrap(ret[0]);
    }
    /**
     * Grover diffusion operator.
     */
    groverDiffusion() {
        wasm.wasmsparseregister_groverDiffusion(this.__wbg_ptr);
    }
    /**
     * @param {number} k
     */
    hadamard(k) {
        wasm.wasmsparseregister_hadamard(this.__wbg_ptr, k);
    }
    /**
     * Full Hilbert space dimension 2^n (as string — can exceed u64).
     * @returns {string}
     */
    hilbertDim() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsparseregister_hilbertDim(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {Uint32Array} positions
     */
    inverseQft(positions) {
        const ptr0 = passArray32ToWasm0(positions, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmsparseregister_inverseQft(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * Destructive measurement of all positions.
     * @param {bigint} seed
     * @returns {string}
     */
    measureAll(seed) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsparseregister_measureAll(this.__wbg_ptr, seed);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Measure a single position (partial collapse).
     * @param {number} k
     * @param {bigint} seed
     * @returns {string}
     */
    measurePosition(k, seed) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsparseregister_measurePosition(this.__wbg_ptr, k, seed);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Memory usage in bytes.
     * @returns {number}
     */
    memoryBytes() {
        const ret = wasm.wasmsparseregister_memoryBytes(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Number of positions (qubits).
     * @returns {number}
     */
    get n() {
        const ret = wasm.wasmsparseregister_n(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Create an n-position sparse register initialized to |0…0⟩.
     * No upper limit on n — memory scales with populated states, not 2^n.
     * @param {number} n
     * @param {string} name
     */
    constructor(n, name) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsparseregister_new(n, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        WasmSparseRegisterFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Normalize amplitudes so total probability = 1.
     */
    normalize() {
        wasm.wasmsparseregister_normalize(this.__wbg_ptr);
    }
    /**
     * @param {number} k
     */
    pauliX(k) {
        wasm.wasmsparseregister_pauliX(this.__wbg_ptr, k);
    }
    /**
     * @param {number} k
     */
    pauliY(k) {
        wasm.wasmsparseregister_pauliY(this.__wbg_ptr, k);
    }
    /**
     * @param {number} k
     */
    pauliZ(k) {
        wasm.wasmsparseregister_pauliZ(this.__wbg_ptr, k);
    }
    /**
     * peek() — Non-destructive full state readout.
     * Impossible on quantum hardware (violates projection postulate).
     * Returns JSON with all populated states, probabilities, and phases.
     * @returns {string}
     */
    peek() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsparseregister_peek(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {number} k
     * @param {number} theta
     */
    phase(k, theta) {
        wasm.wasmsparseregister_phase(this.__wbg_ptr, k, theta);
    }
    /**
     * Number of currently populated basis states.
     * @returns {number}
     */
    population() {
        const ret = wasm.wasmsparseregister_population(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Probability distribution as JSON (only populated states).
     * @returns {string}
     */
    probabilities() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsparseregister_probabilities(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Prune states below threshold. Returns number of states removed.
     * @param {number} threshold
     * @returns {number}
     */
    prune(threshold) {
        const ret = wasm.wasmsparseregister_prune(this.__wbg_ptr, threshold);
        return ret >>> 0;
    }
    /**
     * @param {Uint32Array} positions
     */
    qft(positions) {
        const ptr0 = passArray32ToWasm0(positions, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmsparseregister_qft(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * Reset to |0…0⟩.
     */
    reset() {
        wasm.wasmsparseregister_reset(this.__wbg_ptr);
    }
    /**
     * @param {number} k
     * @param {number} theta
     */
    rx(k, theta) {
        wasm.wasmsparseregister_rx(this.__wbg_ptr, k, theta);
    }
    /**
     * @param {number} k
     * @param {number} theta
     */
    ry(k, theta) {
        wasm.wasmsparseregister_ry(this.__wbg_ptr, k, theta);
    }
    /**
     * @param {number} k
     * @param {number} theta
     */
    rz(k, theta) {
        wasm.wasmsparseregister_rz(this.__wbg_ptr, k, theta);
    }
    /**
     * @param {number} k
     */
    sGate(k) {
        wasm.wasmsparseregister_sGate(this.__wbg_ptr, k);
    }
    /**
     * Non-destructive sample — picks one outcome without collapsing state.
     * @param {bigint} seed
     * @returns {string}
     */
    sampleAll(seed) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsparseregister_sampleAll(this.__wbg_ptr, seed);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Sparsity ratio: populated / 2^n.
     * @returns {number}
     */
    sparsity() {
        const ret = wasm.wasmsparseregister_sparsity(this.__wbg_ptr);
        return ret;
    }
    /**
     * @param {number} a
     * @param {number} b
     */
    swap(a, b) {
        wasm.wasmsparseregister_swap(this.__wbg_ptr, a, b);
    }
    /**
     * @param {number} k
     */
    tGate(k) {
        wasm.wasmsparseregister_tGate(this.__wbg_ptr, k);
    }
    /**
     * @param {number} c1
     * @param {number} c2
     * @param {number} target
     */
    toffoli(c1, c2, target) {
        wasm.wasmsparseregister_toffoli(this.__wbg_ptr, c1, c2, target);
    }
    /**
     * Total probability (should be ≈ 1.0 for normalized state).
     * @returns {number}
     */
    totalProbability() {
        const ret = wasm.wasmsparseregister_totalProbability(this.__wbg_ptr);
        return ret;
    }
}
if (Symbol.dispose) WasmSparseRegister.prototype[Symbol.dispose] = WasmSparseRegister.prototype.free;

/**
 * A SuperBit — multidimensional binary data in superposition.
 */
export class WasmSuperBit {
    static __wrap(ptr) {
        const obj = Object.create(WasmSuperBit.prototype);
        obj.__wbg_ptr = ptr;
        WasmSuperBitFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmSuperBitFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmsuperbit_free(ptr, 0);
    }
    /**
     * Add a new state to the superposition.
     * @param {Uint8Array} bits
     * @param {string} label
     * @param {number} weight
     */
    addState(bits, label, weight) {
        const ptr0 = passArray8ToWasm0(bits, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(label, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        wasm.wasmsuperbit_addState(this.__wbg_ptr, ptr0, len0, ptr1, len1, weight);
    }
    /**
     * Get the dimensional address (JSON).
     * @returns {string}
     */
    address() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsuperbit_address(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Number of bits.
     * @returns {number}
     */
    bitLength() {
        const ret = wasm.wasmsuperbit_bitLength(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Compute the dimensional cascade up to max_dim (JSON array of dimension vectors).
     * @param {number} max_dim
     * @returns {string}
     */
    cascade(max_dim) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsuperbit_cascade(this.__wbg_ptr, max_dim);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Collapse to a specific state by index. Returns JSON of the collapsed state.
     * @param {number} index
     * @returns {string}
     */
    collapseTo(index) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsuperbit_collapseTo(this.__wbg_ptr, index);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Deserialize a SuperBit from bytes.
     * @param {Uint8Array} data
     * @returns {WasmSuperBit}
     */
    static decode(data) {
        const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsuperbit_decode(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return WasmSuperBit.__wrap(ret[0]);
    }
    /**
     * Serialize the SuperBit to bytes.
     * @returns {Uint8Array}
     */
    encode() {
        const ret = wasm.wasmsuperbit_encode(this.__wbg_ptr);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * Fork (clone) the SuperBit.
     * @returns {WasmSuperBit}
     */
    fork() {
        const ret = wasm.wasmsuperbit_fork(this.__wbg_ptr);
        return WasmSuperBit.__wrap(ret);
    }
    /**
     * Create a SuperBit from a binary string like "10110".
     * @param {string} s
     * @returns {WasmSuperBit}
     */
    static fromString(s) {
        const ptr0 = passStringToWasm0(s, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsuperbit_fromString(ptr0, len0);
        return WasmSuperBit.__wrap(ret);
    }
    /**
     * Create a SuperBit from a bit array (e.g., [1, 0, 1, 1]).
     * @param {Uint8Array} bits
     */
    constructor(bits) {
        const ptr0 = passArray8ToWasm0(bits, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsuperbit_new(ptr0, len0);
        this.__wbg_ptr = ret;
        WasmSuperBitFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Non-destructive peek at all superposition states (JSON).
     * @returns {string}
     */
    peek() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsuperbit_peek(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Number of superposition states.
     * @returns {number}
     */
    stateCount() {
        const ret = wasm.wasmsuperbit_stateCount(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get distances between all superposition states (JSON).
     * @returns {string}
     */
    stateDistances() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsuperbit_stateDistances(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}
if (Symbol.dispose) WasmSuperBit.prototype[Symbol.dispose] = WasmSuperBit.prototype.free;

/**
 * Deutsch-Jozsa algorithm on a parity function. Returns "Constant" or "Balanced".
 * @param {number} n_qubits
 * @returns {string}
 */
export function deutschJozsa(n_qubits) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.deutschJozsa(n_qubits);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Compute a dimensional cascade for a binary string. Returns JSON.
 * @param {string} bits_str
 * @param {number} max_dim
 * @returns {string}
 */
export function dimensionalCascade(bits_str, max_dim) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(bits_str, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.dimensionalCascade(ptr0, len0, max_dim);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Grover's search. Finds target in 2^n_qubits search space. Returns JSON.
 * @param {number} n_qubits
 * @param {number} target
 * @returns {string}
 */
export function grover(n_qubits, target) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.grover(n_qubits, target);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Run the benchmark suite and return a text report.
 * @returns {string}
 */
export function runBenchmarks() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.runBenchmarks();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Shor's factoring algorithm. Returns JSON with factors, or null if can't factor.
 * @param {bigint} n
 * @returns {string}
 */
export function shor(n) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.shor(n);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Quantum teleportation. Teleports state alpha|0⟩ + beta|1⟩.
 * Takes alpha_re, alpha_im, beta_re, beta_im.
 * @param {number} alpha_re
 * @param {number} alpha_im
 * @param {number} beta_re
 * @param {number} beta_im
 * @param {bigint} seed
 * @returns {string}
 */
export function teleport(alpha_re, alpha_im, beta_re, beta_im, seed) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.teleport(alpha_re, alpha_im, beta_re, beta_im, seed);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Returns the MDB-OS version string.
 * @returns {string}
 */
export function version() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.version();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_Error_ef53bc310eb298a0: function(arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg___wbindgen_throw_1506f2235d1bdba0: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./mdb_wasm_bg.js": import0,
    };
}

const WasmCircuitFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmcircuit_free(ptr, 1));
const WasmRegisterFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmregister_free(ptr, 1));
const WasmSparseRegisterFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmsparseregister_free(ptr, 1));
const WasmSuperBitFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmsuperbit_free(ptr, 1));

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

function getArrayF64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedFloat64ArrayMemory0 = null;
function getFloat64ArrayMemory0() {
    if (cachedFloat64ArrayMemory0 === null || cachedFloat64ArrayMemory0.byteLength === 0) {
        cachedFloat64ArrayMemory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function passArray32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getUint32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedFloat64ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('mdb_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
