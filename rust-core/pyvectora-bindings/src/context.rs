use pyo3::prelude::*;
use tokio_util::sync::CancellationToken;

#[pyclass]
pub struct PyExecutionContext {
    pub(crate) token: CancellationToken,
}

#[pymethods]
impl PyExecutionContext {
    /// Check if the request has been cancelled
    fn cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    /// Raise an exception if cancelled
    fn raise_if_cancelled(&self) -> PyResult<()> {
        if self.cancelled() {
            return Err(pyo3::exceptions::PyConnectionAbortedError::new_err(
                "Request cancelled",
            ));
        }
        Ok(())
    }
}

impl PyExecutionContext {
    pub fn new(token: CancellationToken) -> Self {
        Self { token }
    }
}
