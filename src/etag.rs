use super::utils::PythonIoBase;
use pyo3::{prelude::*, types::PyList};

pub(super) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(etag_of, m)?)?;
    m.add_function(wrap_pyfunction!(etag_with_parts, m)?)?;
    m.add_function(wrap_pyfunction!(async_etag_of, m)?)?;
    m.add_function(wrap_pyfunction!(async_etag_with_parts, m)?)?;
    Ok(())
}

#[pyfunction]
fn etag_of(io_base: PyObject, _py: Python<'_>) -> PyResult<String> {
    let etag = qiniu_sdk::etag::etag_of(PythonIoBase::new(io_base))?;
    Ok(etag)
}

#[pyfunction]
fn etag_with_parts(io_base: PyObject, parts: Py<PyList>, py: Python<'_>) -> PyResult<String> {
    let parts = convert_py_list_to_usize_vec(parts, py)?;
    let etag = qiniu_sdk::etag::etag_with_parts(PythonIoBase::new(io_base), &parts)?;
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

#[pyfunction]
fn async_etag_with_parts(io_base: PyObject, parts: Py<PyList>, py: Python<'_>) -> PyResult<&PyAny> {
    let parts = convert_py_list_to_usize_vec(parts, py)?;
    pyo3_asyncio::async_std::future_into_py(py, async move {
        let etag = qiniu_sdk::etag::async_etag_with_parts(
            PythonIoBase::new(io_base).into_async_read(),
            &parts,
        )
        .await?;
        Ok(etag)
    })
}

fn convert_py_list_to_usize_vec(parts: Py<PyList>, py: Python<'_>) -> PyResult<Vec<usize>> {
    parts
        .as_ref(py)
        .iter()
        .map(|pyval| pyval.extract::<usize>())
        .collect()
}
