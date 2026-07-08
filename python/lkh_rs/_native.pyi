from __future__ import annotations

from typing import TypedDict

class SolveReport(TypedDict):
    best_cost: int
    best_penalty: int
    runs: int
    dimension: int
    tour: list[int]

def solve_parameter_file(path: str) -> SolveReport: ...
