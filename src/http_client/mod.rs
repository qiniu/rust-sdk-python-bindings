use pyo3::prelude::*;

mod client;
mod region;

pub(super) use client::{
    Authorization, Backoff, Chooser, HttpClient, Idempotent, JsonResponse, RequestRetrier, Resolver,
};
pub(super) use region::{Endpoint, ServiceName};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "http_client")?;
    region::register(py, m)?;
    client::register(py, m)?;
    Ok(m)
}
