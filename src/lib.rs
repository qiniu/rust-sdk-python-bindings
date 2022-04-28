mod credential;
mod etag;
mod exceptions;
mod upload_token;
mod utils;

use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "qiniu_sdk_bindings")]
fn qiniu_sdk_bindings(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    pyo3_log::try_init().ok();
    exceptions::register(py, m)?;

    m.add_submodule(etag::create_module(py)?)?;
    m.add_submodule(credential::create_module(py)?)?;
    m.add_submodule(upload_token::create_module(py)?)?;

    Ok(())
}
