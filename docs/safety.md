# Safety Notes

LKH-rs wraps the LKH3 C solver. The default Rust API is safe to call from Rust, but it is built on top of an upstream C library with several important process-level constraints.

## Global state

LKH stores solver state in process-global mutable variables such as the current parameter file, problem, node set, best tour, costs, and run counters. LKH-rs serializes calls to `solve_parameter_file` and `solve_with_options` with a global mutex so callers do not enter the C solver concurrently.

Do not assume the underlying solver is reentrant or thread-safe. If you need to solve many independent instances in parallel, prefer multiple processes over multiple threads in one process.

## Error handling

The safe Rust layer validates the parameter file path before calling C and returns `Result<T, LkhError>` for Rust-side errors. Some malformed inputs can still reach LKH's C error path. Upstream `eprintf()` prints an error and calls `exit(EXIT_FAILURE)`, which terminates the current process.

For untrusted input or long-running services, run solves in a worker process or another sandbox boundary.

## Raw FFI

The bindgen-generated API is available only with the `unsafe-ffi` Cargo feature:

```bash
cargo build --features unsafe-ffi
```

That API exposes raw pointers, mutable globals, and C functions directly. It is intended as an escape hatch for advanced users who already understand LKH's lifecycle and invariants. Prefer the safe API at the crate root when possible.
