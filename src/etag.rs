use super::utils::PythonIoBase;
use pyo3::prelude::*;

pub(super) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(etag_of, m)?)?;
    m.add_function(wrap_pyfunction!(async_etag_of, m)?)?;
    Ok(())
}

#[pyfunction]
fn etag_of(io_base: PyObject) -> PyResult<String> {
    let etag = qiniu_sdk::etag::etag_of(PythonIoBase::new(io_base))?;
    Ok(etag)
}

#[pyfunction]
fn async_etag_of(io_base: PyObject, py: Python<'_>) -> PyResult<&PyAny> {
    pyo3_asyncio::async_std::future_into_py(py, async move {
        let etag =
            qiniu_sdk::etag::async_etag_of(PythonIoBase::new(io_base).into_async_read()).await?;
        Ok(etag)
    })
}
