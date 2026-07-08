"""Typed Python interface for solving LKH problems from memory.

The public API is intentionally close to the shape Python users know from
OR-Tools: build a problem object, build a search-parameter object, and then call
:func:`solve`. Public configuration uses enums and typed value objects instead
of LKH string constants. The private :mod:`lkh_rs._native` module remains a thin
PyO3 bridge and is not intended for application code.

Common cases should use builders such as :meth:`Problem.tsp_2d`,
:meth:`Problem.distance_matrix`, and :meth:`Problem.cvrp`. Less common LKH
variants can still be represented with :meth:`Problem.raw` by choosing
``ProblemType``, ``ProblemKey``, and ``ProblemSection`` enum members.
"""

from __future__ import annotations

import math
from dataclasses import dataclass, field
from enum import Enum
from typing import Dict, List, Mapping, Optional, Sequence, Tuple, TypedDict, Union

from . import _native


class SolveReport(TypedDict):
    """Summary returned by the native LKH solver."""

    best_cost: int
    best_penalty: int
    runs: int
    dimension: int
    tour: List[int]


class ProblemType(str, Enum):
    """LKH ``TYPE`` values accepted by the in-memory API."""

    TSP = "TSP"
    ATSP = "ATSP"
    SOP = "SOP"
    HCP = "HCP"
    HPP = "HPP"
    BWTSP = "BWTSP"
    CLUVRP = "CLUVRP"
    CCVRP = "CCVRP"
    CVRP = "CVRP"
    DCVRP = "DCVRP"
    ACVRP = "ACVRP"
    ADCVRP = "ADCVRP"
    CVRPTW = "CVRPTW"
    KTSP = "KTSP"
    MLP = "MLP"
    MSCTSP = "MSCTSP"
    OKTSP = "OKTSP"
    OP = "OP"
    OVRP = "OVRP"
    PCTSP = "PCTSP"
    PC_TSP = "PC-TSP"
    PDPTW = "PDPTW"
    PDTSP = "PDTSP"
    PDTSPF = "PDTSPF"
    PDTSPL = "PDTSPL"
    PTP = "PTP"
    PTSP = "PTSP"
    TOP = "TOP"
    TRP = "TRP"
    MTRP = "MTRP"
    MTRPD = "MTRPD"
    RCTVRP = "RCTVRP"
    RCTVRPTW = "RCTVRPTW"
    SOFTCLUVRP = "SOFTCLUVRP"
    STTSP = "STTSP"
    TSPTW = "TSPTW"
    VRPB = "VRPB"
    VRPBTW = "VRPBTW"
    VRPSPD = "VRPSPD"
    VRPSPDTW = "VRPSPDTW"
    VRPMPD = "VRPMPD"
    VRPMPDTW = "VRPMPDTW"
    MVRPB = "MVRPB"
    ONE_PDTSP = "1-PDTSP"
    M_PDTSP = "M-PDTSP"
    M1_PDTSP = "M1-PDTSP"
    TSPDL = "TSPDL"
    TSPPD = "TSPPD"
    CTSP = "CTSP"
    CTSP_D = "CTSP-D"
    GCTSP = "GCTSP"
    CCCTSP = "CCCTSP"
    CBTSP = "CBTSP"
    CBNTSP = "CBNTSP"
    TSPMD = "TSPMD"


class EdgeWeightType(str, Enum):
    """Values for the LKH ``EDGE_WEIGHT_TYPE`` problem keyword."""

    ATT = "ATT"
    CEIL_2D = "CEIL_2D"
    CEIL_3D = "CEIL_3D"
    EUC_2D = "EUC_2D"
    EXACT_2D = "EXACT_2D"
    EUC_3D = "EUC_3D"
    EXACT_3D = "EXACT_3D"
    EXPLICIT = "EXPLICIT"
    FLOOR_2D = "FLOOR_2D"
    FLOOR_3D = "FLOOR_3D"
    MAN_2D = "MAN_2D"
    MAN_3D = "MAN_3D"
    MAX_2D = "MAX_2D"
    MAX_3D = "MAX_3D"
    GEO = "GEO"
    GEOM = "GEOM"
    GEO_MEEUS = "GEO_MEEUS"
    GEOM_MEEUS = "GEOM_MEEUS"
    TOR_2D = "TOR_2D"
    TOR_3D = "TOR_3D"
    XRAY1 = "XRAY1"
    XRAY2 = "XRAY2"
    SPECIAL = "SPECIAL"


class EdgeWeightFormat(str, Enum):
    """Values for the LKH ``EDGE_WEIGHT_FORMAT`` problem keyword."""

    FUNCTION = "FUNCTION"
    FULL_MATRIX = "FULL_MATRIX"
    UPPER_ROW = "UPPER_ROW"
    LOWER_ROW = "LOWER_ROW"
    UPPER_DIAG_ROW = "UPPER_DIAG_ROW"
    LOWER_DIAG_ROW = "LOWER_DIAG_ROW"
    UPPER_COL = "UPPER_COL"
    LOWER_COL = "LOWER_COL"
    UPPER_DIAG_COL = "UPPER_DIAG_COL"
    LOWER_DIAG_COL = "LOWER_DIAG_COL"


class EdgeDataFormat(str, Enum):
    """Values for the LKH ``EDGE_DATA_FORMAT`` problem keyword."""

    EDGE_LIST = "EDGE_LIST"
    ADJ_LIST = "ADJ_LIST"


class NodeCoordType(str, Enum):
    """Values for the LKH ``NODE_COORD_TYPE`` problem keyword."""

    TWOD_COORDS = "TWOD_COORDS"
    THREED_COORDS = "THREED_COORDS"
    NO_COORDS = "NO_COORDS"


class DisplayDataType(str, Enum):
    """Values for the LKH ``DISPLAY_DATA_TYPE`` problem keyword."""

    COORD_DISPLAY = "COORD_DISPLAY"
    TWOD_DISPLAY = "TWOD_DISPLAY"
    NO_DISPLAY = "NO_DISPLAY"


class ProblemKey(str, Enum):
    """Scalar TSPLIB/LKH problem keywords.

    ``NAME``, ``TYPE``, and ``DIMENSION`` are managed by :class:`Problem`
    itself. The remaining scalar fields can be selected here and assigned a
    numeric value or one of the format enums above.
    """

    CAPACITY = "CAPACITY"
    COST_LIMIT = "COST_LIMIT"
    DEMAND_DIMENSION = "DEMAND_DIMENSION"
    DISPLAY_DATA_TYPE = "DISPLAY_DATA_TYPE"
    DISTANCE = "DISTANCE"
    DRONES = "DRONES"
    EDGE_DATA_FORMAT = "EDGE_DATA_FORMAT"
    EDGE_WEIGHT_FORMAT = "EDGE_WEIGHT_FORMAT"
    EDGE_WEIGHT_TYPE = "EDGE_WEIGHT_TYPE"
    ENDURANCE = "ENDURANCE"
    GRID_SIZE = "GRID_SIZE"
    GROUPS = "GROUPS"
    GVRP_SETS = "GVRP_SETS"
    NODE_COORD_TYPE = "NODE_COORD_TYPE"
    RELAXATION_LEVEL = "RELAXATION_LEVEL"
    RISK_THRESHOLD = "RISK_THRESHOLD"
    SALESMEN = "SALESMEN"
    VEHICLES = "VEHICLES"
    SCALE = "SCALE"
    SERVICE_TIME = "SERVICE_TIME"


class ProblemSection(str, Enum):
    """Multiline TSPLIB/LKH problem sections."""

    BACKHAUL = "BACKHAUL_SECTION"
    CTSP_SET = "CTSP_SET_SECTION"
    DEMAND = "DEMAND_SECTION"
    DEPOT = "DEPOT_SECTION"
    DISPLAY_DATA = "DISPLAY_DATA_SECTION"
    DRAFT_LIMIT = "DRAFT_LIMIT_SECTION"
    EDGE_DATA = "EDGE_DATA_SECTION"
    EDGE_WEIGHT = "EDGE_WEIGHT_SECTION"
    FIXED_EDGES = "FIXED_EDGES_SECTION"
    GCTSP = "GCTSP_SECTION"
    GCTSP_SET = "GCTSP_SET_SECTION"
    GROUP = "GROUP_SECTION"
    GVRP_SET = "GVRP_SET_SECTION"
    NODE_COORD = "NODE_COORD_SECTION"
    NODE_PENALTY = "NODE_PENALTY_SECTION"
    NODE_SCORE = "NODE_SCORE_SECTION"
    PICKUP_AND_DELIVERY = "PICKUP_AND_DELIVERY_SECTION"
    REQUIRED_NODES = "REQUIRED_NODES_SECTION"
    SERVICE_TIME = "SERVICE_TIME_SECTION"
    TIME_WINDOW = "TIME_WINDOW_SECTION"
    TOUR = "TOUR_SECTION"


KeywordEnum = Union[
    EdgeWeightType,
    EdgeWeightFormat,
    EdgeDataFormat,
    NodeCoordType,
    DisplayDataType,
]
KeywordValue = Union[int, float, KeywordEnum]
KeywordMap = Mapping[ProblemKey, KeywordValue]
SectionScalar = Union[int, float]
SectionRow = Union[SectionScalar, Sequence[SectionScalar]]
SectionData = Sequence[SectionRow]
SectionMap = Mapping[ProblemSection, SectionData]
Point2D = Tuple[float, float]


@dataclass(frozen=True)
class SearchParameters:
    """Solver search settings for programmatic solves.

    The defaults are small and quiet: one run and no trace output. Create a
    custom instance for heavier benchmark settings or time-bounded searches.
    """

    runs: int = 1
    trace_level: int = 0
    max_trials: Optional[int] = None
    seed: Optional[int] = None
    time_limit: Optional[float] = None
    total_time_limit: Optional[float] = None

    def __post_init__(self) -> None:
        """Validate parameter values before they reach the native solver."""

        if type(self.runs) is not int or self.runs <= 0:
            raise ValueError("runs must be a positive integer")
        if type(self.trace_level) is not int or self.trace_level < 0:
            raise ValueError("trace_level must be a non-negative integer")
        if self.max_trials is not None and (
            type(self.max_trials) is not int or self.max_trials < 0
        ):
            raise ValueError("max_trials must be a non-negative integer")
        if self.seed is not None and (type(self.seed) is not int or self.seed < 0):
            raise ValueError("seed must be a non-negative integer")
        _validate_optional_seconds("time_limit", self.time_limit)
        _validate_optional_seconds("total_time_limit", self.total_time_limit)

    def _native_dict(self) -> Dict[str, Union[int, float]]:
        """Serialize settings for the private PyO3 bridge."""

        data: Dict[str, Union[int, float]] = {
            "runs": self.runs,
            "trace_level": self.trace_level,
        }
        if self.max_trials is not None:
            data["max_trials"] = self.max_trials
        if self.seed is not None:
            data["seed"] = self.seed
        if self.time_limit is not None:
            data["time_limit"] = self.time_limit
        if self.total_time_limit is not None:
            data["total_time_limit"] = self.total_time_limit
        return data


@dataclass(frozen=True)
class Problem:
    """A routing problem represented entirely in memory.

    High-level builders cover common inputs. :meth:`raw` exposes the full LKH
    problem surface while still keeping problem types, keywords, sections, and
    format values enum-based.
    """

    problem_type: ProblemType
    dimension: int
    name: str = "lkh_rs_problem"
    keywords: KeywordMap = field(default_factory=dict)
    sections: SectionMap = field(default_factory=dict)

    def __post_init__(self) -> None:
        """Validate the typed facade before data reaches the native bridge."""

        if not isinstance(self.problem_type, ProblemType):
            raise TypeError("problem_type must be a ProblemType enum value")
        if not isinstance(self.dimension, int) or self.dimension < 2:
            raise ValueError("dimension must be an integer greater than or equal to 2")
        if not isinstance(self.name, str) or not self.name.strip():
            raise ValueError("name must be a non-empty string")
        if any(ch in self.name for ch in "\r\n\0"):
            raise ValueError("name must not contain line breaks or NUL bytes")
        _validate_keywords(self.keywords)
        _validate_sections(self.sections)

    @classmethod
    def tsp_2d(
        cls,
        points: Sequence[Point2D],
        *,
        name: str = "lkh_rs_problem",
        edge_weight_type: EdgeWeightType = EdgeWeightType.EUC_2D,
    ) -> "Problem":
        """Build a symmetric TSP from 2-D coordinates."""

        _validate_points(points)
        rows: List[Tuple[SectionScalar, SectionScalar, SectionScalar]] = [
            (index, point[0], point[1])
            for index, point in enumerate(points, start=1)
        ]
        return cls(
            problem_type=ProblemType.TSP,
            dimension=len(points),
            name=name,
            keywords={ProblemKey.EDGE_WEIGHT_TYPE: edge_weight_type},
            sections={ProblemSection.NODE_COORD: rows},
        )

    @classmethod
    def distance_matrix(
        cls,
        matrix: Sequence[Sequence[int]],
        *,
        name: str = "lkh_rs_problem",
        asymmetric: bool = False,
    ) -> "Problem":
        """Build a TSP or ATSP from a full explicit distance matrix."""

        _validate_matrix(matrix)
        return cls(
            problem_type=ProblemType.ATSP if asymmetric else ProblemType.TSP,
            dimension=len(matrix),
            name=name,
            keywords={
                ProblemKey.EDGE_WEIGHT_TYPE: EdgeWeightType.EXPLICIT,
                ProblemKey.EDGE_WEIGHT_FORMAT: EdgeWeightFormat.FULL_MATRIX,
            },
            sections={ProblemSection.EDGE_WEIGHT: matrix},
        )

    @classmethod
    def cvrp(
        cls,
        distance_matrix: Sequence[Sequence[int]],
        demands: Sequence[int],
        capacity: int,
        *,
        depot: int = 1,
        name: str = "lkh_rs_problem",
    ) -> "Problem":
        """Build a capacitated vehicle routing problem.

        Node ids follow LKH convention and are 1-based. ``demands[0]`` is the
        demand for node 1, which is usually the depot.
        """

        _validate_matrix(distance_matrix)
        if len(demands) != len(distance_matrix):
            raise ValueError("demands length must match the distance matrix dimension")
        if depot < 1 or depot > len(distance_matrix):
            raise ValueError("depot must be a 1-based node id within the matrix")
        demand_rows: List[Tuple[int, int]] = [
            (index, demand) for index, demand in enumerate(demands, start=1)
        ]
        return cls(
            problem_type=ProblemType.CVRP,
            dimension=len(distance_matrix),
            name=name,
            keywords={
                ProblemKey.CAPACITY: capacity,
                ProblemKey.EDGE_WEIGHT_TYPE: EdgeWeightType.EXPLICIT,
                ProblemKey.EDGE_WEIGHT_FORMAT: EdgeWeightFormat.FULL_MATRIX,
            },
            sections={
                ProblemSection.EDGE_WEIGHT: distance_matrix,
                ProblemSection.DEMAND: demand_rows,
                ProblemSection.DEPOT: [depot, -1],
            },
        )

    @classmethod
    def raw(
        cls,
        problem_type: ProblemType,
        dimension: int,
        *,
        name: str = "lkh_rs_problem",
        keywords: Optional[KeywordMap] = None,
        sections: Optional[SectionMap] = None,
    ) -> "Problem":
        """Build a generic LKH problem using typed keywords and sections.

        Section data is provided as numeric rows. For example, a depot section
        is ``[1, -1]`` and an edge-weight matrix is a list of numeric rows.
        """

        return cls(
            problem_type=problem_type,
            dimension=dimension,
            name=name,
            keywords=keywords or {},
            sections=sections or {},
        )

    def with_keyword(
        self,
        key: ProblemKey,
        value: KeywordValue,
    ) -> "Problem":
        """Return a copy with one scalar problem keyword added."""

        return self.with_keywords({key: value})

    def with_keywords(self, keywords: KeywordMap) -> "Problem":
        """Return a copy with several scalar problem keywords added."""

        return Problem(
            problem_type=self.problem_type,
            dimension=self.dimension,
            name=self.name,
            keywords={**self.keywords, **keywords},
            sections=self.sections,
        )

    def with_section(self, key: ProblemSection, rows: SectionData) -> "Problem":
        """Return a copy with one numeric problem section added."""

        sections = dict(self.sections)
        sections[key] = rows
        return Problem(
            problem_type=self.problem_type,
            dimension=self.dimension,
            name=self.name,
            keywords=self.keywords,
            sections=sections,
        )

    def _native_dict(self) -> Dict[str, object]:
        """Serialize problem data for the private PyO3 bridge."""

        return {
            "kind": self.problem_type.value,
            "dimension": self.dimension,
            "name": self.name,
            "keywords": {
                _problem_key_value(key): _keyword_value(value)
                for key, value in self.keywords.items()
            },
            "sections": {
                key.value: _section_lines(rows)
                for key, rows in self.sections.items()
            },
        }


RoutingProblem = Problem


def solve(
    problem: Problem,
    parameters: Optional[SearchParameters] = None,
) -> SolveReport:
    """Solve an in-memory routing problem without temporary files."""

    if not isinstance(problem, Problem):
        raise TypeError("problem must be a Problem instance")
    if parameters is not None and not isinstance(parameters, SearchParameters):
        raise TypeError("parameters must be a SearchParameters instance")
    return _native._solve_problem_data(
        problem._native_dict(),
        (parameters or SearchParameters())._native_dict(),
    )


def solve_parameter_file(path: str) -> SolveReport:
    """Solve an existing LKH ``.par`` file."""

    return _native.solve_parameter_file(path)


def solve_problem(
    problem: Problem,
    parameters: Optional[SearchParameters] = None,
) -> SolveReport:
    """Alias for :func:`solve` kept for discoverability."""

    return solve(problem, parameters)


def solve_euclidean_2d(
    points: Sequence[Point2D],
    parameters: Optional[SearchParameters] = None,
) -> SolveReport:
    """Solve a symmetric TSP from 2-D coordinates."""

    return solve(Problem.tsp_2d(points), parameters)


def solve_distance_matrix(
    matrix: Sequence[Sequence[int]],
    parameters: Optional[SearchParameters] = None,
    *,
    asymmetric: bool = False,
) -> SolveReport:
    """Solve a TSP or ATSP from a full distance matrix."""

    return solve(Problem.distance_matrix(matrix, asymmetric=asymmetric), parameters)


def _validate_keywords(keywords: KeywordMap) -> None:
    for key, value in keywords.items():
        if not isinstance(key, ProblemKey):
            raise TypeError("problem keyword keys must be ProblemKey enum values")
        _keyword_value(value)


def _validate_sections(sections: SectionMap) -> None:
    for key, rows in sections.items():
        if not isinstance(key, ProblemSection):
            raise TypeError("problem section keys must be ProblemSection enum values")
        _section_lines(rows)


def _keyword_value(value: KeywordValue) -> str:
    """Convert typed keyword values to the spelling LKH expects."""

    if isinstance(value, Enum):
        if isinstance(
            value,
            (
                EdgeWeightType,
                EdgeWeightFormat,
                EdgeDataFormat,
                NodeCoordType,
                DisplayDataType,
            ),
        ):
            return str(value.value)
        raise TypeError("unsupported enum value for problem keyword")
    return _format_number(value, "problem keyword value")


def _problem_key_value(key: ProblemKey) -> str:
    """Convert public keyword enums to the spelling accepted by vendored LKH."""

    if key is ProblemKey.ENDURANCE:
        return "ENDURACE"
    return key.value


def _section_lines(rows: SectionData) -> List[str]:
    if isinstance(rows, (str, bytes)):
        raise TypeError("problem section rows must be numeric values or numeric rows")
    lines: List[str] = []
    for row in rows:
        if _is_number(row):
            lines.append(_format_number(row, "problem section value"))
            continue
        if isinstance(row, (str, bytes)):
            raise TypeError("problem section rows must not be strings")
        try:
            values = list(row)
        except TypeError as exc:
            raise TypeError("problem section rows must be numeric sequences") from exc
        if not values:
            raise ValueError("problem section rows must not be empty")
        lines.append(" ".join(_format_number(value, "problem section value") for value in values))
    return lines


def _is_number(value: object) -> bool:
    return type(value) in (int, float)


def _format_number(value: object, context: str) -> str:
    if type(value) is int:
        return str(value)
    if type(value) is float:
        if not math.isfinite(value):
            raise ValueError(f"{context} must be finite")
        return str(value)
    raise TypeError(f"{context} must be an int or float")


def _validate_points(points: Sequence[Point2D]) -> None:
    if len(points) < 2:
        raise ValueError("at least two points are required")
    for index, point in enumerate(points, start=1):
        if len(point) != 2:
            raise ValueError(f"point {index} must contain exactly two coordinates")
        _format_number(point[0], "point coordinate")
        _format_number(point[1], "point coordinate")


def _validate_matrix(matrix: Sequence[Sequence[int]]) -> None:
    dimension = len(matrix)
    if dimension < 2:
        raise ValueError("distance matrix dimension must be at least 2")
    for row_index, row in enumerate(matrix, start=1):
        if len(row) != dimension:
            raise ValueError("distance matrix must be square")
        for value in row:
            if type(value) is not int:
                raise TypeError(f"distance matrix row {row_index} must contain integers")
            if value < 0:
                raise ValueError("distance matrix values must be non-negative")


def _validate_optional_seconds(name: str, value: Optional[float]) -> None:
    if value is None:
        return
    if type(value) not in (int, float) or not math.isfinite(value) or value < 0:
        raise ValueError(f"{name} must be a non-negative finite number")


__all__ = [
    "DisplayDataType",
    "EdgeDataFormat",
    "EdgeWeightFormat",
    "EdgeWeightType",
    "Problem",
    "ProblemKey",
    "ProblemSection",
    "ProblemType",
    "RoutingProblem",
    "SearchParameters",
    "SolveReport",
    "solve",
    "solve_distance_matrix",
    "solve_euclidean_2d",
    "solve_parameter_file",
    "solve_problem",
]
