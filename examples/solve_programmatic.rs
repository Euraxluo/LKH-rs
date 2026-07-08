use lkh_rs::{solve_problem, RoutingProblem, SearchParameters};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let problem = RoutingProblem::euclidean_2d([(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)])?;
    let parameters = SearchParameters::new();
    let report = solve_problem(&problem, &parameters)?;

    println!("Best cost: {}", report.best_cost);
    println!("Tour: {:?}", report.tour);

    Ok(())
}
