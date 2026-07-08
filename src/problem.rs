use crate::error::LkhError;
use std::fmt::Write as _;
use std::path::Path;

const DEFAULT_PROBLEM_NAME: &str = "lkh_rs_problem";

/// A 2-D point used by coordinate-based TSP problems.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2d {
    pub x: f64,
    pub y: f64,
}

impl Point2d {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemKind {
    Tsp,
    Atsp,
}

impl ProblemKind {
    fn as_tsplib_type(self) -> &'static str {
        match self {
            Self::Tsp => "TSP",
            Self::Atsp => "ATSP",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EdgeData {
    Euclidean2d(Vec<Point2d>),
    ExplicitMatrix {
        matrix: Vec<Vec<i64>>,
        asymmetric: bool,
    },
}

/// A routing problem that can be built without parameter or problem files.
///
/// The model is intentionally independent of any filesystem. Native solving
/// initializes LKH directly from this model, and TSPLIB text is only an export
/// format for interoperability.
#[derive(Debug, Clone, PartialEq)]
pub struct RoutingProblem {
    name: String,
    edge_data: EdgeData,
}

impl RoutingProblem {
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
        Ok(Self {
            name,
            edge_data: EdgeData::Euclidean2d(points),
        })
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
        Ok(Self {
            name,
            edge_data: EdgeData::ExplicitMatrix { matrix, asymmetric },
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn dimension(&self) -> usize {
        match &self.edge_data {
            EdgeData::Euclidean2d(points) => points.len(),
            EdgeData::ExplicitMatrix { matrix, .. } => matrix.len(),
        }
    }

    pub fn kind(&self) -> ProblemKind {
        match &self.edge_data {
            EdgeData::Euclidean2d(_) => ProblemKind::Tsp,
            EdgeData::ExplicitMatrix { asymmetric, .. } => {
                if *asymmetric {
                    ProblemKind::Atsp
                } else {
                    ProblemKind::Tsp
                }
            }
        }
    }

    pub(crate) fn edge_data(&self) -> &EdgeData {
        &self.edge_data
    }

    /// Render this problem as TSPLIB text in memory.
    pub fn to_tsplib(&self) -> String {
        match &self.edge_data {
            EdgeData::Euclidean2d(points) => self.render_euclidean_2d(points),
            EdgeData::ExplicitMatrix { matrix, .. } => self.render_explicit_matrix(matrix),
        }
    }

    /// Write this problem as a TSPLIB file.
    ///
    /// This is an explicit export adapter. It is not used by
    /// `solve_problem`, which initializes LKH directly from memory.
    pub fn write_tsplib(&self, path: impl AsRef<Path>) -> Result<(), LkhError> {
        write_text(path.as_ref(), self.to_tsplib())
    }

    fn render_euclidean_2d(&self, points: &[Point2d]) -> String {
        let mut output = String::new();
        writeln!(&mut output, "NAME: {}", self.name).unwrap();
        writeln!(&mut output, "TYPE: {}", self.kind().as_tsplib_type()).unwrap();
        writeln!(&mut output, "COMMENT: generated by LKH-rs programmatic API").unwrap();
        writeln!(&mut output, "DIMENSION: {}", points.len()).unwrap();
        writeln!(&mut output, "EDGE_WEIGHT_TYPE: EUC_2D").unwrap();
        writeln!(&mut output, "NODE_COORD_SECTION").unwrap();
        for (index, point) in points.iter().enumerate() {
            writeln!(&mut output, "{} {} {}", index + 1, point.x, point.y).unwrap();
        }
        writeln!(&mut output, "EOF").unwrap();
        output
    }

    fn render_explicit_matrix(&self, matrix: &[Vec<i64>]) -> String {
        let mut output = String::new();
        writeln!(&mut output, "NAME: {}", self.name).unwrap();
        writeln!(&mut output, "TYPE: {}", self.kind().as_tsplib_type()).unwrap();
        writeln!(&mut output, "COMMENT: generated by LKH-rs programmatic API").unwrap();
        writeln!(&mut output, "DIMENSION: {}", matrix.len()).unwrap();
        writeln!(&mut output, "EDGE_WEIGHT_TYPE: EXPLICIT").unwrap();
        writeln!(&mut output, "EDGE_WEIGHT_FORMAT: FULL_MATRIX").unwrap();
        writeln!(&mut output, "EDGE_WEIGHT_SECTION").unwrap();
        for row in matrix {
            for (index, value) in row.iter().enumerate() {
                if index > 0 {
                    output.push(' ');
                }
                write!(&mut output, "{value}").unwrap();
            }
            output.push('\n');
        }
        writeln!(&mut output, "EOF").unwrap();
        output
    }
}

/// Search settings for programmatic solves.
///
/// The defaults are intentionally lightweight and quiet (`RUNS = 1`,
/// `TRACE_LEVEL = 0`). Heavier benchmark-style runs can be requested by setting
/// the corresponding fields.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchParameters {
    pub runs: i32,
    pub trace_level: i32,
    pub max_trials: Option<i32>,
    pub move_type: Option<i32>,
    pub patching_c: Option<i32>,
    pub patching_a: Option<i32>,
    pub seed: Option<u32>,
    pub time_limit: Option<f64>,
    pub total_time_limit: Option<f64>,
    pub optimum: Option<i64>,
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_runs(mut self, runs: i32) -> Self {
        self.runs = runs;
        self
    }

    pub fn with_trace_level(mut self, trace_level: i32) -> Self {
        self.trace_level = trace_level;
        self
    }

    pub fn with_max_trials(mut self, max_trials: i32) -> Self {
        self.max_trials = Some(max_trials);
        self
    }

    pub fn with_move_type(mut self, move_type: i32) -> Self {
        self.move_type = Some(move_type);
        self
    }

    pub fn with_patching_c(mut self, patching_c: i32) -> Self {
        self.patching_c = Some(patching_c);
        self
    }

    pub fn with_patching_a(mut self, patching_a: i32) -> Self {
        self.patching_a = Some(patching_a);
        self
    }

    pub fn with_seed(mut self, seed: u32) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn with_time_limit(mut self, seconds: f64) -> Self {
        self.time_limit = Some(seconds);
        self
    }

    pub fn with_total_time_limit(mut self, seconds: f64) -> Self {
        self.total_time_limit = Some(seconds);
        self
    }

    pub fn with_optimum(mut self, optimum: i64) -> Self {
        self.optimum = Some(optimum);
        self
    }

    pub fn with_stop_at_optimum(mut self, stop_at_optimum: bool) -> Self {
        self.stop_at_optimum = Some(stop_at_optimum);
        self
    }

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
