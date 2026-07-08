//! Private PyO3 bridge for the public Python facade.
//!
//! `python/lkh_rs/__init__.py` owns the user-facing enums and builders. This
//! module accepts already-normalized dictionaries so the native layer stays
//! small and mirrors the safe Rust API.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::Bound;

#[pymodule]
mod _native {
    use super::*;

    #[pyfunction]
    fn solve_parameter_file(py: Python<'_>, path: &str) -> PyResult<Py<PyDict>> {
        let path = path.to_owned();
        let report = py
            .allow_threads(move || crate::solve_parameter_file(path))
            .map_err(to_py_error)?;
        report_to_dict(py, report)
    }

    #[pyfunction]
    fn _solve_problem_data(
        py: Python<'_>,
        problem: Bound<'_, PyDict>,
        parameters: Bound<'_, PyDict>,
    ) -> PyResult<Py<PyDict>> {
        let problem = routing_problem_from_dict(problem)?;
        let parameters = search_parameters_from_dict(parameters)?;
        let report = py
            .allow_threads(move || crate::solve_problem(&problem, &parameters))
            .map_err(to_py_error)?;
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

    fn routing_problem_from_dict(input: Bound<'_, PyDict>) -> PyResult<crate::RoutingProblem> {
        // Python preserves insertion order for dictionaries, so iterating these
        // mappings keeps raw sections in the order chosen by the public facade.
        let kind = required_item::<String>(&input, "kind")?;
        let dimension = required_item::<usize>(&input, "dimension")?;
        let name =
            optional_item::<String>(&input, "name")?.unwrap_or_else(|| "lkh_rs_problem".to_owned());
        let mut routing_problem =
            crate::RoutingProblem::custom_type(name, kind, dimension).map_err(to_py_error)?;
        if let Some(keywords) = optional_dict(&input, "keywords")? {
            for (key, value) in keywords.iter() {
                let key = key.extract::<String>()?;
                let value = value.extract::<String>()?;
                routing_problem = routing_problem
                    .with_keyword(key, value)
                    .map_err(to_py_error)?;
            }
        }
        if let Some(sections) = optional_dict(&input, "sections")? {
            for (key, lines) in sections.iter() {
                let key = key.extract::<String>()?;
                let lines = lines.extract::<Vec<String>>()?;
                routing_problem = routing_problem
                    .with_section(key, lines)
                    .map_err(to_py_error)?;
            }
        }
        Ok(routing_problem)
    }

    fn search_parameters_from_dict(input: Bound<'_, PyDict>) -> PyResult<crate::SearchParameters> {
        // The Python facade performs early validation. We still validate the
        // Rust value before handing it to LKH so direct bridge calls fail
        // consistently.
        let mut parameters = crate::SearchParameters::new()
            .with_runs(optional_item::<i32>(&input, "runs")?.unwrap_or(1))
            .with_trace_level(optional_item::<i32>(&input, "trace_level")?.unwrap_or(0));
        parameters.max_trials = optional_item(&input, "max_trials")?;
        parameters.seed = optional_item(&input, "seed")?;
        parameters.time_limit = optional_item(&input, "time_limit")?;
        parameters.total_time_limit = optional_item(&input, "total_time_limit")?;
        parameters.validate().map_err(to_py_error)?;
        Ok(parameters)
    }

    fn required_item<T>(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<T>
    where
        for<'a> T: FromPyObject<'a>,
    {
        dict.get_item(key)?
            .ok_or_else(|| PyValueError::new_err(format!("missing problem field: {key}")))?
            .extract()
    }

    fn optional_item<T>(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<T>>
    where
        for<'a> T: FromPyObject<'a>,
    {
        dict.get_item(key)?.map(|value| value.extract()).transpose()
    }

    fn optional_dict<'py>(
        dict: &Bound<'py, PyDict>,
        key: &str,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        Ok(dict
            .get_item(key)?
            .map(|value| value.downcast_into::<PyDict>())
            .transpose()?)
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
