use lkh_rs::solve_parameter_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "source_code/pr2392.par".to_string());
    let report = solve_parameter_file(path)?;

    println!("Best cost: {}", report.best_cost);
    println!("Best penalty: {}", report.best_penalty);
    println!("Runs: {}", report.runs);
    println!("Dimension: {}", report.dimension);
    println!("Tour length: {}", report.tour.len());

    Ok(())
}
