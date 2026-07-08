use lkh_rs::{solve_parameter_file, solve_problem, ProblemKind, RoutingProblem, SearchParameters};
use std::fs;

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

#[test]
fn solves_programmatic_euclidean_tsp() {
    let problem = RoutingProblem::euclidean_2d([(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)])
        .expect("build problem");
    let report = solve_problem(&problem, &SearchParameters::new()).expect("solve programmatic tsp");

    assert_eq!(report.best_cost, 4);
    assert_eq!(report.dimension, 4);
    assert_eq!(report.tour.len(), 4);

    let mut sorted_tour = report.tour.clone();
    sorted_tour.sort_unstable();
    assert_eq!(sorted_tour, [1, 2, 3, 4]);
}

#[test]
fn solves_programmatic_symmetric_matrix_tsp() {
    let problem = RoutingProblem::distance_matrix(vec![
        vec![0, 1, 2, 1],
        vec![1, 0, 1, 2],
        vec![2, 1, 0, 1],
        vec![1, 2, 1, 0],
    ])
    .expect("build problem");
    let report =
        solve_problem(&problem, &SearchParameters::new()).expect("solve programmatic matrix tsp");

    assert_eq!(report.best_cost, 4);
    assert_eq!(report.dimension, 4);
}

#[test]
fn solves_programmatic_asymmetric_matrix_tsp() {
    let problem = RoutingProblem::asymmetric_distance_matrix(vec![
        vec![0, 1, 9, 9],
        vec![9, 0, 1, 9],
        vec![9, 9, 0, 1],
        vec![1, 9, 9, 0],
    ])
    .expect("build problem");
    let report =
        solve_problem(&problem, &SearchParameters::new()).expect("solve programmatic atsp");

    assert_eq!(report.best_cost, 4);
    assert_eq!(report.dimension, 4);
}

#[test]
fn solves_programmatic_tsp_with_additional_lkh_parameter() {
    let problem = RoutingProblem::euclidean_2d([(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)])
        .expect("build problem");
    let parameters = SearchParameters::new()
        .with_lkh_parameter("MAX_CANDIDATES", "2")
        .expect("add parameter");

    let report = solve_problem(&problem, &parameters).expect("solve with extra parameter");

    assert_eq!(report.best_cost, 4);
    assert_eq!(report.dimension, 4);
}

#[test]
fn solves_programmatic_cvrp_from_generic_problem() {
    let problem = RoutingProblem::named("tiny_cvrp", ProblemKind::Cvrp, 4)
        .expect("build base problem")
        .with_keyword("CAPACITY", "3")
        .expect("add capacity")
        .with_keyword("EDGE_WEIGHT_TYPE", "EXPLICIT")
        .expect("add weight type")
        .with_keyword("EDGE_WEIGHT_FORMAT", "FULL_MATRIX")
        .expect("add weight format")
        .with_section(
            "EDGE_WEIGHT_SECTION",
            ["0 1 1 2", "1 0 2 1", "1 2 0 1", "2 1 1 0"],
        )
        .expect("add matrix")
        .with_section("DEMAND_SECTION", ["1 0", "2 1", "3 1", "4 1"])
        .expect("add demands")
        .with_section("DEPOT_SECTION", ["1", "-1"])
        .expect("add depot");
    let parameters = SearchParameters::new()
        .with_move_type(2)
        .with_patching_c(0)
        .with_patching_a(0);

    let report = solve_problem(&problem, &parameters).expect("solve programmatic cvrp");

    assert_eq!(report.best_penalty, 0);
    assert_eq!(report.dimension, 4);
    assert_eq!(report.tour.len(), 4);
}

#[test]
fn renders_programmatic_problem_and_parameters_without_solving() {
    let problem = RoutingProblem::euclidean_2d([(0.0, 0.0), (1.0, 0.0)]).expect("build problem");
    let problem_text = problem.to_tsplib();
    let parameter_text = SearchParameters::new()
        .to_lkh_parameter_file("problem.tsp")
        .expect("render parameters");

    assert!(problem_text.contains("TYPE: TSP"));
    assert!(problem_text.contains("NODE_COORD_SECTION"));
    assert!(parameter_text.contains("PROBLEM_FILE = problem.tsp"));
    assert!(parameter_text.contains("RUNS = 1"));
}

#[test]
fn renders_problem_endurance_with_documented_spelling() {
    let problem = RoutingProblem::named("drone", ProblemKind::Tspmd, 3)
        .expect("build problem")
        .with_keyword("ENDURANCE", "10")
        .expect("add endurance");

    assert!(problem.to_tsplib().contains("ENDURANCE: 10"));
}

#[test]
fn renders_every_lkh_problem_kind() {
    for kind in ProblemKind::ALL {
        let problem = RoutingProblem::named("kind_smoke", kind.clone(), 3).expect("build problem");
        let text = problem.to_tsplib();
        assert!(text.contains(&format!("TYPE: {}", kind.as_tsplib_type())));
    }
}

#[test]
fn explicitly_exports_programmatic_problem_and_parameters() {
    let directory = std::env::temp_dir().join(format!("lkh-rs-export-{}", std::process::id()));
    fs::create_dir_all(&directory).expect("create temp directory");
    let problem_path = directory.join("problem.tsp");
    let parameter_path = directory.join("problem.par");

    let problem =
        RoutingProblem::distance_matrix(vec![vec![0, 1], vec![1, 0]]).expect("build problem");
    let parameters = SearchParameters::new();

    problem.write_tsplib(&problem_path).expect("write problem");
    parameters
        .write_lkh_parameter_file(&parameter_path, "problem.tsp")
        .expect("write parameters");

    assert!(fs::read_to_string(&problem_path)
        .expect("read problem")
        .contains("EDGE_WEIGHT_SECTION"));
    assert!(fs::read_to_string(&parameter_path)
        .expect("read parameters")
        .contains("PROBLEM_FILE = problem.tsp"));

    let _ = fs::remove_file(problem_path);
    let _ = fs::remove_file(parameter_path);
    let _ = fs::remove_dir(directory);
}
