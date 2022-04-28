use super::{
    exceptions::{
        QiniuEmptyChainCredentialsProvider, QiniuInvalidHeaderName, QiniuInvalidHeaderValue,
        QiniuInvalidURLError,
    },
    utils::PythonIoBase,
};
use pyo3::{exceptions::PyValueError, prelude::*};
use qiniu_sdk::credential::{HeaderMap, HeaderName, HeaderValue, Method, Uri};
use std::{collections::HashMap, time::Duration};

pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
    let m = PyModule::new(py, "credential")?;
    m.add_class::<Credential>()?;
    m.add_class::<CredentialProvider>()?;
    m.add_class::<GlobalCredentialProvider>()?;
    m.add_class::<EnvCredentialProvider>()?;
    m.add_class::<StaticCredentialProvider>()?;
    m.add_class::<ChainCredentialsProvider>()?;
    m.add_class::<GetOptions>()?;
    Ok(m)
}

#[pyclass]
#[derive(Clone)]
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
        let signature = self.0.sign_reader(&mut PythonIoBase::new(io_base))?;
        Ok(signature)
    }

    #[pyo3(text_signature = "($self, io_base)")]
    fn sign_async_reader<'p>(&self, io_base: PyObject, py: Python<'p>) -> PyResult<&'p PyAny> {
        let credential = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let signature = credential
                .sign_async_reader(&mut PythonIoBase::new(io_base).into_async_read())
                .await?;
            Ok(signature)
        })
    }

    #[pyo3(text_signature = "($self, url, secs)")]
    fn sign_download_url(&self, url: &str, secs: u64) -> PyResult<String> {
        Ok(self
            .0
            .sign_download_url(Self::parse_uri(url)?, Duration::from_secs(secs))
            .to_string())
    }

    #[pyo3(text_signature = "($self, url, content_type, body)")]
    fn authorization_v1_for_request(
        &self,
        url: &str,
        content_type: Option<&str>,
        body: &[u8],
    ) -> PyResult<String> {
        let url = Self::parse_uri(url)?;
        let content_type = Self::parse_header_value(content_type)?;
        Ok(self
            .0
            .authorization_v1_for_request(&url, content_type.as_ref(), body))
    }

    #[pyo3(text_signature = "($self, url, content_type, body)")]
    fn authorization_v1_for_request_with_body_reader(
        &self,
        url: &str,
        content_type: Option<&str>,
        body: PyObject,
    ) -> PyResult<String> {
        let url = Self::parse_uri(url)?;
        let content_type = Self::parse_header_value(content_type)?;
        let auth = self.0.authorization_v1_for_request_with_body_reader(
            &url,
            content_type.as_ref(),
            &mut PythonIoBase::new(body),
        )?;
        Ok(auth)
    }

    #[pyo3(text_signature = "($self, url, content_type, body)")]
    fn authorization_v1_for_request_with_async_body_reader<'p>(
        &self,
        url: &str,
        content_type: Option<&str>,
        body: PyObject,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let url = Self::parse_uri(url)?;
        let content_type = Self::parse_header_value(content_type)?;
        let credential = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let auth = credential
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
        &self,
        method: &str,
        url: &str,
        headers: HashMap<String, String>,
        body: &[u8],
    ) -> PyResult<String> {
        let method = Self::parse_method(method)?;
        let url = Self::parse_uri(url)?;
        let headers = Self::parse_headers(headers)?;
        Ok(self
            .0
            .authorization_v2_for_request(&method, &url, &headers, body))
    }

    #[pyo3(text_signature = "($self, method, url, headers, body)")]
    fn authorization_v2_for_request_with_body_reader(
        &self,
        method: &str,
        url: &str,
        headers: HashMap<String, String>,
        body: PyObject,
    ) -> PyResult<String> {
        let method = Self::parse_method(method)?;
        let url = Self::parse_uri(url)?;
        let headers = Self::parse_headers(headers)?;
        let auth = self.0.authorization_v2_for_request_with_body_reader(
            &method,
            &url,
            &headers,
            &mut PythonIoBase::new(body),
        )?;
        Ok(auth)
    }

    #[pyo3(text_signature = "($self, method, url, headers, body)")]
    fn authorization_v2_for_request_with_async_body_reader<'p>(
        &self,
        method: &str,
        url: &str,
        headers: HashMap<String, String>,
        body: PyObject,
        py: Python<'p>,
    ) -> PyResult<&'p PyAny> {
        let method = Self::parse_method(method)?;
        let url = Self::parse_uri(url)?;
        let headers = Self::parse_headers(headers)?;
        let credential = self.0.to_owned();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let auth = credential
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

impl Credential {
    fn parse_uri(url: &str) -> PyResult<Uri> {
        let url = url
            .parse::<Uri>()
            .map_err(|err| QiniuInvalidURLError::new_err(err.to_string()))?;
        Ok(url)
    }

    fn parse_method(method: &str) -> PyResult<Method> {
        let method = method
            .parse::<Method>()
            .map_err(|err| PyValueError::new_err(err.to_string()))?;
        Ok(method)
    }

    fn parse_headers(headers: HashMap<String, String>) -> PyResult<HeaderMap> {
        headers
            .into_iter()
            .map(|(name, value)| {
                let name = name
                    .parse::<HeaderName>()
                    .map_err(|err| QiniuInvalidHeaderName::new_err(err.to_string()))?;
                let value = value
                    .parse::<HeaderValue>()
                    .map_err(|err| QiniuInvalidHeaderValue::new_err(err.to_string()))?;
                Ok((name, value))
            })
            .collect()
    }

    fn parse_header_value(header_value: Option<&str>) -> PyResult<Option<HeaderValue>> {
        if let Some(header_value) = header_value {
            let header_value = header_value
                .parse::<HeaderValue>()
                .map_err(|err| QiniuInvalidHeaderValue::new_err(err.to_string()))?;
            Ok(Some(header_value))
        } else {
            Ok(None)
        }
    }
}

#[pyclass(subclass)]
#[derive(Clone)]
pub(super) struct CredentialProvider(Box<dyn qiniu_sdk::credential::CredentialProvider>);

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
    fn setup(credential: Credential) {
        qiniu_sdk::credential::GlobalCredentialProvider::setup(credential.0);
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
    fn setup(credential: &Credential) {
        qiniu_sdk::credential::EnvCredentialProvider::setup(&credential.0);
    }

    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn clear() {
        qiniu_sdk::credential::EnvCredentialProvider::clear();
    }
}

#[pyclass(extends = CredentialProvider)]
struct StaticCredentialProvider;

#[pymethods]
impl StaticCredentialProvider {
    #[new]
    fn new(cred: Credential) -> (Self, CredentialProvider) {
        (Self, CredentialProvider(Box::new(cred.0)))
    }
}

#[pyclass(extends = CredentialProvider)]
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
