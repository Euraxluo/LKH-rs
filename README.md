[![Build status](https://github.com/Euraxluo/LKH-rs/workflows/rust-build/badge.svg)](#)

# LKH-rs
The binding created for the LKH3(Keld Helsgaun HomePage:http://webhotel4.ruc.dk/~keld/research/)

## Requirements
This page lists the requirements for running bindgen and how to get them.

### rust-bindgen
https://rust-lang.github.io/rust-bindgen/requirements.html

## Building the project
```bash
git clone https://github.com/Euraxluo/LKH-rs
cd LKH-rs
cargo build
```

### Windows/Ubuntu/Debian/Osx
```bash
cargo build --vv
```

### Use the LKH-rs
```bash
lkh --par .\source_code\pr2392.par
```


## Roadmap
This project aims to provide full Rust bindings and integrations for the [LKH3](http://webhotel4.ruc.dk/~keld/research/) library for solving **TSP(traveling salesperson problems)**. Here is an overview of our planned roadmap:

**Near Term Goals**
- [x] Complete cross-platform bindings for LKH using Bindgen and cc-rs(#1)
- [x] Implement an end-to-end demo app matching LKH C demo (#2)
- [ ] Set up GitHub Actions for CI/CD across platforms (#3)
    - [ ] Cross compile and test on Windows, Linux, macOS
    - [ ] Automated publishing to Crates.io
- [ ] Add documentation and examples
- [ ] Generate Python bindings using PyO3 with maturin (#4)

**Longer Term Goals**
- [ ] Explore safety improvements using Rust abstractions (#5)
    - [ ] Using Rust's enums to create type-safe wrappers around LKH data structures. This prevents invalid states or values.
    - [ ] Leveraging Rust's ownership and borrowing system to safely pass pointers/references to LKH instead of raw pointers. This helps prevent memory safety issues.
    - [ ] Wrapping unsafe LKH functions in safe Rust abstractions that enforce valid usage at compile time. 
    - [ ] Using options and results to handle error cases instead of just returning error codes that need to be checked.
    - [ ] Providing higher level iterator interfaces to LKH data structures to avoid manual memory management.
    - [ ] Using cargo features to enable optional unsafe functionality, keeping the default safe.
- [ ] Expose more LKH functionality as safe Rust APIs and expose it as an interface to other languages like Python (#6)
- [ ] Optimize performance critical sections with Rust implementations (#7)
    - [ ] parallel computing
    - [ ] Safe and high-performance memory utilization
- [ ] Evaluate WebAssembly integration for web deployment (#8)


Overall the goal is to make use of Rust language features to minimize the unsafe code needed to integrate with LKH, and surface a safer API for users. This would prevent memory safety issues, invalid state bugs etc at compile time rather than just runtime.

Welcome suggestions and collaborations from the community to improve the Rust integration and leverage LKH's capabilities.

The roadmap is subject to change based on feedback, contributions and maintenance needs.

Let me know if you would like me to modify, expand or clarify this roadmap draft further. I tried to cover the key areas and goals you envisioned in an overview format, but I'm happy to refine it as much as needed to accurately communicate our plans to potential contributors and users. 

## change log:

### Version 0.1.0

This is the first public release of the Rust bindings for the LKH library. Key highlights:

- Implements bindings for core LKH algorithms and data structures using Bindgen. This allows calling LKH functions directly from Rust code.

- Supports Windows, Linux and macOS by using cc-rs to compile platform specific C code. Rust bindings are platform agnostic.

- Provides a safe interface to LKH by wrapping unsafe code in safe Rust abstractions. Complex pointer manipulation is handled internally.

- Reimplements the LKH main entry point in Rust for easier integration. 

This initial release focuses on core binding functionality to leverage LKH algorithms in Rust. Further improvements and features will be coming in future releases. We welcome bug reports, feature requests and contributions from the community.
