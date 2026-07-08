[![CI](https://github.com/Euraxluo/LKH-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Euraxluo/LKH-rs/actions/workflows/ci.yml)

# LKH-rs

Rust bindings and safe wrappers for [LKH3](http://webhotel4.ruc.dk/~keld/research/), Keld Helsgaun's heuristic solver for **TSP (traveling salesperson problems)** and related routing problems.

The crate builds the vendored LKH C sources with `cc`, generates Rust bindings with `bindgen`, and exposes a small safe Rust API for solving existing LKH parameter files.

## Requirements

The build uses `bindgen`, so your system needs a working Clang/libclang installation. See the [rust-bindgen requirements](https://rust-lang.github.io/rust-bindgen/requirements.html).

## Building

```bash
git clone https://github.com/Euraxluo/LKH-rs
cd LKH-rs
cargo build
```

For verbose platform diagnostics:

```bash
cargo build --vv
```

## CLI usage

Run the included example parameter file:

```bash
cargo run --bin lkh -- --par source_code/pr2392.par
```

After installing the binary, use:

```bash
lkh --par source_code/pr2392.par
```

## Rust API usage

```rust
use lkh_rs::solve_parameter_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = solve_parameter_file("source_code/pr2392.par")?;
    println!("best cost: {}", report.best_cost);
    println!("tour length: {}", report.tour.len());
    Ok(())
}
```

A complete example is available in [examples/solve_parameter_file.rs](examples/solve_parameter_file.rs).

## Cargo features

| Feature | Description |
| --- | --- |
| `demo` | Builds the lightweight C demo configuration. |
| `unsafe-ffi` | Exposes raw bindgen-generated LKH symbols under `lkh_rs::ffi`. Prefer the safe API when possible. |
| `python` | Enables the PyO3 module used by maturin. |
| `wasm-experimental` | Marks WebAssembly evaluation work; browser Wasm is not currently supported. |

## Python bindings

Build and install the Python extension locally with maturin:

```bash
python -m pip install maturin
maturin develop --features python
python -c "import lkh_rs; print(lkh_rs.solve_parameter_file('tests/fixtures/tiny.par'))"
```

The Python package wraps the same safe Rust solver and returns a dictionary containing `best_cost`, `best_penalty`, `runs`, `dimension`, and `tour`.

See [docs/python.md](docs/python.md) for details.

## Safety model

The upstream LKH C library uses process-global mutable state and C error paths that can call `exit(EXIT_FAILURE)`. LKH-rs serializes safe API calls with a global mutex and returns `Result` for Rust-side validation errors, but malformed inputs that reach deep C parsing may still terminate the process.

Use subprocess isolation for untrusted inputs or service workloads. See [docs/safety.md](docs/safety.md).

## Performance and WebAssembly

- [docs/performance.md](docs/performance.md) describes the current benchmark baseline and why safe parallelism should use multiple processes rather than multiple in-process threads.
- [docs/wasm.md](docs/wasm.md) records the WebAssembly evaluation and current blockers. Browser-ready Wasm deployment is not yet supported.

## Roadmap

This project aims to provide full Rust bindings and practical integrations for LKH3.

**Near Term Goals**

- [x] Complete cross-platform bindings for LKH using bindgen and cc-rs (#1)
- [x] Implement an end-to-end demo app matching the LKH C demo (#2)
- [x] Set up GitHub Actions for CI/CD across platforms (#3)
    - [x] Build and test on Windows, Linux, macOS
    - [x] Add crates.io publishing workflow scaffold
- [x] Add documentation and examples (#4)
- [x] Generate Python bindings using PyO3 with maturin (#5)

**Longer Term Goals**

- [x] Explore safety improvements using Rust abstractions (#6)
    - [x] Add a default safe parameter-file API around LKH's global C state
    - [x] Return `Result` errors for Rust-side validation failures
    - [x] Copy solver results into owned Rust structures
    - [x] Gate raw pointer/global access behind the `unsafe-ffi` feature
    - [x] Document remaining C-side safety limitations
- [x] Expose more LKH functionality as safe Rust APIs and expose it to other languages like Python (#7)
- [x] Optimize performance critical sections with Rust implementations (#8)
    - [x] Add a benchmark baseline and performance roadmap
    - [x] Document process-level parallelism as the safe path for concurrent solves
- [x] Evaluate WebAssembly integration for web deployment (#9)

Overall, LKH-rs uses Rust language features to minimize the unsafe code that application users need to write, while still making the underlying LKH capabilities available for advanced integrations.

## Contribution

We welcome **bug reports**, **feature requests**, and other contributions from the community.

## Change log

### Version 0.1.0

This is the first public release of the Rust bindings for the LKH library. Key highlights:

- Builds LKH C sources with `cc` and generates Rust bindings with bindgen.
- Supports Windows, Linux, and macOS native builds through platform-specific build configuration.
- Provides a safe parameter-file API that wraps LKH's global-state solver behind a serialized Rust boundary.
- Provides raw FFI access behind the explicit `unsafe-ffi` feature.
- Includes Python/maturin scaffolding, examples, tests, and documentation for safety, performance, and WebAssembly evaluation.
