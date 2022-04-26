use super::{
    exceptions::QiniuEmptyChainCredentialsProvider,
    utils::{parse_header_value, parse_headers, parse_method, parse_uri, PythonIoBase},
};
use pyo3::prelude::*;
use std::{collections::HashMap, time::Duration};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "credential")?;
    m.add_class::<Credential>()?;
    m.add_class::<CredentialProvider>()?;
    m.add_class::<GlobalCredentialProvider>()?;
    m.add_class::<EnvCredentialProvider>()?;
    m.add_class::<ChainCredentialsProvider>()?;
    m.add_class::<GetOptions>()?;
    Ok(m)
}

#[pyclass(extends = CredentialProvider)]
#[derive(Debug, Clone)]
struct Credential;

#[pymethods]
impl Credential {
    #[new]
    fn new(access_key: String, secret_key: String) -> (Self, CredentialProvider) {
        (
            Self,
            CredentialProvider(Box::new(qiniu_sdk::credential::Credential::new(
                access_key, secret_key,
            ))),
        )
    }

    fn __repr__(self_: PyRef<'_, Self>) -> String {
        let super_ = self_.as_ref();
        format!("{:?}", super_)
    }

    fn __str__(self_: PyRef<'_, Self>) -> String {
        Self::__repr__(self_)
    }

    #[pyo3(text_signature = "($self)")]
    fn access_key(self_: PyRef<'_, Self>) -> PyResult<String> {
        let super_ = self_.as_ref();
        Ok(super_.0.get(Default::default())?.access_key().to_string())
    }

    #[pyo3(text_signature = "($self)")]
    fn secret_key(self_: PyRef<'_, Self>) -> PyResult<String> {
        let super_ = self_.as_ref();
        Ok(super_.0.get(Default::default())?.secret_key().to_string())
    }

    #[pyo3(text_signature = "($self, data)")]
    fn sign(self_: PyRef<'_, Self>, data: Vec<u8>) -> PyResult<String> {
        let super_ = self_.as_ref();
        Ok(super_.0.get(Default::default())?.sign(&data))
    }

    #[pyo3(text_signature = "($self, io_base)")]
    fn sign_reader(self_: PyRef<'_, Self>, io_base: PyObject) -> PyResult<String> {
        let super_ = self_.as_ref();
        let signature = super_
            .0
            .get(Default::default())?
            .sign_reader(&mut PythonIoBase::new(io_base))?;
        Ok(signature)
    }

    #[pyo3(text_signature = "($self, io_base)")]
    fn sign_async_reader<'p>(
        self_: PyRef<'p, Self>,
        io_base: PyObject,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let super_ = self_.as_ref();
        let credential = super_.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let signature = credential
                .async_get(Default::default())
                .await?
                .sign_async_reader(&mut PythonIoBase::new(io_base).into_async_read())
                .await?;
            Ok(signature)
        })
    }

    #[pyo3(text_signature = "($self, url, secs)")]
    fn sign_download_url(self_: PyRef<'_, Self>, url: &str, secs: u64) -> PyResult<String> {
        let super_ = self_.as_ref();
        Ok(super_
            .0
            .get(Default::default())?
            .sign_download_url(parse_uri(url)?, Duration::from_secs(secs))
            .to_string())
    }

    #[pyo3(text_signature = "($self, url, content_type, body)")]
    fn authorization_v1_for_request(
        self_: PyRef<'_, Self>,
        url: &str,
        content_type: Option<&str>,
        body: &[u8],
    ) -> PyResult<String> {
        let super_ = self_.as_ref();
        let url = parse_uri(url)?;
        let content_type = parse_header_value(content_type)?;
        Ok(super_
            .0
            .get(Default::default())?
            .authorization_v1_for_request(&url, content_type.as_ref(), body))
    }

    #[pyo3(text_signature = "($self, url, content_type, body)")]
    fn authorization_v1_for_request_with_body_reader(
        self_: PyRef<'_, Self>,
        url: &str,
        content_type: Option<&str>,
        body: PyObject,
    ) -> PyResult<String> {
        let super_ = self_.as_ref();
        let url = parse_uri(url)?;
        let content_type = parse_header_value(content_type)?;
        let auth = super_
            .0
            .get(Default::default())?
            .authorization_v1_for_request_with_body_reader(
                &url,
                content_type.as_ref(),
                &mut PythonIoBase::new(body),
            )?;
        Ok(auth)
    }

    #[pyo3(text_signature = "($self, url, content_type, body)")]
    fn authorization_v1_for_request_with_async_body_reader<'p>(
        self_: PyRef<'_, Self>,
        url: &str,
        content_type: Option<&str>,
        body: PyObject,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let super_ = self_.as_ref();
        let url = parse_uri(url)?;
        let content_type = parse_header_value(content_type)?;
        let credential = super_.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let auth = credential
                .async_get(Default::default())
                .await?
                .authorization_v1_for_request_with_async_body_reader(
                    &url,
                    content_type.as_ref(),
                    &mut PythonIoBase::new(body).into_async_read(),
                )
                .await?;
            Ok(auth)
        })
    }

    #[pyo3(text_signature = "($self, method, url, headers, body)")]
    fn authorization_v2_for_request(
        self_: PyRef<'_, Self>,
        method: &str,
        url: &str,
        headers: HashMap<String, String>,
        body: &[u8],
    ) -> PyResult<String> {
        let super_ = self_.as_ref();
        let method = parse_method(method)?;
        let url = parse_uri(url)?;
        let headers = parse_headers(headers)?;
        Ok(super_
            .0
            .get(Default::default())?
            .authorization_v2_for_request(&method, &url, &headers, body))
    }

    #[pyo3(text_signature = "($self, method, url, headers, body)")]
    fn authorization_v2_for_request_with_body_reader(
        self_: PyRef<'_, Self>,
        method: &str,
        url: &str,
        headers: HashMap<String, String>,
        body: PyObject,
    ) -> PyResult<String> {
        let super_ = self_.as_ref();
        let method = parse_method(method)?;
        let url = parse_uri(url)?;
        let headers = parse_headers(headers)?;
        let auth = super_
            .0
            .get(Default::default())?
            .authorization_v2_for_request_with_body_reader(
                &method,
                &url,
                &headers,
                &mut PythonIoBase::new(body),
            )?;
        Ok(auth)
    }

    #[pyo3(text_signature = "($self, method, url, headers, body)")]
    fn authorization_v2_for_request_with_async_body_reader<'p>(
        self_: PyRef<'_, Self>,
        method: &str,
        url: &str,
        headers: HashMap<String, String>,
        body: PyObject,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let super_ = self_.as_ref();
        let method = parse_method(method)?;
        let url = parse_uri(url)?;
        let headers = parse_headers(headers)?;
        let credential = super_.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let auth = credential
                .async_get(Default::default())
                .await?
                .authorization_v2_for_request_with_async_body_reader(
                    &method,
                    &url,
                    &headers,
                    &mut PythonIoBase::new(body).into_async_read(),
                )
                .await?;
            Ok(auth)
        })
    }
}

#[pyclass(subclass)]
#[derive(Debug, Clone)]
pub(super) struct CredentialProvider(Box<dyn qiniu_sdk::credential::CredentialProvider>);

#[pymethods]
impl CredentialProvider {
    #[args(opts = "GetOptions::default()")]
    #[pyo3(text_signature = "($self, opts)")]
    fn get(&self, opts: GetOptions, py: Python<'_>) -> PyResult<Py<Credential>> {
        Py::new(
            py,
            (
                Credential,
                CredentialProvider(Box::new(self.0.get(opts.0)?.into_credential())),
            ),
        )
    }

    #[args(opts = "GetOptions::default()")]
    #[pyo3(text_signature = "($self, opts)")]
    fn async_get<'p>(&self, opts: GetOptions, py: Python<'p>) -> PyResult<&'p PyAny> {
        let credential = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let py_initializer = (
                Credential,
                CredentialProvider(Box::new(
                    credential.async_get(opts.0).await?.into_credential(),
                )),
            );
            Python::with_gil(|py| Py::new(py, py_initializer))
        })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl CredentialProvider {
    pub(super) fn into_inner(self) -> Box<dyn qiniu_sdk::credential::CredentialProvider> {
        self.0
    }
}

#[pyclass(extends = CredentialProvider)]
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
    fn setup(credential: PyRef<'_, Credential>) -> PyResult<()> {
        qiniu_sdk::credential::GlobalCredentialProvider::setup(
            credential.into_super().0.get(Default::default())?.into(),
        );
        Ok(())
    }

    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn clear() {
        qiniu_sdk::credential::GlobalCredentialProvider::clear();
    }
}

#[pyclass(extends = CredentialProvider)]
struct EnvCredentialProvider;

#[pymethods]
impl EnvCredentialProvider {
    #[new]
    fn new() -> (Self, CredentialProvider) {
        (
            Self,
            CredentialProvider(Box::new(qiniu_sdk::credential::EnvCredentialProvider)),
        )
    }

    #[staticmethod]
    #[pyo3(text_signature = "(credential)")]
    fn setup(credential: PyRef<'_, Credential>) -> PyResult<()> {
        qiniu_sdk::credential::EnvCredentialProvider::setup(
            &credential.into_super().0.get(Default::default())?.into(),
        );
        Ok(())
    }

    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn clear() {
        qiniu_sdk::credential::EnvCredentialProvider::clear();
    }
}

#[pyclass(extends = CredentialProvider)]
#[derive(Debug, Copy, Clone, Default)]
struct ChainCredentialsProvider;

#[pymethods]
impl ChainCredentialsProvider {
    #[new]
    fn new(creds: Vec<CredentialProvider>) -> PyResult<(Self, CredentialProvider)> {
        let mut builder: Option<qiniu_sdk::credential::ChainCredentialsProviderBuilder> = None;
        for cred in creds {
            if let Some(builder) = &mut builder {
                builder.append_credential(cred.0);
            } else {
                builder = Some(qiniu_sdk::credential::ChainCredentialsProvider::builder(
                    cred.0,
                ));
            }
        }
        if let Some(builder) = &mut builder {
            Ok((Self, CredentialProvider(Box::new(builder.build()))))
        } else {
            Err(QiniuEmptyChainCredentialsProvider::new_err(
                "creds is empty",
            ))
        }
    }
}

#[pyclass]
#[derive(Default, Copy, Clone)]
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
