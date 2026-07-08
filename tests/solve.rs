use lkh_rs::solve_parameter_file;

#[test]
fn solves_tiny_fixture() {
    let report = solve_parameter_file("tests/fixtures/tiny.par").expect("solve tiny fixture");

    assert!(report.best_cost > 0);
    assert_eq!(report.dimension, 4);
    assert_eq!(report.tour.len(), 4);

    let mut sorted_tour = report.tour.clone();
    sorted_tour.sort_unstable();
    assert_eq!(sorted_tour, [1, 2, 3, 4]);
}
