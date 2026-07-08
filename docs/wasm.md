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

1. **WASI CLI**: compile toward a WASI target and provide a virtual filesystem for parameter/problem/tour files.
2. **Emscripten**: keep more of the C stdio model and use Emscripten's filesystem support.
3. **Backend service**: run native LKH-rs on a server and expose a web API to browser clients.
4. **Long-term Rust migration**: incrementally rewrite isolated data structures and algorithms in Rust until a smaller, in-memory Wasm surface is possible.

For now, the `wasm-experimental` feature documents this evaluation status; it does not claim browser deployment support.
