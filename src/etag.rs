use super::utils::PythonIoBase;
use pyo3::prelude::*;

pub(super) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(etag_of, m)?)?;
    Ok(())
}

#[pyfunction]
fn etag_of(io_base: &PyAny, py: Python<'_>) -> PyResult<String> {
    let etag = qiniu_sdk::etag::etag_of(PythonIoBase::new(io_base, py))?;
    Ok(etag)
}
