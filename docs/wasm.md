# WebAssembly Evaluation

LKH-rs has evaluated WebAssembly integration, but browser-ready Wasm support is not currently enabled.

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
4. **Backend service**: run native LKH-rs on a server and expose a web API to browser clients.
5. **Long-term Rust migration**: incrementally rewrite isolated data structures and algorithms in Rust until a smaller, in-memory Wasm surface is possible.

For now, the `wasm-experimental` feature documents this evaluation status; it does not claim browser deployment support.

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
