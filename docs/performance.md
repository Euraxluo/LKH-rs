# Performance Notes

The current high-cost optimization work is still performed by the vendored LKH C core. LKH-rs focuses on safe integration, result extraction, and packaging foundations.

## Baseline

A lightweight baseline target exists in `benches/solve.rs` and can be run explicitly:

```bash
cargo bench --bench solve -- --ignored
```

The benchmark is intentionally serial because LKH uses process-global mutable state.

## Parallelism

Do not call the in-process solver concurrently from multiple threads. LKH-rs serializes safe API calls with a global mutex to protect LKH's global state.

For throughput across many independent problems, use process-level parallelism: spawn multiple worker processes, each with its own LKH global state.

## Future optimization candidates

- Rust-side parameter prevalidation before entering C.
- Rust tour parsing and serialization.
- More compact and explicit `SolveReport` extraction.
- Gradual Rust rewrites of isolated pure-computation pieces once their inputs/outputs are separated from C globals.
- Process-pool orchestration for many independent solves.
