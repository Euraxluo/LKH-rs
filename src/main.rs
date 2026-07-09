use clap::Parser;
use lkh_rs::{solve_parameter_file, LkhError};

/// The Rust binding created for LKH3.
#[derive(Parser, Debug)]
#[command(author, bin_name = "lkh", version, about, long_about = None)]
struct Args {
    /// Path of the parameter file. Example: ./source_code/LKH-3.0.14/pr2392.par
    #[arg(short, long)]
    par: String,
}

fn main() -> Result<(), LkhError> {
    env_logger::init();
    let args = Args::parse();
    let report = solve_parameter_file(args.par)?;

    println!("Best cost: {}", report.best_cost);
    println!("Best penalty: {}", report.best_penalty);
    println!("Runs: {}", report.runs);
    println!("Dimension: {}", report.dimension);
    println!("Tour length: {}", report.tour.len());

    Ok(())
}
