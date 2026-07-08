use std::path::PathBuf;

/// Errors returned by the safe Rust layer before control reaches LKH's C core.
#[derive(Debug, thiserror::Error)]
pub enum LkhError {
    #[error("parameter file does not exist: {0}")]
    ParameterFileNotFound(PathBuf),

    #[error("parameter file path is not valid UTF-8: {0:?}")]
    NonUtf8Path(PathBuf),

    #[error("failed to canonicalize {path}: {source}")]
    Canonicalize {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to build C string for {context}: {source}")]
    CString {
        context: &'static str,
        #[source]
        source: std::ffi::NulError,
    },

    #[error("the LKH solver lock is poisoned")]
    SolverLockPoisoned,

    #[error("LKH returned no best tour")]
    MissingBestTour,
}
