use lkh_rs::{solve_problem, ProblemKind, RoutingProblem, SearchParameters};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    solve_euclidean_tsp()?;
    solve_distance_matrix_tsp()?;
    solve_cvrp()?;
    render_export_text()?;

    Ok(())
}

fn solve_euclidean_tsp() -> Result<(), Box<dyn std::error::Error>> {
    let problem = RoutingProblem::named_euclidean_2d(
        "square_tsp",
        [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)],
    )?;
    let parameters = SearchParameters::new().with_seed(1).with_max_trials(100);
    let report = solve_problem(&problem, &parameters)?;

    println!("Euclidean TSP best cost: {}", report.best_cost);
    println!("Euclidean TSP tour: {:?}", report.tour);

    Ok(())
}

fn solve_distance_matrix_tsp() -> Result<(), Box<dyn std::error::Error>> {
    let problem = RoutingProblem::named_distance_matrix(
        "matrix_tsp",
        vec![
            vec![0, 1, 2, 1],
            vec![1, 0, 1, 2],
            vec![2, 1, 0, 1],
            vec![1, 2, 1, 0],
        ],
    )?;
    let report = solve_problem(&problem, &SearchParameters::new())?;

    println!("Matrix TSP best cost: {}", report.best_cost);
    println!("Matrix TSP tour: {:?}", report.tour);

    Ok(())
}

fn solve_cvrp() -> Result<(), Box<dyn std::error::Error>> {
    let problem = RoutingProblem::named("tiny_cvrp", ProblemKind::Cvrp, 4)?
        .with_keyword("CAPACITY", "3")?
        .with_keyword("EDGE_WEIGHT_TYPE", "EXPLICIT")?
        .with_keyword("EDGE_WEIGHT_FORMAT", "FULL_MATRIX")?
        .with_section(
            "EDGE_WEIGHT_SECTION",
            ["0 1 1 2", "1 0 2 1", "1 2 0 1", "2 1 1 0"],
        )?
        .with_section("DEMAND_SECTION", ["1 0", "2 1", "3 1", "4 1"])?
        .with_section("DEPOT_SECTION", ["1", "-1"])?;
    let parameters = SearchParameters::new()
        .with_move_type(2)
        .with_patching_c(0)
        .with_patching_a(0);
    let report = solve_problem(&problem, &parameters)?;

    println!("CVRP best cost: {}", report.best_cost);
    println!("CVRP best penalty: {}", report.best_penalty);
    println!("CVRP tour: {:?}", report.tour);

    Ok(())
}

fn render_export_text() -> Result<(), Box<dyn std::error::Error>> {
    let problem = RoutingProblem::euclidean_2d([(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)])?;
    let parameters = SearchParameters::new().with_runs(1);

    let problem_text = problem.to_tsplib();
    let parameter_text = parameters.to_lkh_parameter_file("problem.tsp")?;

    println!("Rendered problem text has {} bytes", problem_text.len());
    println!("Rendered parameter text has {} bytes", parameter_text.len());

    Ok(())
}
