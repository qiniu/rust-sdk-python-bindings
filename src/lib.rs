mod credential;
mod etag;
mod utils;

use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "qiniu_sdk_python_bindings")]
fn qiniu_sdk_python_bindings(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    etag::register(py, m)?;
    credential::register(py, m)?;

    Ok(())
}
