from __future__ import annotations

from typing import Dict, List, TypedDict, Union

class SolveReport(TypedDict):
    best_cost: int
    best_penalty: int
    runs: int
    dimension: int
    tour: List[int]

class ProblemData(TypedDict):
    kind: str
    dimension: int
    name: str
    keywords: Dict[str, str]
    sections: Dict[str, List[str]]

class SearchParameterData(TypedDict, total=False):
    runs: int
    trace_level: int
    max_trials: int
    seed: int
    time_limit: float
    total_time_limit: float

def solve_parameter_file(path: str) -> SolveReport: ...

def _solve_problem_data(
    problem: ProblemData,
    parameters: SearchParameterData,
) -> SolveReport: ...
