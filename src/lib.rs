mod credential;
mod etag;
mod utils;

use pyo3::prelude::*;

#[pymodule]
#[pyo3(name = "qiniu_sdk_bindings")]
fn qiniu_sdk_bindings(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    pyo3_log::try_init().ok();
    m.add_submodule(etag::create_module(py)?)?;
    m.add_submodule(credential::create_module(py)?)?;

    Ok(())
}
