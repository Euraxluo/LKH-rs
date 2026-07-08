"""Discovery and execution of LKH parameter-file cases for the demo backend."""

from __future__ import annotations

import hashlib
import os
import re
import tempfile
import time
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Tuple, Union

import lkh_rs


BASE_DIR = Path(__file__).resolve().parent
REPO_ROOT = BASE_DIR.parents[1]
DEFAULT_CASE_OVERRIDES: Dict[str, Union[int, float, str]] = {
    "RUNS": 1,
    "TRACE_LEVEL": 0,
    "MAX_TRIALS": 100,
}

INPUT_FILE_KEYS = {
    "CANDIDATE_FILE",
    "EDGE_FILE",
    "INITIAL_TOUR_FILE",
    "INPUT_TOUR_FILE",
    "MERGE_TOUR_FILE",
    "PI_FILE",
    "PROBLEM_FILE",
    "SUBPROBLEM_TOUR_FILE",
}
OUTPUT_FILE_KEYS = {
    "MTSP_SOLUTION_FILE",
    "OUTPUT_TOUR_FILE",
    "SINTEF_SOLUTION_FILE",
    "TOP_SOLUTION_FILE",
    "TOUR_FILE",
}
PENALTY_OBJECTIVE_TYPES = {
    "CCCTSP",
    "CCVRP",
    "CBNTSP",
    "CBTSP",
    "GCTSP",
    "KTSP",
    "MLP",
    "MSCTSP",
    "OKTSP",
    "OP",
    "PC_TSP",
    "PTP",
    "PTSP",
    "TOP",
    "TRP",
    "TSPMD",
}
KEY_VALUE_RE = re.compile(
    r"^(?P<indent>\s*)(?P<key>[A-Za-z_][A-Za-z0-9_]*)"
    r"(?P<sep>\s*[:=]\s*)(?P<value>.*?)(?P<trailing>\s*)$"
)


@dataclass(frozen=True)
class CaseInfo:
    """One discovered LKH parameter-file case."""

    id: str
    path: Path
    source: str
    relative_path: str
    label: str
    problem_file: Optional[Path]
    problem_name: Optional[str]
    problem_type: Optional[str]
    dimension: Optional[int]
    edge_weight_type: Optional[str]
    mtsp_objective: Optional[str]
    known_optimum: Optional[int]
    display_points: List[List[float]]


def list_case_summaries() -> Dict[str, Any]:
    """Return a compact catalog for the browser dropdown."""

    cases = sorted(_discover_cases(), key=lambda item: (item.source, item.relative_path))
    return {
        "case_count": len(cases),
        "sources": sorted({case.source for case in cases}),
        "default_overrides": dict(DEFAULT_CASE_OVERRIDES),
        "cases": [_case_summary(case) for case in cases],
    }


def get_case_payload(case_id: str) -> Dict[str, Any]:
    """Return one case plus a JSON solve request for that case."""

    case = _case_by_id(case_id)
    parameter_text = case.path.read_text(encoding="utf-8", errors="replace")
    return {
        **_case_summary(case),
        "parameter_text": parameter_text,
        "display": {
            "points": case.display_points,
            "dimension": case.dimension,
        },
        "objective_metric": _case_objective_metric(case),
        "objective_label": _case_objective_label(case),
        "known_objective": case.known_optimum,
        "known_objective_label": _case_known_objective_label(case),
        "known_target_metric": _case_objective_metric(case),
        "known_target_label": _case_known_objective_label(case),
        "payload": {
            "case_id": case.id,
            "overrides": dict(DEFAULT_CASE_OVERRIDES),
        },
    }


def solve_case_payload(
    case_id: str,
    *,
    parameter_text: Optional[str] = None,
    overrides: Optional[Dict[str, Union[int, float, str]]] = None,
) -> Dict[str, Any]:
    """Solve a case by rewriting file references into a temporary parameter file."""

    case = _case_by_id(case_id)
    text = parameter_text if parameter_text is not None else case.path.read_text()
    overrides = dict(DEFAULT_CASE_OVERRIDES if overrides is None else overrides)

    started = time.perf_counter()
    with tempfile.TemporaryDirectory(prefix="lkh-rs-case-") as tmp:
        temp_dir = Path(tmp)
        normalized = _normalize_parameter_text(
            text,
            parameter_file=case.path,
            temp_dir=temp_dir,
            overrides=overrides,
        )
        temp_par = temp_dir / case.path.name
        temp_par.write_text(normalized, encoding="utf-8")
        report = lkh_rs.solve_parameter_file(str(temp_par))

    objective = _objective(report, case)
    solution = _solution(report, case, text)
    return {
        "ok": True,
        "elapsed_ms": round((time.perf_counter() - started) * 1000.0, 3),
        "case": _case_summary(case),
        "known_optimum": case.known_optimum,
        "known_objective": objective["known"],
        "objective": objective,
        "objective_metric": objective["metric"],
        "objective_label": objective["label"],
        "known_objective_label": objective["known_label"],
        "known_target_metric": objective["metric"],
        "known_target_label": objective["known_label"],
        "gap_percent": objective["gap_percent"],
        "comparison": objective,
        "solution": solution,
        "report": report,
    }


def _case_summary(case: CaseInfo) -> Dict[str, Any]:
    return {
        "id": case.id,
        "label": case.label,
        "source": case.source,
        "relative_path": case.relative_path,
        "problem_name": case.problem_name,
        "problem_type": case.problem_type,
        "dimension": case.dimension,
        "edge_weight_type": case.edge_weight_type,
        "known_optimum": case.known_optimum,
        "known_objective": case.known_optimum,
        "objective_metric": _case_objective_metric(case),
        "objective_label": _case_objective_label(case),
        "known_objective_label": _case_known_objective_label(case),
        "known_target_metric": _case_objective_metric(case),
        "known_target_label": _case_known_objective_label(case),
        "nodes": case.dimension or len(case.display_points),
    }


def _case_by_id(case_id: str) -> CaseInfo:
    for case in _discover_cases():
        if case.id == case_id:
            return case
    raise KeyError(case_id)


@lru_cache(maxsize=1)
def _discover_cases() -> Tuple[CaseInfo, ...]:
    seen: set[Path] = set()
    cases: List[CaseInfo] = []
    for root in _case_roots():
        if not root.exists():
            continue
        for path in sorted(root.rglob("*.par")):
            resolved = path.resolve()
            if resolved in seen:
                continue
            seen.add(resolved)
            cases.append(_inspect_case(resolved, root))
    return tuple(cases)


def _case_roots() -> Tuple[Path, ...]:
    roots = [
        REPO_ROOT / "source_code",
        REPO_ROOT / "tests" / "fixtures",
    ]
    official_root = Path("/tmp/lkh-official/bench-extract")
    if official_root.exists():
        roots.append(official_root)
    for value in os.environ.get("LKH_RS_CASE_ROOTS", "").split(os.pathsep):
        if value:
            roots.append(Path(value).expanduser())

    unique: List[Path] = []
    seen: set[Path] = set()
    for root in roots:
        resolved = root.resolve()
        if resolved not in seen and resolved.exists():
            seen.add(resolved)
            unique.append(resolved)
    return tuple(unique)


def _inspect_case(path: Path, root: Path) -> CaseInfo:
    text = path.read_text(encoding="utf-8", errors="replace")
    values = _parameter_values(text)
    problem_file = _resolve_input_reference(path, values.get("PROBLEM_FILE", ""))
    metadata = _problem_metadata(problem_file) if problem_file else {}
    known_optimum = _parse_int(values.get("OPTIMUM")) or _parse_int(metadata.get("OPTIMUM"))
    relative_path = _relative_to(path, root)
    problem_name = metadata.get("NAME") or path.stem
    problem_type = metadata.get("TYPE")
    label_parts = [path.stem]
    if problem_type:
        label_parts.append(problem_type)
    if metadata.get("DIMENSION"):
        label_parts.append(str(metadata["DIMENSION"]))
    return CaseInfo(
        id=_case_id(path),
        path=path,
        source=_source_label(path),
        relative_path=relative_path,
        label=" · ".join(label_parts),
        problem_file=problem_file,
        problem_name=problem_name,
        problem_type=problem_type,
        dimension=_parse_int(metadata.get("DIMENSION")),
        edge_weight_type=metadata.get("EDGE_WEIGHT_TYPE"),
        mtsp_objective=_canonical_keyword_value(values.get("MTSP_OBJECTIVE")),
        known_optimum=known_optimum,
        display_points=_display_points(problem_file),
    )


def _case_id(path: Path) -> str:
    digest = hashlib.sha1(str(path).encode("utf-8")).hexdigest()[:16]
    return f"par_{digest}"


def _source_label(path: Path) -> str:
    if _is_relative_to(path, REPO_ROOT):
        return "repository"
    official_root = Path("/tmp/lkh-official/bench-extract")
    if official_root.exists() and _is_relative_to(path, official_root.resolve()):
        return "official"
    return "external"


def _relative_to(path: Path, root: Path) -> str:
    try:
        return str(path.relative_to(root))
    except ValueError:
        return str(path)


def _is_relative_to(path: Path, root: Path) -> bool:
    try:
        path.relative_to(root)
        return True
    except ValueError:
        return False


def _parameter_values(text: str) -> Dict[str, str]:
    values: Dict[str, str] = {}
    for line in text.splitlines():
        parsed = _parse_assignment(line)
        if parsed is None:
            continue
        key, value = parsed
        values[key] = value
    return values


def _parse_assignment(line: str) -> Optional[Tuple[str, str]]:
    match = KEY_VALUE_RE.match(line)
    if match is None:
        return None
    return match.group("key").upper(), match.group("value").strip()


def _normalize_parameter_text(
    text: str,
    *,
    parameter_file: Path,
    temp_dir: Path,
    overrides: Dict[str, Union[int, float, str]],
) -> str:
    normalized_lines: List[str] = []
    seen_overrides: set[str] = set()
    sanitized_overrides = {
        _validate_parameter_key(key): _validate_parameter_value(value)
        for key, value in overrides.items()
    }

    for line in text.splitlines():
        match = KEY_VALUE_RE.match(line)
        if match is None:
            normalized_lines.append(line)
            continue

        key = match.group("key").upper()
        if key in sanitized_overrides:
            normalized_lines.append(f"{key} = {sanitized_overrides[key]}")
            seen_overrides.add(key)
            continue

        value = match.group("value").strip()
        if key in INPUT_FILE_KEYS:
            resolved = _resolve_input_reference(parameter_file, value)
            normalized_lines.append(f"{key} = {resolved}")
        elif key in OUTPUT_FILE_KEYS:
            normalized_lines.append(f"{key} = {temp_dir / Path(value).name}")
        else:
            normalized_lines.append(line)

    for key, value in sanitized_overrides.items():
        if key not in seen_overrides:
            normalized_lines.append(f"{key} = {value}")
    return "\n".join(normalized_lines) + "\n"


def _resolve_input_reference(parameter_file: Path, value: str) -> Optional[Path]:
    if not value:
        return None
    raw = value.strip().strip('"')
    candidate = Path(raw).expanduser()
    if candidate.is_absolute():
        return candidate

    bases = [parameter_file.parent]
    if parameter_file.parent.name.upper() == "TMP":
        bases.append(parameter_file.parent.parent)
    bases.append(REPO_ROOT)
    bases.append(Path.cwd())

    for base in bases:
        resolved = (base / candidate).resolve()
        if resolved.exists():
            return resolved
    return (parameter_file.parent / candidate).resolve()


def _problem_metadata(path: Optional[Path]) -> Dict[str, str]:
    if path is None or not path.exists():
        return {}
    metadata: Dict[str, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped.endswith("_SECTION"):
            break
        parsed = _parse_assignment(line)
        if parsed is None:
            continue
        key, value = parsed
        if key in {"NAME", "TYPE", "DIMENSION", "EDGE_WEIGHT_TYPE", "OPTIMUM"}:
            metadata[key] = value
    return metadata


def _display_points(path: Optional[Path], *, limit: int = 2000) -> List[List[float]]:
    if path is None or not path.exists():
        return []
    points: List[List[float]] = []
    in_section = False
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        stripped = line.strip()
        if stripped in {"NODE_COORD_SECTION", "DISPLAY_DATA_SECTION"}:
            in_section = True
            continue
        if not in_section:
            continue
        if not stripped or stripped == "EOF" or stripped.endswith("_SECTION"):
            break
        parts = stripped.split()
        if len(parts) < 3:
            continue
        try:
            points.append([float(parts[1]), float(parts[2])])
        except ValueError:
            continue
        if len(points) >= limit:
            break
    return points


def _parse_int(value: Optional[str]) -> Optional[int]:
    if value is None:
        return None
    try:
        return int(value)
    except ValueError:
        return None


def _gap_percent(found: int, known: Optional[int], problem_type: Optional[str]) -> Optional[float]:
    if known is None or known == 0:
        return None
    sign = -1.0 if _canonical_problem_type(problem_type) == "MSCTSP" else 1.0
    return round(sign * ((found - known) / abs(known)) * 100.0, 6)


def _objective(report: Dict[str, Any], case: CaseInfo) -> Dict[str, Any]:
    metric = _case_objective_metric(case)
    source_field = "best_penalty" if metric == "penalty" else "best_cost"
    found = report["best_penalty"] if metric == "penalty" else report["best_cost"]
    comparable = case.known_optimum is not None
    if metric == "cost" and report["best_penalty"] != 0:
        comparable = False
    return {
        "metric": metric,
        "source_field": source_field,
        "label": _case_objective_label(case),
        "known_label": _case_known_objective_label(case),
        "known": case.known_optimum,
        "best": found,
        "found": found,
        "comparable": comparable,
        "gap_percent": _gap_percent(found, case.known_optimum, case.problem_type)
        if comparable
        else None,
    }


def _solution(report: Dict[str, Any], case: CaseInfo, parameter_text: str) -> Dict[str, Any]:
    problem_type = _canonical_problem_type(case.problem_type)
    if problem_type == "OKTSP":
        return _oktsp_solution(report, case, parameter_text)
    return _tour_solution(report, case)


def _tour_solution(report: Dict[str, Any], case: CaseInfo) -> Dict[str, Any]:
    nodes = [int(node) for node in report.get("tour", [])]
    edges = _edges_for_nodes(nodes, closed=True, problem_file=case.problem_file)
    edge_cost_sum = _edge_cost_sum(edges)
    return {
        "kind": "tour",
        "closed": True,
        "nodes": nodes,
        "edges": edges,
        "edge_cost_sum": edge_cost_sum,
        "matches_objective": edge_cost_sum == int(report["best_cost"])
        if edge_cost_sum is not None
        else None,
    }


def _oktsp_solution(
    report: Dict[str, Any],
    case: CaseInfo,
    parameter_text: str,
) -> Dict[str, Any]:
    values = _parameter_values(parameter_text)
    tour = [int(node) for node in report.get("tour", [])]
    if not tour:
        return {"kind": "path", "closed": False, "nodes": [], "edges": []}

    k = _parse_int(values.get("K")) or len(tour)
    depot = _parse_int(values.get("DEPOT")) or _parse_int(values.get("MTSP_DEPOT")) or 1
    candidates: List[List[int]] = []
    if depot in tour:
        candidates.append(_rotate_to(tour, depot))
        candidates.append(_rotate_to(list(reversed(tour)), depot))
    else:
        candidates.append(tour)

    best_nodes = candidates[0][: min(k, len(candidates[0]))]
    best_edges = _edges_for_nodes(best_nodes, closed=False, problem_file=case.problem_file)
    best_delta: Optional[int] = None
    for candidate in candidates:
        nodes = candidate[: min(k, len(candidate))]
        edges = _edges_for_nodes(nodes, closed=False, problem_file=case.problem_file)
        cost = _edge_cost_sum(edges)
        delta = None if cost is None else abs(cost - int(report["best_penalty"]))
        if delta == 0:
            best_nodes = nodes
            best_edges = edges
            best_delta = delta
            break
        if best_delta is None or (delta is not None and delta < best_delta):
            best_nodes = nodes
            best_edges = edges
            best_delta = delta

    return {
        "kind": "path",
        "closed": False,
        "nodes": best_nodes,
        "edges": best_edges,
        "depot": depot,
        "k": k,
        "edge_cost_sum": _edge_cost_sum(best_edges),
        "matches_objective": _edge_cost_sum(best_edges) == int(report["best_penalty"])
        if _edge_cost_sum(best_edges) is not None
        else None,
    }


def _rotate_to(nodes: List[int], start: int) -> List[int]:
    index = nodes.index(start)
    return nodes[index:] + nodes[:index]


def _edges_for_nodes(
    nodes: List[int],
    *,
    closed: bool,
    problem_file: Optional[Path],
) -> List[Dict[str, Any]]:
    if len(nodes) < 2:
        return []
    matrix = _explicit_full_matrix(problem_file)
    pairs = list(zip(nodes, nodes[1:]))
    if closed:
        pairs.append((nodes[-1], nodes[0]))
    edges: List[Dict[str, Any]] = []
    for from_node, to_node in pairs:
        edge: Dict[str, Any] = {"from": from_node, "to": to_node}
        if matrix and 1 <= from_node <= len(matrix) and 1 <= to_node <= len(matrix):
            edge["cost"] = matrix[from_node - 1][to_node - 1]
        edges.append(edge)
    return edges


def _edge_cost_sum(edges: List[Dict[str, Any]]) -> Optional[int]:
    total = 0
    for edge in edges:
        cost = edge.get("cost")
        if cost is None:
            return None
        total += int(cost)
    return total


def _explicit_full_matrix(path: Optional[Path]) -> Optional[List[List[int]]]:
    if path is None or not path.exists():
        return None
    metadata = _problem_metadata(path)
    if metadata.get("EDGE_WEIGHT_TYPE") != "EXPLICIT":
        return None
    text = path.read_text(encoding="utf-8", errors="replace")
    values = _parameter_values(text)
    if values.get("EDGE_WEIGHT_FORMAT", "FULL_MATRIX").upper() != "FULL_MATRIX":
        return None
    dimension = _parse_int(metadata.get("DIMENSION"))
    if dimension is None or dimension <= 0:
        return None
    numbers: List[int] = []
    in_section = False
    for line in text.splitlines():
        stripped = line.strip()
        if stripped == "EDGE_WEIGHT_SECTION":
            in_section = True
            continue
        if not in_section:
            continue
        if not stripped or stripped == "EOF" or stripped.endswith("_SECTION"):
            break
        for token in stripped.split():
            try:
                numbers.append(int(float(token)))
            except ValueError:
                return None
    if len(numbers) < dimension * dimension:
        return None
    return [
        numbers[index * dimension : (index + 1) * dimension]
        for index in range(dimension)
    ]


def _objective_metric(problem_type: Optional[str]) -> str:
    if _canonical_problem_type(problem_type) in PENALTY_OBJECTIVE_TYPES:
        return "penalty"
    return "cost"


def _case_objective_metric(case: CaseInfo) -> str:
    if case.mtsp_objective in {"MINMAX", "MINMAX_SIZE"}:
        return "penalty"
    return _objective_metric(case.problem_type)


def _case_objective_label(case: CaseInfo) -> str:
    if case.mtsp_objective == "MINMAX":
        return "Longest route"
    if case.mtsp_objective == "MINMAX_SIZE":
        return "Largest route size"
    if case.mtsp_objective == "MINSUM":
        return "Total route cost"
    return "Objective"


def _case_known_objective_label(case: CaseInfo) -> str:
    if case.mtsp_objective == "MINMAX":
        return "Known longest route"
    if case.mtsp_objective == "MINMAX_SIZE":
        return "Known largest route size"
    if case.mtsp_objective == "MINSUM":
        return "Known total route cost"
    return "Known objective"


def _canonical_problem_type(problem_type: Optional[str]) -> Optional[str]:
    if not problem_type:
        return None
    token = re.split(r"[\s(]", problem_type.strip(), maxsplit=1)[0]
    return token.upper().replace("-", "_")


def _canonical_keyword_value(value: Optional[str]) -> Optional[str]:
    if value is None:
        return None
    return value.strip().upper().replace("-", "_")


def _validate_parameter_key(key: str) -> str:
    normalized = key.upper()
    if not re.fullmatch(r"[A-Z][A-Z0-9_]*", normalized):
        raise ValueError(f"invalid parameter key: {key}")
    return normalized


def _validate_parameter_value(value: Union[int, float, str]) -> str:
    text = str(value)
    if not text or "\n" in text or "\r" in text or "\0" in text:
        raise ValueError("parameter override values must be single-line values")
    return text
