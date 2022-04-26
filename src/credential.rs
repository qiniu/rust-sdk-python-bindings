use std::time::Duration;

use super::utils::PythonIoBase;
use pyo3::{
    exceptions::{PyIOError, PyValueError},
    prelude::*,
};
use qiniu_sdk::credential::Uri;

pub(super) fn create_module<'p>(py: Python<'p>) -> PyResult<&'p PyModule> {
    let m = PyModule::new(py, "credential")?;
    m.add_class::<Credential>()?;
    m.add_class::<CredentialProvider>()?;
    m.add_class::<GlobalCredentialProvider>()?;
    m.add_class::<GetOptions>()?;
    Ok(m)
}

#[pyclass]
#[derive(Debug, Clone)]
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

    #[pyo3(text_signature = "($self)")]
    fn access_key(&self) -> String {
        self.0.access_key().to_string()
    }

    #[pyo3(text_signature = "($self)")]
    fn secret_key(&self) -> String {
        self.0.secret_key().to_string()
    }

    #[pyo3(text_signature = "($self, data)")]
    fn sign(&self, data: Vec<u8>) -> String {
        self.0.sign(&data)
    }

    #[pyo3(text_signature = "($self, io_base)")]
    fn sign_reader(&self, io_base: PyObject) -> PyResult<String> {
        self.0
            .sign_reader(&mut PythonIoBase::new(io_base))
            .map_err(PyIOError::new_err)
    }

    #[pyo3(text_signature = "($self, io_base)")]
    fn sign_async_reader<'p>(&self, io_base: PyObject, py: Python<'p>) -> PyResult<&'p PyAny> {
        let credential = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            credential
                .sign_async_reader(&mut PythonIoBase::new(io_base).into_async_read())
                .await
                .map_err(PyIOError::new_err)
        })
    }

    #[pyo3(text_signature = "($self, url, secs)")]
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

#[pyclass(subclass)]
#[derive(Debug, Clone)]
struct CredentialProvider(Box<dyn qiniu_sdk::credential::CredentialProvider>);

#[pymethods]
impl CredentialProvider {
    #[args(opts = "GetOptions::default()")]
    #[pyo3(text_signature = "($self, opts)")]
    fn get(&self, opts: GetOptions) -> PyResult<Credential> {
        Ok(Credential(self.0.get(opts.0)?.into()))
    }

    #[args(opts = "GetOptions::default()")]
    #[pyo3(text_signature = "($self, opts)")]
    fn async_get<'p>(&self, opts: GetOptions, py: Python<'p>) -> PyResult<&'p PyAny> {
        let credential = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            Ok(Credential(credential.async_get(opts.0).await?.into()))
        })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

#[pyclass(extends = CredentialProvider)]
#[derive(Debug, Copy, Clone, Default)]
struct GlobalCredentialProvider;

#[pymethods]
impl GlobalCredentialProvider {
    #[new]
    fn new() -> (Self, CredentialProvider) {
        (
            Self,
            CredentialProvider(Box::new(qiniu_sdk::credential::GlobalCredentialProvider)),
        )
    }

    #[staticmethod]
    #[pyo3(text_signature = "(credential)")]
    fn setup(credential: Credential) {
        qiniu_sdk::credential::GlobalCredentialProvider::setup(credential.0);
    }

    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn clear() {
        qiniu_sdk::credential::GlobalCredentialProvider::clear();
    }
}

#[pyclass]
#[derive(Debug, Copy, Clone, Default)]
struct GetOptions(qiniu_sdk::credential::GetOptions);

#[pymethods]
impl GetOptions {
    #[new]
    fn new() -> Self {
        Default::default()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}
