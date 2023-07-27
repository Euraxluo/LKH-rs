use pathdiff;
use std::{env, ffi::CStr, ffi::CString, path::Path, ptr};

use dunce;
use log::info;
use LKH::*;

use clap::Parser;

/// The rust binding created for the LKH3
#[derive(Parser, Debug)]
#[command(author, bin_name = "LKH", version, about, long_about = None)]
struct Args {
    /// Path of the param file. eg:./source_code/pr2392.par
    #[arg(short, long)]
    par: String,
}

fn main() {
    let args = Args::parse();
    env_logger::init();
    let cargo_path = env!("CARGO_MANIFEST_DIR");
    let exe_path = env::current_exe().unwrap();
    let exe_dir = exe_path.parent().unwrap();
    let work_dir = env::current_dir().unwrap();
    let param_file_path = dunce::canonicalize(Path::new(&cargo_path).join(args.par))
        .expect("path not found,can not canonicalize")
        .to_str()
        .unwrap()
        .to_string();

    let rel_path = pathdiff::diff_paths(param_file_path, work_dir.display().to_string())
        .unwrap()
        .display()
        .to_string();
    info!("cargo_path: {:?}", cargo_path);
    info!("exe_path: {:?}", exe_path);
    info!("exe_dir: {:?}", exe_dir);
    info!("work_dir: {:?}", work_dir);
    info!("rel_path: {:?}", rel_path);
    unsafe {
        {
            TraceLevel = 2;
            ParameterFileName = CString::new(rel_path).unwrap().into_raw();
            ReadParameters();
            let mut last_time = GetTime();
            StartTime = last_time;
            MaxMatrixDimension = 20000;
            MergeWithTour = if Recombination == RecombinationTypes_GPX2 {
                Some(MergeWithTourGPX2)
            } else if Recombination == RecombinationTypes_CLARIST {
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
                return;
            }
            AllocateStructures();
            info!("AllocateStructures: {:?}", GetTime() - StartTime);

            if ProblemType == Types_TSPTW {
                TSPTW_Reduce();
            }
            if ProblemType == Types_VRPB || ProblemType == Types_VRPBTW {
                VRPB_Reduce();
            }
            if ProblemType == Types_PDPTW {
                PDPTW_Reduce();
            }
            CreateCandidateSet();
            InitializeStatistics();
            info!("InitializeStatistics: {:?}", GetTime() - StartTime);
            info!("Penalty: {:?}", Penalty.is_some());

            if Norm != 0 || Penalty.is_some() {
                Norm = 9999;
                BestCost = PLUS_INFINITY;
                BestPenalty = PLUS_INFINITY;
                CurrentPenalty = PLUS_INFINITY;
                info!("Norm or Penalty");
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
            info!("Runs: {:?}", Runs);
            info!("BestCost: {:?}", BestCost);

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

                // 种群合并
                if MaxPopulationSize > 1 && TSPTW_Makespan == 0 {
                    let mut i = 0;
                    loop {
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
                                if ProblemType != Types_CCVRP
                                    && ProblemType != Types_TRP
                                    && ProblemType != Types_MLP
                                    && ProblemType != Types_CVRPTW
                                    && MTSPObjective != Objectives_MINMAX
                                    && MTSPObjective != Objectives_MINMAX_SIZE
                                {
                                    print!(
                                        ", Gap = {:0.4}%",
                                        ((cost - Optimum) / Optimum) as f64 * 100.0
                                    );
                                } else {
                                    print!(
                                        ", Gap = {:0.4}%",
                                        ((CurrentPenalty - Optimum) / Optimum) as f64 * 100.0
                                    );
                                }
                            }

                            print!("\n");
                        }
                        i += 1;
                        if i >= PopulationSize {
                            break;
                        }
                    }
                    if HasFitness(CurrentPenalty, cost) == 0 {
                        if PopulationSize < MaxPopulationSize {
                            AddToPopulation(CurrentPenalty, cost);
                            if TraceLevel >= 1 {
                                PrintPopulation();
                            }
                        } else if smaller_fitness(
                            CurrentPenalty,
                            cost,
                            (PopulationSize - 1) as isize,
                        ) {
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
                if CurrentPenalty < BestPenalty
                    || (CurrentPenalty == BestPenalty && cost < BestCost)
                {
                    BestPenalty = CurrentPenalty;
                    BestCost = cost;
                    RecordBetterTour();
                    RecordBestTour();
                    WriteTour(TourFileName, BestTour, BestCost);
                }
                let old_optimum = Optimum;
                if Penalty.is_none()
                    || (MTSPObjective != Objectives_MINMAX
                        && MTSPObjective != Objectives_MINMAX_SIZE)
                {
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
                    if ProblemType != Types_CCVRP
                        && ProblemType != Types_TRP
                        && ProblemType != Types_KTSP
                        && ProblemType != Types_MLP
                        && MTSPObjective != Objectives_MINMAX
                        && if MTSPObjective != Objectives_MINMAX_SIZE {
                            CurrentPenalty == 0 && cost == Optimum
                        } else {
                            CurrentPenalty == Optimum
                        }
                    {
                        Runs = Run;
                        break;
                    }
                }
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

                    let first_node_ptr = FirstNode;
                    let mut current = &mut *FirstNode;
                    loop {
                        if ProblemType != Types_HCP && ProblemType != Types_HPP {
                            let d = C.unwrap()(current, current.Suc);
                            AddCandidate(current, current.Suc, d, i32::MAX);
                            AddCandidate(current.Suc, current, d, i32::MAX);
                        }

                        let next = current.Suc;
                        current.InputSuc = current.Suc;
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
                    let nodes = std::slice::from_raw_parts_mut(NodeSet, Dimension as usize);
                    let tours = std::slice::from_raw_parts_mut(BestTour, DimensionSaved as usize);
                    for i in 1..=Dimension {
                        let n_tour = *tours.get_unchecked((i - 1) as usize);
                        let next_tour = *tours.get_unchecked(i as usize);

                        let n_node: &Node = nodes.get_unchecked(n_tour as usize);
                        let next_node: &Node = nodes.get_unchecked(next_tour as usize);

                        let n_suc: *mut Node = n_node.Suc;
                        let next_pred: *mut Node = next_node.Pred;
                        // 主要就是想将 n_suc设置为next_node,将 next_pred设置为n_node
                        *n_suc = *next_node;
                        *next_pred = *n_node;
                    }
                } else {
                    let nodes = std::slice::from_raw_parts_mut(NodeSet, Dimension as usize);
                    let tours = std::slice::from_raw_parts_mut(BestTour, DimensionSaved as usize);
                    for i in 1..=DimensionSaved {
                        let n1_tour = *tours.get_unchecked((i - 1) as usize);
                        let n2_tour = *tours.get_unchecked(i as usize);

                        let n1: &Node = nodes.get_unchecked(n1_tour as usize);
                        let n2: &Node = nodes.get_unchecked(n2_tour as usize);

                        let m1: &Node = nodes.get_unchecked((n1.Id + DimensionSaved) as usize);
                        let m2: &Node = nodes.get_unchecked((n2.Id + DimensionSaved) as usize);

                        let m1_suc: *mut Node = m1.Suc;
                        let n1_pred: *mut Node = n1.Pred;

                        let n1_suc: *mut Node = n1.Suc;
                        let m2_pred: *mut Node = m2.Pred;

                        let m2_suc: *mut Node = m2.Suc;
                        let n2_pred: *mut Node = n2.Pred;

                        *m1_suc = *n1;
                        *n1_pred = *m1;

                        *n1_suc = *m2;
                        *m2_pred = *n1;

                        *m2_suc = *n2;
                        *n2_pred = *m2;
                    }
                }
                CurrentPenalty = BestPenalty;
                MTSP_Report(BestPenalty, BestCost);
                MTSP_WriteSolution(MTSPSolutionFileName, BestPenalty, BestCost);
                SINTEF_WriteSolution(SINTEFSolutionFileName, BestCost);
            }
            if ProblemType == Types_ACVRP.try_into().unwrap()
                || ProblemType == Types_BWTSP.try_into().unwrap()
                || ProblemType == Types_CCVRP.try_into().unwrap()
                || ProblemType == Types_CTSP.try_into().unwrap()
                || ProblemType == Types_CVRP.try_into().unwrap()
                || ProblemType == Types_CVRPTW.try_into().unwrap()
                || ProblemType == Types_MLP.try_into().unwrap()
                || ProblemType == Types_M_PDTSP.try_into().unwrap()
                || ProblemType == Types_M1_PDTSP.try_into().unwrap()
                || MTSPObjective != -1
                || ProblemType == Types_ONE_PDTSP.try_into().unwrap()
                || ProblemType == Types_OVRP.try_into().unwrap()
                || ProblemType == Types_PDTSP.try_into().unwrap()
                || ProblemType == Types_PDTSPL.try_into().unwrap()
                || ProblemType == Types_PDPTW.try_into().unwrap()
                || ProblemType == Types_RCTVRP.try_into().unwrap()
                || ProblemType == Types_RCTVRPTW.try_into().unwrap()
                || ProblemType == Types_SOP.try_into().unwrap()
                || ProblemType == Types_TRP.try_into().unwrap()
                || ProblemType == Types_TSPTW.try_into().unwrap()
                || ProblemType == Types_VRPB.try_into().unwrap()
                || ProblemType == Types_VRPBTW.try_into().unwrap()
                || ProblemType == Types_VRPPD.try_into().unwrap()
            {
                print!(
                    "Best {:?} solution:\n",
                    CStr::from_ptr(Type).to_str().unwrap()
                );
                CurrentPenalty = BestPenalty;
                SOP_Report(BestCost);
            }
            print!("\n");
        }
    }
}
