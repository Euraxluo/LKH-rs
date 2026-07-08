# Python Bindings

LKH-rs includes a small PyO3/maturin wrapper around the safe Rust API.

## Build locally

```bash
python -m pip install maturin
maturin develop --features python
python -c "import lkh_rs; print(lkh_rs.solve_parameter_file('tests/fixtures/tiny.par'))"
```

The Python package exposes:

```python
import lkh_rs

report = lkh_rs.solve_parameter_file("tests/fixtures/tiny.par")
print(report["best_cost"])
print(report["tour"])
```

The returned dictionary contains `best_cost`, `best_penalty`, `runs`, `dimension`, and `tour`.

## Current best-practice notes

The packaging uses a mixed Rust/Python layout with `python-source = "python"` and `module-name = "lkh_rs._native"` in `pyproject.toml`, matching maturin's documented project layout. PyO3 is optional in Cargo and is enabled by the `python` feature during maturin builds.

The optional PyO3 dependency accepts current PyO3 releases while staying compatible with the cached version used by local offline validation. Current maturin builds set the extension-module build configuration for Python extension builds, so normal Rust checks can compile the `python` feature without needing to link as a Python extension. This crate uses PyO3 `abi3-py38` so one wheel per platform can support Python 3.8+.

References:

- [maturin configuration](https://www.maturin.rs/config)
- [maturin project layout](https://www.maturin.rs/project_layout.html)
- [maturin PyO3 bindings](https://www.maturin.rs/bindings)
- [PyO3 features](https://pyo3.rs/main/features.html)
- [PyO3 building and distribution](https://pyo3.rs/main/building-and-distribution.html)
- [PyO3 modules](https://pyo3.rs/main/module.html)

## Limitations

The Python function calls the same serialized Rust solver as the Rust API. It does not expose the raw LKH object model, and it inherits the same upstream C limitation: deep C-side errors may terminate the process through `eprintf()`. For untrusted input, isolate the solve in a subprocess.
