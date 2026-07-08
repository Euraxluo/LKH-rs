# Changelog

All notable changes to this project are documented here.

## 0.1.0 - Unreleased

Initial public release candidate.

### Added

- Safe serialized Rust API around LKH's process-global C solver state.
- Parameter-file solving through `solve_parameter_file`.
- Programmatic in-memory routing API with typed Python and Rust facades.
- Python package built with PyO3 and maturin.
- FastAPI JSON demo backend with a browser route visualizer and real LKH case
  catalog loading.
- Rust examples and Python examples for programmatic solves.
- CI workflows for Rust checks/tests and Python wheel builds across Linux,
  macOS, and Windows.
- Release workflow for publishing Python wheels/sdist to PyPI and the Rust
  crate to crates.io from `v*` tags.

### Fixed

- Reset LKH file-related globals before each parameter-file solve so settings
  such as `INITIAL_TOUR_FILE` cannot leak between cases.
- Release the Python GIL while native solves are running.
- Report mTSP `MINMAX` objectives using LKH's penalty objective rather than the
  aggregate tour cost in the FastAPI demo.

### Changed

- Documented FastAPI as the browser integration path and recorded current Wasm
  limitations.
- Documented subprocess isolation requirements for service workloads because
  upstream LKH can still terminate the current process on some malformed inputs.
