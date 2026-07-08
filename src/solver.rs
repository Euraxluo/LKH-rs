#![allow(static_mut_refs)]

//! Safe solver entry points around LKH's process-global C implementation.
//!
//! LKH stores most solver state in mutable globals. This module serializes all
//! safe calls behind a mutex, copies the final result into owned Rust values,
//! and provides both legacy parameter-file solving and the newer in-memory
//! programmatic path.

use crate::error::LkhError;
use crate::problem::{RoutingProblem, SearchParameters};
use crate::sys::*;
use crate::{MINUS_INFINITY, PLUS_INFINITY};
use std::ffi::CString;
use std::io;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::{Mutex, OnceLock};
use std::thread::JoinHandle;

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
    /// Best objective cost found by LKH.
    pub best_cost: i64,
    /// Best penalty found by LKH. Feasible VRP-like solutions usually have
    /// penalty zero.
    pub best_penalty: i64,
    /// Number of runs actually completed.
    pub runs: i32,
    /// Dimension reported by LKH after reading the problem.
    pub dimension: i32,
    /// Best tour copied from LKH's `BestTour` array.
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
    /// Optional backend-level trace override applied after reading parameters.
    pub trace_level_override: Option<i32>,
    /// Maximum dimension used for explicit matrix allocation.
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

struct InMemoryFile {
    path: String,
    read_fd: Option<libc::c_int>,
    writer: Option<JoinHandle<io::Result<()>>>,
}

impl InMemoryFile {
    fn new(label: &'static str, contents: String) -> Result<Self, LkhError> {
        let (path, read_fd, writer) = spawn_in_memory_file(label, contents)?;
        Ok(Self {
            path,
            read_fd: Some(read_fd),
            writer: Some(writer),
        })
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn finish(mut self) -> Result<(), LkhError> {
        if let Some(read_fd) = self.read_fd.take() {
            close_fd(read_fd);
        }
        finish_in_memory_writer(self.writer.take())
    }
}

impl Drop for InMemoryFile {
    fn drop(&mut self) {
        if let Some(read_fd) = self.read_fd.take() {
            close_fd(read_fd);
        }
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
    }
}

#[cfg(unix)]
fn spawn_in_memory_file(
    label: &'static str,
    contents: String,
) -> Result<(String, libc::c_int, JoinHandle<io::Result<()>>), LkhError> {
    use std::thread;

    let mut fds = [0; 2];
    // SAFETY: `fds` points to two valid integers. On success both descriptors
    // are owned by Rust below.
    let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if rc != 0 {
        return Err(LkhError::InMemoryInitialization(format!(
            "failed to create {label} pipe: {}",
            io::Error::last_os_error()
        )));
    }
    let read_fd = fds[0];
    let write_fd = fds[1];
    let path = fd_path(read_fd).ok_or_else(|| {
        close_fd(read_fd);
        close_fd(write_fd);
        LkhError::InMemoryInitialization(format!(
            "this platform does not expose /dev/fd or /proc/self/fd for {label}"
        ))
    })?;
    let writer = thread::spawn(move || {
        let result = write_all_to_fd(write_fd, contents.as_bytes());
        close_fd(write_fd);
        result
    });
    Ok((path, read_fd, writer))
}

#[cfg(unix)]
fn fd_path(fd: libc::c_int) -> Option<String> {
    let candidates = [format!("/dev/fd/{fd}"), format!("/proc/self/fd/{fd}")];
    candidates
        .into_iter()
        .find(|candidate| Path::new(candidate).exists())
}

#[cfg(unix)]
fn write_all_to_fd(fd: libc::c_int, mut bytes: &[u8]) -> io::Result<()> {
    while !bytes.is_empty() {
        // SAFETY: `fd` is a write descriptor owned by this thread. The pointer
        // and length come from a live Rust slice.
        let written = unsafe { libc::write(fd, bytes.as_ptr().cast(), bytes.len()) };
        if written < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        if written == 0 {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "pipe write returned zero",
            ));
        }
        bytes = &bytes[written as usize..];
    }
    Ok(())
}

#[cfg(unix)]
fn close_fd(fd: libc::c_int) {
    // SAFETY: Closing an fd is safe as long as ownership is not reused. Callers
    // only pass descriptors created by `pipe` in this module.
    unsafe {
        libc::close(fd);
    }
}

#[cfg(not(unix))]
fn spawn_in_memory_file(
    label: &'static str,
    _contents: String,
) -> Result<(String, libc::c_int, JoinHandle<io::Result<()>>), LkhError> {
    Err(LkhError::InMemoryInitialization(format!(
        "programmatic {label} solving without files is not supported on this platform yet"
    )))
}

fn finish_in_memory_writer(writer: Option<JoinHandle<io::Result<()>>>) -> Result<(), LkhError> {
    if let Some(writer) = writer {
        match writer.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(source)) => Err(LkhError::InMemoryInitialization(format!(
                "failed to write in-memory data: {source}"
            ))),
            Err(_) => Err(LkhError::InMemoryInitialization(
                "in-memory data writer panicked".to_owned(),
            )),
        }
    } else {
        Ok(())
    }
}

struct StdoutSilencer {
    saved_fd: Option<libc::c_int>,
}

impl StdoutSilencer {
    fn new() -> Result<Self, LkhError> {
        silence_stdout()
    }
}

impl Drop for StdoutSilencer {
    fn drop(&mut self) {
        if let Some(saved_fd) = self.saved_fd.take() {
            restore_stdout(saved_fd);
        }
    }
}

#[cfg(unix)]
fn silence_stdout() -> Result<StdoutSilencer, LkhError> {
    let dev_null = CString::new("/dev/null").unwrap();
    // SAFETY: These calls operate on process-level stdout. Safe API calls are
    // serialized by `SOLVER_LOCK`, so no other LKH solve is running here.
    unsafe {
        libc::fflush(ptr::null_mut());
        let saved_fd = libc::dup(libc::STDOUT_FILENO);
        if saved_fd < 0 {
            return Err(LkhError::InMemoryInitialization(format!(
                "failed to duplicate stdout: {}",
                io::Error::last_os_error()
            )));
        }
        let null_fd = libc::open(dev_null.as_ptr(), libc::O_WRONLY);
        if null_fd < 0 {
            close_fd(saved_fd);
            return Err(LkhError::InMemoryInitialization(format!(
                "failed to open /dev/null: {}",
                io::Error::last_os_error()
            )));
        }
        if libc::dup2(null_fd, libc::STDOUT_FILENO) < 0 {
            let source = io::Error::last_os_error();
            close_fd(null_fd);
            restore_stdout(saved_fd);
            return Err(LkhError::InMemoryInitialization(format!(
                "failed to redirect stdout: {source}"
            )));
        }
        close_fd(null_fd);
        Ok(StdoutSilencer {
            saved_fd: Some(saved_fd),
        })
    }
}

#[cfg(unix)]
fn restore_stdout(saved_fd: libc::c_int) {
    // SAFETY: `saved_fd` was returned by `dup(STDOUT_FILENO)` in
    // `silence_stdout`.
    unsafe {
        libc::fflush(ptr::null_mut());
        libc::dup2(saved_fd, libc::STDOUT_FILENO);
        close_fd(saved_fd);
    }
}

#[cfg(not(unix))]
fn silence_stdout() -> Result<StdoutSilencer, LkhError> {
    Ok(StdoutSilencer { saved_fd: None })
}

#[cfg(not(unix))]
fn restore_stdout(_saved_fd: libc::c_int) {}

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
    // Keep the programmatic path aligned with LKHmain.c: reset state, read
    // parameters, read the problem, then run the same search loop.
    Gain23_Reset();
    reset_programmatic_run_state();
    read_programmatic_parameters(parameters, options)?;
    let last_time = GetTime();
    StartTime = last_time;
    read_programmatic_problem(problem)?;
    run_lkh_search(last_time)
}

unsafe fn reset_programmatic_run_state() {
    // LKH globals survive between calls. Reset the fields touched by
    // ReadParameters, ReadProblem, and the search loop before loading a new
    // in-memory instance.
    FreeStructures();
    FirstNode = ptr::null_mut();
    Depot = ptr::null_mut();
    WeightType = -1;
    WeightFormat = -1;
    ProblemType = -1;
    CoordType = CoordTypes_NO_COORDS as i32;
    Distance = None;
    OldDistance = None;
    C = None;
    D = None;
    c = None;
    Penalty = None;
    MergeWithTour = Some(MergeWithTourIPT);
    OptimizePenalty = 0;
    CurrentPenalty = 0;
    CurrentGain = 0;
    BestCost = PLUS_INFINITY;
    BestPenalty = PLUS_INFINITY;
    LowerBound = 0.0;
    M = 0;
    Swaps = 0;
    OldSwaps = 0;
    Hash = 0;
    CacheMask = 0;
    PredSucCostAvailable = 0;
    IsChild = 0;
    Run = 0;
    Trial = 0;
    PopulationSize = 0;
    ColorCount = ptr::null_mut();
    FirstConstraint = ptr::null_mut();
    FirstActive = ptr::null_mut();
    LastActive = ptr::null_mut();
    FirstSegment = ptr::null_mut();
    FirstSSegment = ptr::null_mut();
    Reversed = 0;
    ParameterFile = ptr::null_mut();
    ProblemFile = ptr::null_mut();
    PiFile = ptr::null_mut();
    InputTourFile = ptr::null_mut();
    InitialTourFile = ptr::null_mut();
    SubproblemTourFile = ptr::null_mut();
    MergeTourFile = ptr::null_mut();
}

unsafe fn read_programmatic_parameters(
    parameters: &SearchParameters,
    options: &ProgrammaticSolveOptions,
) -> Result<(), LkhError> {
    // The public model is filesystem-free. We still render parameter text and
    // feed it through LKH's unchanged parser so native defaults and keyword
    // handling remain in one place.
    let parameter_text = parameters.to_lkh_parameter_file("__lkh_rs_in_memory_problem__")?;
    let parameter_file = InMemoryFile::new("parameters", parameter_text)?;
    let parameter_name =
        CString::new(parameter_file.path()).map_err(|source| LkhError::CString {
            context: "programmatic parameter path",
            source,
        })?;
    ParameterFileName = parameter_name.as_ptr() as *mut c_char;
    {
        let _silencer = StdoutSilencer::new()?;
        ReadParameters();
    }
    ParameterFileName = ptr::null_mut();
    parameter_file.finish()?;
    MaxMatrixDimension = options.max_matrix_dimension;
    if let Some(trace_level) = options.trace_level_override {
        TraceLevel = trace_level;
    }
    MergeWithTour = if Recombination == RecombinationTypes_GPX2 as i32 {
        Some(MergeWithTourGPX2)
    } else if Recombination == RecombinationTypes_CLARIST as i32 {
        Some(MergeWithTourCLARIST)
    } else {
        Some(MergeWithTourIPT)
    };
    Ok(())
}

unsafe fn read_programmatic_problem(problem: &RoutingProblem) -> Result<(), LkhError> {
    // Feed generated TSPLIB text through LKH's existing `fopen`-based parser.
    // On Unix-like native targets the path points at an anonymous pipe under
    // `/dev/fd`, so no temporary problem file is created.
    let problem_text = problem.to_tsplib();
    let problem_file = InMemoryFile::new("problem", problem_text)?;
    let problem_name = CString::new(problem_file.path()).map_err(|source| LkhError::CString {
        context: "programmatic problem path",
        source,
    })?;
    ProblemFileName = problem_name.as_ptr() as *mut c_char;
    {
        let _silencer = StdoutSilencer::new()?;
        ReadProblem();
    }
    ProblemFileName = ptr::null_mut();
    problem_file.finish()?;
    Ok(())
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

unsafe fn run_lkh_search(mut last_time: f64) -> Result<SolveReport, LkhError> {
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
                    let replacement = ReplacementIndividual(CurrentPenalty, cost);
                    ReplaceIndividualWithTour(replacement, CurrentPenalty, cost);
                    if TraceLevel >= 1 {
                        PrintPopulation();
                    }
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
            if TraceLevel >= 1 {
                print!("*** New OPTIMUM = {:#?} ***\n", Optimum);
            }
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
        if TraceLevel >= 1 && cost != PLUS_INFINITY {
            print!("Run {}: ", Run);
            let empty = CString::new("").unwrap();
            StatusReport(cost, last_time, empty.as_ptr() as *mut c_char);
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
            let mut current = FirstNode;
            loop {
                if ProblemType != Types_HCP as i32 && ProblemType != Types_HPP as i32 {
                    let d = C.unwrap()(current, (*current).Suc);
                    AddCandidate(current, (*current).Suc, d, i32::MAX);
                    AddCandidate((*current).Suc, current, d, i32::MAX);
                }

                let next = (*current).Suc;
                (*current).InitialSuc = (*current).Suc;
                current = next;

                if ptr::eq(current, first_node_ptr) {
                    break;
                }
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
        if TraceLevel >= 1 {
            MTSP_Report(BestPenalty, BestCost);
        }
    }

    if TraceLevel >= 1 && should_report_special_solution() {
        CurrentPenalty = BestPenalty;
        SOP_Report(BestCost);
    }
    report_from_globals()
}

unsafe fn should_report_special_solution() -> bool {
    ProblemType == Types_ACVRP as i32
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
