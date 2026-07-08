use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

#[pymodule]
mod _native {
    use super::*;

    #[pyfunction]
    fn solve_parameter_file(py: Python<'_>, path: &str) -> PyResult<Py<PyDict>> {
        let report = crate::solve_parameter_file(path).map_err(to_py_error)?;
        report_to_dict(py, report)
    }

    #[pyfunction]
    #[pyo3(signature = (points, *, runs=1, trace_level=0, max_trials=None, seed=None, time_limit=None, total_time_limit=None))]
    fn solve_euclidean_2d(
        py: Python<'_>,
        points: Vec<(f64, f64)>,
        runs: i32,
        trace_level: i32,
        max_trials: Option<i32>,
        seed: Option<u32>,
        time_limit: Option<f64>,
        total_time_limit: Option<f64>,
    ) -> PyResult<Py<PyDict>> {
        let problem = crate::RoutingProblem::euclidean_2d(points).map_err(to_py_error)?;
        let parameters = search_parameters(
            runs,
            trace_level,
            max_trials,
            seed,
            time_limit,
            total_time_limit,
        )?;
        let report = crate::solve_problem(&problem, &parameters).map_err(to_py_error)?;
        report_to_dict(py, report)
    }

    #[pyfunction]
    #[pyo3(signature = (matrix, *, asymmetric=false, runs=1, trace_level=0, max_trials=None, seed=None, time_limit=None, total_time_limit=None))]
    fn solve_distance_matrix(
        py: Python<'_>,
        matrix: Vec<Vec<i64>>,
        asymmetric: bool,
        runs: i32,
        trace_level: i32,
        max_trials: Option<i32>,
        seed: Option<u32>,
        time_limit: Option<f64>,
        total_time_limit: Option<f64>,
    ) -> PyResult<Py<PyDict>> {
        let problem = if asymmetric {
            crate::RoutingProblem::asymmetric_distance_matrix(matrix)
        } else {
            crate::RoutingProblem::distance_matrix(matrix)
        }
        .map_err(to_py_error)?;
        let parameters = search_parameters(
            runs,
            trace_level,
            max_trials,
            seed,
            time_limit,
            total_time_limit,
        )?;
        let report = crate::solve_problem(&problem, &parameters).map_err(to_py_error)?;
        report_to_dict(py, report)
    }

    fn report_to_dict(py: Python<'_>, report: crate::SolveReport) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("best_cost", report.best_cost)?;
        dict.set_item("best_penalty", report.best_penalty)?;
        dict.set_item("runs", report.runs)?;
        dict.set_item("dimension", report.dimension)?;
        dict.set_item("tour", report.tour)?;
        Ok(dict.into())
    }

    fn search_parameters(
        runs: i32,
        trace_level: i32,
        max_trials: Option<i32>,
        seed: Option<u32>,
        time_limit: Option<f64>,
        total_time_limit: Option<f64>,
    ) -> PyResult<crate::SearchParameters> {
        let mut parameters = crate::SearchParameters::new()
            .with_runs(runs)
            .with_trace_level(trace_level);
        if let Some(max_trials) = max_trials {
            parameters.max_trials = Some(max_trials);
        }
        if let Some(seed) = seed {
            parameters.seed = Some(seed);
        }
        if let Some(time_limit) = time_limit {
            parameters.time_limit = Some(time_limit);
        }
        if let Some(total_time_limit) = total_time_limit {
            parameters.total_time_limit = Some(total_time_limit);
        }
        parameters.validate().map_err(to_py_error)?;
        Ok(parameters)
    }

    fn to_py_error(err: crate::LkhError) -> PyErr {
        match err {
            crate::LkhError::ParameterFileNotFound(_)
            | crate::LkhError::NonUtf8Path(_)
            | crate::LkhError::Canonicalize { .. }
            | crate::LkhError::WriteFile { .. }
            | crate::LkhError::CString { .. }
            | crate::LkhError::InvalidProblem(_)
            | crate::LkhError::InvalidSearchParameters(_)
            | crate::LkhError::UnsupportedProgrammaticParameter(_)
            | crate::LkhError::InMemoryInitialization(_) => PyValueError::new_err(err.to_string()),
            crate::LkhError::SolverLockPoisoned | crate::LkhError::MissingBestTour => {
                PyRuntimeError::new_err(err.to_string())
            }
        }
    }
}
