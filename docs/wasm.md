# WebAssembly Evaluation

LKH-rs has evaluated WebAssembly integration, but the active browser path is now
the FastAPI JSON backend documented in [docs/fastapi.md](fastapi.md). Wasm is
kept here as an engineering note rather than a supported deployment target.

## Blockers

The vendored LKH C code currently assumes a native process environment:

- parameter/problem/tour files are read through C `FILE*` APIs;
- solver state is stored in process-global mutable variables;
- C errors call `exit(EXIT_FAILURE)` through `eprintf()`;
- stdout/stderr are used for reporting;
- the build uses bindgen plus a target-specific C compiler.

Those constraints do not map cleanly to `wasm32-unknown-unknown` browser modules.

## Possible future paths

1. **In-memory Wasm backend**: preserve `RoutingProblem` and `SearchParameters` as the public API and initialize solver structures without filesystem access.
2. **WASI CLI**: compile toward a WASI target and provide a virtual filesystem for legacy parameter/problem/tour files.
3. **Emscripten**: keep more of the C stdio model and use Emscripten's filesystem support.
4. **Backend service**: run native LKH-rs on a server and expose a web API to browser clients. This is the current recommended path.
5. **Long-term Rust migration**: incrementally rewrite isolated data structures and algorithms in Rust until a smaller, in-memory Wasm surface is possible.

For now, the `wasm-experimental` feature documents this evaluation status; it
does not claim full LKH browser deployment support.

The programmatic `RoutingProblem` and `SearchParameters` API is designed around
in-memory data rather than file paths. It is generic over LKH problem types:
callers provide a `TYPE`, scalar keywords, and raw sections, with convenience
builders layered on top for common TSP inputs. Native `solve_problem` renders
that model to in-memory LKH/TSPLIB text and feeds the existing parser without
creating temporary files.

On Unix-like native targets this currently uses C memory streams internally to
reuse LKH's parser. A browser-oriented Wasm backend should keep the same public
model while replacing the remaining `FILE*`, stdout/stderr, and `exit()` runtime
assumptions listed above.

## Wasm-facing model

The important design rule for Wasm callers is that application data enters as
plain in-memory values: coordinates, matrices, demands, capacities, time
windows, pickup-delivery rows, or any other TSPLIB/LKH section data. The solver
boundary should receive a `RoutingProblem` plus `SearchParameters`, not a path.

In a future `wasm-bindgen` wrapper, the public call shape should stay close to:

```rust
let problem = RoutingProblem::named_euclidean_2d("browser_points", points)?;
let parameters = SearchParameters::new()
    .with_runs(1)
    .with_max_trials(100)
    .with_seed(7);
let report = solve_problem(&problem, &parameters)?;
```

For generic LKH variants, use the same pattern as native Rust: choose a
`ProblemKind`, add scalar keywords, and add numeric section rows. Files remain
an explicit export format only. A Wasm wrapper may expose `to_tsplib()` for
debugging, download, or interoperability, but it should not require writing that
text to a filesystem before solving.

## Browser alternative

Use the FastAPI example for browser demos:

```bash
uvicorn app:app --app-dir examples/fastapi_backend --host 127.0.0.1 --port 8877
```

The frontend speaks JSON only. The backend maps that JSON into the typed Python
API and runs native LKH-rs, so it avoids browser toolchain and C runtime
constraints while preserving the same problem model.

I also checked the direct Rust `wasm32-unknown-unknown` route for the full crate.
It currently fails before linking the solver because bindgen cannot find a C
target sysroot for LKH's standard headers:

```text
source_code/LKH-3.0.14/SRC/INCLUDE/LKH.h:10:10: fatal error: 'assert.h' file not found
```

That result points toward either an Emscripten/WASI build path with a C runtime,
or a deeper in-memory backend that avoids the existing C parser and process
runtime assumptions. Neither path is currently shipped as a browser demo.
