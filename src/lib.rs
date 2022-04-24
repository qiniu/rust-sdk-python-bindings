mod etag;
mod utils;

use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "qiniu_rust_bindings")]
fn qiniu_rust_bindings(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    etag::register(py, m)?;

    Ok(())
}
