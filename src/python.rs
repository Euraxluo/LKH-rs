use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

#[pymodule]
mod _native {
    use super::*;

    #[pyfunction]
    fn solve_parameter_file(py: Python<'_>, path: &str) -> PyResult<Py<PyDict>> {
        let report = crate::solve_parameter_file(path).map_err(|err| match err {
            crate::LkhError::ParameterFileNotFound(_)
            | crate::LkhError::NonUtf8Path(_)
            | crate::LkhError::Canonicalize { .. }
            | crate::LkhError::CString { .. } => PyValueError::new_err(err.to_string()),
            crate::LkhError::SolverLockPoisoned | crate::LkhError::MissingBestTour => {
                PyRuntimeError::new_err(err.to_string())
            }
        })?;

        let dict = PyDict::new(py);
        dict.set_item("best_cost", report.best_cost)?;
        dict.set_item("best_penalty", report.best_penalty)?;
        dict.set_item("runs", report.runs)?;
        dict.set_item("dimension", report.dimension)?;
        dict.set_item("tour", report.tour)?;
        Ok(dict.into())
    }
}
