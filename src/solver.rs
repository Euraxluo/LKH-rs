#![allow(static_mut_refs)]

use crate::error::LkhError;
use crate::problem::{EdgeData, RoutingProblem, SearchParameters};
use crate::sys::*;
use crate::{MINUS_INFINITY, PLUS_INFINITY};
use std::ffi::CString;
use std::mem;
use std::os::raw::c_ulong;
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

/// Solve a programmatically constructed routing problem.
///
/// This entry point initializes LKH directly from the in-memory
/// `RoutingProblem` model. It does not read or write parameter/problem files.
pub fn solve_problem(
    problem: &RoutingProblem,
    parameters: &SearchParameters,
) -> Result<SolveReport, LkhError> {
    solve_problem_with_options(problem, parameters, ProgrammaticSolveOptions::default())
}

/// Native options for solving a programmatic problem.
#[derive(Debug, Clone)]
pub struct ProgrammaticSolveOptions {
    pub trace_level_override: Option<i32>,
    pub max_matrix_dimension: i32,
}

impl Default for ProgrammaticSolveOptions {
    fn default() -> Self {
        Self {
            trace_level_override: None,
            max_matrix_dimension: 20_000,
        }
    }
}

/// Solve a programmatic problem with native backend options.
pub fn solve_problem_with_options(
    problem: &RoutingProblem,
    parameters: &SearchParameters,
    options: ProgrammaticSolveOptions,
) -> Result<SolveReport, LkhError> {
    parameters.validate()?;
    validate_programmatic_parameters(parameters)?;

    let lock = SOLVER_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().map_err(|_| LkhError::SolverLockPoisoned)?;

    // SAFETY: The mutex serializes all access to LKH's process-global mutable
    // state. Programmatic initialization does not hand C any borrowed Rust data.
    unsafe { run_lkh_in_memory(problem, parameters, &options) }
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
            unsafe { free_and_reset_problem_globals() };
        }
    }
}

unsafe fn run_lkh(
    parameter_file_name: *mut std::os::raw::c_char,
    options: &SolveOptions,
) -> Result<SolveReport, LkhError> {
    Gain23_Reset();
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

unsafe fn run_lkh_in_memory(
    problem: &RoutingProblem,
    parameters: &SearchParameters,
    options: &ProgrammaticSolveOptions,
) -> Result<SolveReport, LkhError> {
    Gain23_Reset();
    initialize_programmatic_parameters(parameters, options)?;
    initialize_programmatic_problem(problem)?;
    run_lkh_search()
}

unsafe fn initialize_programmatic_parameters(
    parameters: &SearchParameters,
    options: &ProgrammaticSolveOptions,
) -> Result<(), LkhError> {
    ParameterFileName = ptr::null_mut();
    ProblemFileName = ptr::null_mut();
    PiFileName = ptr::null_mut();
    InputTourFileName = ptr::null_mut();
    OutputTourFileName = ptr::null_mut();
    TourFileName = ptr::null_mut();
    InitialTourFileName = ptr::null_mut();
    SubproblemTourFileName = ptr::null_mut();
    MTSPSolutionFileName = ptr::null_mut();
    SINTEFSolutionFileName = ptr::null_mut();
    TOPSolutionFileName = ptr::null_mut();
    CandidateFiles = 0;
    MergeTourFiles = 0;

    Alpha = 0.0;
    AscentCandidates = 50;
    BackboneTrials = 0;
    Backtracking = 0;
    BWTSP_B = 0;
    BWTSP_Q = 0;
    BWTSP_L = i32::MAX;
    CandidateSetSymmetric = 0;
    CandidateSetType = CandidateSetTypes_ALPHA as i32;
    Crossover = Some(ERXT);
    DelaunayPartitioning = 0;
    DelaunayPure = 0;
    DemandDimension = 1;
    DistanceLimit = f64::MAX;
    Drones = 1;
    Endurance = 0.0;
    Excess = -1.0;
    ExternalSalesmen = 0;
    ExtraCandidates = 0;
    ExtraCandidateSetSymmetric = 0;
    ExtraCandidateSetType = CandidateSetTypes_QUADRANT as i32;
    Gain23Used = 1;
    GainCriterionUsed = 1;
    GridSize = 1_000_000.0;
    InitialPeriod = -1;
    InitialStepSize = 0;
    InitialTourAlgorithm = InitialTourAlgorithms_WALK as i32;
    InitialTourFraction = 1.0;
    KarpPartitioning = 0;
    KCenterPartitioning = 0;
    KMeansPartitioning = 0;
    Kicks = 1;
    KickType = 0;
    MaxBreadth = i32::MAX;
    MaxCandidates = 5;
    MaxPopulationSize = 0;
    MaxSwaps = -1;
    MaxTrials = parameters.max_trials.unwrap_or(-1);
    MaxMatrixDimension = options.max_matrix_dimension;
    MoorePartitioning = 0;
    MoveType = parameters.move_type.unwrap_or(5);
    MoveTypeSpecial = 0;
    MTSPDepot = 1;
    MTSPMinSize = 1;
    MTSPMaxSize = -1;
    MTSPObjective = -1;
    NonsequentialMoveType = -1;
    Optimum = parameters.optimum.unwrap_or(MINUS_INFINITY) as GainType;
    PatchingA = parameters.patching_a.unwrap_or(1);
    PatchingC = parameters.patching_c.unwrap_or(0);
    PatchingAExtended = 0;
    PatchingARestricted = 0;
    PatchingCExtended = 0;
    PatchingCRestricted = 0;
    Precision = 100;
    Probability = 100;
    POPMUSIC_InitialTour = 0;
    POPMUSIC_MaxNeighbors = 5;
    POPMUSIC_SampleSize = 10;
    POPMUSIC_Solutions = 50;
    POPMUSIC_Trials = 1;
    Recombination = RecombinationTypes_IPT as i32;
    RestrictedSearch = 1;
    RohePartitioning = 0;
    Runs = parameters.runs;
    Salesmen = 1;
    Scale = -1;
    Seed = parameters.seed.unwrap_or(1);
    SierpinskiPartitioning = 0;
    StopAtOptimum = parameters.stop_at_optimum.unwrap_or(true) as i32;
    Subgradient = 1;
    SubproblemBorders = 0;
    SubproblemsCompressed = 0;
    SubproblemSize = 0;
    SubsequentMoveType = 0;
    SubsequentMoveTypeSpecial = 0;
    SubsequentPatching = 1;
    TimeLimit = parameters.time_limit.unwrap_or(f64::MAX);
    TotalTimeLimit = parameters.total_time_limit.unwrap_or(f64::MAX);
    TraceLevel = options
        .trace_level_override
        .unwrap_or(parameters.trace_level);
    TSPTW_Makespan = 0;
    ServiceTime = 0.0;
    RiskThreshold = 0;
    Capacity = 0;
    ColorCount = ptr::null_mut();
    Penalty = None;
    OptimizePenalty = 0;
    CurrentPenalty = 0;
    CurrentGain = 0;
    LowerBound = 0.0;
    M = 0;
    Swaps = 0;
    OldSwaps = 0;
    Serial = 1;
    RouteNodes = 0;
    RouteCost = 0;
    RouteScore = 0;
    PredSucCostAvailable = 0;
    IsChild = 0;
    Run = 0;
    Trial = 0;
    Hash = 0;
    CacheMask = 0;
    RelaxationLevel = 0;

    MergeWithTour = Some(MergeWithTourIPT);
    Ok(())
}

unsafe fn initialize_programmatic_problem(problem: &RoutingProblem) -> Result<(), LkhError> {
    free_and_reset_problem_globals();

    match problem.edge_data() {
        EdgeData::Euclidean2d(points) => initialize_euclidean_problem(points),
        EdgeData::ExplicitMatrix { matrix, asymmetric } => {
            if *asymmetric {
                initialize_asymmetric_matrix_problem(matrix)
            } else {
                initialize_symmetric_matrix_problem(matrix)
            }
        }
    }
}

unsafe fn free_and_reset_problem_globals() {
    FreeStructures();
    reset_problem_globals();
}

unsafe fn reset_problem_globals() {
    FirstNode = ptr::null_mut();
    Depot = ptr::null_mut();
    WeightType = -1;
    WeightFormat = -1;
    ProblemType = -1;
    CoordType = CoordTypes_NO_COORDS as i32;
    Name = ptr::null_mut();
    Type = ptr::null_mut();
    EdgeWeightType = ptr::null_mut();
    EdgeWeightFormat = ptr::null_mut();
    EdgeDataFormat = ptr::null_mut();
    NodeCoordType = ptr::null_mut();
    DisplayDataType = ptr::null_mut();
    Distance = None;
    OldDistance = None;
    C = None;
    D = None;
    c = None;
    Asymmetric = 0;
    Dimension = 0;
    DimensionSaved = 0;
    Dim = 0;
    CostMatrix = ptr::null_mut();
    FirstConstraint = ptr::null_mut();
    FirstActive = ptr::null_mut();
    LastActive = ptr::null_mut();
    FirstSegment = ptr::null_mut();
    FirstSSegment = ptr::null_mut();
    Reversed = 0;
}

unsafe fn initialize_euclidean_problem(points: &[crate::Point2d]) -> Result<(), LkhError> {
    let dimension = checked_dimension(points.len())?;
    Dimension = dimension;
    DimensionSaved = dimension;
    Dim = dimension;
    ProblemType = Types_TSP as i32;
    WeightType = EdgeWeightTypes_EUC_2D as i32;
    WeightFormat = EdgeWeightFormats_FUNCTION as i32;
    CoordType = CoordTypes_TWOD_COORDS as i32;
    Distance = Some(Distance_EUC_2D);
    c = Some(c_EUC_2D);
    Scale = 1;

    allocate_node_ring(dimension)?;
    for (index, point) in points.iter().enumerate() {
        let node = NodeSet.add(index + 1);
        (*node).X = point.x;
        (*node).Y = point.y;
        (*node).Z = 0.0;
    }
    precompute_symmetric_distance_matrix(dimension)?;
    finalize_basic_problem()
}

unsafe fn initialize_symmetric_matrix_problem(matrix: &[Vec<i64>]) -> Result<(), LkhError> {
    let dimension = checked_dimension(matrix.len())?;
    Dimension = dimension;
    DimensionSaved = dimension;
    Dim = dimension;
    ProblemType = Types_TSP as i32;
    WeightType = EdgeWeightTypes_EXPLICIT as i32;
    WeightFormat = EdgeWeightFormats_FULL_MATRIX as i32;
    CoordType = CoordTypes_NO_COORDS as i32;
    Distance = Some(Distance_EXPLICIT);
    c = None;

    allocate_node_ring(dimension)?;
    let entries = (dimension as usize)
        .checked_mul((dimension - 1) as usize)
        .and_then(|value| value.checked_div(2))
        .ok_or_else(|| {
            LkhError::InMemoryInitialization("symmetric matrix is too large".to_owned())
        })?;
    CostMatrix = calloc(
        entries as c_ulong,
        mem::size_of::<std::os::raw::c_int>() as c_ulong,
    ) as *mut std::os::raw::c_int;
    if CostMatrix.is_null() {
        return Err(LkhError::InMemoryInitialization(
            "failed to allocate symmetric cost matrix".to_owned(),
        ));
    }
    for i in 2..=dimension {
        let node = NodeSet.add(i as usize);
        (*node).C = CostMatrix.add(((i - 1) * (i - 2) / 2) as usize).offset(-1);
        for j in 1..i {
            *(*node).C.offset(j as isize) = matrix[(i - 1) as usize][(j - 1) as usize] as i32;
        }
    }
    finalize_basic_problem()
}

unsafe fn initialize_asymmetric_matrix_problem(matrix: &[Vec<i64>]) -> Result<(), LkhError> {
    let original_dimension = checked_dimension(matrix.len())?;
    let transformed_dimension = original_dimension.checked_mul(2).ok_or_else(|| {
        LkhError::InMemoryInitialization("asymmetric matrix dimension is too large".to_owned())
    })?;
    Dimension = transformed_dimension;
    DimensionSaved = original_dimension;
    Dim = original_dimension;
    ProblemType = Types_ATSP as i32;
    WeightType = -1;
    WeightFormat = EdgeWeightFormats_FULL_MATRIX as i32;
    CoordType = CoordTypes_NO_COORDS as i32;
    Asymmetric = 1;
    M = max_matrix_entry(matrix).saturating_add(1);
    Distance = Some(Distance_ATSP);
    c = None;

    allocate_node_ring(transformed_dimension)?;
    let entries = (original_dimension as usize)
        .checked_mul(original_dimension as usize)
        .ok_or_else(|| {
            LkhError::InMemoryInitialization("asymmetric matrix is too large".to_owned())
        })?;
    CostMatrix = calloc(
        entries as c_ulong,
        mem::size_of::<std::os::raw::c_int>() as c_ulong,
    ) as *mut std::os::raw::c_int;
    if CostMatrix.is_null() {
        return Err(LkhError::InMemoryInitialization(
            "failed to allocate asymmetric cost matrix".to_owned(),
        ));
    }
    for i in 1..=original_dimension {
        let node = NodeSet.add(i as usize);
        (*node).C = CostMatrix
            .add(((i - 1) * original_dimension) as usize)
            .offset(-1);
        for j in 1..=original_dimension {
            *(*node).C.offset(j as isize) = matrix[(i - 1) as usize][(j - 1) as usize] as i32;
        }
    }
    for i in 1..=original_dimension {
        let first = NodeSet.add(i as usize);
        let second = NodeSet.add((i + original_dimension) as usize);
        if !fix_edge(first, second) {
            return Err(LkhError::InMemoryInitialization(
                "failed to fix asymmetric split-node edge".to_owned(),
            ));
        }
    }
    finalize_basic_problem()
}

unsafe fn precompute_symmetric_distance_matrix(dimension: i32) -> Result<(), LkhError> {
    if dimension > MaxMatrixDimension || Distance.is_none() {
        return Ok(());
    }
    let entries = (dimension as usize)
        .checked_mul((dimension - 1) as usize)
        .and_then(|value| value.checked_div(2))
        .ok_or_else(|| {
            LkhError::InMemoryInitialization("symmetric matrix is too large".to_owned())
        })?;
    CostMatrix = calloc(
        entries as c_ulong,
        mem::size_of::<std::os::raw::c_int>() as c_ulong,
    ) as *mut std::os::raw::c_int;
    if CostMatrix.is_null() {
        return Err(LkhError::InMemoryInitialization(
            "failed to allocate symmetric cost matrix".to_owned(),
        ));
    }
    let distance = Distance.expect("distance function checked above");
    for i in 2..=dimension {
        let node = NodeSet.add(i as usize);
        (*node).C = CostMatrix.add(((i - 1) * (i - 2) / 2) as usize).offset(-1);
        for j in 1..i {
            let other = NodeSet.add(j as usize);
            *(*node).C.offset(j as isize) = distance(node, other);
        }
    }
    c = None;
    WeightType = EdgeWeightTypes_EXPLICIT as i32;
    C = Some(C_EXPLICIT);
    D = Some(D_EXPLICIT);
    Ok(())
}

unsafe fn allocate_node_ring(dimension: i32) -> Result<(), LkhError> {
    let count = (dimension as usize)
        .checked_add(1)
        .ok_or_else(|| LkhError::InMemoryInitialization("dimension is too large".to_owned()))?;
    NodeSet = calloc(count as c_ulong, mem::size_of::<Node>() as c_ulong) as *mut Node;
    if NodeSet.is_null() {
        return Err(LkhError::InMemoryInitialization(
            "failed to allocate node set".to_owned(),
        ));
    }

    for i in 1..=dimension {
        let node = NodeSet.add(i as usize);
        (*node).Id = i;
        (*node).OriginalId = i;
        (*node).Earliest = 0.0;
        (*node).Latest = i32::MAX as f64;
        (*node).ServiceTime = 0.0;
        if i == 1 {
            FirstNode = node;
        }
        let pred = if i == 1 {
            NodeSet.add(dimension as usize)
        } else {
            NodeSet.add((i - 1) as usize)
        };
        let suc = if i == dimension {
            NodeSet.add(1)
        } else {
            NodeSet.add((i + 1) as usize)
        };
        (*node).Pred = pred;
        (*node).Suc = suc;
    }
    Ok(())
}

unsafe fn finalize_basic_problem() -> Result<(), LkhError> {
    Depot = NodeSet.add(MTSPDepot as usize);
    if !Depot.is_null() {
        (*Depot).DepotId = 1;
    }

    if Scale < 1 {
        Scale = 1;
    }
    if Precision == 0 {
        Precision = 100;
    }
    if InitialStepSize == 0 {
        InitialStepSize = 1;
    }
    if MaxSwaps < 0 {
        MaxSwaps = Dimension;
    }
    if KickType > Dimension / 2 {
        KickType = Dimension / 2;
    }
    if MaxCandidates > Dimension - 1 {
        MaxCandidates = Dimension - 1;
    }
    if ExtraCandidates > Dimension - 1 {
        ExtraCandidates = Dimension - 1;
    }
    if SubproblemSize >= Dimension {
        SubproblemSize = Dimension;
    } else if SubproblemSize == 0 {
        if AscentCandidates > Dimension - 1 {
            AscentCandidates = Dimension - 1;
        }
        if InitialPeriod < 0 {
            InitialPeriod = Dimension / 2;
            if InitialPeriod < 100 {
                InitialPeriod = 100;
            }
        }
        if Excess < 0.0 {
            Excess = Salesmen as f64 / DimensionSaved as f64;
        }
        if MaxTrials == -1 {
            MaxTrials = Dimension;
        }
    }
    if POPMUSIC_MaxNeighbors > Dimension - 1 {
        POPMUSIC_MaxNeighbors = Dimension - 1;
    }
    if POPMUSIC_SampleSize > Dimension {
        POPMUSIC_SampleSize = Dimension;
    }
    if MTSPMaxSize == -1 {
        MTSPMaxSize = Dimension - 1;
    }

    if SubsequentMoveType == 0 {
        SubsequentMoveType = MoveType;
        SubsequentMoveTypeSpecial = MoveTypeSpecial;
    }
    k = if MoveType >= SubsequentMoveType || SubsequentPatching == 0 {
        MoveType
    } else {
        SubsequentMoveType
    };
    if PatchingC > k {
        PatchingC = k;
    }
    if PatchingA > 1 && PatchingA >= PatchingC {
        PatchingA = if PatchingC > 2 { PatchingC - 1 } else { 1 };
    }
    if NonsequentialMoveType == -1 || NonsequentialMoveType > k + PatchingC + PatchingA - 1 {
        NonsequentialMoveType = k + PatchingC + PatchingA - 1;
    }
    configure_move_functions();
    C = if WeightType == EdgeWeightTypes_EXPLICIT as i32 {
        Some(C_EXPLICIT)
    } else {
        Some(C_FUNCTION)
    };
    D = if WeightType == EdgeWeightTypes_EXPLICIT as i32 {
        Some(D_EXPLICIT)
    } else {
        Some(D_FUNCTION)
    };
    Ok(())
}

unsafe fn fix_edge(a: *mut Node, b: *mut Node) -> bool {
    if (*a).FixedTo1.is_null() || ptr::eq((*a).FixedTo1, b) {
        (*a).FixedTo1 = b;
    } else if (*a).FixedTo2.is_null() || ptr::eq((*a).FixedTo2, b) {
        (*a).FixedTo2 = b;
    } else {
        return false;
    }
    if (*b).FixedTo1.is_null() || ptr::eq((*b).FixedTo1, a) {
        (*b).FixedTo1 = a;
    } else if (*b).FixedTo2.is_null() || ptr::eq((*b).FixedTo2, a) {
        (*b).FixedTo2 = a;
    } else {
        return false;
    }
    true
}

unsafe fn configure_move_functions() {
    if PatchingC >= 1 {
        BestMove = Some(BestKOptMove);
        BestSubsequentMove = Some(BestKOptMove);
        if SubsequentPatching == 0 && SubsequentMoveType <= 5 {
            BestSubsequentMove = opt_move(SubsequentMoveType);
        }
    } else {
        BestMove = if MoveType <= 5 {
            opt_move(MoveType)
        } else {
            Some(BestKOptMove)
        };
        BestSubsequentMove = if SubsequentMoveType <= 5 {
            opt_move(SubsequentMoveType)
        } else {
            Some(BestKOptMove)
        };
    }
    if MoveTypeSpecial != 0 {
        BestMove = Some(BestSpecialOptMove);
    }
    if SubsequentMoveTypeSpecial != 0 {
        BestSubsequentMove = Some(BestSpecialOptMove);
    }
}

unsafe fn opt_move(move_type: i32) -> MoveFunction {
    match move_type {
        2 => Some(Best2OptMove),
        3 => Some(Best3OptMove),
        4 => Some(Best4OptMove),
        5 => Some(Best5OptMove),
        _ => None,
    }
}

unsafe fn run_lkh_search() -> Result<SolveReport, LkhError> {
    let mut last_time = GetTime();
    StartTime = last_time;

    AllocateStructures();
    let _structures = StructureGuard::enabled();

    CreateCandidateSet();
    InitializeStatistics();

    if Norm != 0 || Penalty.is_some() {
        Norm = 9999;
        BestCost = PLUS_INFINITY;
        BestPenalty = PLUS_INFINITY;
        CurrentPenalty = PLUS_INFINITY;
    } else {
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
        Runs = 0;
    }

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

        if MaxPopulationSize > 1 && TSPTW_Makespan == 0 {
            let mut i = 0;
            while i < PopulationSize {
                cost = MergeTourWithIndividual(i);
                i += 1;
            }
            if HasFitness(CurrentPenalty, cost) == 0 {
                if PopulationSize < MaxPopulationSize {
                    AddToPopulation(CurrentPenalty, cost);
                } else if smaller_fitness(CurrentPenalty, cost, (PopulationSize - 1) as isize) {
                    let replacement = ReplacementIndividual(CurrentPenalty, cost);
                    ReplaceIndividualWithTour(replacement, CurrentPenalty, cost);
                }
            }
        } else if Run > 1 && TSPTW_Makespan == 0 {
            cost = MergeTourWithBestTour();
        }

        if CurrentPenalty < BestPenalty || (CurrentPenalty == BestPenalty && cost < BestCost) {
            BestPenalty = CurrentPenalty;
            BestCost = cost;
            RecordBetterTour();
            RecordBestTour();
        }

        let old_optimum = Optimum;
        if Penalty.is_none() || OptimizePenalty == 0 {
            if CurrentPenalty == 0 && cost < Optimum {
                Optimum = cost;
            }
        } else if CurrentPenalty < Optimum {
            Optimum = CurrentPenalty;
        }
        if Optimum < old_optimum && !FirstNode.is_null() && !(*FirstNode).InputSuc.is_null() {
            let first_node_ptr = FirstNode;
            let mut current = FirstNode;
            loop {
                let next = (*current).Suc;
                (*current).InputSuc = (*current).Suc;
                current = next;
                if ptr::eq(current, first_node_ptr) {
                    break;
                }
            }
        }

        UpdateStatistics(cost, fabs(GetTime() - last_time));
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

        Seed += 1;
        SRandom(Seed);
        Run += 1;
        if Run > Runs {
            break;
        }
    }

    if TraceLevel >= 1 {
        PrintStatistics();
    }
    report_from_globals()
}

fn validate_programmatic_parameters(parameters: &SearchParameters) -> Result<(), LkhError> {
    if !parameters.additional_parameters.is_empty() {
        return Err(LkhError::UnsupportedProgrammaticParameter(
            "additional_parameters during in-memory solve; use typed fields or export the parameter file"
                .to_owned(),
        ));
    }
    Ok(())
}

fn checked_dimension(dimension: usize) -> Result<i32, LkhError> {
    i32::try_from(dimension)
        .map_err(|_| LkhError::InMemoryInitialization("dimension exceeds i32::MAX".to_owned()))
}

fn max_matrix_entry(matrix: &[Vec<i64>]) -> i32 {
    matrix
        .iter()
        .flat_map(|row| row.iter())
        .copied()
        .max()
        .unwrap_or(0) as i32
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
