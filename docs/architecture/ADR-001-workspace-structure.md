# ADR-001: Cargo Workspace Structure

## Status

Accepted

## Context

The Mycelial P2P Bootstrap project requires a modular architecture that separates concerns while allowing code reuse. We need to support both native (desktop/server) and browser (WASM) targets.

## Decision

We will use a Cargo workspace with the following crates:

1. **mycelial-core**: Foundational types and traits (no I/O dependencies)
2. **mycelial-network**: libp2p networking (server-side only)
3. **mycelial-protocol**: Message definitions and serialization
4. **mycelial-state**: Persistence and state management
5. **mycelial-wasm**: Browser WASM bridge
6. **mycelial-node**: Main executable combining all components

### Dependency Graph

```
mycelial-core (no deps)
     │
     ├─────────────────┬─────────────────┐
     ▼                 ▼                 ▼
mycelial-protocol  mycelial-network  mycelial-wasm
     │                 │                 │
     └────────┬────────┘                 │
              ▼                          │
        mycelial-state                   │
              │                          │
              └──────────┬───────────────┘
                         ▼
                   mycelial-node
```

### Key Principles

1. **Core is dependency-free**: `mycelial-core` has no async runtime or I/O dependencies, enabling WASM compilation
2. **Protocol is pure data**: `mycelial-protocol` only handles serialization, no networking
3. **Network is server-only**: `mycelial-network` uses libp2p, not compiled for WASM
4. **WASM is minimal**: `mycelial-wasm` exposes only what the browser needs

## Consequences

### Positive

- Clear separation of concerns
- Easy to test individual components
- WASM bundle size is minimized
- Native performance for server components

### Negative

- More boilerplate for cross-crate type usage
- Version coordination required across crates
- Some code duplication between native and WASM networking

## References

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [libp2p Rust Documentation](https://docs.rs/libp2p)
- [wasm-bindgen Guide](https://rustwasm.github.io/wasm-bindgen/)
