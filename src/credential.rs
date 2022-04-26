use std::time::Duration;

use super::utils::PythonIoBase;
use pyo3::{
    exceptions::{PyIOError, PyValueError},
    prelude::*,
};
use qiniu_sdk::credential::Uri;

pub(super) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Credential>()?;
    Ok(())
}

#[pyclass]
struct Credential(qiniu_sdk::credential::Credential);

#[pymethods]
impl Credential {
    #[new]
    fn new(access_key: String, secret_key: String) -> Self {
        Self(qiniu_sdk::credential::Credential::new(
            access_key, secret_key,
        ))
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn access_key(&self) -> String {
        self.0.access_key().to_string()
    }

    fn secret_key(&self) -> String {
        self.0.secret_key().to_string()
    }

    fn sign(&self, data: Vec<u8>) -> String {
        self.0.sign(&data)
    }

    fn sign_reader(&self, io_base: PyObject) -> PyResult<String> {
        self.0
            .sign_reader(&mut PythonIoBase::new(io_base))
            .map_err(PyIOError::new_err)
    }

    fn sign_async_reader<'p>(&self, io_base: PyObject, py: Python<'p>) -> PyResult<&'p PyAny> {
        let credential = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            credential
                .sign_async_reader(&mut PythonIoBase::new(io_base).into_async_read())
                .await
                .map_err(PyIOError::new_err)
        })
    }

    fn sign_download_url(&self, url: String, secs: u64) -> PyResult<String> {
        let url = url
            .parse::<Uri>()
            .map_err(|err| PyValueError::new_err(err.to_string()))?;
        Ok(self
            .0
            .sign_download_url(url, Duration::from_secs(secs))
            .to_string())
    }
}
