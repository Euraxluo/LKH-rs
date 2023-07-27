#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bindings.rs"));

pub const PLUS_INFINITY: i64 = std::i64::MAX;
pub const MINUS_INFINITY: i64 = std::i64::MIN;

pub fn smaller_fitness(penalty: GainType, cost: GainType, i: isize) -> bool {
    (penalty < unsafe { *PenaltyFitness.offset(i) })
        || (penalty == unsafe { *PenaltyFitness.offset(i) } && cost < unsafe { *Fitness.offset(i) })
}

pub fn larger_fitness(penalty: GainType, cost: GainType, i: isize) -> bool {
    (penalty > unsafe { *PenaltyFitness.offset(i) })
        || (penalty == unsafe { *PenaltyFitness.offset(i) } && cost > unsafe { *Fitness.offset(i) })
}
