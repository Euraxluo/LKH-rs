use lkh_rs::solve_parameter_file;
use std::time::Instant;

#[test]
#[ignore = "baseline benchmark; run with `cargo bench --bench solve -- --ignored`"]
fn baseline_tiny_fixture() {
    let start = Instant::now();
    let report = solve_parameter_file("tests/fixtures/tiny.par").expect("solve tiny fixture");
    eprintln!(
        "tiny fixture: best_cost={}, dimension={}, elapsed={:?}",
        report.best_cost,
        report.dimension,
        start.elapsed()
    );
}
