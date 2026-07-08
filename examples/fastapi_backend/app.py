"""FastAPI JSON backend for the lkh-rs Python package.

The API is intentionally file-free from the browser's point of view: clients
send JSON problem data and receive JSON solve reports. The server converts
wire strings back into the public lkh_rs enums before calling the native
solver, so the backend still uses the typed Python facade rather than a loose
stringly wrapper.
"""

from __future__ import annotations

import time
import json
import subprocess
import sys
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Literal, Optional, Tuple, Union

from fastapi import FastAPI, HTTPException
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field

import lkh_rs
from case_catalog import (
    get_case_payload,
    list_case_summaries,
    solve_case_payload,
)


Number = Union[int, float]
Point = Tuple[float, float]
SectionValue = Union[Number, List[Number]]
KeywordJsonValue = Union[int, float, str]

BASE_DIR = Path(__file__).resolve().parent
STATIC_DIR = BASE_DIR / "static"

app = FastAPI(
    title="LKH-rs JSON Solver",
    version="0.1.0",
)
app.mount("/static", StaticFiles(directory=STATIC_DIR), name="static")


class SearchParametersPayload(BaseModel):
    """Search settings accepted by the JSON API."""

    runs: int = Field(default=1, ge=1)
    trace_level: int = Field(default=0, ge=0)
    max_trials: Optional[int] = Field(default=None, ge=0)
    seed: Optional[int] = Field(default=None, ge=0)
    time_limit: Optional[float] = Field(default=None, ge=0)
    total_time_limit: Optional[float] = Field(default=None, ge=0)


class ProblemPayload(BaseModel):
    """A JSON-friendly problem description.

    ``kind`` chooses a convenience builder or the generic raw builder:

    - ``tsp_2d`` uses ``points``.
    - ``distance_matrix`` uses ``matrix`` and optional ``asymmetric``.
    - ``cvrp`` uses ``matrix``, ``demands``, ``capacity``, and ``depot``.
    - ``raw`` uses ``problem_type``, ``dimension``, ``keywords``, and
      ``sections`` to cover less common LKH variants.
    """

    kind: Literal["tsp_2d", "distance_matrix", "cvrp", "raw"]
    name: str = "lkh_rs_problem"
    points: Optional[List[Point]] = None
    matrix: Optional[List[List[int]]] = None
    asymmetric: bool = False
    demands: Optional[List[int]] = None
    capacity: Optional[int] = Field(default=None, ge=0)
    depot: int = Field(default=1, ge=1)
    problem_type: Optional[str] = None
    dimension: Optional[int] = Field(default=None, ge=2)
    keywords: Dict[str, KeywordJsonValue] = Field(default_factory=dict)
    sections: Dict[str, List[SectionValue]] = Field(default_factory=dict)


class SolveRequest(BaseModel):
    """Top-level request body for ``POST /api/solve``."""

    problem: ProblemPayload
    parameters: SearchParametersPayload = Field(default_factory=SearchParametersPayload)


class SolveCaseRequest(BaseModel):
    """JSON request body for solving one backend LKH parameter-file case."""

    case_id: str
    parameter_text: Optional[str] = None
    overrides: Dict[str, Union[int, float, str]] = Field(default_factory=dict)


@app.get("/", include_in_schema=False)
def index() -> FileResponse:
    """Serve the example browser frontend."""

    return FileResponse(STATIC_DIR / "index.html")


@app.get("/api/health")
def health() -> Dict[str, Any]:
    """Return a small health payload for frontend probes."""

    return {"ok": True, "backend": "fastapi", "solver": "lkh_rs"}


@app.get("/api/metadata")
def metadata() -> Dict[str, Any]:
    """Expose enum values so clients can build validated raw payloads."""

    return {
        "problem_types": _enum_items(lkh_rs.ProblemType),
        "problem_keys": _enum_items(lkh_rs.ProblemKey),
        "problem_sections": _enum_items(lkh_rs.ProblemSection),
        "edge_weight_types": _enum_items(lkh_rs.EdgeWeightType),
        "edge_weight_formats": _enum_items(lkh_rs.EdgeWeightFormat),
        "edge_data_formats": _enum_items(lkh_rs.EdgeDataFormat),
        "node_coord_types": _enum_items(lkh_rs.NodeCoordType),
        "display_data_types": _enum_items(lkh_rs.DisplayDataType),
    }


@app.get("/api/cases")
def cases() -> Dict[str, Any]:
    """Return discovered LKH parameter-file cases as JSON."""

    return list_case_summaries()


@app.get("/api/cases/{case_id}")
def case(case_id: str) -> Dict[str, Any]:
    """Return one backend LKH case as JSON."""

    try:
        return get_case_payload(case_id)
    except KeyError as exc:
        raise HTTPException(status_code=404, detail=f"unknown case: {case_id}")
    except OSError as exc:
        raise HTTPException(status_code=500, detail=str(exc)) from exc


@app.post("/api/solve-case")
def solve_case(request: SolveCaseRequest) -> Dict[str, Any]:
    """Solve one backend LKH parameter-file case and return JSON."""

    try:
        get_case_payload(request.case_id)
        return _solve_case_in_worker(
            request.case_id,
            parameter_text=request.parameter_text,
            overrides=request.overrides,
        )
    except KeyError as exc:
        raise HTTPException(status_code=404, detail=f"unknown case: {request.case_id}") from exc
    except (TypeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    except Exception as exc:
        raise HTTPException(status_code=500, detail=f"solver failed: {exc}") from exc


@app.post("/api/solve")
def solve(request: SolveRequest) -> Dict[str, Any]:
    """Solve a JSON problem with lkh-rs and return a JSON report."""

    try:
        problem = _build_problem(request.problem)
        parameters = _build_parameters(request.parameters)
    except (TypeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    started = time.perf_counter()
    try:
        report = lkh_rs.solve(problem, parameters)
    except (TypeError, ValueError) as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    except Exception as exc:
        raise HTTPException(status_code=500, detail=f"solver failed: {exc}") from exc

    return {
        "ok": True,
        "elapsed_ms": round((time.perf_counter() - started) * 1000.0, 3),
        "report": report,
    }


def _build_parameters(payload: SearchParametersPayload) -> lkh_rs.SearchParameters:
    data = _model_data(payload, exclude_none=True)
    return lkh_rs.SearchParameters(**data)


def _solve_case_in_worker(
    case_id: str,
    *,
    parameter_text: Optional[str],
    overrides: Dict[str, Union[int, float, str]],
) -> Dict[str, Any]:
    """Run native parameter-file solving outside the FastAPI process.

    Upstream LKH reports many invalid input states through ``eprintf()``, which
    exits the current process. The demo backend isolates real benchmark cases in
    a short-lived Python worker so one bad case can become a JSON error instead
    of taking down the web server.
    """

    payload = {
        "case_id": case_id,
        "parameter_text": parameter_text,
        "overrides": overrides,
    }
    completed = subprocess.run(
        [
            sys.executable,
            "-c",
            _SOLVE_CASE_WORKER,
        ],
        cwd=str(BASE_DIR),
        input=json.dumps(payload),
        text=True,
        capture_output=True,
        timeout=_solve_case_timeout(overrides),
        check=False,
    )
    marker = "__LKH_RS_SOLVE_CASE_JSON__"
    if completed.returncode != 0:
        detail = _tail_text(completed.stderr or completed.stdout)
        raise RuntimeError(
            f"worker exited with status {completed.returncode}: {detail}"
        )
    if marker not in completed.stdout:
        detail = _tail_text(completed.stderr or completed.stdout)
        raise RuntimeError(f"worker returned no JSON payload: {detail}")
    raw_json = completed.stdout.rsplit(marker, 1)[1].strip()
    return json.loads(raw_json)


def _solve_case_timeout(overrides: Dict[str, Union[int, float, str]]) -> float:
    for key in ("TOTAL_TIME_LIMIT", "TIME_LIMIT"):
        value = overrides.get(key)
        if value is None:
            continue
        try:
            return max(15.0, float(value) + 10.0)
        except (TypeError, ValueError):
            continue
    return 180.0


def _tail_text(text: str, *, limit: int = 2000) -> str:
    stripped = text.strip()
    if len(stripped) <= limit:
        return stripped
    return stripped[-limit:]


def _build_problem(payload: ProblemPayload) -> lkh_rs.Problem:
    if payload.kind == "tsp_2d":
        points = _require(payload.points, "problem.points")
        return lkh_rs.Problem.tsp_2d(points, name=payload.name)

    if payload.kind == "distance_matrix":
        matrix = _require(payload.matrix, "problem.matrix")
        return lkh_rs.Problem.distance_matrix(
            matrix,
            name=payload.name,
            asymmetric=payload.asymmetric,
        )

    if payload.kind == "cvrp":
        matrix = _require(payload.matrix, "problem.matrix")
        demands = _require(payload.demands, "problem.demands")
        capacity = _require(payload.capacity, "problem.capacity")
        return lkh_rs.Problem.cvrp(
            distance_matrix=matrix,
            demands=demands,
            capacity=capacity,
            depot=payload.depot,
            name=payload.name,
        )

    if payload.kind == "raw":
        problem_type_text = _require(payload.problem_type, "problem.problem_type")
        dimension = _require(payload.dimension, "problem.dimension")
        problem_type = _enum_by_text(lkh_rs.ProblemType, problem_type_text, "problem_type")
        keywords = {
            _enum_by_text(lkh_rs.ProblemKey, key, f"keyword {key!r}"): _keyword_value(value)
            for key, value in payload.keywords.items()
        }
        sections = {
            _enum_by_text(lkh_rs.ProblemSection, key, f"section {key!r}"): value
            for key, value in payload.sections.items()
        }
        return lkh_rs.Problem.raw(
            problem_type,
            dimension,
            name=payload.name,
            keywords=keywords,
            sections=sections,
        )

    raise ValueError(f"unsupported problem kind: {payload.kind}")


def _require(value: Optional[Any], field_name: str) -> Any:
    if value is None:
        raise ValueError(f"{field_name} is required")
    return value


def _enum_by_text(enum_type: Any, text: str, field_name: str) -> Enum:
    if not isinstance(text, str):
        raise TypeError(f"{field_name} must be a string enum name or value")
    for item in enum_type:
        if text == item.name or text == item.value:
            return item
    allowed = ", ".join(item.value for item in enum_type)
    raise ValueError(f"{field_name} must be one of: {allowed}")


def _keyword_value(value: KeywordJsonValue) -> KeywordJsonValue:
    if isinstance(value, bool):
        raise TypeError("keyword values must be numbers or supported enum strings")
    if isinstance(value, (int, float)):
        return value
    for enum_type in (
        lkh_rs.EdgeWeightType,
        lkh_rs.EdgeWeightFormat,
        lkh_rs.EdgeDataFormat,
        lkh_rs.NodeCoordType,
        lkh_rs.DisplayDataType,
    ):
        for item in enum_type:
            if value == item.name or value == item.value:
                return item
    raise ValueError(
        "string keyword values must match an LKH format enum exposed by lkh_rs"
    )


def _enum_items(enum_type: Any) -> List[Dict[str, str]]:
    return [{"name": item.name, "value": item.value} for item in enum_type]


def _model_data(model: BaseModel, *, exclude_none: bool = False) -> Dict[str, Any]:
    if hasattr(model, "model_dump"):
        return model.model_dump(exclude_none=exclude_none)
    return model.dict(exclude_none=exclude_none)


_SOLVE_CASE_WORKER = r"""
import json
import sys

from case_catalog import solve_case_payload

request = json.loads(sys.stdin.read())
result = solve_case_payload(
    request["case_id"],
    parameter_text=request.get("parameter_text"),
    overrides=request.get("overrides") or {},
)
print("__LKH_RS_SOLVE_CASE_JSON__")
print(json.dumps(result))
"""
