#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bindings.rs"));

#[cfg(not(feature = "demo"))]
pub const PLUS_INFINITY: i64 = std::i64::MAX;

#[cfg(not(feature = "demo"))]
pub const MINUS_INFINITY: i64 = std::i64::MIN;

#[cfg(not(feature = "demo"))]
pub fn smaller_fitness(penalty: GainType, cost: GainType, i: isize) -> bool {
    (penalty < unsafe { *PenaltyFitness.offset(i) })
        || (penalty == unsafe { *PenaltyFitness.offset(i) } && cost < unsafe { *Fitness.offset(i) })
}

#[cfg(not(feature = "demo"))]
pub fn larger_fitness(penalty: GainType, cost: GainType, i: isize) -> bool {
    (penalty > unsafe { *PenaltyFitness.offset(i) })
        || (penalty == unsafe { *PenaltyFitness.offset(i) } && cost > unsafe { *Fitness.offset(i) })
}
