//! In-memory problem and search-parameter models.
//!
//! This module is the safe Rust representation that sits above LKH's TSPLIB
//! parser. Callers build `RoutingProblem` and `SearchParameters` values in
//! memory; rendering to TSPLIB or `.par` text is an explicit export step or an
//! implementation detail of the native solver bridge.

use crate::error::LkhError;
use std::fmt::Write as _;
use std::path::Path;

const DEFAULT_PROBLEM_NAME: &str = "lkh_rs_problem";

/// A 2-D point used by coordinate-based TSP problems.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2d {
    /// X coordinate passed to LKH.
    pub x: f64,
    /// Y coordinate passed to LKH.
    pub y: f64,
}

impl Point2d {
    /// Create a 2-D point.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

impl From<(f64, f64)> for Point2d {
    fn from((x, y): (f64, f64)) -> Self {
        Self::new(x, y)
    }
}

impl From<[f64; 2]> for Point2d {
    fn from([x, y]: [f64; 2]) -> Self {
        Self::new(x, y)
    }
}

/// The TSPLIB/LKH problem class represented by a programmatic problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProblemKind {
    Tsp,
    Atsp,
    Sop,
    Hcp,
    Hpp,
    Bwtsp,
    Cluvrp,
    Ccvrp,
    Cvrp,
    Dcvrp,
    Acvrp,
    Adcvrp,
    Cvrptw,
    Ktsp,
    Mlp,
    Msctsp,
    Oktsp,
    Op,
    Ovrp,
    Pctsp,
    PcTsp,
    Pdptw,
    Pdtsp,
    Pdtspf,
    Pdtspl,
    Ptp,
    Ptsp,
    Top,
    Trp,
    Mtrp,
    Mtrpd,
    Rctvrp,
    Rctvrptw,
    SoftCluvrp,
    Sttsp,
    Tsptw,
    Vrpb,
    Vrpbtw,
    Vrpspd,
    Vrpspdtw,
    Vrpmpd,
    Vrpmpdtw,
    Mvrpb,
    OnePdtsp,
    MPdtsp,
    M1Pdtsp,
    Tspdl,
    Tsppd,
    Ctsp,
    CtspD,
    Gctsp,
    Ccctsp,
    Cbtsp,
    Cbntsp,
    Tspmd,
    /// Escape hatch for future or locally patched LKH `TYPE` values.
    Custom(String),
}

impl ProblemKind {
    /// All problem type spellings accepted by the vendored LKH parser.
    ///
    /// `TOUR` is intentionally omitted because upstream recognizes it only to
    /// abort with "Type not implemented".
    pub const ALL: &'static [Self] = &[
        Self::Tsp,
        Self::Atsp,
        Self::Sop,
        Self::Hcp,
        Self::Hpp,
        Self::Bwtsp,
        Self::Cluvrp,
        Self::Ccvrp,
        Self::Cvrp,
        Self::Dcvrp,
        Self::Acvrp,
        Self::Adcvrp,
        Self::Cvrptw,
        Self::Ktsp,
        Self::Mlp,
        Self::Msctsp,
        Self::Oktsp,
        Self::Op,
        Self::Ovrp,
        Self::Pctsp,
        Self::PcTsp,
        Self::Pdptw,
        Self::Pdtsp,
        Self::Pdtspf,
        Self::Pdtspl,
        Self::Ptp,
        Self::Ptsp,
        Self::Top,
        Self::Trp,
        Self::Mtrp,
        Self::Mtrpd,
        Self::Rctvrp,
        Self::Rctvrptw,
        Self::SoftCluvrp,
        Self::Sttsp,
        Self::Tsptw,
        Self::Vrpb,
        Self::Vrpbtw,
        Self::Vrpspd,
        Self::Vrpspdtw,
        Self::Vrpmpd,
        Self::Vrpmpdtw,
        Self::Mvrpb,
        Self::OnePdtsp,
        Self::MPdtsp,
        Self::M1Pdtsp,
        Self::Tspdl,
        Self::Tsppd,
        Self::Ctsp,
        Self::CtspD,
        Self::Gctsp,
        Self::Ccctsp,
        Self::Cbtsp,
        Self::Cbntsp,
        Self::Tspmd,
    ];

    /// Build a validated custom problem type.
    ///
    /// Prefer the built-in variants for known LKH types. This escape hatch is
    /// for locally patched or future upstream `TYPE` values.
    pub fn custom(value: impl Into<String>) -> Result<Self, LkhError> {
        Ok(Self::Custom(validate_type_name(value.into())?))
    }

    /// Return the exact `TYPE` spelling expected by the LKH problem parser.
    pub fn as_tsplib_type(&self) -> &str {
        match self {
            Self::Tsp => "TSP",
            Self::Atsp => "ATSP",
            Self::Sop => "SOP",
            Self::Hcp => "HCP",
            Self::Hpp => "HPP",
            Self::Bwtsp => "BWTSP",
            Self::Cluvrp => "CLUVRP",
            Self::Ccvrp => "CCVRP",
            Self::Cvrp => "CVRP",
            Self::Dcvrp => "DCVRP",
            Self::Acvrp => "ACVRP",
            Self::Adcvrp => "ADCVRP",
            Self::Cvrptw => "CVRPTW",
            Self::Ktsp => "KTSP",
            Self::Mlp => "MLP",
            Self::Msctsp => "MSCTSP",
            Self::Oktsp => "OKTSP",
            Self::Op => "OP",
            Self::Ovrp => "OVRP",
            Self::Pctsp => "PCTSP",
            Self::PcTsp => "PC-TSP",
            Self::Pdptw => "PDPTW",
            Self::Pdtsp => "PDTSP",
            Self::Pdtspf => "PDTSPF",
            Self::Pdtspl => "PDTSPL",
            Self::Ptp => "PTP",
            Self::Ptsp => "PTSP",
            Self::Top => "TOP",
            Self::Trp => "TRP",
            Self::Mtrp => "MTRP",
            Self::Mtrpd => "MTRPD",
            Self::Rctvrp => "RCTVRP",
            Self::Rctvrptw => "RCTVRPTW",
            Self::SoftCluvrp => "SOFTCLUVRP",
            Self::Sttsp => "STTSP",
            Self::Tsptw => "TSPTW",
            Self::Vrpb => "VRPB",
            Self::Vrpbtw => "VRPBTW",
            Self::Vrpspd => "VRPSPD",
            Self::Vrpspdtw => "VRPSPDTW",
            Self::Vrpmpd => "VRPMPD",
            Self::Vrpmpdtw => "VRPMPDTW",
            Self::Mvrpb => "MVRPB",
            Self::OnePdtsp => "1-PDTSP",
            Self::MPdtsp => "M-PDTSP",
            Self::M1Pdtsp => "M1-PDTSP",
            Self::Tspdl => "TSPDL",
            Self::Tsppd => "TSPPD",
            Self::Ctsp => "CTSP",
            Self::CtspD => "CTSP-D",
            Self::Gctsp => "GCTSP",
            Self::Ccctsp => "CCCTSP",
            Self::Cbtsp => "CBTSP",
            Self::Cbntsp => "CBNTSP",
            Self::Tspmd => "TSPMD",
            Self::Custom(value) => value,
        }
    }
}

/// One ordered entry in the generated TSPLIB/LKH problem file.
///
/// LKH problem files are order-sensitive for some variants because sections
/// must appear after their defining scalar keywords. `RoutingProblem` preserves
/// insertion order by storing entries as this flat list.
#[derive(Debug, Clone, PartialEq)]
pub enum ProblemEntry {
    /// A scalar `KEY: VALUE` problem specification.
    Keyword { key: String, value: String },
    /// A multiline section such as `NODE_COORD_SECTION`.
    Section { key: String, lines: Vec<String> },
}

/// A routing problem that can be built without parameter or problem files.
///
/// The model is intentionally independent of any filesystem. Native solving
/// feeds LKH's parser from memory, and TSPLIB text is only an export format for
/// interoperability.
#[derive(Debug, Clone, PartialEq)]
pub struct RoutingProblem {
    name: String,
    kind: ProblemKind,
    dimension: usize,
    entries: Vec<ProblemEntry>,
}

impl RoutingProblem {
    /// Build an empty problem of any LKH problem kind.
    pub fn new(kind: ProblemKind, dimension: usize) -> Result<Self, LkhError> {
        Self::named(DEFAULT_PROBLEM_NAME, kind, dimension)
    }

    /// Build a named empty problem of any LKH problem kind.
    pub fn named(
        name: impl Into<String>,
        kind: ProblemKind,
        dimension: usize,
    ) -> Result<Self, LkhError> {
        let name = validate_name(name.into())?;
        validate_dimension(dimension)?;
        Ok(Self {
            name,
            kind,
            dimension,
            entries: Vec::new(),
        })
    }

    /// Build a problem with a custom `TYPE` string.
    pub fn custom_type(
        name: impl Into<String>,
        type_name: impl Into<String>,
        dimension: usize,
    ) -> Result<Self, LkhError> {
        Self::named(name, ProblemKind::custom(type_name)?, dimension)
    }

    /// Build a symmetric TSP from 2-D coordinates using LKH's `EUC_2D` metric.
    pub fn euclidean_2d<I, P>(points: I) -> Result<Self, LkhError>
    where
        I: IntoIterator<Item = P>,
        P: Into<Point2d>,
    {
        Self::named_euclidean_2d(DEFAULT_PROBLEM_NAME, points)
    }

    /// Build a named symmetric TSP from 2-D coordinates.
    pub fn named_euclidean_2d<I, P>(name: impl Into<String>, points: I) -> Result<Self, LkhError>
    where
        I: IntoIterator<Item = P>,
        P: Into<Point2d>,
    {
        let name = validate_name(name.into())?;
        let points = points.into_iter().map(Into::into).collect::<Vec<_>>();
        validate_points(&points)?;
        let lines = points
            .iter()
            .enumerate()
            .map(|(index, point)| format!("{} {} {}", index + 1, point.x, point.y))
            .collect::<Vec<_>>();
        Self::named(name, ProblemKind::Tsp, points.len())?
            .with_keyword("EDGE_WEIGHT_TYPE", "EUC_2D")?
            .with_section("NODE_COORD_SECTION", lines)
    }

    /// Build a symmetric TSP from a full square distance matrix.
    pub fn distance_matrix(matrix: Vec<Vec<i64>>) -> Result<Self, LkhError> {
        Self::named_distance_matrix(DEFAULT_PROBLEM_NAME, matrix)
    }

    /// Build a named symmetric TSP from a full square distance matrix.
    pub fn named_distance_matrix(
        name: impl Into<String>,
        matrix: Vec<Vec<i64>>,
    ) -> Result<Self, LkhError> {
        Self::named_explicit_matrix(name, matrix, false)
    }

    /// Build an asymmetric TSP from a full square distance matrix.
    pub fn asymmetric_distance_matrix(matrix: Vec<Vec<i64>>) -> Result<Self, LkhError> {
        Self::named_asymmetric_distance_matrix(DEFAULT_PROBLEM_NAME, matrix)
    }

    /// Build a named asymmetric TSP from a full square distance matrix.
    pub fn named_asymmetric_distance_matrix(
        name: impl Into<String>,
        matrix: Vec<Vec<i64>>,
    ) -> Result<Self, LkhError> {
        Self::named_explicit_matrix(name, matrix, true)
    }

    fn named_explicit_matrix(
        name: impl Into<String>,
        matrix: Vec<Vec<i64>>,
        asymmetric: bool,
    ) -> Result<Self, LkhError> {
        let name = validate_name(name.into())?;
        validate_matrix(&matrix, asymmetric)?;
        let lines = matrix
            .iter()
            .map(|row| {
                row.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>();
        let kind = if asymmetric {
            ProblemKind::Atsp
        } else {
            ProblemKind::Tsp
        };
        Self::named(name, kind, matrix.len())?
            .with_keyword("EDGE_WEIGHT_TYPE", "EXPLICIT")?
            .with_keyword("EDGE_WEIGHT_FORMAT", "FULL_MATRIX")?
            .with_section("EDGE_WEIGHT_SECTION", lines)
    }

    /// Return the problem name rendered into the TSPLIB header.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the declared node dimension.
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Return a cloned problem kind.
    pub fn kind(&self) -> ProblemKind {
        self.kind.clone()
    }

    /// Borrow the problem kind without cloning.
    pub fn kind_ref(&self) -> &ProblemKind {
        &self.kind
    }

    /// Return the ordered problem entries that will be rendered after the
    /// standard `NAME`, `TYPE`, `COMMENT`, and `DIMENSION` header.
    pub fn entries(&self) -> &[ProblemEntry] {
        &self.entries
    }

    /// Add or override a scalar TSPLIB/LKH problem keyword.
    pub fn with_keyword(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, LkhError> {
        let key = validate_problem_key(key.into())?;
        let value = validate_problem_value(value.into())?;
        self.entries.push(ProblemEntry::Keyword { key, value });
        Ok(self)
    }

    /// Add a raw TSPLIB/LKH data section.
    pub fn with_section<I, S>(mut self, key: impl Into<String>, lines: I) -> Result<Self, LkhError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let key = validate_problem_key(key.into())?;
        let lines = lines
            .into_iter()
            .map(|line| validate_section_line(line.into()))
            .collect::<Result<Vec<_>, _>>()?;
        self.entries.push(ProblemEntry::Section { key, lines });
        Ok(self)
    }

    /// Render this problem as TSPLIB text in memory.
    pub fn to_tsplib(&self) -> String {
        let mut output = String::new();
        writeln!(&mut output, "NAME: {}", self.name).unwrap();
        writeln!(&mut output, "TYPE: {}", self.kind.as_tsplib_type()).unwrap();
        writeln!(&mut output, "COMMENT: generated by LKH-rs programmatic API").unwrap();
        writeln!(&mut output, "DIMENSION: {}", self.dimension).unwrap();
        for entry in &self.entries {
            match entry {
                ProblemEntry::Keyword { key, value } => {
                    writeln!(&mut output, "{key}: {value}").unwrap();
                }
                ProblemEntry::Section { key, lines } => {
                    writeln!(&mut output, "{key}").unwrap();
                    for line in lines {
                        writeln!(&mut output, "{line}").unwrap();
                    }
                }
            }
        }
        writeln!(&mut output, "EOF").unwrap();
        output
    }

    /// Write this problem as a TSPLIB file.
    ///
    /// This is an explicit export adapter. It is not used by
    /// `solve_problem`, which initializes LKH directly from memory.
    pub fn write_tsplib(&self, path: impl AsRef<Path>) -> Result<(), LkhError> {
        write_text(path.as_ref(), self.to_tsplib())
    }
}

/// Search settings for programmatic solves.
///
/// The defaults are intentionally lightweight and quiet (`RUNS = 1`,
/// `TRACE_LEVEL = 0`). Heavier benchmark-style runs can be requested by setting
/// the corresponding fields.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchParameters {
    /// Number of independent LKH runs.
    pub runs: i32,
    /// Native LKH trace level; zero keeps programmatic solves quiet.
    pub trace_level: i32,
    /// Maximum trials per run. `None` lets LKH choose its default.
    pub max_trials: Option<i32>,
    /// LKH move type. `None` keeps the upstream default.
    pub move_type: Option<i32>,
    /// Patching C parameter. `None` keeps the upstream default.
    pub patching_c: Option<i32>,
    /// Patching A parameter. `None` keeps the upstream default.
    pub patching_a: Option<i32>,
    /// Random seed used by LKH.
    pub seed: Option<u32>,
    /// Per-run time limit in seconds.
    pub time_limit: Option<f64>,
    /// Total time limit in seconds across all runs.
    pub total_time_limit: Option<f64>,
    /// Known optimum used by LKH for reporting and optional early stopping.
    pub optimum: Option<i64>,
    /// Whether LKH should stop when the optimum is reached.
    pub stop_at_optimum: Option<bool>,
    /// Expert escape hatch for LKH parameters not yet modeled directly.
    ///
    /// Unknown or incompatible keywords can still terminate inside upstream C,
    /// so prefer typed fields when available.
    pub additional_parameters: Vec<(String, String)>,
}

impl Default for SearchParameters {
    fn default() -> Self {
        Self {
            runs: 1,
            trace_level: 0,
            max_trials: None,
            move_type: None,
            patching_c: None,
            patching_a: None,
            seed: None,
            time_limit: None,
            total_time_limit: None,
            optimum: None,
            stop_at_optimum: None,
            additional_parameters: Vec::new(),
        }
    }
}

impl SearchParameters {
    /// Create default lightweight search settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a copy with a custom run count.
    pub fn with_runs(mut self, runs: i32) -> Self {
        self.runs = runs;
        self
    }

    /// Return a copy with a custom native trace level.
    pub fn with_trace_level(mut self, trace_level: i32) -> Self {
        self.trace_level = trace_level;
        self
    }

    /// Return a copy with a maximum trial count.
    pub fn with_max_trials(mut self, max_trials: i32) -> Self {
        self.max_trials = Some(max_trials);
        self
    }

    /// Return a copy with a custom LKH move type.
    pub fn with_move_type(mut self, move_type: i32) -> Self {
        self.move_type = Some(move_type);
        self
    }

    /// Return a copy with a custom `PATCHING_C` value.
    pub fn with_patching_c(mut self, patching_c: i32) -> Self {
        self.patching_c = Some(patching_c);
        self
    }

    /// Return a copy with a custom `PATCHING_A` value.
    pub fn with_patching_a(mut self, patching_a: i32) -> Self {
        self.patching_a = Some(patching_a);
        self
    }

    /// Return a copy with a deterministic LKH seed.
    pub fn with_seed(mut self, seed: u32) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Return a copy with a per-run time limit in seconds.
    pub fn with_time_limit(mut self, seconds: f64) -> Self {
        self.time_limit = Some(seconds);
        self
    }

    /// Return a copy with a total time limit in seconds.
    pub fn with_total_time_limit(mut self, seconds: f64) -> Self {
        self.total_time_limit = Some(seconds);
        self
    }

    /// Return a copy with a known optimum.
    pub fn with_optimum(mut self, optimum: i64) -> Self {
        self.optimum = Some(optimum);
        self
    }

    /// Return a copy with `STOP_AT_OPTIMUM` enabled or disabled.
    pub fn with_stop_at_optimum(mut self, stop_at_optimum: bool) -> Self {
        self.stop_at_optimum = Some(stop_at_optimum);
        self
    }

    /// Add a native LKH parameter not yet modeled as a typed field.
    ///
    /// This is intentionally an expert escape hatch. Prefer typed fields so the
    /// Rust layer can validate values before handing them to upstream C.
    pub fn with_lkh_parameter(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, LkhError> {
        let key = validate_parameter_key(key.into())?;
        let value = validate_parameter_value(value.into())?;
        self.additional_parameters.push((key, value));
        Ok(self)
    }

    /// Validate search settings without running LKH.
    pub fn validate(&self) -> Result<(), LkhError> {
        if self.runs <= 0 {
            return Err(LkhError::InvalidSearchParameters(
                "runs must be positive".to_owned(),
            ));
        }
        if let Some(max_trials) = self.max_trials {
            if max_trials < 0 {
                return Err(LkhError::InvalidSearchParameters(
                    "max_trials must be non-negative".to_owned(),
                ));
            }
        }
        if let Some(move_type) = self.move_type {
            if move_type < 2 {
                return Err(LkhError::InvalidSearchParameters(
                    "move_type must be at least 2".to_owned(),
                ));
            }
        }
        if let Some(patching_c) = self.patching_c {
            if patching_c < 0 {
                return Err(LkhError::InvalidSearchParameters(
                    "patching_c must be non-negative".to_owned(),
                ));
            }
        }
        if let Some(patching_a) = self.patching_a {
            if patching_a < 0 {
                return Err(LkhError::InvalidSearchParameters(
                    "patching_a must be non-negative".to_owned(),
                ));
            }
        }
        validate_optional_seconds("time_limit", self.time_limit)?;
        validate_optional_seconds("total_time_limit", self.total_time_limit)?;
        for (key, value) in &self.additional_parameters {
            validate_parameter_key(key.clone())?;
            validate_parameter_value(value.clone())?;
        }
        Ok(())
    }

    /// Render an LKH parameter file in memory for a given problem file name.
    pub fn to_lkh_parameter_file(&self, problem_file_name: &str) -> Result<String, LkhError> {
        validate_problem_file_name(problem_file_name)?;
        self.validate()?;

        let mut output = String::new();
        writeln!(&mut output, "PROBLEM_FILE = {problem_file_name}").unwrap();
        writeln!(&mut output, "RUNS = {}", self.runs).unwrap();
        writeln!(&mut output, "TRACE_LEVEL = {}", self.trace_level).unwrap();
        if let Some(max_trials) = self.max_trials {
            writeln!(&mut output, "MAX_TRIALS = {max_trials}").unwrap();
        }
        if let Some(move_type) = self.move_type {
            writeln!(&mut output, "MOVE_TYPE = {move_type}").unwrap();
        }
        if let Some(patching_c) = self.patching_c {
            writeln!(&mut output, "PATCHING_C = {patching_c}").unwrap();
        }
        if let Some(patching_a) = self.patching_a {
            writeln!(&mut output, "PATCHING_A = {patching_a}").unwrap();
        }
        if let Some(seed) = self.seed {
            writeln!(&mut output, "SEED = {seed}").unwrap();
        }
        if let Some(time_limit) = self.time_limit {
            writeln!(&mut output, "TIME_LIMIT = {time_limit}").unwrap();
        }
        if let Some(total_time_limit) = self.total_time_limit {
            writeln!(&mut output, "TOTAL_TIME_LIMIT = {total_time_limit}").unwrap();
        }
        if let Some(optimum) = self.optimum {
            writeln!(&mut output, "OPTIMUM = {optimum}").unwrap();
        }
        if let Some(stop_at_optimum) = self.stop_at_optimum {
            let value = if stop_at_optimum { "YES" } else { "NO" };
            writeln!(&mut output, "STOP_AT_OPTIMUM = {value}").unwrap();
        }
        for (key, value) in &self.additional_parameters {
            writeln!(&mut output, "{key} = {value}").unwrap();
        }
        Ok(output)
    }

    /// Write an LKH parameter file for an explicitly exported problem file.
    ///
    /// This is an explicit export adapter. Programmatic solves use typed
    /// parameters directly and do not read this file.
    pub fn write_lkh_parameter_file(
        &self,
        path: impl AsRef<Path>,
        problem_file_name: &str,
    ) -> Result<(), LkhError> {
        let text = self.to_lkh_parameter_file(problem_file_name)?;
        write_text(path.as_ref(), text)
    }
}

fn write_text(path: &Path, text: String) -> Result<(), LkhError> {
    std::fs::write(path, text).map_err(|source| LkhError::WriteFile {
        path: path.to_path_buf(),
        source,
    })
}

fn validate_name(name: String) -> Result<String, LkhError> {
    if name.trim().is_empty() {
        return Err(LkhError::InvalidProblem(
            "problem name must not be empty".to_owned(),
        ));
    }
    if contains_line_break_or_nul(&name) {
        return Err(LkhError::InvalidProblem(
            "problem name must not contain line breaks or NUL bytes".to_owned(),
        ));
    }
    Ok(name)
}

fn validate_type_name(value: String) -> Result<String, LkhError> {
    if value.trim().is_empty() {
        return Err(LkhError::InvalidProblem(
            "problem type must not be empty".to_owned(),
        ));
    }
    if contains_line_break_or_nul(&value) {
        return Err(LkhError::InvalidProblem(
            "problem type must not contain line breaks or NUL bytes".to_owned(),
        ));
    }
    Ok(value)
}

fn validate_dimension(dimension: usize) -> Result<(), LkhError> {
    if dimension < 2 {
        return Err(LkhError::InvalidProblem(
            "dimension must be at least 2".to_owned(),
        ));
    }
    if dimension > i32::MAX as usize {
        return Err(LkhError::InvalidProblem(
            "dimension exceeds i32::MAX".to_owned(),
        ));
    }
    Ok(())
}

fn validate_points(points: &[Point2d]) -> Result<(), LkhError> {
    if points.len() < 2 {
        return Err(LkhError::InvalidProblem(
            "at least two points are required".to_owned(),
        ));
    }
    for (index, point) in points.iter().enumerate() {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(LkhError::InvalidProblem(format!(
                "point {} must contain finite coordinates",
                index + 1
            )));
        }
    }
    Ok(())
}

fn validate_problem_key(key: String) -> Result<String, LkhError> {
    if key.is_empty()
        || !key
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        return Err(LkhError::InvalidProblem(
            "problem keys must use A-Z, 0-9, or _".to_owned(),
        ));
    }
    Ok(key)
}

fn validate_problem_value(value: String) -> Result<String, LkhError> {
    if value.trim().is_empty() {
        return Err(LkhError::InvalidProblem(
            "problem values must not be empty".to_owned(),
        ));
    }
    if contains_line_break_or_nul(&value) {
        return Err(LkhError::InvalidProblem(
            "problem values must not contain line breaks or NUL bytes".to_owned(),
        ));
    }
    Ok(value)
}

fn validate_section_line(line: String) -> Result<String, LkhError> {
    if contains_line_break_or_nul(&line) {
        return Err(LkhError::InvalidProblem(
            "problem section lines must not contain line breaks or NUL bytes".to_owned(),
        ));
    }
    Ok(line)
}

fn validate_matrix(matrix: &[Vec<i64>], asymmetric: bool) -> Result<(), LkhError> {
    let dimension = matrix.len();
    if dimension < 2 {
        return Err(LkhError::InvalidProblem(
            "distance matrix dimension must be at least 2".to_owned(),
        ));
    }
    for (row_index, row) in matrix.iter().enumerate() {
        if row.len() != dimension {
            return Err(LkhError::InvalidProblem(format!(
                "distance matrix row {} has length {}, expected {}",
                row_index + 1,
                row.len(),
                dimension
            )));
        }
        for (column_index, &value) in row.iter().enumerate() {
            if value < 0 {
                return Err(LkhError::InvalidProblem(format!(
                    "distance matrix entry ({}, {}) is negative",
                    row_index + 1,
                    column_index + 1
                )));
            }
            if value > i32::MAX as i64 {
                return Err(LkhError::InvalidProblem(format!(
                    "distance matrix entry ({}, {}) exceeds i32::MAX",
                    row_index + 1,
                    column_index + 1
                )));
            }
        }
        if row[row_index] != 0 {
            return Err(LkhError::InvalidProblem(format!(
                "distance matrix diagonal entry ({}, {}) must be zero",
                row_index + 1,
                row_index + 1
            )));
        }
    }
    if !asymmetric {
        for i in 0..dimension {
            for j in (i + 1)..dimension {
                if matrix[i][j] != matrix[j][i] {
                    return Err(LkhError::InvalidProblem(format!(
                        "symmetric distance matrix entries ({}, {}) and ({}, {}) differ",
                        i + 1,
                        j + 1,
                        j + 1,
                        i + 1
                    )));
                }
            }
        }
    }
    Ok(())
}

fn validate_optional_seconds(name: &str, value: Option<f64>) -> Result<(), LkhError> {
    if let Some(value) = value {
        if !value.is_finite() || value < 0.0 {
            return Err(LkhError::InvalidSearchParameters(format!(
                "{name} must be a finite non-negative number"
            )));
        }
    }
    Ok(())
}

fn validate_problem_file_name(value: &str) -> Result<(), LkhError> {
    if value.trim().is_empty() {
        return Err(LkhError::InvalidSearchParameters(
            "problem file name must not be empty".to_owned(),
        ));
    }
    if contains_line_break_or_nul(value) {
        return Err(LkhError::InvalidSearchParameters(
            "problem file name must not contain line breaks or NUL bytes".to_owned(),
        ));
    }
    Ok(())
}

fn validate_parameter_key(key: String) -> Result<String, LkhError> {
    if key.is_empty()
        || !key
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        return Err(LkhError::InvalidSearchParameters(
            "additional LKH parameter keys must use A-Z, 0-9, or _".to_owned(),
        ));
    }
    Ok(key)
}

fn validate_parameter_value(value: String) -> Result<String, LkhError> {
    if value.trim().is_empty() {
        return Err(LkhError::InvalidSearchParameters(
            "additional LKH parameter values must not be empty".to_owned(),
        ));
    }
    if contains_line_break_or_nul(&value) {
        return Err(LkhError::InvalidSearchParameters(
            "additional LKH parameter values must not contain line breaks or NUL bytes".to_owned(),
        ));
    }
    Ok(value)
}

fn contains_line_break_or_nul(value: &str) -> bool {
    value.bytes().any(|byte| matches!(byte, b'\n' | b'\r' | 0))
}
