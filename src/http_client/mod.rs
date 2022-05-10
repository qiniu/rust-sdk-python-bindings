use pyo3::prelude::*;

mod region;

pub(super) use region::Endpoint;

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "http_client")?;
    region::register(py, m)?;
    Ok(m)
}
