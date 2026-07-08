#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

//! Safe Rust entry points for the LKH3 heuristic solver.
//!
//! By default this crate exposes a small, serialized, parameter-file based API
//! around the vendored LKH C library. The raw bindgen surface is available only
//! with the `unsafe-ffi` feature because it contains mutable globals and raw
//! pointers inherited from LKH.

mod sys {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    #![allow(clippy::all)]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

#[cfg(feature = "unsafe-ffi")]
pub mod ffi {
    //! Raw bindgen-generated LKH symbols.
    //!
    //! This module is an unsafe escape hatch. The underlying C library uses
    //! process-global mutable state, raw pointers, and process-terminating error
    //! paths. Prefer the safe functions at the crate root when possible.

    pub use crate::sys::*;

    #[cfg(not(feature = "demo"))]
    pub const PLUS_INFINITY: GainType = i64::MAX;

    #[cfg(not(feature = "demo"))]
    pub const MINUS_INFINITY: GainType = i64::MIN;

    /// Compare a penalty/cost pair with an indexed global fitness entry.
    ///
    /// # Safety
    ///
    /// `PenaltyFitness` and `Fitness` must have been initialized by LKH, and
    /// `i` must be within both arrays.
    #[cfg(not(feature = "demo"))]
    pub unsafe fn smaller_fitness(penalty: GainType, cost: GainType, i: isize) -> bool {
        (penalty < *PenaltyFitness.offset(i))
            || (penalty == *PenaltyFitness.offset(i) && cost < *Fitness.offset(i))
    }

    /// Compare a penalty/cost pair with an indexed global fitness entry.
    ///
    /// # Safety
    ///
    /// `PenaltyFitness` and `Fitness` must have been initialized by LKH, and
    /// `i` must be within both arrays.
    #[cfg(not(feature = "demo"))]
    pub unsafe fn larger_fitness(penalty: GainType, cost: GainType, i: isize) -> bool {
        (penalty > *PenaltyFitness.offset(i))
            || (penalty == *PenaltyFitness.offset(i) && cost > *Fitness.offset(i))
    }
}

pub mod error;
#[cfg(not(feature = "demo"))]
pub mod problem;
#[cfg(not(feature = "demo"))]
pub mod solver;

#[cfg(all(feature = "python", not(feature = "demo")))]
mod python;

pub use error::LkhError;
#[cfg(not(feature = "demo"))]
pub use problem::{Point2d, ProblemEntry, ProblemKind, RoutingProblem, SearchParameters};
#[cfg(not(feature = "demo"))]
pub use solver::{
    solve_parameter_file, solve_problem, solve_problem_with_options, solve_with_options,
    ProgrammaticSolveOptions, SolveOptions, SolveReport,
};

#[cfg(not(feature = "demo"))]
const PLUS_INFINITY: sys::GainType = i64::MAX;

#[cfg(not(feature = "demo"))]
const MINUS_INFINITY: sys::GainType = i64::MIN;
