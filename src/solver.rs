#![allow(static_mut_refs)]

use crate::error::LkhError;
use crate::sys::*;
use crate::{MINUS_INFINITY, PLUS_INFINITY};
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::{Mutex, OnceLock};

static SOLVER_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Options for running LKH from a TSPLIB parameter file.
#[derive(Debug, Clone)]
pub struct SolveOptions {
    /// Path to the `.par` file consumed by LKH.
    pub parameter_file: PathBuf,
    /// Override LKH's trace level after reading parameters.
    pub trace_level: Option<i32>,
    /// Maximum dimension used for explicit matrices.
    pub max_matrix_dimension: i32,
    /// Directory used to make the parameter file path relative for LKH output.
    pub working_directory: Option<PathBuf>,
}

impl SolveOptions {
    /// Create options with the same defaults used by the original Rust CLI port.
    pub fn new(parameter_file: impl Into<PathBuf>) -> Self {
        Self {
            parameter_file: parameter_file.into(),
            trace_level: None,
            max_matrix_dimension: 20_000,
            working_directory: None,
        }
    }
}

/// Summary copied out of LKH's global state after a solve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolveReport {
    pub best_cost: i64,
    pub best_penalty: i64,
    pub runs: i32,
    pub dimension: i32,
    pub tour: Vec<i32>,
}

/// Solve an LKH parameter file with default options.
pub fn solve_parameter_file(path: impl AsRef<Path>) -> Result<SolveReport, LkhError> {
    solve_with_options(SolveOptions::new(path.as_ref().to_path_buf()))
}

/// Solve an LKH parameter file.
///
/// The underlying LKH C library uses process-global mutable state, so calls are
/// serialized with a global mutex. The Rust layer validates paths and returns a
/// `Result`, but malformed inputs that reach LKH's `eprintf` path can still
/// terminate the process because that behavior is implemented in upstream C.
pub fn solve_with_options(options: SolveOptions) -> Result<SolveReport, LkhError> {
    let parameter_file = canonical_parameter_file(&options.parameter_file)?;
    let working_directory = options
        .working_directory
        .clone()
        .map(|path| {
            dunce::canonicalize(&path).map_err(|source| LkhError::Canonicalize { path, source })
        })
        .transpose()?
        .unwrap_or(
            std::env::current_dir().map_err(|source| LkhError::Canonicalize {
                path: PathBuf::from("."),
                source,
            })?,
        );

    let parameter_for_lkh = pathdiff::diff_paths(&parameter_file, &working_directory)
        .unwrap_or_else(|| parameter_file.clone());
    let parameter_for_lkh = parameter_for_lkh
        .to_str()
        .ok_or_else(|| LkhError::NonUtf8Path(parameter_for_lkh.clone()))?
        .to_owned();
    let parameter_file_name =
        CString::new(parameter_for_lkh).map_err(|source| LkhError::CString {
            context: "parameter file",
            source,
        })?;

    let lock = SOLVER_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().map_err(|_| LkhError::SolverLockPoisoned)?;

    // SAFETY: The mutex serializes all access to LKH's process-global mutable
    // state. The CString lives for the duration of the C calls below.
    unsafe { run_lkh(parameter_file_name.as_ptr() as *mut _, &options) }
}

fn canonical_parameter_file(path: &Path) -> Result<PathBuf, LkhError> {
    if !path.exists() {
        return Err(LkhError::ParameterFileNotFound(path.to_path_buf()));
    }
    dunce::canonicalize(path).map_err(|source| LkhError::Canonicalize {
        path: path.to_path_buf(),
        source,
    })
}

struct StructureGuard {
    enabled: bool,
}

impl StructureGuard {
    fn enabled() -> Self {
        Self { enabled: true }
    }
}

impl Drop for StructureGuard {
    fn drop(&mut self) {
        if self.enabled {
            // SAFETY: This is called while the solver mutex is held and only
            // after LKH structures have been allocated.
            unsafe { FreeStructures() };
        }
    }
}

unsafe fn run_lkh(
    parameter_file_name: *mut std::os::raw::c_char,
    options: &SolveOptions,
) -> Result<SolveReport, LkhError> {
    ParameterFileName = parameter_file_name;
    ReadParameters();
    if let Some(trace_level) = options.trace_level {
        TraceLevel = trace_level;
    }
    let mut last_time = GetTime();
    StartTime = last_time;
    MaxMatrixDimension = options.max_matrix_dimension;
    MergeWithTour = if Recombination == RecombinationTypes_GPX2 as i32 {
        Some(MergeWithTourGPX2)
    } else if Recombination == RecombinationTypes_CLARIST as i32 {
        Some(MergeWithTourCLARIST)
    } else {
        Some(MergeWithTourIPT)
    };
    ReadProblem();

    if SubproblemSize > 0 {
        if DelaunayPartitioning != 0 {
            SolveDelaunaySubproblems();
        } else if KarpPartitioning != 0 {
            SolveKarpSubproblems();
        } else if KCenterPartitioning != 0 {
            SolveKCenterSubproblems();
        } else if KMeansPartitioning != 0 {
            SolveKMeansSubproblems();
        } else if RohePartitioning != 0 {
            SolveRoheSubproblems();
        } else if MoorePartitioning != 0 || SierpinskiPartitioning != 0 {
            SolveSFCSubproblems();
        } else {
            SolveTourSegmentSubproblems();
        }
        return report_from_globals();
    }

    AllocateStructures();
    let _structures = StructureGuard::enabled();

    if ProblemType == Types_TSPTW as i32 {
        TSPTW_Reduce();
    }
    if ProblemType == Types_VRPB as i32 || ProblemType == Types_VRPBTW as i32 {
        VRPB_Reduce();
    }
    if ProblemType == Types_PDPTW as i32 {
        PDPTW_Reduce();
    }
    CreateCandidateSet();
    InitializeStatistics();

    if Norm != 0 || Penalty.is_some() {
        Norm = 9999;
        BestCost = PLUS_INFINITY;
        BestPenalty = PLUS_INFINITY;
        CurrentPenalty = PLUS_INFINITY;
    } else {
        /* The ascent has solved the problem! */
        Optimum = LowerBound as GainType;
        BestCost = LowerBound as GainType;
        UpdateStatistics(Optimum, GetTime() - last_time);
        RecordBetterTour();
        RecordBestTour();
        CurrentPenalty = PLUS_INFINITY;
        BestPenalty = if Penalty.is_some() {
            Penalty.unwrap()()
        } else {
            0
        };
        CurrentPenalty = BestPenalty;
        WriteTour(OutputTourFileName, BestTour, BestCost);
        WriteTour(TourFileName, BestTour, BestCost);
        Runs = 0;
    }

    // Find a specified number (Runs) of local optima:
    Run = 1;
    loop {
        last_time = GetTime();
        if last_time - StartTime >= TotalTimeLimit {
            if TraceLevel >= 1 {
                print!("*** Time limit exceeded ***\n");
            }
            Run -= 1;
            break;
        }
        let mut cost = FindTour();

        // Merge population individuals.
        if MaxPopulationSize > 1 && TSPTW_Makespan == 0 {
            let mut i = 0;
            while i < PopulationSize {
                let old_penalty: GainType = CurrentPenalty;
                let old_cost: GainType = cost;
                cost = MergeTourWithIndividual(i);
                if TraceLevel >= 1
                    && (CurrentPenalty < old_penalty
                        || (CurrentPenalty == old_penalty && cost < old_cost))
                {
                    if CurrentPenalty != 0 {
                        print!(
                            "  Merged with {}: Cost = {}_{}",
                            i + 1,
                            CurrentPenalty,
                            cost
                        );
                    } else {
                        print!("  Merged with {}: Cost = {}", i + 1, cost);
                    }

                    if Optimum != MINUS_INFINITY && Optimum != 0 {
                        if OptimizePenalty != 0 {
                            let sign = if ProblemType == Types_MSCTSP as i32 {
                                -1.0
                            } else {
                                1.0
                            };
                            print!(
                                ", Gap = {:0.4}%",
                                sign * (CurrentPenalty - Optimum) as f64 / Optimum as f64 * 100.0
                            );
                        } else {
                            print!(
                                ", Gap = {:0.4}%",
                                (cost - Optimum) as f64 / Optimum as f64 * 100.0
                            );
                        }
                    }

                    print!("\n");
                }
                i += 1;
            }
            if HasFitness(CurrentPenalty, cost) == 0 {
                if PopulationSize < MaxPopulationSize {
                    AddToPopulation(CurrentPenalty, cost);
                    if TraceLevel >= 1 {
                        PrintPopulation();
                    }
                } else if smaller_fitness(CurrentPenalty, cost, (PopulationSize - 1) as isize) {
                    i = ReplacementIndividual(CurrentPenalty, cost);
                    ReplaceIndividualWithTour(i, CurrentPenalty, cost);
                    if TraceLevel >= 1 {
                        PrintPopulation();
                    }
                }
            }
        } else if Run > 1 && TSPTW_Makespan == 0 {
            cost = MergeTourWithBestTour();
        }
        // update better tour
        if CurrentPenalty < BestPenalty || (CurrentPenalty == BestPenalty && cost < BestCost) {
            BestPenalty = CurrentPenalty;
            BestCost = cost;
            RecordBetterTour();
            RecordBestTour();
            WriteTour(TourFileName, BestTour, BestCost);
        }
        let old_optimum = Optimum;
        if Penalty.is_none() || OptimizePenalty == 0 {
            if CurrentPenalty == 0 && cost < Optimum {
                Optimum = cost;
            }
        } else if CurrentPenalty < Optimum {
            Optimum = CurrentPenalty;
        }
        if Optimum < old_optimum {
            print!("*** New OPTIMUM = {:#?} ***\n", Optimum);

            if !(&*FirstNode).InputSuc.is_null() {
                let first_node_ptr = FirstNode;
                let mut current = &mut *FirstNode;
                loop {
                    let next = current.Suc;
                    current.InputSuc = current.Suc;
                    current = &mut *next;
                    if ptr::eq(current, &mut *first_node_ptr) {
                        break;
                    }
                }
            }
        }
        UpdateStatistics(cost, fabs(GetTime() - last_time));
        if TraceLevel >= 1 && cost != PLUS_INFINITY {
            print!("*** Run times:{:?}/{:?} ***", Run, Runs);
            StatusReport(cost, last_time, CString::new("").unwrap().into_raw());
            print!("\n");
        }

        if StopAtOptimum != 0 && MaxPopulationSize >= 1 {
            let optimum_reached = if OptimizePenalty != 0 {
                CurrentPenalty == Optimum
            } else {
                CurrentPenalty == 0 && cost == Optimum
            };
            if optimum_reached {
                Runs = Run;
                break;
            }
        }
        IsChild = 0;
        if PopulationSize >= 2
            && (PopulationSize == MaxPopulationSize || Run >= 2 * MaxPopulationSize)
            && Run < Runs
        {
            let parent1 = LinearSelection(PopulationSize, 1.25);
            let mut parent2;
            loop {
                parent2 = LinearSelection(PopulationSize, 1.25);
                if parent2 != parent1 {
                    break;
                }
            }

            ApplyCrossover(parent1, parent2);
            IsChild = 1;

            let first_node_ptr = FirstNode;
            let mut current = &mut *FirstNode;
            loop {
                if ProblemType != Types_HCP as i32 && ProblemType != Types_HPP as i32 {
                    let d = C.unwrap()(current, current.Suc);
                    AddCandidate(current, current.Suc, d, i32::MAX);
                    AddCandidate(current.Suc, current, d, i32::MAX);
                }

                let next = current.Suc;
                current.InitialSuc = current.Suc;
                current = &mut *next;

                if ptr::eq(current, &mut *first_node_ptr) {
                    break;
                }
            }
        }
        Seed += 1;
        SRandom(Seed);

        Run += 1;
        if Run > Runs {
            break; // do while
        }
    }
    PrintStatistics();
    if Salesmen > 1 {
        if Dimension == DimensionSaved {
            for i in 1..=Dimension {
                let n = NodeSet.add(*BestTour.add((i - 1) as usize) as usize);
                let next = NodeSet.add(*BestTour.add(i as usize) as usize);
                (*n).Suc = next;
                (*next).Pred = n;
            }
        } else {
            for i in 1..=DimensionSaved {
                let n1 = NodeSet.add(*BestTour.add((i - 1) as usize) as usize);
                let n2 = NodeSet.add(*BestTour.add(i as usize) as usize);
                let m1 = NodeSet.add(((*n1).Id + DimensionSaved) as usize);
                let m2 = NodeSet.add(((*n2).Id + DimensionSaved) as usize);

                (*m1).Suc = n1;
                (*n1).Pred = m1;

                (*n1).Suc = m2;
                (*m2).Pred = n1;

                (*m2).Suc = n2;
                (*n2).Pred = m2;
            }
        }
        CurrentPenalty = BestPenalty;
        MTSP_Report(BestPenalty, BestCost);
        MTSP_WriteSolution(MTSPSolutionFileName, BestPenalty, BestCost);
    }
    SINTEF_WriteSolution(SINTEFSolutionFileName, BestCost);
    TOP_WriteSolution(TOPSolutionFileName, BestCost);
    if ProblemType == Types_ACVRP as i32
        || ProblemType == Types_BWTSP as i32
        || ProblemType == Types_CCVRP as i32
        || ProblemType == Types_CTSP as i32
        || ProblemType == Types_CVRP as i32
        || ProblemType == Types_CVRPTW as i32
        || ProblemType == Types_GCTSP as i32
        || ProblemType == Types_CCCTSP as i32
        || ProblemType == Types_MLP as i32
        || ProblemType == Types_MSCTSP as i32
        || ProblemType == Types_M_PDTSP as i32
        || ProblemType == Types_M1_PDTSP as i32
        || MTSPObjective != -1
        || ProblemType == Types_ONE_PDTSP as i32
        || ProblemType == Types_OP as i32
        || ProblemType == Types_OVRP as i32
        || ProblemType == Types_PCTSP as i32
        || ProblemType == Types_PC_TSP as i32
        || ProblemType == Types_PDTSP as i32
        || ProblemType == Types_PDTSPL as i32
        || ProblemType == Types_PDPTW as i32
        || ProblemType == Types_PTSP as i32
        || ProblemType == Types_PTP as i32
        || ProblemType == Types_RCTVRP as i32
        || ProblemType == Types_RCTVRPTW as i32
        || ProblemType == Types_SOP as i32
        || ProblemType == Types_TRP as i32
        || ProblemType == Types_TSPMD as i32
        || ProblemType == Types_TSPTW as i32
        || ProblemType == Types_VRPB as i32
        || ProblemType == Types_VRPBTW as i32
        || ProblemType == Types_VRPPD as i32
    {
        CurrentPenalty = BestPenalty;
        SOP_Report(BestCost);
    }
    print!("\n");

    report_from_globals()
}

unsafe fn report_from_globals() -> Result<SolveReport, LkhError> {
    let dimension = DimensionSaved;
    let tour = if BestTour.is_null() || dimension <= 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(BestTour.add(1), dimension as usize).to_vec()
    };

    if tour.is_empty() && dimension > 0 {
        return Err(LkhError::MissingBestTour);
    }

    Ok(SolveReport {
        best_cost: BestCost,
        best_penalty: BestPenalty,
        runs: Runs,
        dimension,
        tour,
    })
}

unsafe fn smaller_fitness(penalty: GainType, cost: GainType, i: isize) -> bool {
    (penalty < *PenaltyFitness.offset(i))
        || (penalty == *PenaltyFitness.offset(i) && cost < *Fitness.offset(i))
}
