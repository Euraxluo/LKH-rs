from __future__ import annotations

from typing import TypedDict

class SolveReport(TypedDict):
    best_cost: int
    best_penalty: int
    runs: int
    dimension: int
    tour: list[int]

def solve_parameter_file(path: str) -> SolveReport: ...

def solve_euclidean_2d(
    points: list[tuple[float, float]],
    *,
    runs: int = 1,
    trace_level: int = 0,
    max_trials: int | None = None,
    seed: int | None = None,
    time_limit: float | None = None,
    total_time_limit: float | None = None,
) -> SolveReport: ...

def solve_distance_matrix(
    matrix: list[list[int]],
    *,
    asymmetric: bool = False,
    runs: int = 1,
    trace_level: int = 0,
    max_trials: int | None = None,
    seed: int | None = None,
    time_limit: float | None = None,
    total_time_limit: float | None = None,
) -> SolveReport: ...
